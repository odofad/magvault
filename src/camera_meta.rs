//! Camera model hints from metadata next to live media (docs/camera-identification-
//! findings.md, "How to reliably identify a camera"). Only metadata that belongs to a clip
//! on the card counts:
//!
//! - Sony clip sidecar `<stem>M01.XML` (NonRealTimeMeta `Device modelName`)
//! - Sony `MEDIAPRO.XML` `systemKind` (prefix, ` ver.x` dropped) — only when that file's
//!   folder holds a clip; a leftover MEDIAPRO.XML (or anything in `AVF_INFO`) names the
//!   camera that *formatted* the card and is ignored
//! - Panasonic P2 sidecar `CONTENTS/CLIP/<stem>.XML` and XML embedded in MOV / MXF clips
//!   (`<ModelName>`), Sony XML embedded in MP4 / MXF clips (`modelName="…"`)
//! - Sony MPEG `MVI_DATA` in `.MPG` clips (`SONY` … `NEX-VG20E`)
//! - EXIF Make / Model of a clip's thumbnail or a DCIM still
//!
//! The result is a hint for the operator, not a decision: it is shown with its source,
//! stored on the mag card (`camera_model_hint`) and, when it names a profile's `model:`,
//! it outweighs layout and file names in the camera guess.

use crate::probe::{Probe, Role};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub model: String,
    pub source: &'static str,
}

/// Clips looked at (spread over the card), and bytes read at each end of a clip.
const MAX_CLIPS: usize = 8;
const SCAN_BYTES: u64 = 1024 * 1024;
const XML_MAX: u64 = 1024 * 1024;

/// Distinct model strings found next to live media, in the order found.
pub fn read(p: &Probe) -> Vec<Hint> {
    let mut out: Vec<Hint> = Vec::new();
    let mut add = |model: Option<String>, source: &'static str| {
        if let Some(m) = model.map(|m| normalise(&m)).filter(|m| !m.is_empty()) {
            match out.iter_mut().find(|h| same_body(&h.model, &m)) {
                // Two spellings of one body (P2 `CX350`, MOV `AG-CX350`): keep the longer.
                Some(h) if m.len() > h.model.len() => *h = Hint { model: m, source },
                Some(_) => {}
                None => out.push(Hint { model: m, source }),
            }
        }
    };
    let n = p.clips.len();
    let picks: Vec<usize> = if n <= MAX_CLIPS { (0..n).collect() } else { (0..MAX_CLIPS).map(|i| i * n / MAX_CLIPS).collect() };
    for &ci in &picks {
        let clip_file = &p.files[p.clips[ci].file];
        let mut found = false;
        // Paired sidecars (Sony M01.XML, same-stem XML).
        for f in p.files.iter().filter(|f| f.role == Role::Sidecar && f.clip == Some(ci)) {
            if f.rel.to_lowercase().ends_with(".xml") {
                if let Some(m) = xml_model(&read_head(&f.abs, XML_MAX)) {
                    add(Some(m), "clip sidecar XML");
                    found = true;
                }
            }
        }
        // Panasonic P2: CONTENTS/AVCLIP/<stem>.MXF ↔ CONTENTS/CLIP/<stem>.XML.
        if let Some(x) = p2_sidecar(&clip_file.abs) {
            if let Some(m) = xml_model(&read_head(&x, XML_MAX)) {
                add(Some(m), "P2 clip XML");
                found = true;
            }
        }
        if found {
            continue;
        }
        // Metadata inside the clip (embedded XML; MPEG MVI_DATA).
        let bytes = read_ends(&clip_file.abs, SCAN_BYTES);
        let lower = clip_file.rel.to_lowercase();
        if lower.ends_with(".mpg") {
            add(mvi_model(&bytes), "MPEG MVI_DATA");
        } else {
            add(xml_model(&bytes), "XML inside the clip");
        }
    }
    // MEDIAPRO.XML only when its own folder tree holds a clip (not a leftover).
    for f in &p.files {
        let name = Path::new(&f.rel).file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default();
        if name != "MEDIAPRO.XML" || f.rel.to_uppercase().starts_with("AVF_INFO/") {
            continue;
        }
        let dir = Path::new(&f.rel).parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
        let live = p.clips.iter().any(|c| {
            let r = &p.files[c.file].rel;
            dir.is_empty() || r.starts_with(&format!("{dir}/"))
        });
        if live {
            add(attr(&read_head(&f.abs, XML_MAX), "systemKind"), "MEDIAPRO.XML next to the clips");
        }
    }
    // EXIF: thumbnails of live clips (same stem prefix), DCIM stills.
    let stems: Vec<String> = p
        .clips
        .iter()
        .map(|c| Path::new(&p.files[c.file].rel).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default())
        .collect();
    let mut jpgs = 0;
    for f in &p.files {
        let l = f.rel.to_lowercase();
        if !(l.ends_with(".jpg") || l.ends_with(".jpeg")) || jpgs >= 4 {
            continue;
        }
        let stem = Path::new(&l).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let thumb = stems.iter().any(|s| !s.is_empty() && stem.starts_with(s.as_str()));
        let still = l.starts_with("dcim/");
        if thumb || still {
            jpgs += 1;
            add(exif_model(&read_head(&f.abs, 256 * 1024)), if thumb { "thumbnail EXIF" } else { "DCIM still EXIF" });
        }
    }
    out
}

