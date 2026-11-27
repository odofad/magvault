//! Daily log files `logs/YYYY-MM-DD.log` (UTC day), one entry per line:
//! `2026-11-22T21:15:03Z  INFO   ingest    message` (PLAN.md "Logs module").
//! Context lines start with two spaces and are written in the same write(2) as their entry.
//! Every control character in a message or context line is escaped (`\u{1b}`, `\n`), so a
//! card file name cannot inject lines or terminal sequences (PLAN.md Threat model).

use crate::time;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
    pub fn parse(s: &str) -> Option<Level> {
        Some(match s {
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" => Level::Warn,
            "ERROR" => Level::Error,
            _ => return None,
        })
    }
}

// ---- process-wide settings (PLAN.md "Debug logging") ---------------------------------------
//
// `configure` is called once the config is loaded: the log folder for code that has no
// `Log` at hand (rsync, hash verify, helper calls), whether DEBUG lines are written
// (`debug: yes` in magvault.conf, or --verbose / --debug), and on a client the forwarder
// that queues lines for the server log (client issues).

static DEBUG: AtomicBool = AtomicBool::new(false);
static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
type Forward = fn(Level, &str, &str);
static FORWARD: Mutex<Option<Forward>> = Mutex::new(None);

pub fn configure(dir: &Path, debug: bool) {
    *DIR.lock().unwrap() = Some(dir.to_path_buf());
    DEBUG.store(debug, Ordering::SeqCst);
}

pub fn set_forward(f: Option<Forward>) {
    *FORWARD.lock().unwrap() = f;
}

pub fn debug_enabled() -> bool {
    DEBUG.load(Ordering::SeqCst)
}

/// A DEBUG line in the configured log folder (no-op when debug is off or no folder is set).
pub fn dbg(module: &str, msg: &str) {
    if !debug_enabled() {
        return;
    }
    let dir = DIR.lock().unwrap().clone();
    if let Some(d) = dir {
        Log::new(&d).write(Level::Debug, module, msg, &[]);
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
        // Client: DEBUG lines are also queued for the server log (operational problems are
        // queued explicitly, see client::issue).
        let fwd = *FORWARD.lock().unwrap();
        if let (Some(f), Level::Debug) = (fwd, level) {
            f(level, module, message);
        }
    }

    pub fn debug(&self, module: &str, msg: &str) {
        if debug_enabled() {
            self.write(Level::Debug, module, msg, &[]);
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
