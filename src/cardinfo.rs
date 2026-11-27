//! Card summary and project requirements (PLAN.md "Time display" Block 2, "Project
//! requirements warning").
//!
//! The summary is one value per card: clip count, total size, clip time range, and the
//! frame rate, resolution and GOP of the clips (the dominant value, "mixed" when clips
//! differ). Requirements are fixed values chosen at project creation; at ingest a card that
//! does not meet them gets the warning "Camera footage does not meet project
//! requirements." It is not a hard stop; the mismatch is logged to the project.

use crate::probe::Probe;
use std::collections::BTreeMap;

pub const WARNING: &str = "Camera footage does not meet project requirements.";
pub const FPS_LIST: [u32; 5] = [24, 25, 30, 50, 60];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Res {
    K4,
    Hd,
    Sd,
}

impl Res {
    pub const ALL: [Res; 3] = [Res::K4, Res::Hd, Res::Sd];
    pub fn as_str(self) -> &'static str {
        match self {
            Res::K4 => "4K",
            Res::Hd => "HD",
            Res::Sd => "SD",
        }
    }
    pub fn parse(s: &str) -> Option<Res> {
        Res::ALL.into_iter().find(|r| r.as_str().eq_ignore_ascii_case(s.trim()))
    }
    /// 4K: 3840 wide (or 2160 high) and up; HD: 1280 / 720 and up; SD below.
    pub fn of(w: i64, h: i64) -> Res {
        if w >= 3840 || h >= 2160 {
            Res::K4
        } else if w >= 1280 || h >= 720 {
            Res::Hd
        } else {
            Res::Sd
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gop {
    Intra,
    LongGop,
}

impl Gop {
    pub const ALL: [Gop; 2] = [Gop::Intra, Gop::LongGop];
    pub fn as_str(self) -> &'static str {
        match self {
            Gop::Intra => "intra",
            Gop::LongGop => "long-GOP",
        }
    }
    pub fn parse(s: &str) -> Option<Gop> {
        Gop::ALL.into_iter().find(|g| g.as_str().eq_ignore_ascii_case(s.trim()))
    }
    /// From the video codec and profile: intra-only codecs, and H.264 / HEVC "Intra"
    /// profiles (XAVC-I) are intra; other H.264 / HEVC / MPEG are long-GOP.
    pub fn of(codec: &str, profile: Option<&str>) -> Option<Gop> {
        let c = codec.to_ascii_lowercase();
        let intra_profile = profile.is_some_and(|p| p.to_ascii_lowercase().contains("intra"));
        match c.as_str() {
            "prores" | "dnxhd" | "mjpeg" | "cfhd" | "jpeg2000" | "rawvideo" | "v210" | "r210" | "ffv1" | "dvvideo" => Some(Gop::Intra),
            "h264" | "hevc" | "h265" if intra_profile => Some(Gop::Intra),
            "h264" | "hevc" | "h265" | "mpeg2video" | "mpeg1video" | "mpeg4" | "vp9" | "av1" => Some(Gop::LongGop),
            _ => None,
        }
    }
}

/// `24000/1001` → 23.976…; `25` → 25.
pub fn fps_value(rate: &str) -> Option<f64> {
    let v = match rate.split_once('/') {
        Some((n, d)) => {
            let (n, d): (f64, f64) = (n.trim().parse().ok()?, d.trim().parse().ok()?);
            if d == 0.0 {
                return None;
            }
            n / d
        }
        None => rate.trim().parse().ok()?,
    };
    (v > 0.0).then_some(v)
}

/// The listed frame rate a clip runs at: 23.976 counts as 24, 29.97 as 30, 59.94 as 60.
pub fn fps_class(v: f64) -> Option<u32> {
    FPS_LIST.into_iter().find(|f| (v - *f as f64).abs() < 0.1 || (v - *f as f64 * 1000.0 / 1001.0).abs() < 0.01)
}

fn fps_label(v: f64) -> String {
    if (v - v.round()).abs() < 0.005 {
        format!("{} fps", v.round() as i64)
    } else {
        format!("{v:.3} fps").replace(".000", "")
    }
}

/// One value for the whole card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Measure<T> {
    /// No clip reported it (no ffprobe, or no clips).
    Unknown,
    One(T),
    /// Clips differ: the value most clips have, and every value seen.
    Mixed { dominant: T, all: Vec<T> },
}

