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
//!   status <folder>    queued | verifying | committed | error: … | unknown
//!   projects           open projects as `uuid | name` lines
//!   ping               connectivity check
//!
//! Results are written by `magvault serve` (as `magvault`) into a folder the client can read
//! but not write, so a client cannot forge "committed".

use crate::error::{Context, Error, Result};
use crate::ingest::is_mag_folder;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

extern "C" {
    fn umask(mask: u32) -> u32;
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
