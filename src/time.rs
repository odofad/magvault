//! Date and time helpers (std only). Magvault stores UTC everywhere (log day boundary
//! is UTC). Local time is used for the guessed shoot day (a calendar date as the camera
//! crew saw it) and for what the operator sees (`DisplayStyle`, PLAN.md "Time display").

use std::ffi::{c_char, c_int, c_long};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub fn system_time_secs(t: SystemTime) -> i64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// Days since 1970-01-01 -> (year, month, day). Howard Hinnant's civil_from_days.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub struct Utc {
    pub y: i64,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
    pub s: u32,
}

pub fn utc(secs: i64) -> Utc {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    Utc { y, mo, d, h: (rem / 3600) as u32, mi: (rem % 3600 / 60) as u32, s: (rem % 60) as u32 }
}

/// `2026-11-22T19:21:03Z`
pub fn iso_utc(secs: i64) -> String {
    let t = utc(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", t.y, t.mo, t.d, t.h, t.mi, t.s)
}

/// `2026-11-22` (UTC day, used for log file names)
pub fn utc_date(secs: i64) -> String {
    let t = utc(secs);
    format!("{:04}-{:02}-{:02}", t.y, t.mo, t.d)
}

/// `20261122-192103` (UTC), the magcard backup stamp from PLAN.md "Conflict handling".
pub fn backup_stamp(secs: i64) -> String {
    let t = utc(secs);
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.y, t.mo, t.d, t.h, t.mi, t.s)
}

/// `2026-11-22_192103Z`, the date/time part of an ASC MHL manifest file name.
pub fn mhl_name_stamp(secs: i64) -> String {
    let t = utc(secs);
    format!("{:04}-{:02}-{:02}_{:02}{:02}{:02}Z", t.y, t.mo, t.d, t.h, t.mi, t.s)
}

/// Parse an ISO-8601 date-time as produced by ffprobe (`2025-05-20T14:02:11.000000Z`,
/// `2025-05-20 14:02:11`, or with a `+02:00` offset). No offset means UTC.
pub fn parse_iso(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, se) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    let mut rest = &s[19..];
    if let Some(r) = rest.strip_prefix('.') {
        let n = r.bytes().take_while(|b| b.is_ascii_digit()).count();
        rest = &r[n..];
    }
    let offset = match rest {
        "" | "Z" | "z" => 0,
        r if (r.starts_with('+') || r.starts_with('-')) && r.len() >= 6 => {
            let sign = if r.starts_with('-') { -1 } else { 1 };
            let oh: i64 = r.get(1..3)?.parse().ok()?;
            let om: i64 = r.get(4..6)?.parse().ok()?;
            sign * (oh * 3600 + om * 60)
        }
        _ => return None,
    };
    Some(days_from_civil(y, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + se - offset)
}

/// Parse `YYYY-MM-DD` (operator input) into (y, m, d), validating the calendar date.
pub fn parse_date(s: &str) -> Option<(i64, u32, u32)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || d == 0 {
        return None;
    }
    // Round-trip through days to reject 2025-02-30 and friends.
    let (y2, m2, d2) = civil_from_days(days_from_civil(y, m, d));
    if (y2, m2, d2) != (y, m, d) {
        return None;
    }
    Some((y, m, d))
}

#[repr(C)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

extern "C" {
    fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    fn tzset();
}

/// Local date and time of an epoch second, using the machine's time zone:
/// (year, month, day, hour, minute, second).
pub fn local(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    // SAFETY: tzset/localtime_r are thread-safe libc calls writing into our struct.
    let ok = unsafe {
        tzset();
        !localtime_r(&secs, &mut tm).is_null()
    };
    if ok {
        (
            tm.tm_year as i64 + 1900,
            tm.tm_mon as u32 + 1,
            tm.tm_mday as u32,
            tm.tm_hour as u32,
            tm.tm_min as u32,
            tm.tm_sec as u32,
        )
    } else {
        let t = utc(secs);
        (t.y, t.mo, t.d, t.h, t.mi, t.s)
    }
}

/// Local calendar date of an epoch second, using the machine's time zone.
pub fn local_date(secs: i64) -> (i64, u32, u32) {
    let (y, m, d, ..) = local(secs);
    (y, m, d)
}

/// How dates and times are shown to the operator (PLAN.md "Time display"): 24-hour and
/// DD/MM/YYYY by default; `date_format` and `time_format` in magvault.conf change them.
/// Stored values (magcard, database, logs) stay ISO / UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayStyle {
    pub order: DateOrder,
    pub h24: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateOrder {
    Dmy,
    Ymd,
    Mdy,
}

impl Default for DisplayStyle {
    fn default() -> Self {
        DisplayStyle { order: DateOrder::Dmy, h24: true }
    }
}

