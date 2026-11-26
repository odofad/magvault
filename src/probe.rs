//! Probe a card (or folder): walk files and sidecars, classify them, ffprobe the media,
//! and guess camera / shoot day for the operator to confirm (PLAN.md "Ingest flow" 1-3).

use crate::error::{Context, Result};
use crate::profiles::{glob_match, CameraProfile};
use crate::time;
use crate::util::esc;
use std::collections::HashMap;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Junk that is never copied (NOTES.md: "only skip junk like AppleDouble"; ingest mockup
/// marks `.DS_Store` as skipped). Names are matched exactly; `._*` is AppleDouble.
const JUNK_NAMES: &[&str] = &[".DS_Store", ".Spotlight-V100", ".Trashes", ".fseventsd", ".TemporaryItems"];

/// Extensions treated as media (one media file = one clip in v1). Everything else is a
/// sidecar (same folder, same stem as a media file) or "other".
const MEDIA_EXT: &[&str] = &[
    "mov", "mp4", "m4v", "mxf", "avi", "mts", "m2ts", "mkv", "r3d", "braw", "crm", "ari", "insv", "360",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role {
    Media,
    Sidecar,
    Other,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Media => "media",
            Role::Sidecar => "sidecar",
            Role::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbedFile {
    pub rel: String,
    pub abs: PathBuf,
    pub size: u64,
    pub mtime: i64,
    pub role: Role,
    /// For media: index into `Probe::clips`. For sidecars: the clip they belong to.
    pub clip: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct MediaInfo {
    pub duration_ms: Option<i64>,
    pub fps: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub audio_streams: Option<i64>,
    pub audio_channels: Option<String>,
    pub creation_time: Option<i64>,
    pub probe_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Clip {
    pub file: usize,
    pub info: MediaInfo,
}

#[derive(Debug)]
pub struct Probe {
    pub root: PathBuf,
    pub files: Vec<ProbedFile>,
    pub clips: Vec<Clip>,
    pub skipped: Vec<String>,
    pub total_bytes: u64,
}

fn is_junk(name: &str) -> bool {
    JUNK_NAMES.contains(&name) || name.starts_with("._")
}

fn ext_lower(name: &str) -> String {
    Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn stem_lower(name: &str) -> String {
    Path::new(name).file_stem().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Walk the source tree. Refuses symlinks, devices, FIFOs, sockets, non-UTF-8 names and
/// names with `|` or control characters (they cannot be written to the text cards).
pub fn walk(root: &Path) -> Result<(Vec<ProbedFile>, Vec<String>)> {
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut problems = Vec::new();
    walk_dir(root, root, &mut files, &mut skipped, &mut problems)?;
    if !problems.is_empty() {
        let shown: Vec<String> = problems.iter().take(20).cloned().collect();
        crate::bail!(
            "the source contains {} item(s) that cannot be ingested:\n  {}{}",
            problems.len(),
            shown.join("\n  "),
            if problems.len() > 20 { "\n  …" } else { "" }
        );
    }
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok((files, skipped))
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    files: &mut Vec<ProbedFile>,
    skipped: &mut Vec<String>,
    problems: &mut Vec<String>,
) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ctx(format!("cannot read {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .ctx(format!("cannot read {}", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let rel_path = path.strip_prefix(root).unwrap_or(&path);
        let Some(rel) = rel_path.to_str().map(str::to_string) else {
            problems.push(format!("{} (name is not valid UTF-8)", esc(&rel_path.to_string_lossy())));
            continue;
        };
        let name = e.file_name().to_string_lossy().into_owned();
        if is_junk(&name) {
            skipped.push(rel);
            continue;
        }
        if name.contains('|') || name.chars().any(char::is_control) {
            problems.push(format!("{} (name contains '|' or a control character)", esc(&rel)));
            continue;
        }
        let ft = e.file_type().ctx(format!("cannot stat {}", path.display()))?;
        if ft.is_symlink() {
            problems.push(format!("{} (symbolic link)", esc(&rel)));
        } else if ft.is_dir() {
            walk_dir(root, &path, files, skipped, problems)?;
        } else if ft.is_file() {
            let md = e.metadata().ctx(format!("cannot stat {}", path.display()))?;
            let mtime = md.modified().map(time::system_time_secs).unwrap_or(0);
            files.push(ProbedFile { rel, abs: path, size: md.len(), mtime, role: Role::Other, clip: None });
        } else if ft.is_block_device() || ft.is_char_device() || ft.is_fifo() || ft.is_socket() {
            problems.push(format!("{} (device, FIFO or socket)", esc(&rel)));
        }
    }
    Ok(())
}

/// Classify files: media by extension; sidecars share folder and stem with a media file.
pub fn classify(files: &mut [ProbedFile]) -> Vec<Clip> {
    let mut clips = Vec::new();
    let mut by_key: HashMap<(String, String), usize> = HashMap::new();
    for (i, f) in files.iter_mut().enumerate() {
        let name = Path::new(&f.rel).file_name().unwrap().to_string_lossy().into_owned();
        if MEDIA_EXT.contains(&ext_lower(&name).as_str()) {
            f.role = Role::Media;
            f.clip = Some(clips.len());
            let parent = Path::new(&f.rel).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            by_key.entry((parent, stem_lower(&name))).or_insert(clips.len());
            clips.push(Clip { file: i, info: MediaInfo::default() });
        }
    }
    for f in files.iter_mut().filter(|f| f.role != Role::Media) {
        let name = Path::new(&f.rel).file_name().unwrap().to_string_lossy().into_owned();
        let parent = Path::new(&f.rel).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(&c) = by_key.get(&(parent, stem_lower(&name))) {
            f.role = Role::Sidecar;
            f.clip = Some(c);
        }
    }
    clips
}

pub fn ffprobe_available() -> bool {
    Command::new("ffprobe").arg("-version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Run ffprobe on one file. Failures are recorded, not fatal (RAW formats may be unknown).
pub fn ffprobe(path: &Path) -> MediaInfo {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:format_tags=creation_time:stream=codec_type,width,height,r_frame_rate,channels",
            "-of",
            "flat",
        ])
        .arg(path)
        .env("LC_ALL", "C")
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => return MediaInfo { probe_error: Some(format!("ffprobe: {e}")), ..Default::default() },
    };
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("ffprobe failed").to_string();
        return MediaInfo { probe_error: Some(err), ..Default::default() };
    }
    parse_flat(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_flat(text: &str) -> MediaInfo {
    let mut kv: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            kv.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    let mut info = MediaInfo {
        duration_ms: kv.get("format.duration").and_then(|d| d.parse::<f64>().ok()).map(|d| (d * 1000.0).round() as i64),
        creation_time: kv.get("format.tags.creation_time").and_then(|t| time::parse_iso(t)),
        ..Default::default()
    };
    let mut audio: Vec<String> = Vec::new();
    for i in 0.. {
        let p = format!("streams.stream.{i}.");
        let Some(kind) = kv.get(&format!("{p}codec_type")) else { break };
        match kind.as_str() {
            "video" if info.width.is_none() => {
                info.width = kv.get(&format!("{p}width")).and_then(|v| v.parse().ok());
                info.height = kv.get(&format!("{p}height")).and_then(|v| v.parse().ok());
                info.fps = kv.get(&format!("{p}r_frame_rate")).filter(|v| v.as_str() != "0/0").cloned();
            }
            "audio" => audio.push(kv.get(&format!("{p}channels")).cloned().unwrap_or_else(|| "?".into())),
            _ => {}
        }
    }
    info.audio_streams = Some(audio.len() as i64);
    if !audio.is_empty() {
        info.audio_channels = Some(audio.join(","));
    }
    info
}

/// Probe a mag folder (Magvault's own `magcard.txt`, `ascmhl/`, `.magvault/` left out).
pub fn probe_mag(root: &Path) -> Result<Probe> {
    let (files, skipped) = walk(root)?;
    let mut files: Vec<ProbedFile> = files
        .into_iter()
        .filter(|f| {
            f.rel != "magcard.txt"
                && !f.rel.starts_with("ascmhl/")
                && !f.rel.starts_with(".magvault/")
        })
        .collect();
    let mut clips = classify(&mut files);
    if ffprobe_available() {
        for c in clips.iter_mut() {
            c.info = ffprobe(&files[c.file].abs);
        }
    }
    let total_bytes = files.iter().map(|f| f.size).sum();
    Ok(Probe {
        root: root.to_path_buf(),
        files,
        clips,
        skipped,
        total_bytes,
    })
}

pub fn probe(root: &Path, run_ffprobe: bool) -> Result<Probe> {
    let (mut files, skipped) = walk(root)?;
    let mut clips = classify(&mut files);
    if run_ffprobe {
        for c in clips.iter_mut() {
            c.info = ffprobe(&files[c.file].abs);
        }
    }
    let total_bytes = files.iter().map(|f| f.size).sum();
    Ok(Probe { root: root.to_path_buf(), files, clips, skipped, total_bytes })
}

/// Guess the camera profile: every `card_layout` folder present at the card root, and/or
/// `filename_pattern` matching a media file. Highest score wins; a tie means no guess.
pub fn guess_camera<'a>(probe: &Probe, cams: &'a [CameraProfile]) -> Option<&'a CameraProfile> {
    let top: Vec<String> = std::fs::read_dir(&probe.root)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default();
    let media_names: Vec<String> = probe
        .clips
        .iter()
        .map(|c| Path::new(&probe.files[c.file].rel).file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let mut scored: Vec<(u32, &CameraProfile)> = cams
        .iter()
        .map(|c| {
            let mut score = 0;
            if !c.card_layout.is_empty() && c.card_layout.iter().all(|d| top.contains(&d.to_lowercase())) {
                score += 1;
            }
            if c.filename_pattern.iter().any(|p| media_names.iter().any(|n| glob_match(p, n))) {
                score += 2;
            }
            (score, c)
        })
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    match scored.as_slice() {
        [] => None,
        [(_, c)] => Some(c),
        [(a, c), (b, _), ..] if a > b => Some(c),
        _ => None,
    }
}

/// Shoot-day guess (PLAN.md Mag rules): earliest media date. Uses the container's
/// creation_time when ffprobe reports one, otherwise the file's modification time
/// ("camera date-modified"). Returned as the local calendar date, `YYYY-MM-DD`.
pub fn guess_shoot_day(probe: &Probe) -> Option<(String, &'static str)> {
    let from_meta = probe.clips.iter().filter_map(|c| c.info.creation_time).min();
    let (secs, src) = match from_meta {
        Some(s) => (s, "earliest media creation time"),
        None => {
            let pool: Vec<i64> = if probe.clips.is_empty() {
                probe.files.iter().map(|f| f.mtime).collect()
            } else {
                probe.clips.iter().map(|c| probe.files[c.file].mtime).collect()
            };
            (pool.into_iter().min()?, "earliest file modification time")
        }
    };
    let (y, m, d) = time::local_date(secs);
    Some((format!("{y:04}-{m:02}-{d:02}"), src))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_parse() {
        let t = "streams.stream.0.codec_type=\"video\"\nstreams.stream.0.width=320\nstreams.stream.0.height=180\n\
streams.stream.0.r_frame_rate=\"24000/1001\"\nstreams.stream.1.codec_type=\"audio\"\nstreams.stream.1.channels=2\n\
streams.stream.1.r_frame_rate=\"0/0\"\nstreams.stream.2.codec_type=\"audio\"\nstreams.stream.2.channels=1\n\
format.duration=\"1.001000\"\nformat.tags.creation_time=\"2025-05-20T14:02:11.000000Z\"\n";
        let i = parse_flat(t);
        assert_eq!(i.width, Some(320));
        assert_eq!(i.fps.as_deref(), Some("24000/1001"));
        assert_eq!(i.audio_streams, Some(2));
        assert_eq!(i.audio_channels.as_deref(), Some("2,1"));
        assert_eq!(i.duration_ms, Some(1001));
        assert!(i.creation_time.is_some());
    }

    fn pf(rel: &str) -> ProbedFile {
        ProbedFile { rel: rel.into(), abs: PathBuf::from(rel), size: 1, mtime: 0, role: Role::Other, clip: None }
    }

    #[test]
    fn sidecars_by_stem() {
        let mut files = vec![
            pf("DCIM/100GOPRO/GX010412.MP4"),
            pf("DCIM/100GOPRO/GX010412.THM"),
            pf("DCIM/100GOPRO/GX010412.LRV"),
            pf("DCIM/101GOPRO/GX010412.THM"),
            pf("INFO.TXT"),
        ];
        let clips = classify(&mut files);
        assert_eq!(clips.len(), 1);
        assert_eq!(files[1].role, Role::Sidecar);
        assert_eq!(files[2].role, Role::Sidecar);
        assert_eq!(files[3].role, Role::Other); // different folder
        assert_eq!(files[4].role, Role::Other);
    }
}
