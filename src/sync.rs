//! Reconnect-and-sync (PLAN.md "Client ingest and field backups"): send every pending mag
//! (local staging and mounted field sets) to the server.
//!
//! For each mag: re-verify the local copy against the SHA-256 in its `magcard.txt`, upload
//! it, ask the server to commit, wait for `committed`. Only then: a field-set mag is marked
//! synced on both drives; a local-staging mag is removed (autoclean). Anything that fails
//! stays pending.

use crate::catalog;
use crate::client::Transport;
use crate::config::ClientConfig;
use crate::error::{Context, Error, Result};
use crate::fieldset::{self, SetStatus};
use crate::ingest::{self, is_mag_folder};
use crate::interrupt;
use crate::log::Log;
use crate::magcard::{self, MagCard};
use crate::util::esc;
use std::path::{Path, PathBuf};

/// Verify a local mag copy, upload it, commit, and wait for the server's confirmation.
pub fn send(t: &Transport, cc: &ClientConfig, mag_dir: &Path, folder: &str) -> Result<()> {
    t.wait_ready(cc.committer_wait, "sync")?;
    let text = std::fs::read_to_string(mag_dir.join(magcard::FILE_NAME))
        .ctx(format!("{folder} has no readable magcard.txt"))?;
    let card = MagCard::parse(&text).map_err(|e| Error(format!("{folder}: magcard.txt: {e}")))?;
    let rec = catalog::record_from_card(&card)?;
    if rec.folder != folder {
        crate::bail!("{folder}: magcard belongs to {}", rec.folder);
    }
    let expected: Vec<(&str, u64, &str)> = rec
        .manifest
        .iter()
        .map(|m| (m.rel.as_str(), m.size, m.sha256.as_str()))
        .collect();
    ingest::verify_tree(mag_dir, &expected, "verifying local copy")?;
    let mut rels: Vec<String> = rec.manifest.iter().map(|m| m.rel.clone()).collect();
    rels.extend(crate::client_ingest::internal_files(mag_dir)?);
    let refs: Vec<&str> = rels.iter().map(String::as_str).collect();
    let tmp = cc.staging.join(".incoming");
    std::fs::create_dir_all(&tmp).ctx(format!("cannot create {}", tmp.display()))?;
    let list = tmp.join(format!(".{folder}.sync-files"));
    let r = (|| -> Result<()> {
        t.upload(mag_dir, &refs, folder, &list)?;
        t.commit(folder)?;
        t.wait_committed(folder, cc.committer_wait)
    })();
    let _ = std::fs::remove_file(&list);
    r
}

pub struct Report {
    pub synced: usize,
    pub failed: usize,
    pub waiting: Vec<String>,
}

pub fn run(cc: &ClientConfig, scan_roots: &[PathBuf], set_label: Option<&str>) -> Result<Report> {
    let log = Log::new(&cc.logs);
    let t = Transport::new(cc)
        .ok_or_else(|| Error("no server configured (server_host in path.conf)".into()))?;
    t.ping().map_err(|e| {
        Error(format!(
            "the server is not reachable; nothing synced, mags stay pending ({})",
            e.0
        ))
    })?;
    // A committer that stays down ends the whole sync (each mag would wait again).
    let mut committer_down = false;
    interrupt::install();
    let mut rep = Report {
        synced: 0,
        failed: 0,
        waiting: Vec::new(),
    };

    // Local staging: pending until confirmed, then removed.
    let mut staged: Vec<String> = std::fs::read_dir(&cc.staging)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| is_mag_folder(n))
                .collect()
        })
        .unwrap_or_default();
    staged.sort();
    for folder in staged {
        if interrupt::requested() || committer_down {
            break;
        }
        let dir = cc.staging.join(&folder);
        crate::say!("sync {folder} (local staging) …");
        match send(&t, cc, &dir, &folder) {
            Ok(()) => {
                std::fs::remove_dir_all(&dir).ctx("autoclean of local staging")?;
                log.info(
                    "sync",
                    &format!("Mag {folder} committed on the server; removed from local staging"),
                );
                crate::out!("synced    {folder}  (local staging, cleaned)");
                rep.synced += 1;
            }
            Err(e) => {
                committer_down = e.0.contains(crate::client::COMMITTER_DOWN);
                fail(&log, &mut rep, &folder, &e)
            }
        }
    }

    // Field sets: master is the sync source; synced is recorded on both drives.
    for mut set in fieldset::sets(&fieldset::drives(scan_roots)) {
        if set_label.is_some_and(|l| l != set.label) {
            continue;
        }
        let pending: Vec<String> = set.pending().iter().map(|m| m.folder.clone()).collect();
        if pending.is_empty() {
            continue;
        }
        if set.status != SetStatus::Valid {
            let why = match &set.status {
                SetStatus::Incomplete(r) | SetStatus::Inconsistent(r) => r.clone(),
                SetStatus::Valid => String::new(),
            };
            crate::out!(
                "waiting   set {}: {} pending mag(s), not synced: {why} (mount both drives)",
                set.label,
                pending.len()
            );
            rep.waiting.push(set.label.clone());
            continue;
        }
        let master = set.roots()?.0.to_path_buf();
        for folder in pending {
            if interrupt::requested() || committer_down {
                break;
            }
            crate::say!("sync {folder} (field set {}) …", set.label);
            match send(&t, cc, &master.join(&folder), &folder) {
                Ok(()) => {
                    fieldset::mark_synced(&mut set, &folder)?;
                    log.info(
                        "sync",
                        &format!(
                            "Mag {folder} committed on the server; marked synced on field set {}",
                            set.label
                        ),
                    );
                    crate::out!("synced    {folder}  (field set {})", set.label);
                    rep.synced += 1;
                }
                Err(e) => {
                    committer_down = e.0.contains(crate::client::COMMITTER_DOWN);
                    fail(&log, &mut rep, &folder, &e)
                }
            }
        }
    }
    Ok(rep)
}

fn fail(log: &Log, rep: &mut Report, folder: &str, e: &Error) {
    if !interrupt::requested() {
        crate::client::queue_issue(
            crate::log::Level::Error,
            &format!("sync: Mag {folder} failed, stays pending: {}", e.0.lines().next().unwrap_or("")),
        );
    }
    crate::out!(
        "FAILED    {folder}  {}",
        esc(e.0.lines().next().unwrap_or(""))
    );
    log.write(
        crate::log::Level::Error,
        "sync",
        &format!("Mag {folder} not synced; stays pending"),
        &e.0.lines().map(String::from).collect::<Vec<_>>(),
    );
    rep.failed += 1;
}