impl<T: Clone + Ord> Measure<T> {
    fn from(values: Vec<T>) -> Measure<T> {
        let mut count: BTreeMap<T, usize> = BTreeMap::new();
        for v in values {
            *count.entry(v).or_default() += 1;
        }
        match count.len() {
            0 => Measure::Unknown,
            1 => Measure::One(count.into_keys().next().unwrap()),
            _ => {
                let max = *count.values().max().unwrap();
                let dominant = count.iter().find(|(_, n)| **n == max).map(|(v, _)| v.clone()).unwrap();
                Measure::Mixed { dominant, all: count.into_keys().collect() }
            }
        }
    }
    pub fn values(&self) -> Vec<T> {
        match self {
            Measure::Unknown => Vec::new(),
            Measure::One(v) => vec![v.clone()],
            Measure::Mixed { all, .. } => all.clone(),
        }
    }
}

impl<T> Measure<T> {
    /// "25 fps", "25 fps  mixed", "unknown".
    pub fn label(&self, f: impl Fn(&T) -> String) -> String {
        match self {
            Measure::Unknown => "unknown".into(),
            Measure::One(v) => f(v),
            Measure::Mixed { dominant, .. } => format!("{}  mixed", f(dominant)),
        }
    }
}

/// Frame rate of a clip: its label and its listed class (if any).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fps {
    pub label: String,
    pub class: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub clips: usize,
    pub bytes: u64,
    /// Earliest and latest clip time (creation time, else file mtime), epoch seconds.
    pub first: Option<i64>,
    pub last: Option<i64>,
    pub fps: Measure<Fps>,
    pub res: Measure<Res>,
    pub gop: Measure<Gop>,
}

pub fn summarize(p: &Probe) -> Summary {
    let times: Vec<i64> = if p.clips.is_empty() {
        p.files.iter().map(|f| f.mtime).collect()
    } else {
        p.clips.iter().map(|c| c.info.creation_time.unwrap_or(p.files[c.file].mtime)).collect()
    };
    let fps = p
        .clips
        .iter()
        .filter_map(|c| c.info.fps.as_deref().and_then(fps_value))
        .map(|v| Fps { label: fps_label(v), class: fps_class(v) })
        .collect();
    let res = p.clips.iter().filter_map(|c| Some(Res::of(c.info.width?, c.info.height?))).collect();
    let gop = p.clips.iter().filter_map(|c| Gop::of(c.info.codec.as_deref()?, c.info.profile.as_deref())).collect();
    Summary {
        clips: p.clips.len(),
        bytes: p.total_bytes,
        first: times.iter().min().copied(),
        last: times.iter().max().copied(),
        fps: Measure::from(fps),
        res: Measure::from(res),
        gop: Measure::from(gop),
    }
}

impl Summary {
    pub fn fps_label(&self) -> String {
        self.fps.label(|f| f.label.clone())
    }
    pub fn res_label(&self) -> String {
        self.res.label(|r| r.as_str().to_string())
    }
    pub fn gop_label(&self) -> String {
        self.gop.label(|g| g.as_str().to_string())
    }
    /// `27/11/2026  11:30 – 15:43` (a range over midnight shows both dates).
    pub fn shoot_range(&self, st: &crate::time::DisplayStyle) -> Option<String> {
        let (a, b) = (self.first?, self.last?);
        let (y, m, d, h, mi, _) = crate::time::local(a);
        let (y2, m2, d2, h2, mi2, _) = crate::time::local(b);
        Some(if (y, m, d) == (y2, m2, d2) {
            format!("{}  {} – {}", st.date(y, m, d), st.clock(h, mi), st.clock(h2, mi2))
        } else {
            format!("{} {} – {} {}", st.date(y, m, d), st.clock(h, mi), st.date(y2, m2, d2), st.clock(h2, mi2))
        })
    }
}

