//! `magvault gate` — the forced command for client SSH keys (PLAN.md Implementation
//! defaults: "forced-command gate", "per-client incoming spool via rrsync -wo", "clients
//! never write directly to mags/"). Runs as `magvaultclient` on the server. The key line
//! (served by AuthorizedKeysCommand later, hand-written in Phase 1):
//!
//!   restrict,command="/usr/local/bin/magvault gate --spool /magvault/mags/.incoming/.clients \
//!     --results /magvault/mags/.incoming/.results --client dit-a" ssh-ed25519 AAAA… dit-a
//!
//! Allowed commands (`SSH_ORIGINAL_COMMAND`), nothing else:
//!   rsync --server …   upload only, into this client's spool (exec `rrsync -wo <spool>/<client>`)
//!   commit <folder>    ask the server to verify and commit an uploaded mag
//!   status <folder>    queued | verifying | committed | error: … | unknown, then the
//!                      committer line (see ping)
//!   projects           open projects as `uuid | name` lines
//!   ping               `magvault-gate ok <client>`, then `serve alive` or `serve down: …`
//!                      (the heartbeat `magvault serve` writes while it accepts commits)
//!   log                client issue / debug lines on stdin (`LEVEL message`), queued for
//!                      the server log (PLAN.md "Client issues on the server log")
//!
//! Results are written by `magvault serve` (as `magvault`) into a folder the client can read
//! but not write, so a client cannot forge "committed".

use crate::error::{Context, Error, Result};
use crate::ingest::is_mag_folder;
use crate::log::Level;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

extern "C" {
    fn umask(mask: u32) -> u32;
}

/// `serve.alive` in the results folder: `alive <unix> <pid> <state>` every 2 s while
/// `magvault serve` runs, `stopped <unix>` when it stops.
pub const ALIVE_FILE: &str = "serve.alive";
/// A heartbeat older than this means the committer is down.
pub const ALIVE_MAX_AGE: u64 = 10;
/// The per-client file the gate appends client lines to and `serve` drains.
pub const CLIENT_LOG: &str = ".client-log";
const CLIENT_LOG_MAX: u64 = 256 * 1024;
const LOG_INPUT_MAX: u64 = 64 * 1024;
const LINE_MAX: usize = 300;

/// `serve alive` or `serve down: <why>`, from the heartbeat.
pub fn serve_state(results: &Path) -> String {
    let now = crate::time::now_secs().max(0) as u64;
    let text = std::fs::read_to_string(results.join(ALIVE_FILE)).unwrap_or_default();
    let w: Vec<&str> = text.split_whitespace().collect();
    let ts = |s: &&str| s.parse::<u64>().ok();
    match w.as_slice() {
        ["alive", t, ..] => match ts(t) {
            Some(t) if now.saturating_sub(t) <= ALIVE_MAX_AGE => "serve alive".into(),
            Some(t) => format!("serve down: no heartbeat for {} s", now.saturating_sub(t)),
            None => "serve down: unreadable heartbeat".into(),
        },
        ["stopped", t, ..] => match ts(t) {
            Some(t) => format!("serve down: stopped at {}", crate::time::iso_utc(t as i64)),
            None => "serve down: stopped".into(),
        },
        _ => "serve down: not running".into(),
    }
}

/// One client line, made safe for the server log: printable, one line, bounded.
pub fn clean_line(s: &str) -> String {
    let mut t: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if t.chars().count() > LINE_MAX {
        t = t.chars().take(LINE_MAX).collect::<String>() + "…";
    }
    t.trim().to_string()
}

/// Append lines to the client's queue for the server log (bounded; drops when full).
fn queue_lines(mine: &Path, lines: &[(Level, String)]) -> usize {
    let path = mine.join(CLIENT_LOG);
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size >= CLIENT_LOG_MAX || lines.is_empty() {
        return 0;
    }
    let now = crate::time::now_secs();
    let text: String = lines.iter().map(|(l, m)| format!("{now} {} {}\n", l.as_str(), clean_line(m))).collect();
    let ok = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .is_ok();
    if ok {
        lines.len()
    } else {
        0
    }
}

