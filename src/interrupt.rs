//! Ctrl+C / SIGTERM during an ingest (PLAN.md Threat model: "Incoming cleaned on failure").
//!
//! The handler is installed only after the operator has confirmed the pull; before that
//! nothing has been written and the default behaviour (exit at once) is right, including at
//! the confirmation prompt. The handler only records the signal. Long loops (hashing,
//! verifying) and the rsync wait check it and stop; the ingest error path then removes the
//! staging folder and its list and lock files and logs ERROR. The rename into `mags/` and
//! the database commit do not check it, so a signal arriving there is held off until the
//! commit is done. SIGKILL cannot be caught: the leftover is reported by the next ingest.

use crate::error::Error;
use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;

static RECEIVED: AtomicI32 = AtomicI32::new(0);

extern "C" {
    fn signal(signum: c_int, handler: usize) -> usize;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

extern "C" fn on_signal(sig: c_int) {
    // Async-signal-safe: a single atomic store.
    RECEIVED.store(sig, Ordering::SeqCst);
}

/// Catch SIGINT and SIGTERM from now on. Children started later get the default
/// disposition back on exec, so rsync still dies on Ctrl+C.
pub fn install() {
    let handler: extern "C" fn(c_int) = on_signal;
    // SAFETY: installing a handler that only performs an atomic store.
    unsafe {
        signal(SIGINT, handler as usize);
        signal(SIGTERM, handler as usize);
    }
}

/// Stop a child (rsync) with SIGTERM so it can stop its own helper processes, then wait;
/// SIGKILL only if it has not exited after 10 s.
pub fn stop_child(child: &mut std::process::Child) {
    // SAFETY: plain kill(2) on our own child's pid.
    unsafe {
        kill(child.id() as c_int, SIGTERM);
    }
    for _ in 0..100 {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn requested() -> bool {
    RECEIVED.load(Ordering::SeqCst) != 0
}

pub fn error() -> Error {
    let name = match RECEIVED.load(Ordering::SeqCst) {
        SIGTERM => "SIGTERM",
        _ => "SIGINT",
    };
    Error(format!("interrupted by operator ({name}); nothing committed"))
}

/// `Err` if a signal has arrived.
pub fn check() -> crate::error::Result<()> {
    if requested() {
        Err(error())
    } else {
        Ok(())
    }
}
