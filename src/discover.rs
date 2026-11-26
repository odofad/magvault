//! Card discovery (PLAN.md "Ingest flow" step 1): find mounted volumes that look like
//! camera cards, with label, size and guessed camera. The operator still confirms.
//!
//! Volumes come from `/proc/self/mountinfo` (mount points under `/media`, `/run/media`,
//! `/mnt`). For tests and fixtures, `--scan-root <dir>` (or `MAGVAULT_SCAN_ROOTS`,
//! colon-separated) replaces that: every non-hidden folder directly inside a scan root is
//! treated as a mounted volume.

use crate::fieldset;
use crate::magcard;
use crate::probe;
use crate::profiles::CameraProfile;
use crate::util::{esc, human};
use std::path::{Path, PathBuf};

pub const MOUNT_PARENTS: &[&str] = &["/media", "/run/media", "/mnt"];

#[derive(Debug, Clone)]
pub struct Volume {
    pub mount: PathBuf,
    pub device: String,
    pub fstype: String,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub volume: Volume,
    pub layout: String,
    pub camera: Option<(String, String)>,
    pub files: usize,
    pub bytes: u64,
    pub problem: Option<String>,
}

impl Card {
    pub fn line(&self) -> String {
        let cam = self
            .camera
            .as_ref()
            .map(|(id, name)| format!("{name} [{id}]"))
            .unwrap_or_else(|| "camera not guessed".into());
        let content = match &self.problem {
            Some(p) => format!("cannot ingest: {p}"),
            None => format!("{} files · {}", self.files, human(self.bytes)),
        };
        format!(
            "{}  ·  {}  ·  {}  ·  {}  ·  {} {}",
            esc(&self.volume.mount.display().to_string()),
            self.layout,
            cam,
            content,
            esc(&self.volume.device),
            esc(&self.volume.fstype)
        )
    }
}