pub fn valid_client(l: &str) -> bool {
    let b = l.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

pub fn run(spool: &Path, results: &Path, client: &str, rrsync: &Path) -> Result<()> {
    if !valid_client(client) {
        crate::bail!("invalid client label");
    }
    let cmd = std::env::var("SSH_ORIGINAL_COMMAND")
        .map_err(|_| Error("this key only runs the Magvault gate".into()))?;
    // Files a client uploads stay group-readable/writable for the server user.
    // SAFETY: plain libc call.
    unsafe {
        umask(0o007);
    }
    let mine = spool.join(client);
    if !mine.exists() {
        std::fs::create_dir_all(&mine).ctx("cannot create the client spool")?;
        let _ = std::fs::set_permissions(&mine, std::fs::Permissions::from_mode(0o2770));
    }
    let words: Vec<&str> = cmd.split_whitespace().collect();
    // Gate verbs for the server's DEBUG log (written by serve only when its debug is on).
    let verb = match words.as_slice() {
        ["rsync", "--server", ..] => "rsync upload".to_string(),
        [v, a] if *v == "commit" || *v == "status" => format!("{v} {}", clean_line(a)),
        [v] => clean_line(v),
        _ => "refused command".to_string(),
    };
    if verb != "log" {
        queue_lines(&mine, &[(Level::Debug, format!("gate {verb}"))]);
    }
    match words.as_slice() {
        ["rsync", "--server", rest @ ..] => {
            if rest.contains(&"--sender") {
                crate::bail!("only uploads are allowed");
            }
            let err = Command::new(rrsync).arg("-wo").arg(&mine).exec();
            Err(Error(format!("cannot run {}: {err}", rrsync.display())))
        }
        ["ping"] => {
            println!("magvault-gate ok {client}");
            println!("{}", serve_state(results));
            Ok(())
        }
        ["log"] => {
            let mut input = String::new();
            std::io::stdin().take(LOG_INPUT_MAX).read_to_string(&mut input).ctx("cannot read log lines")?;
            let lines: Vec<(Level, String)> = input
                .lines()
                .filter_map(|l| {
                    let (lv, msg) = l.split_once(' ')?;
                    Some((Level::parse(lv)?, msg.to_string()))
                })
                .collect();
            let n = queue_lines(&mine, &lines);
            if n == lines.len() {
                println!("logged {n}");
            } else {
                println!("log full; {} dropped", lines.len() - n);
            }
            Ok(())
        }
        ["projects"] => {
            print!(
                "{}",
                std::fs::read_to_string(results.join("projects.txt")).unwrap_or_default()
            );
            Ok(())
        }
        ["commit", folder] => {
            check_folder(folder)?;
            let dir = mine.join(folder);
            if !std::fs::symlink_metadata(&dir)
                .map(|m| m.is_dir())
                .unwrap_or(false)
            {
                crate::bail!("{folder} has not been uploaded");
            }
            std::fs::write(mine.join(format!(".{folder}.ready")), b"")
                .ctx("cannot queue the commit")?;
            println!("queued");
            Ok(())
        }
        ["status", folder] => {
            check_folder(folder)?;
            if mine.join(format!(".{folder}.ready")).exists() {
                println!("queued");
            } else if let Ok(r) =
                std::fs::read_to_string(results.join(client).join(format!("{folder}.result")))
            {
                println!("{}", r.trim());
            } else {
                println!("unknown");
            }
            println!("{}", serve_state(results));
            Ok(())
        }
        _ => Err(Error("command refused by the Magvault gate".into())),
    }
}

fn check_folder(f: &str) -> Result<()> {
    if !is_mag_folder(f) {
        crate::bail!("not a mag folder name");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_state_from_heartbeat() {
        let d = std::env::temp_dir().join(format!("mv-alive-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(serve_state(&d), "serve down: not running");
        let now = crate::time::now_secs();
        std::fs::write(d.join(ALIVE_FILE), format!("alive {now} 42 idle\n")).unwrap();
        assert_eq!(serve_state(&d), "serve alive");
        std::fs::write(d.join(ALIVE_FILE), format!("alive {} 42 idle\n", now - 60)).unwrap();
        assert!(serve_state(&d).starts_with("serve down: no heartbeat for 6"), "{}", serve_state(&d));
        std::fs::write(d.join(ALIVE_FILE), format!("stopped {now}\n")).unwrap();
        assert!(serve_state(&d).starts_with("serve down: stopped at 20"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn client_lines_are_one_bounded_printable_line() {
        assert_eq!(clean_line("a\nb\x1b[31mc\t"), "a b [31mc");
        let long = "x".repeat(1000);
        assert_eq!(clean_line(&long).chars().count(), LINE_MAX + 1);
    }
}