impl DisplayStyle {
    /// From `date_format` (DD/MM/YYYY, YYYY-MM-DD, MM/DD/YYYY) and `time_format` (24h, 12h).
    pub fn parse(date: Option<&str>, clock: Option<&str>) -> Result<DisplayStyle, String> {
        let order = match date.map(str::trim).filter(|s| !s.is_empty()) {
            None | Some("DD/MM/YYYY") => DateOrder::Dmy,
            Some("YYYY-MM-DD") => DateOrder::Ymd,
            Some("MM/DD/YYYY") => DateOrder::Mdy,
            Some(o) => return Err(format!("date_format must be DD/MM/YYYY, YYYY-MM-DD or MM/DD/YYYY, not {o:?}")),
        };
        let h24 = match clock.map(str::trim).filter(|s| !s.is_empty()) {
            None | Some("24h") => true,
            Some("12h") => false,
            Some(o) => return Err(format!("time_format must be 24h or 12h, not {o:?}")),
        };
        Ok(DisplayStyle { order, h24 })
    }

    pub fn date(&self, y: i64, m: u32, d: u32) -> String {
        match self.order {
            DateOrder::Dmy => format!("{d:02}/{m:02}/{y:04}"),
            DateOrder::Ymd => format!("{y:04}-{m:02}-{d:02}"),
            DateOrder::Mdy => format!("{m:02}/{d:02}/{y:04}"),
        }
    }

    /// An ISO date `YYYY-MM-DD` in this style (unchanged when it does not parse).
    pub fn iso_date(&self, iso: &str) -> String {
        match parse_date(iso) {
            Some((y, m, d)) => self.date(y, m, d),
            None => iso.to_string(),
        }
    }

    /// An operator-typed date in this style or ISO, as ISO `YYYY-MM-DD`.
    pub fn read_date(&self, s: &str) -> Option<String> {
        let s = s.trim();
        if parse_date(s).is_some() {
            return Some(s.to_string());
        }
        let p: Vec<&str> = s.split('/').collect();
        let [a, b, c] = p.as_slice() else { return None };
        let (d, m, y) = match self.order {
            DateOrder::Mdy => (b, a, c),
            _ => (a, b, c),
        };
        let (y, m, d): (i64, u32, u32) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
        let iso = format!("{y:04}-{m:02}-{d:02}");
        parse_date(&iso).map(|_| iso)
    }

    pub fn clock(&self, h: u32, mi: u32) -> String {
        if self.h24 {
            format!("{h:02}:{mi:02}")
        } else {
            let (h12, ap) = match h {
                0 => (12, "am"),
                1..=11 => (h, "am"),
                12 => (12, "pm"),
                _ => (h - 12, "pm"),
            };
            format!("{h12}:{mi:02} {ap}")
        }
    }

    pub fn clock_secs(&self, h: u32, mi: u32, s: u32) -> String {
        if self.h24 {
            format!("{h:02}:{mi:02}:{s:02}")
        } else {
            let c = self.clock(h, mi);
            let (hm, ap) = c.split_once(' ').unwrap_or((&c, ""));
            format!("{hm}:{s:02} {ap}")
        }
    }

    /// Local date and time of an epoch second: `28/11/2026  01:12:07`.
    pub fn now_line(&self, secs: i64) -> String {
        let (y, m, d, h, mi, s) = local(secs);
        format!("{}  {}", self.date(y, m, d), self.clock_secs(h, mi, s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_days() {
        for days in [-1000i64, -1, 0, 1, 10_957, 20_000, 30_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_414), (2025, 11, 22));
    }

    #[test]
    fn formats() {
        assert_eq!(iso_utc(1_763_839_263), "2025-11-22T19:21:03Z");
        assert_eq!(backup_stamp(1_763_839_263), "20251122-192103");
        assert_eq!(mhl_name_stamp(1_763_839_263), "2025-11-22_192103Z");
    }

    #[test]
    fn parse_ffprobe_times() {
        assert_eq!(parse_iso("2025-11-22T19:21:03.000000Z"), Some(1_763_839_263));
        assert_eq!(parse_iso("2025-11-22 19:21:03"), Some(1_763_839_263));
        assert_eq!(parse_iso("2025-11-22T21:21:03+02:00"), Some(1_763_839_263));
        assert_eq!(parse_iso("garbage"), None);
    }

    #[test]
    fn display_style() {
        let st = DisplayStyle::default();
        assert_eq!(st.date(2026, 11, 28), "28/11/2026");
        assert_eq!(st.clock(9, 5), "09:05");
        assert_eq!(st.iso_date("2026-11-27"), "27/11/2026");
        assert_eq!(st.read_date("27/11/2026").as_deref(), Some("2026-11-27"));
        assert_eq!(st.read_date("2026-11-27").as_deref(), Some("2026-11-27"));
        assert_eq!(st.read_date("31/02/2026"), None);
        let us = DisplayStyle::parse(Some("MM/DD/YYYY"), Some("12h")).unwrap();
        assert_eq!(us.date(2026, 11, 28), "11/28/2026");
        assert_eq!(us.read_date("11/27/2026").as_deref(), Some("2026-11-27"));
        assert_eq!(us.clock(0, 7), "12:07 am");
        assert_eq!(us.clock(15, 43), "3:43 pm");
        assert_eq!(us.clock_secs(15, 43, 9), "3:43:09 pm");
        assert!(DisplayStyle::parse(Some("D.M.Y"), None).is_err());
    }

    #[test]
    fn dates() {
        assert_eq!(parse_date("2025-05-20"), Some((2025, 5, 20)));
        assert_eq!(parse_date("2025-02-30"), None);
        assert_eq!(parse_date("20250520"), None);
    }
}
