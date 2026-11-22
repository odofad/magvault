//! Shared plain-text helpers: `key: value` parsing and atomic writes.

use crate::error::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Parse a config-style file (`path.conf`, `magvault.conf`, `.cam`, `.enc`).
/// - `#` at line start is a comment; ` #` after a value starts an inline comment
///   (PLAN.md's `.enc` example uses `type: 320p   # 320p | hq | custom`).
/// - Separator is `:`; `key=value` is also accepted (PLAN.md's telemetry example).
pub fn parse_config(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let colon = t.find(':');
        let eq = t.find('=');
        let idx = match (colon, eq) {
            (Some(c), Some(e)) => c.min(e),
            (Some(c), None) => c,
            (None, Some(e)) => e,
            (None, None) => continue,
        };
        let key = t[..idx].trim();
        if key.is_empty() || key.contains(char::is_whitespace) {
            continue;
        }
        let mut value = &t[idx + 1..];
        if let Some(pos) = find_inline_comment(value) {
            value = &value[..pos];
        }
        out.push((key.to_string(), value.trim().to_string()));
    }
    out
}

fn find_inline_comment(v: &str) -> Option<usize> {
    let b = v.as_bytes();
    (1..b.len()).find(|&i| b[i] == b'#' && (b[i - 1] == b' ' || b[i - 1] == b'\t'))
}

/// Comma-separated list, trimmed, empty items dropped.
pub fn split_list(v: &str) -> Vec<String> {
    v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// A value that will be written into a card (header value or list-section field).
/// Pipes and control characters are refused (PLAN.md Implementation defaults).
pub fn check_card_field(what: &str, v: &str) -> Result<()> {
    if v.contains('|') {
        crate::bail!("{what} contains '|', which is the card field separator: {v:?}");
    }
    if v.chars().any(|c| c.is_control()) {
        crate::bail!("{what} contains a control character (newline, tab, …): {v:?}");
    }
    Ok(())
}

/// Write via `<name>.tmp` + fsync + rename + fsync(dir).
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.tmp"));
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .ctx(format!("cannot write {}", tmp.display()))?;
        f.write_all(data).ctx(format!("cannot write {}", tmp.display()))?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).ctx(format!("cannot replace {}", path.display()))?;
    sync_dir(dir);
    Ok(())
}

pub fn sync_dir(dir: &Path) {
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parsing() {
        let t = "# comment\nname: default-320p\ntype: 320p          # 320p | hq | custom\n\
                 telemetry_live=/dev/shm/x.json\nnotes: Shot#3 stays\r\nbad line\n";
        let kv = parse_config(t);
        assert_eq!(kv[0], ("name".into(), "default-320p".into()));
        assert_eq!(kv[1], ("type".into(), "320p".into()));
        assert_eq!(kv[2], ("telemetry_live".into(), "/dev/shm/x.json".into()));
        assert_eq!(kv[3], ("notes".into(), "Shot#3 stays".into()));
        assert_eq!(kv.len(), 4);
    }

    #[test]
    fn card_fields() {
        assert!(check_card_field("x", "fine value").is_ok());
        assert!(check_card_field("x", "a|b").is_err());
        assert!(check_card_field("x", "a\nb").is_err());
    }
}
