//! Small helpers: UUIDs, identity, free space, prompts, formatting.

use crate::error::{Context, Result};
use std::io::{BufRead, IsTerminal, Read, Write};
use std::path::Path;
use std::process::Command;

/// Random (version 4) UUID, lowercase canonical form.
pub fn uuid_v4() -> Result<String> {
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .ctx("cannot read /dev/urandom")?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = crate::hash::hex(&b);
    Ok(format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]))
}

#[cfg(test)]
pub fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit() && !c.is_ascii_uppercase(),
        })
}

pub fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".into())
}

/// Who did it (tech spec M12): `<user>@<host>`. On the server the user is `magvault`.
pub fn identity() -> String {
    let user = std::env::var("USER")
        .ok()
        .filter(|u| !u.is_empty())
        .or_else(|| {
            Command::new("id").arg("-un").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    format!("{user}@{}", hostname())
}

/// Bytes available to this user on the filesystem holding `path` (`df`, coreutils).
pub fn free_bytes(path: &Path) -> Result<u64> {
    let out = Command::new("df")
        .args(["-B1", "--output=avail"])
        .arg(path)
        .env("LC_ALL", "C")
        .output()
        .ctx("cannot run df")?;
    if !out.status.success() {
        crate::bail!("df failed for {}: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .nth(1)
        .and_then(|l| l.trim().parse().ok())
        .ok_or_else(|| crate::error::Error(format!("cannot parse df output for {}", path.display())))
}

pub fn human(bytes: u64) -> String {
    const U: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1000.0 && i < U.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

/// Ask a yes/no question on the terminal. Refuses when stdin is not a terminal.
pub fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        crate::bail!("confirmation needed but stdin is not a terminal; re-run with --yes after checking the values");
    }
    eprint!("{question} [y/N] ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
}

/// Progress line on stderr, throttled to once per ~half second.
pub struct Progress {
    label: String,
    total: u64,
    done: u64,
    last: std::time::Instant,
    tty: bool,
}

impl Progress {
    pub fn new(label: &str, total: u64) -> Self {
        Progress {
            label: label.into(),
            total,
            done: 0,
            last: std::time::Instant::now() - std::time::Duration::from_secs(1),
            tty: std::io::stderr().is_terminal(),
        }
    }
    pub fn add(&mut self, n: u64) {
        self.done += n;
        if self.tty && self.last.elapsed().as_millis() >= 500 {
            self.draw();
            self.last = std::time::Instant::now();
        }
    }
    fn draw(&self) {
        let pct = (self.done * 100).checked_div(self.total).unwrap_or(100);
        eprint!("\r{}  {} / {}  ({pct}%)   ", self.label, human(self.done), human(self.total));
    }
    pub fn finish(&self) {
        if self.tty {
            self.draw();
            eprintln!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_format() {
        let u = uuid_v4().unwrap();
        assert!(is_uuid(&u), "{u}");
        assert_eq!(&u[14..15], "4");
        assert!(!is_uuid("3F6C2A1E-9B0D-4C55-8E21-A3F9C2D41B77"));
    }

    #[test]
    fn human_sizes() {
        assert_eq!(human(999), "999 B");
        assert_eq!(human(42_700_000_000), "42.7 GB");
    }
}