/// A project's fixed requirements. All optional; empty = no check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Requirements {
    pub fps: Option<u32>,
    pub res: Option<Res>,
    pub gop: Option<Gop>,
}

impl Requirements {
    pub fn is_empty(&self) -> bool {
        self.fps.is_none() && self.res.is_none() && self.gop.is_none()
    }

    /// From the ingest flags `--require-fps`, `--require-resolution`, `--require-gop`.
    pub fn from_flags(fps: Option<&str>, res: Option<&str>, gop: Option<&str>) -> Result<Requirements, String> {
        let fps = match fps {
            None => None,
            Some(f) => Some(
                f.trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|n| FPS_LIST.contains(n))
                    .ok_or_else(|| format!("--require-fps {f:?} must be one of 24, 25, 30, 50, 60"))?,
            ),
        };
        let res = match res {
            None => None,
            Some(r) => Some(Res::parse(r).ok_or_else(|| format!("--require-resolution {r:?} must be 4K, HD or SD"))?),
        };
        let gop = match gop {
            None => None,
            Some(g) => Some(Gop::parse(g).ok_or_else(|| format!("--require-gop {g:?} must be intra or long-GOP"))?),
        };
        Ok(Requirements { fps, res, gop })
    }

    /// Stored form (database, magcard, gate): `fps=25 res=HD gop=intra`; empty = none.
    pub fn render(&self) -> String {
        let mut v = Vec::new();
        if let Some(f) = self.fps {
            v.push(format!("fps={f}"));
        }
        if let Some(r) = self.res {
            v.push(format!("res={}", r.as_str()));
        }
        if let Some(g) = self.gop {
            v.push(format!("gop={}", g.as_str()));
        }
        v.join(" ")
    }

    pub fn parse(s: &str) -> Requirements {
        let mut r = Requirements::default();
        for w in s.split_whitespace() {
            match w.split_once('=') {
                Some(("fps", v)) => r.fps = v.parse().ok().filter(|n| FPS_LIST.contains(n)),
                Some(("res", v)) => r.res = Res::parse(v),
                Some(("gop", v)) => r.gop = Gop::parse(v),
                _ => {}
            }
        }
        r
    }

    /// For the operator: `25 fps · HD · intra`, or `none`.
    pub fn display(&self) -> String {
        let mut v = Vec::new();
        if let Some(f) = self.fps {
            v.push(format!("{f} fps"));
        }
        if let Some(r) = self.res {
            v.push(r.as_str().to_string());
        }
        if let Some(g) = self.gop {
            v.push(g.as_str().to_string());
        }
        if v.is_empty() {
            "none".into()
        } else {
            v.join(" · ")
        }
    }

    /// What does not meet the requirements, in priority order (frame rate, resolution,
    /// GOP). A value the card does not report is not a mismatch (`unchecked` says so).
    pub fn check(&self, s: &Summary) -> Check {
        let mut c = Check::default();
        if let Some(want) = self.fps {
            match &s.fps {
                Measure::Unknown => c.unchecked.push("frame rate (not reported)".into()),
                m => {
                    let bad: Vec<String> = m.values().into_iter().filter(|f| f.class != Some(want)).map(|f| f.label).collect();
                    if !bad.is_empty() {
                        c.mismatches.push(format!("frame rate {} (project requires {want} fps)", s.fps_label()));
                    }
                }
            }
        }
        if let Some(want) = self.res {
            match &s.res {
                Measure::Unknown => c.unchecked.push("resolution (not reported)".into()),
                m => {
                    if m.values().into_iter().any(|r| r != want) {
                        c.mismatches.push(format!("resolution {} (project requires {})", s.res_label(), want.as_str()));
                    }
                }
            }
        }
        if let Some(want) = self.gop {
            match &s.gop {
                Measure::Unknown => c.unchecked.push("GOP (codec not reported)".into()),
                m => {
                    if m.values().into_iter().any(|g| g != want) {
                        c.mismatches.push(format!("GOP {} (project requires {})", s.gop_label(), want.as_str()));
                    }
                }
            }
        }
        c
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Check {
    pub mismatches: Vec<String>,
    pub unchecked: Vec<String>,
}

impl Check {
    pub fn ok(&self) -> bool {
        self.mismatches.is_empty()
    }
    /// One line for logs and the mag card.
    pub fn line(&self) -> String {
        self.mismatches.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::{Clip, MediaInfo, ProbedFile, Role};
    use std::path::PathBuf;

    fn probe(clips: &[(&str, i64, i64, &str, Option<&str>)]) -> Probe {
        let mut files = Vec::new();
        let mut cl = Vec::new();
        for (i, (fps, w, h, codec, profile)) in clips.iter().enumerate() {
            files.push(ProbedFile { rel: format!("C{i}.MP4"), abs: PathBuf::new(), size: 100, mtime: 1_764_243_000 + i as i64 * 600, role: Role::Media, clip: Some(i) });
            cl.push(Clip {
                file: i,
                info: MediaInfo {
                    fps: Some(fps.to_string()),
                    width: Some(*w),
                    height: Some(*h),
                    codec: Some(codec.to_string()),
                    profile: profile.map(String::from),
                    ..Default::default()
                },
            });
        }
        Probe { root: PathBuf::new(), total_bytes: files.len() as u64 * 100, files, clips: cl }
    }

    #[test]
    fn one_value_per_card_and_mixed() {
        let s = summarize(&probe(&[("25/1", 3840, 2160, "h264", Some("High")), ("25/1", 3840, 2160, "h264", None)]));
        assert_eq!((s.fps_label(), s.res_label(), s.gop_label()), ("25 fps".into(), "4K".into(), "long-GOP".into()));
        let s = summarize(&probe(&[
            ("25/1", 1920, 1080, "prores", None),
            ("50/1", 1920, 1080, "prores", None),
            ("25/1", 3840, 2160, "h264", Some("High 4:2:2 Intra")),
        ]));
        assert_eq!(s.fps_label(), "25 fps  mixed");
        assert_eq!(s.res_label(), "HD  mixed");
        assert_eq!(s.gop_label(), "intra");
        assert_eq!(s.clips, 3);
        assert_eq!(s.last.unwrap() - s.first.unwrap(), 1200);
    }

    #[test]
    fn ntsc_rates_count_as_listed() {
        assert_eq!(fps_value("24000/1001").and_then(fps_class), Some(24));
        assert_eq!(fps_value("30000/1001").and_then(fps_class), Some(30));
        assert_eq!(fps_value("60000/1001").and_then(fps_class), Some(60));
        assert_eq!(fps_value("120/1").and_then(fps_class), None);
        assert_eq!(fps_label(24000.0 / 1001.0), "23.976 fps");
    }

    #[test]
    fn requirements_check_in_priority_order() {
        let r = Requirements::from_flags(Some("25"), Some("HD"), Some("intra")).unwrap();
        assert_eq!(r.render(), "fps=25 res=HD gop=intra");
        assert_eq!(Requirements::parse(&r.render()), r);
        assert_eq!(r.display(), "25 fps · HD · intra");
        let s = summarize(&probe(&[("25/1", 3840, 2160, "h264", Some("High"))]));
        let c = r.check(&s);
        assert_eq!(c.mismatches, vec!["resolution 4K (project requires HD)".to_string(), "GOP long-GOP (project requires intra)".to_string()]);
        // Mixed frame rate fails when any clip differs.
        let s = summarize(&probe(&[("25/1", 1920, 1080, "prores", None), ("50/1", 1920, 1080, "prores", None)]));
        assert_eq!(r.check(&s).mismatches, vec!["frame rate 25 fps  mixed (project requires 25 fps)".to_string()]);
        // No media details: not a mismatch, reported as unchecked.
        let mut p = probe(&[("25/1", 1920, 1080, "prores", None)]);
        p.clips[0].info = MediaInfo::default();
        let c = r.check(&summarize(&p));
        assert!(c.ok() && c.unchecked.len() == 3, "{c:?}");
        assert!(Requirements::default().check(&s).ok());
        assert!(Requirements::from_flags(Some("23.976"), None, None).is_err());
        assert!(Requirements::from_flags(None, Some("8K"), None).is_err());
    }
}