/// One line for the operator: `ILME-FX6V (clip sidecar XML)`, or every model when the
/// card holds more than one (a reused card): `ILCE-7M3, FDR-AX53 (mixed: …)`.
pub fn describe(h: &[Hint]) -> Option<String> {
    match h {
        [] => None,
        [one] => Some(format!("{} ({})", one.model, one.source)),
        many => Some(format!(
            "{} (more than one camera on this card: {})",
            many.iter().map(|h| h.model.as_str()).collect::<Vec<_>>().join(", "),
            many.iter().map(|h| h.source).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// One body, two spellings: equal, or one is the other with a maker prefix
/// (`AG-CX350` / `CX350`).
fn same_body(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_uppercase(), b.to_uppercase());
    a == b || a.ends_with(&format!("-{b}")) || b.ends_with(&format!("-{a}"))
}

/// `PXW-FS5 ver.4.020` → `PXW-FS5`; trims and drops control characters.
pub fn normalise(m: &str) -> String {
    let m: String = m.chars().filter(|c| !c.is_control()).collect();
    let m = m.trim();
    let m = match m.find(" ver.") {
        Some(i) => &m[..i],
        None => m,
    };
    m.trim().to_string()
}

/// Does a hint name this profile model? Equal, or the hint continues with a letter (a
/// region / variant suffix: NEX-VG20 → NEX-VG20E, ILME-FX6 → ILME-FX6V). A following
/// digit is a different model (FDR-AX3 does not match FDR-AX33 or FDR-AX53).
pub fn model_matches(hint: &str, model: &str) -> bool {
    let (h, m) = (hint.to_uppercase(), model.trim().to_uppercase());
    if m.is_empty() {
        return false;
    }
    if h == m {
        return true;
    }
    match h.strip_prefix(&m) {
        Some(rest) => rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == ' '),
        None => false,
    }
}

fn read_head(path: &Path, max: u64) -> Vec<u8> {
    let mut v = Vec::new();
    if let Ok(f) = std::fs::File::open(path) {
        let _ = f.take(max).read_to_end(&mut v);
    }
    v
}

/// First and last `n` bytes of a file (metadata boxes sit at either end).
fn read_ends(path: &Path, n: u64) -> Vec<u8> {
    let Ok(mut f) = std::fs::File::open(path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut v = Vec::new();
    let _ = (&mut f).take(n).read_to_end(&mut v);
    if len > 2 * n && f.seek(SeekFrom::Start(len - n)).is_ok() {
        let _ = f.take(n).read_to_end(&mut v);
    }
    v
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Value of `name="…"` in XML bytes.
fn attr(b: &[u8], name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let i = find(b, key.as_bytes())? + key.len();
    let end = b[i..].iter().position(|&c| c == b'"')?;
    let v = String::from_utf8_lossy(&b[i..i + end]).into_owned();
    (v.len() <= 64).then_some(v)
}

/// Text of `<tag>…</tag>`.
fn element(b: &[u8], tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let i = find(b, open.as_bytes())? + open.len();
    let end = find(&b[i..], b"<")?;
    let v = String::from_utf8_lossy(&b[i..i + end]).into_owned();
    (v.len() <= 64).then_some(v)
}

/// Sony NonRealTimeMeta `<Device … modelName="…">` or Panasonic `<ModelName>…</ModelName>`.
fn xml_model(b: &[u8]) -> Option<String> {
    attr(b, "modelName").or_else(|| element(b, "ModelName")).filter(|m| !m.trim().is_empty())
}

/// Sony MPEG private data: `SONY` followed (within a few bytes) by a model token such
/// as `NEX-VG20E`; maker codes like `HDR-000010G` are not models.
fn mvi_model(b: &[u8]) -> Option<String> {
    let mut from = 0;
    while let Some(i) = find(&b[from..], b"SONY") {
        let start = from + i + 4;
        let window = &b[start..(start + 64).min(b.len())];
        for tok in window.split(|c| !(c.is_ascii_alphanumeric() || *c == b'-')) {
            let t = String::from_utf8_lossy(tok);
            let looks_like_model = t.len() >= 5
                && t.contains('-')
                && t.split('-').next().is_some_and(|p| (2..=5).contains(&p.len()) && p.chars().all(|c| c.is_ascii_uppercase()))
                && t.chars().any(|c| c.is_ascii_digit());
            let maker_code = t.split('-').nth(1).is_some_and(|r| r.starts_with("0000"));
            if looks_like_model && !maker_code {
                return Some(t.into_owned());
            }
        }
        from = start;
    }
    None
}

/// EXIF Make / Model (IFD0) from a JPEG: `SONY ILCE-7M3` → `ILCE-7M3`.
fn exif_model(b: &[u8]) -> Option<String> {
    let app1 = find(b, b"Exif\0\0")? + 6;
    let t = b.get(app1..)?;
    let le = match t.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let u16_at = |o: usize| t.get(o..o + 2).map(|x| if le { u16::from_le_bytes([x[0], x[1]]) } else { u16::from_be_bytes([x[0], x[1]]) });
    let u32_at = |o: usize| {
        t.get(o..o + 4).map(|x| {
            if le {
                u32::from_le_bytes([x[0], x[1], x[2], x[3]])
            } else {
                u32::from_be_bytes([x[0], x[1], x[2], x[3]])
            }
        })
    };
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    let mut model = None;
    for k in 0..count.min(64) {
        let e = ifd + 2 + k * 12;
        let (tag, typ, n) = (u16_at(e)?, u16_at(e + 2)?, u32_at(e + 4)? as usize);
        if tag == 0x0110 && typ == 2 && n <= 64 {
            let off = if n <= 4 { e + 8 } else { u32_at(e + 8)? as usize };
            let raw = t.get(off..off + n)?;
            model = Some(String::from_utf8_lossy(raw).trim_end_matches('\0').trim().to_string());
        }
    }
    model.filter(|m| !m.is_empty())
}

/// P2: a clip in `CONTENTS/AVCLIP` (or `VIDEO`) has its XML in `CONTENTS/CLIP/<stem>.XML`.
fn p2_sidecar(clip: &Path) -> Option<PathBuf> {
    let dir = clip.parent()?;
    let dname = dir.file_name()?.to_string_lossy().to_uppercase();
    if dname != "AVCLIP" && dname != "VIDEO" {
        return None;
    }
    let stem = clip.file_stem()?.to_string_lossy().to_uppercase();
    let clipdir = crate::probe::find_dir_ci(dir.parent()?, "CLIP")?;
    std::fs::read_dir(clipdir).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&stem))
            && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("xml"))
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn model_matching_rules() {
        assert!(model_matches("NEX-VG20E", "NEX-VG20"));
        assert!(model_matches("ILME-FX6V", "ILME-FX6"));
        assert!(model_matches("AG-CX350", "AG-CX350"));
        assert!(model_matches("cx350", "CX350"));
        assert!(!model_matches("FDR-AX53", "FDR-AX3"), "AX3 is the user's name; firmware writes AX53");
        assert!(!model_matches("FDR-AX33", "FDR-AX3"));
        assert!(!model_matches("ILCE-7M3", "ILCE-7SM3"));
        assert_eq!(normalise("PXW-FS5 ver.4.020"), "PXW-FS5");
        assert_eq!(normalise(" ILME-FX6V ver.5.020 "), "ILME-FX6V");
    }

    #[test]
    fn parsers() {
        let sony = br#"<NonRealTimeMeta><Device manufacturer="Sony" modelName="ILME-FX6V" serialNo="2509191"/></NonRealTimeMeta>"#;
        assert_eq!(xml_model(sony).as_deref(), Some("ILME-FX6V"));
        let pana = b"<ClipMain><Device><Manufacturer>Panasonic</Manufacturer><ModelName>AG-CX350</ModelName></Device></ClipMain>";
        assert_eq!(xml_model(pana).as_deref(), Some("AG-CX350"));
        let mpg = b"\x00\x00\x01\xbf..MVI_DATA..SONY\x00\x00NEX-VG20E\x00HDR-000010G\x00";
        assert_eq!(mvi_model(mpg).as_deref(), Some("NEX-VG20E"));
        assert_eq!(mvi_model(b"SONY\x00HDR-000010G_MC1"), None);
        assert_eq!(attr(br#"<MediaProfile systemKind="PXW-FS5 ver.4.020">"#, "systemKind").as_deref(), Some("PXW-FS5 ver.4.020"));
        assert_eq!(exif_model(&tiny_jpeg_exif("SONY", "ILCE-7M3")).as_deref(), Some("ILCE-7M3"));
    }

    /// A minimal JPEG with an APP1 EXIF block (IFD0: Make, Model), little-endian.
    pub fn tiny_jpeg_exif(make: &str, model: &str) -> Vec<u8> {
        let mut tiff: Vec<u8> = b"II*\0".to_vec();
        tiff.extend(8u32.to_le_bytes());
        let entries = 2u16;
        let data_off = 8 + 2 + entries as usize * 12 + 4;
        let make_z = format!("{make}\0");
        let model_z = format!("{model}\0");
        tiff.extend(entries.to_le_bytes());
        for (tag, s, off) in [(0x010Fu16, &make_z, data_off), (0x0110u16, &model_z, data_off + make_z.len())] {
            tiff.extend(tag.to_le_bytes());
            tiff.extend(2u16.to_le_bytes());
            tiff.extend((s.len() as u32).to_le_bytes());
            tiff.extend((off as u32).to_le_bytes());
        }
        tiff.extend(0u32.to_le_bytes());
        tiff.extend(make_z.as_bytes());
        tiff.extend(model_z.as_bytes());
        let mut j = vec![0xFF, 0xD8, 0xFF, 0xE1];
        j.extend(((tiff.len() + 8) as u16).to_be_bytes());
        j.extend(b"Exif\0\0");
        j.extend(tiff);
        j.extend([0xFF, 0xD9]);
        j
    }
}