/// Scan roots from `--scan-root` arguments, else `MAGVAULT_SCAN_ROOTS`, else none
/// (none = real mounts).
pub fn scan_roots(args: &[PathBuf]) -> Vec<PathBuf> {
    if !args.is_empty() {
        return args.to_vec();
    }
    std::env::var("MAGVAULT_SCAN_ROOTS")
        .ok()
        .map(|v| {
            v.split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

pub fn volumes(roots: &[PathBuf]) -> Vec<Volume> {
    if roots.is_empty() {
        return std::fs::read_to_string("/proc/self/mountinfo")
            .map(|t| parse_mountinfo(&t))
            .unwrap_or_default();
    }
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else {
            continue;
        };
        let mut dirs: Vec<PathBuf> = rd
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .map(|e| e.path())
            .collect();
        dirs.sort();
        out.extend(dirs.into_iter().map(|mount| Volume {
            mount,
            device: "-".into(),
            fstype: "directory".into(),
        }));
    }
    out
}

fn unescape_mount(s: &str) -> String {
    // mountinfo escapes space, tab, newline and backslash as \ooo (octal).
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
        {
            out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Mounted filesystems under `/media`, `/run/media`, `/mnt` (not the parents themselves).
pub fn parse_mountinfo(text: &str) -> Vec<Volume> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((left, right)) = line.split_once(" - ") else {
            continue;
        };
        let fields: Vec<&str> = left.split(' ').collect();
        if fields.len() < 5 {
            continue;
        }
        let mount = PathBuf::from(unescape_mount(fields[4]));
        let mut r = right.split(' ');
        let fstype = r.next().unwrap_or("").to_string();
        let device = unescape_mount(r.next().unwrap_or(""));
        if fstype == "autofs" {
            continue;
        }
        let under = MOUNT_PARENTS
            .iter()
            .any(|p| mount.starts_with(p) && mount != Path::new(p));
        if under {
            out.push(Volume {
                mount,
                device,
                fstype,
            });
        }
    }
    out
}

fn child_names(dir: &Path) -> Vec<(String, bool)> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| {
                    (
                        e.file_name().to_string_lossy().into_owned(),
                        e.file_type().map(|t| t.is_dir()).unwrap_or(false),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn has_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    child_names(dir)
        .into_iter()
        .find(|(n, d)| *d && n.eq_ignore_ascii_case(name))
        .map(|(n, _)| dir.join(n))
}

/// Known camera layouts (PLAN.md list; specific layouts before generic DCIM), then any
/// camera profile whose `card_layout` folders are all present. Field backup drives and mag
/// folders are never cards.
pub fn layout_of(root: &Path, cams: &[CameraProfile]) -> Option<String> {
    if root.join(fieldset::FILE).exists() || root.join(magcard::FILE_NAME).exists() {
        return None;
    }
    let top = child_names(root);
    if let Some(private) = has_dir(root, "PRIVATE") {
        if has_dir(&private, "M4ROOT").is_some() {
            return Some("Sony XAVC (PRIVATE/M4ROOT)".into());
        }
        if has_dir(&private, "AVCHD").is_some() {
            return Some("AVCHD (PRIVATE/AVCHD)".into());
        }
    }
    if has_dir(root, "XDROOT").is_some() {
        return Some("Sony XDCAM (XDROOT)".into());
    }
    if let Some(contents) = has_dir(root, "CONTENTS") {
        if has_dir(&contents, "CLIPS001").is_some() {
            return Some("Canon XF (CONTENTS/CLIPS001)".into());
        }
    }
    if top
        .iter()
        .any(|(n, d)| *d && n.to_uppercase().ends_with(".RDM"))
    {
        return Some("RED (.RDM)".into());
    }
    if top
        .iter()
        .any(|(n, d)| !*d && n.to_lowercase().ends_with(".braw"))
    {
        return Some("Blackmagic RAW (.braw)".into());
    }
    if let Some(dcim) = has_dir(root, "DCIM") {
        let subs: Vec<String> = child_names(&dcim)
            .into_iter()
            .filter(|(_, d)| *d)
            .map(|(n, _)| n.to_uppercase())
            .collect();
        let any = |f: &dyn Fn(&str) -> bool| subs.iter().any(|s| f(s));
        if any(&|s| s.ends_with("GOPRO")) {
            return Some("GoPro (DCIM/…GOPRO)".into());
        }
        if any(&|s| s.ends_with("CANON")) {
            return Some("Canon (DCIM/…CANON)".into());
        }
        if any(&|s| s.starts_with("DJI_") || s.ends_with("MEDIA")) {
            return Some("DJI (DCIM)".into());
        }
        if any(&|s| s.starts_with("CAMERA")) {
            return Some("Insta360 (DCIM/Camera…)".into());
        }
        return Some("Generic DCIM".into());
    }
    for c in cams {
        if !c.card_layout.is_empty() && c.card_layout.iter().all(|d| has_dir(root, d).is_some()) {
            return Some(format!("camera profile {}", c.id));
        }
    }
    None
}

pub fn find_cards(vols: &[Volume], cams: &[CameraProfile]) -> Vec<Card> {
    let mut cards = Vec::new();
    for v in vols {
        let Some(layout) = layout_of(&v.mount, cams) else {
            continue;
        };
        let mut card = Card {
            volume: v.clone(),
            layout,
            camera: None,
            files: 0,
            bytes: 0,
            problem: None,
        };
        match probe::probe(&v.mount, false) {
            Ok(p) => {
                card.files = p.files.len();
                card.bytes = p.total_bytes;
                card.camera = probe::guess_camera(&p, cams).map(|c| (c.id.clone(), c.name.clone()));
            }
            Err(e) => card.problem = Some(e.0.lines().next().unwrap_or("").to_string()),
        }
        cards.push(card);
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::parse_camera;
    use std::fs;

    fn fixture(root: &Path, files: &[&str]) {
        for f in files {
            let p = root.join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"x").unwrap();
        }
    }

    #[test]
    fn fixtures_sony_canon_generic() {
        let base = std::env::temp_dir().join(format!("mv-discover-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fixture(
            &base.join("A7SIII_128"),
            &[
                "DCIM/100MSDCF/DSC00001.JPG",
                "PRIVATE/M4ROOT/CLIP/C0001.MP4",
                "PRIVATE/M4ROOT/CLIP/C0001M01.XML",
            ],
        );
        fixture(
            &base.join("C300_CFAST"),
            &["CONTENTS/CLIPS001/A001C001_220101AB_CANON.MXF"],
        );
        fixture(&base.join("EOS_R5"), &["DCIM/100CANON/MVI_0001.MP4"]);
        fixture(&base.join("GENERIC_64"), &["DCIM/100ABCDE/IMG_0001.MOV"]);
        fixture(&base.join("FIELD-A_M"), &[fieldset::FILE]);
        fixture(&base.join("Documents"), &["notes.txt"]);
        let vols = volumes(std::slice::from_ref(&base));
        assert_eq!(vols.len(), 6);
        let sony = parse_camera(
            Path::new("sony-a7siii.cam"),
            "name: Sony A7S III\nid: sony-a7siii\ncard_layout: DCIM, PRIVATE\nfilename_pattern: C*.MP4\nfile_formats: XAVC S\n\
             default_format: XAVC S\ngamma: S-Log3\ndefault_gamma: S-Log3\ngamut: S-Gamut3.Cine\ndefault_gamut: S-Gamut3.Cine\n",
        )
        .unwrap();
        let cards = find_cards(&vols, std::slice::from_ref(&sony));
        let by_name = |n: &str| cards.iter().find(|c| c.volume.mount.ends_with(n));
        let s = by_name("A7SIII_128").unwrap();
        assert_eq!(s.layout, "Sony XAVC (PRIVATE/M4ROOT)");
        assert_eq!(s.camera.as_ref().unwrap().0, "sony-a7siii");
        assert_eq!(s.files, 3);
        assert_eq!(
            by_name("C300_CFAST").unwrap().layout,
            "Canon XF (CONTENTS/CLIPS001)"
        );
        assert_eq!(by_name("EOS_R5").unwrap().layout, "Canon (DCIM/…CANON)");
        assert_eq!(by_name("GENERIC_64").unwrap().layout, "Generic DCIM");
        assert!(
            by_name("FIELD-A_M").is_none(),
            "field backup drives are not cards"
        );
        assert!(by_name("Documents").is_none());
        assert_eq!(cards.len(), 4);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn mountinfo_parsing() {
        let t = "22 1 8:1 / / rw - ext4 /dev/sda1 rw\n\
                 50 22 8:17 / /media/op/A7SIII\\040128 rw,nosuid - exfat /dev/sdb1 rw\n\
                 51 22 0:44 / /media rw - tmpfs tmpfs rw\n\
                 52 22 8:33 / /run/media/op/CARD rw - vfat /dev/sdc1 rw\n";
        let v = parse_mountinfo(t);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].mount, PathBuf::from("/media/op/A7SIII 128"));
        assert_eq!(v[0].device, "/dev/sdb1");
        assert_eq!(v[0].fstype, "exfat");
        assert_eq!(v[1].mount, PathBuf::from("/run/media/op/CARD"));
    }
}
