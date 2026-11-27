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
    /// Model read next to live media, with its source (camera_meta), for the operator.
    pub model_hint: Option<String>,
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
        let cam = match &self.model_hint {
            Some(h) => format!("{cam} (model hint: {h})"),
            None => cam,
        };
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
            // A mount point listed again (mounted over) shows only the top mount.
            out.retain(|v: &Volume| v.mount != mount);
            out.push(Volume {
                mount,
                device,
                fstype,
            });
        }
    }
    // One device mounted at several places (automount plus a manual mount) is one card:
    // keep the mount under the earliest parent in MOUNT_PARENTS (/media first).
    let rank = |v: &Volume| MOUNT_PARENTS.iter().position(|p| v.mount.starts_with(p)).unwrap_or(usize::MAX);
    let mut keep: Vec<Volume> = Vec::new();
    for v in out {
        if !v.device.starts_with("/dev/") {
            keep.push(v);
            continue;
        }
        match keep.iter().position(|k| k.device == v.device) {
            Some(i) if rank(&v) < rank(&keep[i]) => keep[i] = v,
            Some(_) => {}
            None => keep.push(v),
        }
    }
    keep
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

/// Camera media trees, named for the operator. A card is labelled by the trees that hold
/// media (all of them, joined by " + "), so an empty `PRIVATE/M4ROOT` left by a format
/// does not hide `PRIVATE/XDROOT` (docs/camera-identification-findings.md).
const MEDIA_TREES: &[(&str, &str)] = &[
    ("CONTENTS/AVCLIP", "Panasonic P2 (CONTENTS)"),
    ("CONTENTS/VIDEO", "Panasonic P2 (CONTENTS)"),
    ("PRIVATE/PANA_GRP", "Panasonic (PRIVATE/PANA_GRP)"),
    ("PRIVATE/M4ROOT", "Sony XAVC (PRIVATE/M4ROOT)"),
    ("PRIVATE/XDROOT", "Sony XDCAM (PRIVATE/XDROOT)"),
    ("XDROOT", "Sony XDCAM (XDROOT)"),
    ("MP_ROOT", "Sony MPEG (MP_ROOT)"),
    ("CONTENTS/CLIPS001", "Canon XF (CONTENTS/CLIPS001)"),
    ("PRIVATE/AVCHD", "AVCHD (PRIVATE/AVCHD)"),
];

/// Known camera layouts: the media trees that hold media; else (an empty, freshly
/// formatted card) the first tree that exists; then RED, BRAW, DCIM variants, then any
/// camera profile whose `card_layout` matches. Field backup drives and mag folders are
/// never cards.
pub fn layout_of(root: &Path, cams: &[CameraProfile]) -> Option<String> {
    if root.join(fieldset::FILE).exists() || root.join(magcard::FILE_NAME).exists() {
        return None;
    }
    let top = child_names(root);
    let mut live: Vec<&str> = Vec::new();
    let mut present: Option<&str> = None;
    for (rel, label) in MEDIA_TREES {
        if let Some(dir) = probe::find_dir_ci(root, rel) {
            present.get_or_insert(label);
            if probe::holds_media(&dir) && !live.contains(label) {
                live.push(label);
            }
        }
    }
    if !live.is_empty() {
        return Some(live.join(" + "));
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
        if present.is_none() {
            return Some("Generic DCIM".into());
        }
    }
    if let Some(p) = present {
        return Some(p.to_string());
    }
    for c in cams {
        if probe::layout_matches(root, &c.card_layout) {
            return Some(format!("camera profile {}", c.id));
        }
    }
    None
}

/// Vendor of a recognised layout (the text `layout_of` returns), for the camera guess.
fn layout_vendor(layout: &str) -> Option<&'static str> {
    let l = layout.to_lowercase();
    [
        ("sony", "Sony"),
        ("panasonic", "Panasonic"),
        ("canon", "Canon"),
        ("red (", "RED"),
        ("blackmagic", "Blackmagic"),
        ("gopro", "GoPro"),
        ("dji", "DJI"),
        ("insta360", "Insta360"),
    ]
    .into_iter()
    .find(|(k, _)| l.starts_with(k))
    .map(|(_, v)| v)
}

