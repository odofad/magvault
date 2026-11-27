//! Reconnect-and-sync (PLAN.md "Client ingest and field backups"): send every pending mag
//! (local staging and mounted field sets) to the server.
//!
//! For each mag: check its camera values against the server's profiles (nothing is
//! uploaded when the server would refuse them), re-verify the local copy against the
//! SHA-256 in its `magcard.txt`, upload it, ask the server to commit, wait for
//! `committed`. Only then: a field-set mag is marked synced on both drives; a local-staging
//! mag is marked on the server and removed once two backups exist (server + field set,
//! PLAN.md "Client staging cache"). Anything that fails stays pending.

use crate::catalog;
use crate::client::{self, Transport};
use crate::staging;
use crate::config::ClientConfig;
use crate::error::{Context, Error, Result};
use crate::fieldset::{self, SetStatus};
use crate::ingest;
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

/// The mag's camera values against the server's profiles, before any upload.
fn precheck(cams: Option<&[client::ServerCamera]>, mag_dir: &Path, folder: &str) -> Result<()> {
    let Some(cams) = cams else { return Ok(()) };
    let text = std::fs::read_to_string(mag_dir.join(magcard::FILE_NAME)).ctx(format!("{folder} has no readable magcard.txt"))?;
    let card = MagCard::parse(&text).map_err(|e| Error(format!("{folder}: magcard.txt: {e}")))?;
    let g = |k: &str| card.get(k).unwrap_or("").to_string();
    client::check_server_camera(cams, &g("camera_profile_id"), &g("file_format"), &g("gamma"), &g("gamut"))
        .map_err(|why| Error(format!("not uploaded: {why}")))
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
    let mut sets = fieldset::sets(&fieldset::drives(scan_roots));
    let cams = client::server_cameras(cc, Some(&t));

    // Local staging: pending until the server confirms.
    for folder in staging::pending(&cc.staging) {
        if interrupt::requested() || committer_down {
            break;
        }
        let dir = cc.staging.join(&folder);
        crate::say!("sync {folder} (local staging) …");
        match precheck(cams.as_deref(), &dir, &folder).and_then(|_| send(&t, cc, &dir, &folder)) {
            Ok(()) => {
                staging::mark_synced(&cc.staging, &folder)?;
                // A mounted field set with the same mag: its copy is on the server too.
                let mut on_set = None;
                if let Some(s) = sets.iter_mut().find(|s| s.status == SetStatus::Valid && s.mags.iter().any(|m| m.folder == folder)) {
                    if s.pending().iter().any(|m| m.folder == folder) {
                        fieldset::mark_synced(s, &folder)?;
                    }
                    on_set = Some(s.label.clone());
                }
                staging::record_server(cc, &folder, on_set.as_deref());
                log.info("sync", &format!("Mag {folder} committed on the server (from local staging)"));
                crate::out!("synced    {folder}  (local staging)");
                rep.synced += 1;
            }
            Err(e) => {
                committer_down = e.0.contains(crate::client::COMMITTER_DOWN);
                fail(&log, &mut rep, &folder, &e)
            }
        }
    }

    // Staged mags on the server: autoclean once two backups exist (server + a mounted
    // field set that holds the mag).
    for folder in staging::synced(&cc.staging) {
        let b = staging::Backups {
            server: true,
            field_set: staging::set_holding(&sets, &folder).map(|s| s.label.clone()),
        };
        match staging::autoclean(cc, &folder, &b, &log)? {
            staging::Clean::Removed => crate::out!("cleaned   {folder}  (local staging: two backups exist)"),
            staging::Clean::Kept(why) => crate::out!("kept      {folder}  (local staging: {why})"),
        }
    }
    let confirmed = staging::synced(&cc.staging);

    // Field sets: master is the sync source; synced is recorded on both drives.
    for set in sets.iter_mut() {
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
            let sent = if confirmed.contains(&folder) {
                // Already confirmed from its local staging copy: no second upload.
                Ok(())
            } else {
                precheck(cams.as_deref(), &master.join(&folder), &folder).and_then(|_| send(&t, cc, &master.join(&folder), &folder))
            };
            match sent {
                Ok(()) => {
                    fieldset::mark_synced(set, &folder)?;
                    staging::record_server(cc, &folder, Some(&set.label));
                    log.info(
                        "sync",
                        &format!(
                            "Mag {folder} committed on the server; marked synced on field set {}",
                            set.label
                        ),
                    );
                    crate::out!("synced    {folder}  (field set {})", set.label);
                    rep.synced += 1;
                    if confirmed.contains(&folder) {
                        let b = staging::Backups { server: true, field_set: Some(set.label.clone()) };
                        if staging::autoclean(cc, &folder, &b, &log)? == staging::Clean::Removed {
                            crate::out!("cleaned   {folder}  (local staging: two backups exist)");
                        }
                    }
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
