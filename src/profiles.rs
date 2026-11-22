//! Camera (`.cam`) and encoder (`.enc`) profiles. Files are the source of truth; they
//! are parsed into memory (no table, tech spec M13). Rules from PLAN.md
//! "`.cam` file schema (locked)" and "Encoder profiles".

use crate::textfile::{parse_config, split_list};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CameraProfile {
    pub id: String,
    pub name: String,
    pub card_layout: Vec<String>,
    pub filename_pattern: Vec<String>,
    pub file_formats: Vec<String>,
    pub default_format: String,
    pub gamma: Vec<String>,
    pub default_gamma: String,
    pub gamut: Vec<String>,
    pub default_gamut: String,
}

#[derive(Debug, Clone)]
pub struct EncoderProfile {
    pub id: String,
    #[allow(dead_code)] // shown by the Encoder module (Phase 2)
    pub name: String,
    pub kind: String, // 320p | hq | custom
}

pub struct Loaded<T> {
    pub profiles: Vec<T>,
    pub invalid: Vec<(PathBuf, String)>,
}

fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == ext) && p.is_file())
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_alphanumeric()
        && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

pub fn parse_camera(path: &Path, text: &str) -> Result<CameraProfile, String> {
    let kv: BTreeMap<String, String> = parse_config(text).into_iter().collect();
    let get = |k: &str| kv.get(k).cloned().unwrap_or_default();
    let req = ["name", "id", "file_formats", "default_format", "gamma", "default_gamma", "gamut", "default_gamut"];
    let missing: Vec<&str> = req.iter().copied().filter(|k| get(k).is_empty()).collect();
    if !missing.is_empty() {
        return Err(format!("missing required keys: {}", missing.join(", ")));
    }
    let id = get("id");
    if !valid_id(&id) {
        return Err(format!("id {id:?} must be lowercase letters, digits and '-'"));
    }
    if id != stem(path) {
        return Err(format!("id {id:?} does not match file name {:?}", path.file_name().unwrap_or_default()));
    }
    let p = CameraProfile {
        id,
        name: get("name"),
        card_layout: split_list(&get("card_layout")),
        filename_pattern: split_list(&get("filename_pattern")),
        file_formats: split_list(&get("file_formats")),
        default_format: get("default_format"),
        gamma: split_list(&get("gamma")),
        default_gamma: get("default_gamma"),
        gamut: split_list(&get("gamut")),
        default_gamut: get("default_gamut"),
    };
    for (what, list, def) in [
        ("default_format", &p.file_formats, &p.default_format),
        ("default_gamma", &p.gamma, &p.default_gamma),
        ("default_gamut", &p.gamut, &p.default_gamut),
    ] {
        if !list.contains(def) {
            return Err(format!("{what} {def:?} is not in its list"));
        }
    }
    for v in p.file_formats.iter().chain(&p.gamma).chain(&p.gamut).chain([&p.name]) {
        if v.contains('|') || v.chars().any(char::is_control) {
            return Err(format!("value {v:?} contains '|' or a control character"));
        }
    }
    Ok(p)
}

pub fn parse_encoder(path: &Path, text: &str) -> Result<EncoderProfile, String> {
    let kv: BTreeMap<String, String> = parse_config(text).into_iter().collect();
    let get = |k: &str| kv.get(k).cloned().unwrap_or_default();
    let missing: Vec<&str> =
        ["name", "id", "type", "codec", "height", "rate_control", "audio_codec"].into_iter().filter(|k| get(k).is_empty()).collect();
    if !missing.is_empty() {
        return Err(format!("missing required keys: {}", missing.join(", ")));
    }
    let id = get("id");
    if !valid_id(&id) {
        return Err(format!("id {id:?} must be lowercase letters, digits and '-'"));
    }
    if id != stem(path) {
        return Err(format!("id {id:?} does not match file name {:?}", path.file_name().unwrap_or_default()));
    }
    let kind = get("type");
    if !["320p", "hq", "custom"].contains(&kind.as_str()) {
        return Err(format!("type {kind:?} must be 320p, hq or custom"));
    }
    let height = get("height");
    if height != "source" && height.parse::<u32>().map_or(true, |h| h == 0) {
        return Err(format!("height {height:?} must be a positive integer or 'source'"));
    }
    match get("rate_control").as_str() {
        "crf" if !get("crf").is_empty() => {}
        "bitrate" if !get("bitrate").is_empty() => {}
        "crf" | "bitrate" => return Err("rate_control needs its matching crf or bitrate value".into()),
        other => return Err(format!("rate_control {other:?} must be crf or bitrate")),
    }
    Ok(EncoderProfile { id, name: get("name"), kind })
}

pub fn load_cameras(dir: &Path) -> Loaded<CameraProfile> {
    load(dir, "cam", parse_camera)
}

pub fn load_encoders(dir: &Path) -> Loaded<EncoderProfile> {
    load(dir, "enc", parse_encoder)
}

fn load<T>(dir: &Path, ext: &str, parse: fn(&Path, &str) -> Result<T, String>) -> Loaded<T> {
    let mut profiles = Vec::new();
    let mut invalid = Vec::new();
    for f in files_with_ext(dir, ext) {
        match std::fs::read_to_string(&f) {
            Ok(text) => match parse(&f, &text) {
                Ok(p) => profiles.push(p),
                Err(e) => invalid.push((f, e)),
            },
            Err(e) => invalid.push((f, e.to_string())),
        }
    }
    Loaded { profiles, invalid }
}

/// Case-insensitive glob with `*` and `?`.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SONY: &str = "# Magvault camera profile\nname: Sony A7S III\nid: sony-a7siii\n\nvendor: Sony\nmodel: A7S III\n\
card_layout: DCIM, PRIVATE\nfilename_pattern: A001_C*.mov\nfile_formats: ProRes RAW, XAVC S, XAVC HS\ndefault_format: ProRes RAW\n\
gamma: S-Log3, S-Log2, S-Cinetone, Rec.709\ndefault_gamma: S-Log3\ngamut: S-Gamut3.Cine, S-Gamut3, Rec.709\ndefault_gamut: S-Gamut3.Cine\n";

    #[test]
    fn parses_plan_example() {
        let p = parse_camera(Path::new("cameras/sony-a7siii.cam"), SONY).unwrap();
        assert_eq!(p.card_layout, vec!["DCIM", "PRIVATE"]);
        assert_eq!(p.default_gamma, "S-Log3");
        assert!(parse_camera(Path::new("cameras/other.cam"), SONY).is_err());
        let bad = SONY.replace("default_gamma: S-Log3", "default_gamma: HLG");
        assert!(parse_camera(Path::new("cameras/sony-a7siii.cam"), &bad).is_err());
    }

    #[test]
    fn parses_encoder_example() {
        let t = "name: default-320p\nid: default-320p\ntype: 320p          # 320p | hq | custom\ncodec: libx264\n\
height: 320         # width calculated\nrate_control: crf   # crf | bitrate\ncrf: 23\npreset: veryfast\naudio_codec: aac\n";
        let p = parse_encoder(Path::new("default-320p.enc"), t).unwrap();
        assert_eq!(p.kind, "320p");
        assert!(parse_encoder(Path::new("default-320p.enc"), &t.replace("crf: 23\n", "")).is_err());
    }

    #[test]
    fn globbing() {
        assert!(glob_match("A001_C*.mov", "a001_c002_0101r7.MOV"));
        assert!(glob_match("GX??????.MP4", "GX010412.MP4"));
        assert!(!glob_match("A001_C*.mov", "B001_C001.mov"));
        assert!(glob_match("*", "anything"));
    }
}
