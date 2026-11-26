//! Daily log files `logs/YYYY-MM-DD.log` (UTC day), one entry per line:
//! `2026-11-22T21:15:03Z  INFO   ingest    message` (PLAN.md "Logs module").
//! Context lines start with two spaces and are written in the same write(2) as their entry.
//! Every control character in a message or context line is escaped (`\u{1b}`, `\n`), so a
//! card file name cannot inject lines or terminal sequences (PLAN.md Threat model).

use crate::time;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub struct Log {
    dir: PathBuf,
}

impl Log {
    pub fn new(dir: &Path) -> Log {
        Log { dir: dir.to_path_buf() }
    }

    pub fn write(&self, level: Level, module: &str, message: &str, context: &[String]) {
        let now = time::now_secs();
        let one_line = |s: &str| crate::util::esc(s);
        let mut entry = format!("{}  {:<5}  {:<8}  {}\n", time::iso_utc(now), level.as_str(), module, one_line(message));
        for c in context {
            entry.push_str("  ");
            entry.push_str(&one_line(c));
            entry.push('\n');
        }
        let path = self.dir.join(format!("{}.log", time::utc_date(now)));
        let res = std::fs::create_dir_all(&self.dir)
            .and_then(|_| OpenOptions::new().create(true).append(true).open(&path))
            .and_then(|mut f| f.write_all(entry.as_bytes()));
        if let Err(e) = res {
            eprintln!("warning: cannot write log {}: {e}", path.display());
        }
    }

    pub fn info(&self, module: &str, msg: &str) {
        self.write(Level::Info, module, msg, &[]);
    }
    pub fn warn(&self, module: &str, msg: &str) {
        self.write(Level::Warn, module, msg, &[]);
    }
    pub fn error(&self, module: &str, msg: &str, context: &[String]) {
        self.write(Level::Error, module, msg, context);
    }
}
