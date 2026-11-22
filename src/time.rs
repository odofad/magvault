//! Date and time helpers (std only). Magvault stores UTC everywhere (log day boundary
//! is UTC). The only local-time value is the guessed shoot day, which is a calendar
//! date as the camera crew saw it.

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

/// Local calendar date of an epoch second, using the machine's time zone.
pub fn local_date(secs: i64) -> (i64, u32, u32) {
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
        (tm.tm_year as i64 + 1900, tm.tm_mon as u32 + 1, tm.tm_mday as u32)
    } else {
        let t = utc(secs);
        (t.y, t.mo, t.d)
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
    fn dates() {
        assert_eq!(parse_date("2025-05-20"), Some((2025, 5, 20)));
        assert_eq!(parse_date("2025-02-30"), None);
        assert_eq!(parse_date("20250520"), None);
    }
}
