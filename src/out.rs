//! Operator-facing output of the ingest / sync / Prepare / Clean code. On the command line
//! it goes to stderr (`say!`) or stdout (`out!`) as before. While the TUI runs one of these
//! steps, it sets a sink and the same lines and progress arrive in the TUI instead, so the
//! TUI never drops to the raw terminal and runs exactly the code the CLI runs.

use std::sync::mpsc::Sender;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    /// A line the CLI would print (`err` = stderr).
    Line { err: bool, text: String },
    /// Progress of the current stage (hashing, copying, verifying, uploading, …).
    Progress { label: String, done: u64, total: u64 },
}

static SINK: Mutex<Option<Sender<Msg>>> = Mutex::new(None);

pub fn set_sink(s: Option<Sender<Msg>>) {
    *SINK.lock().unwrap() = s;
}

pub fn active() -> bool {
    SINK.lock().unwrap().is_some()
}

/// Send to the TUI if a sink is set; `false` means: print it yourself.
pub fn send(m: Msg) -> bool {
    match SINK.lock().unwrap().as_ref() {
        Some(tx) => {
            let _ = tx.send(m);
            true
        }
        None => false,
    }
}

pub fn line(err: bool, text: String) {
    if !send(Msg::Line { err, text: text.clone() }) {
        if err {
            eprintln!("{text}");
        } else {
            println!("{text}");
        }
    }
}

/// Operator message on stderr (or into the TUI).
#[macro_export]
macro_rules! say {
    () => { $crate::out::line(true, String::new()) };
    ($($t:tt)*) => { $crate::out::line(true, format!($($t)*)) };
}

/// Result line on stdout (or into the TUI).
#[macro_export]
macro_rules! out {
    () => { $crate::out::line(false, String::new()) };
    ($($t:tt)*) => { $crate::out::line(false, format!($($t)*)) };
}