/// Camera guess used by discovery and ingest alike: the profile guess (card_layout /
/// filename_pattern, `probe::guess_camera`); if that gives nothing, the recognised card
/// layout's vendor when exactly one profile has that vendor. `Err` says why there is no
/// guess, for the operator.
pub fn guess_camera<'a>(
    p: &probe::Probe,
    cams: &'a [CameraProfile],
    hints: &[crate::camera_meta::Hint],
) -> Result<&'a CameraProfile, String> {
    if let Some(c) = probe::guess_camera(p, cams, hints) {
        return Ok(c);
    }
    // The card names its camera but no profile has that model: say so; never fall back
    // to a profile of the same vendor (that would be a hard guess against the metadata).
    if let Some(d) = crate::camera_meta::describe(hints) {
        return Err(format!("no camera profile has the model this card names: {d}"));
    }
    let layout = layout_of(&p.root, cams);
    let Some(vendor) = layout.as_deref().and_then(layout_vendor) else {
        return Err(match layout {
            Some(l) => format!("no camera profile matches this card (layout: {l})"),
            None => "no camera profile matches this card (no known camera layout)".into(),
        });
    };
    let same: Vec<&CameraProfile> =
        cams.iter().filter(|c| !c.is_generic() && c.vendor.eq_ignore_ascii_case(vendor)).collect();
    match same.as_slice() {
        [c] => Ok(c),
        [] => Err(format!("card layout is {vendor} but no camera profile has vendor {vendor}")),
        many => Err(format!(
            "card layout is {vendor} and several profiles match ({})",
            many.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", ")
        )),
    }
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
            model_hint: None,
            files: 0,
            bytes: 0,
            problem: None,
        };
        match probe::probe(&v.mount, false) {
            Ok(p) => {
                card.files = p.files.len();
                card.bytes = p.total_bytes;
                let hints = crate::camera_meta::read(&p);
                card.camera = guess_camera(&p, cams, &hints).ok().map(|c| (c.id.clone(), c.name.clone()));
                card.model_hint = crate::camera_meta::describe(&hints);
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

    #[test]
    fn mountinfo_one_card_mounted_twice_is_one_volume() {
        let t = "50 22 8:17 / /mnt/card rw - exfat /dev/sdb1 rw\n\
                 51 22 8:17 / /media/op/CARD rw - exfat /dev/sdb1 rw\n\
                 52 22 8:33 / /media/op/OTHER rw - vfat /dev/sdc1 rw\n\
                 53 52 8:49 / /media/op/OTHER rw - vfat /dev/sdd1 rw\n";
        let v = parse_mountinfo(t);
        let m: Vec<_> = v.iter().map(|v| (v.mount.display().to_string(), v.device.as_str())).collect();
        assert_eq!(m, vec![("/media/op/CARD".to_string(), "/dev/sdb1"), ("/media/op/OTHER".to_string(), "/dev/sdd1")]);
    }

    #[test]
    fn camera_from_layout_vendor() {
        let base = std::env::temp_dir().join(format!("mv-vendor-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fixture(&base.join("XAVC"), &["PRIVATE/M4ROOT/CLIP/C0001.MP4"]);
        fixture(&base.join("DCIMONLY"), &["DCIM/100ABCDE/IMG_0001.MOV"]);
        let cam = |id: &str, vendor: &str| {
            parse_camera(
                Path::new(&format!("{id}.cam")),
                &format!(
                    "name: {id}\nid: {id}\nvendor: {vendor}\ncard_layout: DCIM, PRIVATE\nfilename_pattern: A001_C*.mov\n\
                     file_formats: XAVC S\ndefault_format: XAVC S\ngamma: S-Log3\ndefault_gamma: S-Log3\ngamut: S-Gamut3\ndefault_gamut: S-Gamut3\n"
                ),
            )
            .unwrap()
        };
        let one = vec![cam("sony-a7siii", "Sony")];
        let p = probe::probe(&base.join("XAVC"), false).unwrap();
        assert_eq!(guess_camera(&p, &one, &[]).unwrap().id, "sony-a7siii");
        let two = vec![cam("sony-a7siii", "Sony"), cam("sony-fx6", "Sony")];
        assert!(guess_camera(&p, &two, &[]).unwrap_err().contains("several profiles match (sony-a7siii, sony-fx6)"));
        let p = probe::probe(&base.join("DCIMONLY"), false).unwrap();
        assert!(guess_camera(&p, &one, &[]).unwrap_err().contains("layout: Generic DCIM"));
        let _ = fs::remove_dir_all(&base);
    }
    /// The seven reference cards of docs/camera-identification-findings.md in miniature:
    /// same folders, file names and metadata strings (tiny files), with the shipped
    /// profiles. Checks layout label, camera guess, model hint, sidecar pairing and junk.
    #[test]
    fn reference_cards() {
        let base = std::env::temp_dir().join(format!("mv-refcards-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let put = |rel: &str, bytes: &[u8]| {
            let p = base.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        };
        let nrt = |model: &str| format!(r#"<?xml version="1.0"?><NonRealTimeMeta><Device manufacturer="Sony" modelName="{model}" serialNo="4294967295"/></NonRealTimeMeta>"#);
        let mediapro = |kind: &str| format!(r#"<MediaProfile><Properties><System systemId="04C461FFFE1806E4" systemKind="{kind}"/></Properties></MediaProfile>"#);
        // FDR-AX3 (firmware FDR-AX53): XAVC S + AVCHD.
        put("AX3/PRIVATE/M4ROOT/CLIP/C0001.MP4", b"mp4");
        put("AX3/PRIVATE/M4ROOT/CLIP/C0001M01.XML", nrt("FDR-AX53").as_bytes());
        put("AX3/PRIVATE/M4ROOT/MEDIAPRO.XML", mediapro("FDR-AX53").as_bytes());
        put("AX3/PRIVATE/AVCHD/BDMV/STREAM/00000.MTS", b"0002HDR-000010G_MC1");
        put("AX3/AVF_INFO/AVIN0001.INT", b"x");
        // PXW-FS5: live XDCAM under PRIVATE/XDROOT; an empty M4ROOT with a leftover MEDIAPRO.
        put("FS5/PRIVATE/XDROOT/Clip/Clip0001.MXF", b"mxf");
        put("FS5/PRIVATE/XDROOT/Clip/Clip0001M01.XML", nrt("PXW-FS5").as_bytes());
        put("FS5/PRIVATE/XDROOT/MEDIAPRO.XML", mediapro("PXW-FS5 ver.4.020").as_bytes());
        put("FS5/PRIVATE/M4ROOT/MEDIAPRO.XML", mediapro("FDR-AX53").as_bytes());
        put("FS5/PRIVATE/AVCHD/BDMV/STREAM/00000.MTS", b"mts");
        // FX6 over USB: top-level XDROOT, PDTV-FX6 names, SALVAGE.TMP.
        put("FX6/XDROOT/Clip/PDTV-FX6_3985.MXF", b"mxf");
        put("FX6/XDROOT/Clip/PDTV-FX6_3985M01.XML", nrt("ILME-FX6V").as_bytes());
        put("FX6/XDROOT/Clip/PDTV-FX6_3986.MXF", b"mxf");
        put("FX6/XDROOT/Clip/PDTV-FX6_3986M01.XML", nrt("ILME-FX6V").as_bytes());
        put("FX6/XDROOT/MEDIAPRO.XML", mediapro("ILME-FX6V ver.5.020").as_bytes());
        put("FX6/XDROOT/General/Sony/SALVAGE.TMP", b"salvage");
        put("FX6/SONY/x.dat", b"x");
        // NEX-VG20: MP_ROOT MPEG-2 with MVI_DATA, AVCHD; leftover AX53 profile in AVF_INFO.
        put("VG20/MP_ROOT/100PNV01/M2U00001.MPG", b"\x00\x00\x01\xbfMVI_DATA SONY\x00\x00NEX-VG20E\x00HDR-000010G\x00");
        put("VG20/PRIVATE/AVCHD/BDMV/STREAM/00000.MTS", b"mts");
        put("VG20/AVF_INFO/AVIN0001.INT", mediapro("FDR-AX53").as_bytes());
        // PXW-FS7: top-level XDROOT, A###C### names, R01.BIM sidecars.
        put("FS7/XDROOT/Clip/A019C001_26112750.MXF", b"mxf");
        put("FS7/XDROOT/Clip/A019C001_26112750M01.XML", nrt("PXW-FS7").as_bytes());
        put("FS7/XDROOT/Clip/A019C001_26112750R01.BIM", b"bim");
        // A7 III: XAVC S like the AX53; thumb EXIF; empty AVCHD (index only).
        put("A7III/PRIVATE/M4ROOT/CLIP/C0001.MP4", b"mp4");
        put("A7III/PRIVATE/M4ROOT/CLIP/C0001M01.XML", nrt("ILCE-7M3").as_bytes());
        put("A7III/PRIVATE/M4ROOT/THMBNL/C0001T01.JPG", &crate::camera_meta::tests::tiny_jpeg_exif("SONY", "ILCE-7M3"));
        put("A7III/PRIVATE/AVCHD/BDMV/INDEX.BDM", b"DSLR-00002G_MC1");
        // Panasonic AG-CX350: PANA_GRP MOV with embedded XML, P2 MXF + CLIP XML, AVCHD, empty DCIM.
        put("CX350/PRIVATE/PANA_GRP/001RBQAM/A039C001_261127_L34B.MOV", b"moov<ClipMain><Device><ModelName>AG-CX350</ModelName></Device></ClipMain>");
        put("CX350/CONTENTS/AVCLIP/001GW5GN.MXF", b"mxf");
        put("CX350/CONTENTS/CLIP/001GW5GN.XML", b"<P2Main><Device><ModelName>CX350</ModelName></Device></P2Main>");
        put("CX350/PRIVATE/AVCHD/BDMV/STREAM/00000.MTS", b"LSLX64G");
        put("CX350/LASTCLIP.TXT", b"001GW5GN");
        fs::create_dir_all(base.join("CX350/DCIM")).unwrap();

        let cams: Vec<CameraProfile> = [
            include_str!("../share/cameras/generic-avchd.cam"),
            include_str!("../share/cameras/sony-a7siii.cam"),
            include_str!("../share/cameras/sony-fdr-ax33.cam"),
            include_str!("../share/cameras/sony-vg20.cam"),
        ]
        .iter()
        .map(|t| {
            let id = t.lines().find_map(|l| l.strip_prefix("id: ")).unwrap();
            parse_camera(Path::new(&format!("{id}.cam")), t).unwrap()
        })
        .collect();
        let check = |card: &str, layout: &str, camera: Option<&str>, hint: Option<&str>| {
            let root = base.join(card);
            let p = probe::probe(&root, false).unwrap();
            assert_eq!(layout_of(&root, &cams).as_deref(), Some(layout), "{card} layout");
            let hints = crate::camera_meta::read(&p);
            let got = guess_camera(&p, &cams, &hints).ok().map(|c| c.id.as_str());
            assert_eq!(got, camera, "{card} camera ({hints:?})");
            assert_eq!(crate::camera_meta::describe(&hints).as_deref(), hint, "{card} hint");
            p
        };
        check(
            "AX3",
            "Sony XAVC (PRIVATE/M4ROOT) + AVCHD (PRIVATE/AVCHD)",
            Some("generic-avchd"),
            Some("FDR-AX53 (clip sidecar XML)"),
        );
        check(
            "FS5",
            "Sony XDCAM (PRIVATE/XDROOT) + AVCHD (PRIVATE/AVCHD)",
            Some("generic-avchd"),
            Some("PXW-FS5 (clip sidecar XML)"),
        );
        let fx6 = check("FX6", "Sony XDCAM (XDROOT)", None, Some("ILME-FX6V (clip sidecar XML)"));
        assert!(fx6.skipped.iter().any(|s| s.ends_with("SALVAGE.TMP")), "SALVAGE.TMP is junk");
        assert!(!fx6.files.iter().any(|f| f.rel.ends_with("SALVAGE.TMP")));
        let side: Vec<_> = fx6.files.iter().filter(|f| f.role == probe::Role::Sidecar).map(|f| f.rel.as_str()).collect();
        assert_eq!(side, ["XDROOT/Clip/PDTV-FX6_3985M01.XML", "XDROOT/Clip/PDTV-FX6_3986M01.XML"], "M01.XML paired");
        assert!(fx6.files.iter().filter(|f| f.role == probe::Role::Sidecar).all(|f| f.clip.is_some()));
        check(
            "VG20",
            "Sony MPEG (MP_ROOT) + AVCHD (PRIVATE/AVCHD)",
            Some("sony-vg20"),
            Some("NEX-VG20E (MPEG MVI_DATA)"),
        );
        let fs7 = check("FS7", "Sony XDCAM (XDROOT)", None, Some("PXW-FS7 (clip sidecar XML)"));
        assert_eq!(fs7.files.iter().filter(|f| f.role == probe::Role::Sidecar).count(), 2, "M01.XML and R01.BIM paired");
        check("A7III", "Sony XAVC (PRIVATE/M4ROOT)", None, Some("ILCE-7M3 (clip sidecar XML)"));
        check(
            "CX350",
            "Panasonic P2 (CONTENTS) + Panasonic (PRIVATE/PANA_GRP) + AVCHD (PRIVATE/AVCHD)",
            Some("generic-avchd"),
            Some("AG-CX350 (XML inside the clip)"),
        );
        // A VG20 profile without MP_ROOT never claims an AVCHD card; generic-avchd never
        // outranks a specific match; the operator's --model drives the guess like a hint.
        let op = [crate::camera_meta::Hint { model: "NEX-VG20".into(), source: "given by the operator" }];
        let p = probe::probe(&base.join("AX3"), false).unwrap();
        assert_eq!(guess_camera(&p, &cams, &op).unwrap().id, "sony-vg20");
        let _ = fs::remove_dir_all(&base);
    }
}

