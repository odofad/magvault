//! Client ingest (PLAN.md "Client ingest and field backups", Phase 1).
//!
//! Routes (`--via`):
//! - `staging` (default): card → local staging → rsync to the server → server confirms →
//!   autoclean staging.
//! - `set`: the field backup set is the staging location: card → master, master → slave
//!   (same mags on both, always), listed as pending; then sync to the server, and on
//!   confirmation the mag is marked synced on both drives (it stays on the set until Clean).
//! - `direct`: card → server; no local copy.
//!
//! `--mirror-set` adds a copy on the field set to the `staging` route. With `direct` it
//! makes the set the local copy, i.e. the same as `--via set`.
//!
//! Hashes come from the first read of the card; every copy (staging, master, slave, server)
//! is verified against them. If the server has not confirmed and no field-set copy exists,
//! a warning says so: the card is still the only other copy.

use crate::client::{self, Transport};
use crate::config::{self, ClientConfig};
use crate::error::{Context, Error, Result};
use crate::fieldset::{self, FieldSet};
use crate::ingest::{self, Args, FileEntry, Staged};
use crate::interrupt;
use crate::log::{Level, Log};
use crate::magcard;
use crate::mhl;
use crate::probe::{self, Probe};
use crate::sync;
use crate::util::{self, esc, human};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    Staging,
    Set,
    Direct,
}

impl Via {
    pub fn parse(s: &str) -> Result<Via> {
        match s {
            "staging" => Ok(Via::Staging),
            "set" => Ok(Via::Set),
            "direct" => Ok(Via::Direct),
            other => Err(Error(format!(
                "--via {other:?} must be staging, set or direct"
            ))),
        }
    }
}

pub struct ClientArgs {
    pub base: Args,
    pub via: Via,
    pub mirror_set: bool,
    pub set_label: Option<String>,
    pub no_sync: bool,
    pub scan_roots: Vec<PathBuf>,
}

/// Extra free space kept on client targets (staging, field drives).
const MARGIN: u64 = 16 << 20;

pub fn run(cc: &ClientConfig, mut a: ClientArgs) -> Result<String> {
    let log = Log::new(&cc.logs);
    ingest::require_tool(
        "rsync",
        &["--version"],
        "rsync is required for every copy (PLAN.md \"Copy method\")",
    )?;
    config::check_not_tmp("staging", &cc.staging)?;
    if a.via == Via::Direct && a.mirror_set {
        a.via = Via::Set;
    }
    let src = ingest::check_source(&a.base.source, &[cc.staging.as_path(), cc.logs.as_path()])?;
    ingest::warn_stale_incoming(&cc.staging.join(".incoming"), &log);

    let transport = Transport::new(cc);
    let online = if a.no_sync {
        false
    } else {
        transport
            .as_ref()
            .map(|t| t.ping().is_ok())
            .unwrap_or(false)
    };
    if a.via == Via::Direct && !online {
        crate::bail!("--via direct needs the server, and it is not reachable; use --via staging or --via set");
    }
    let mut set: Option<FieldSet> = None;
    if a.via == Via::Set || a.mirror_set {
        let s = fieldset::choose_set(
            fieldset::sets(&fieldset::drives(&a.scan_roots)),
            a.set_label.as_deref(),
        )?;
        s.roots()?;
        set = Some(s);
    }

    let cams = ingest::load_cameras_logged(&cc.cameras, &log);
    let probe = ingest::probe_source(&src)?;
    let d = ingest::choose_details(&a.base, &probe, &cams, &src)?;
    let known = client::projects(cc, if online { transport.as_ref() } else { None });
    let project = client::choose_project(&a.base, &known)?;

    let route = match (a.via, a.mirror_set) {
        (Via::Staging, false) => format!("card → local staging {} → server", cc.staging.display()),
        (Via::Staging, true) => format!(
            "card → local staging {} + field set → server",
            cc.staging.display()
        ),
        (Via::Set, _) => "card → field set (master, then slave) → server".to_string(),
        (Via::Direct, _) => "card → server (no local copy)".to_string(),
    };
    let mut extra = vec![format!("route       {route}")];
    if let Some(s) = &set {
        extra.push(format!("field set   {}", s.line()));
    }
    extra.push(match (&transport, online) {
        (Some(t), true) => format!("server      {} (reachable)", t.describe()),
        (Some(t), false) if a.no_sync => format!(
            "server      {} (not contacted: --no-sync; mag stays pending)",
            t.describe()
        ),
        (Some(t), false) => format!(
            "server      {} NOT reachable: the mag stays pending until `magvault sync`",
            t.describe()
        ),
        (None, _) => "server      none configured: the mag stays pending".to_string(),
    });
    if set.is_none() {
        extra.push("backup      none: keep the card until the server confirms".into());
    }
    ingest::print_summary(&src, &probe, &a.base, &d, &project, &extra);
    if !a.base.yes && !util::confirm("Pull (verified copy) with these values?")? {
        crate::bail!("cancelled; nothing was written");
    }

    interrupt::install();
    client::remember_local_project(cc, &project)?;
    let result = (|| -> Result<(String, bool)> {
        let hashes = ingest::hash_source(&probe)?;
        let (folder, mag_uuid) = ingest::new_folder_name(&d.shoot_date)?;
        let entries = ingest::file_entries(&probe, &hashes);
        let rec = ingest::build_record(&folder, &mag_uuid, &project, &d, &entries);
        let c4s: Vec<String> = hashes.iter().map(|h| h.c4.clone()).collect();
        let fill_from_card = |dir: &Path, list: &Path| -> Result<()> {
            let rels: Vec<&str> = probe.files.iter().map(|f| f.rel.as_str()).collect();
            ingest::rsync_copy(&probe.root, &rels, dir, list)?;
            ingest::verify_tree(dir, &ingest::expected_of(&entries), "verifying")?;
            ingest::write_cards(dir, &rec, &d.cam.name, &entries, &c4s)
        };

        // Local copies first.
        let mut local: Option<PathBuf> = None;
        match a.via {
            Via::Staging => {
                ingest::check_space(&cc.staging, probe.total_bytes, MARGIN)?;
                let dir =
                    put_in_staging(&cc.staging, &folder, |dir, list| fill_from_card(dir, list))?;
                log.info(
                    "ingest",
                    &format!(
                        "Mag {folder} copied to local staging ({})",
                        human(probe.total_bytes)
                    ),
                );
                if let Some(s) = set.as_mut() {
                    put_on_set(s, &folder, &entries, |set_dir, list| {
                        copy_whole_mag(&dir, set_dir, list, &entries)
                    })?;
                    log.info(
                        "fieldset",
                        &format!("Mag {folder} mirrored to field set {} (pending)", s.label),
                    );
                }
                local = Some(dir);
            }
            Via::Set => {
                let s = set.as_mut().expect("set chosen");
                put_on_set(s, &folder, &entries, |dir, list| fill_from_card(dir, list))?;
                log.info(
                    "fieldset",
                    &format!(
                        "Mag {folder} copied to field set {} master and slave (pending)",
                        s.label
                    ),
                );
                local = Some(s.roots()?.0.join(&folder));
            }
            Via::Direct => {}
        }

        // Then the server.
        let mut synced = false;
        if online {
            let t = transport.as_ref().expect("online implies a transport");
            let sent = match &local {
                Some(dir) => sync::send(t, cc, dir, &folder),
                None => send_direct(t, cc, &probe, &folder, &rec, &d.cam.name, &entries, &c4s),
            };
            match sent {
                Ok(()) => {
                    synced = true;
                    log.info(
                        "sync",
                        &format!("Mag {folder} committed on the server ({})", t.describe()),
                    );
                    if a.via == Via::Staging {
                        std::fs::remove_dir_all(cc.staging.join(&folder))
                            .ctx("autoclean of local staging")?;
                    }
                    if let Some(s) = set.as_mut() {
                        fieldset::mark_synced(s, &folder)?;
                    }
                }
                Err(e) if a.via == Via::Direct => return Err(e),
                Err(e) if interrupt::requested() => return Err(e),
                Err(e) => {
                    eprintln!(
                        "warning: server sync failed: {}",
                        esc(e.0.lines().next().unwrap_or(""))
                    );
                    log.write(
                        Level::Warn,
                        "sync",
                        &format!("Mag {folder} not synced; stays pending"),
                        std::slice::from_ref(&e.0),
                    );
                }
            }
        }
        Ok((folder, synced))
    })();

    match result {
        Ok((folder, synced)) => {
            log.write(
                Level::Info,
                "ingest",
                &format!(
                    "Mag {folder} ingested on client ({}) · {} files, {} clips · project {}",
                    human(probe.total_bytes),
                    probe.files.len(),
                    probe.clips.len(),
                    project.name()
                ),
                &ingest::skipped_context(&probe),
            );
            if !synced && set.is_none() {
                let msg = format!(
                    "no field-set backup and the server has not confirmed {folder}: the only copies are {} and the card. Keep the card until `magvault sync` succeeds.",
                    cc.staging.join(&folder).display()
                );
                eprintln!("WARNING: {msg}");
                log.warn("ingest", &msg);
            }
            println!("ingested {folder}");
            println!("  route   {route}");
            println!(
                "  server  {}",
                if synced {
                    "committed"
                } else {
                    "pending (run `magvault sync`)"
                }
            );
            Ok(folder)
        }
        Err(e) => {
            ingest::log_failure(&log, &src, &e);
            Err(e)
        }
    }
}

/// Write a new mag into `<root>/.incoming/<folder>` with `fill`, then rename it to
/// `<root>/<folder>`. Leftovers are removed on failure.
pub fn put_in_staging(
    root: &Path,
    folder: &str,
    fill: impl FnOnce(&Path, &Path) -> Result<()>,
) -> Result<PathBuf> {
    let st = Staged::create(&root.join(".incoming"), folder)?;
    let target = root.join(folder);
    let r = (|| -> Result<()> {
        if target.exists() {
            crate::bail!("{} already exists", target.display());
        }
        std::fs::create_dir(&st.dir).ctx(format!("cannot create {}", st.dir.display()))?;
        fill(&st.dir, &st.list)?;
        interrupt::check()?;
        std::fs::rename(&st.dir, &target).ctx(format!("cannot move into {}", root.display()))?;
        Ok(())
    })();
    st.finish(r.is_ok());
    r.map(|_| target)
}

/// `magcard.txt` and every file under `ascmhl/` (relative paths).
pub fn internal_files(dir: &Path) -> Result<Vec<String>> {
    let mut v = vec![magcard::FILE_NAME.to_string()];
    let m = dir.join(mhl::DIR);
    if m.is_dir() {
        let (files, _) = probe::walk(&m)?;
        v.extend(files.into_iter().map(|f| format!("{}/{}", mhl::DIR, f.rel)));
    }
    Ok(v)
}

fn same_internal(a: &Path, b: &Path) -> Result<()> {
    let fa = internal_files(a)?;
    let fb = internal_files(b)?;
    if fa != fb {
        crate::bail!("integrity error: magcard / ASC MHL files differ between copies");
    }
    for rel in fa {
        let ha = crate::hash::hash_file(&a.join(&rel), false, &mut |_| true)?.sha256;
        let hb = crate::hash::hash_file(&b.join(&rel), false, &mut |_| true)?.sha256;
        if ha != hb {
            crate::bail!("integrity error: {rel} differs between copies");
        }
    }
    Ok(())
}

/// Copy a complete mag folder (originals + magcard + MHL) and verify it.
fn copy_whole_mag(src: &Path, dir: &Path, list: &Path, entries: &[FileEntry]) -> Result<()> {
    let mut rels: Vec<String> = entries.iter().map(|e| e.rel.clone()).collect();
    rels.extend(internal_files(src)?);
    let refs: Vec<&str> = rels.iter().map(String::as_str).collect();
    ingest::rsync_copy(src, &refs, dir, list)?;
    ingest::verify_tree(dir, &ingest::expected_of(entries), "verifying")?;
    same_internal(src, dir)
}

/// Put a new mag on both drives of a field set: `fill` writes the master copy; the slave is
/// copied from the master and verified against the same SHA-256; then both are renamed into
/// place and the mag is listed as pending on both. A mag is never added to one drive
/// without the other.
pub fn put_on_set(
    set: &mut FieldSet,
    folder: &str,
    entries: &[FileEntry],
    fill: impl FnOnce(&Path, &Path) -> Result<()>,
) -> Result<()> {
    let (m, s) = set.roots()?;
    let (m, s) = (m.to_path_buf(), s.to_path_buf());
    let total: u64 = entries.iter().map(|e| e.size).sum();
    ingest::check_space(&m, total, MARGIN)?;
    ingest::check_space(&s, total, MARGIN)?;
    let (mt, stt) = (m.join(folder), s.join(folder));
    if mt.exists() || stt.exists() {
        crate::bail!("{folder} already exists on the field set");
    }
    let sm = Staged::create(&m.join(".incoming"), folder)?;
    let ss = match Staged::create(&s.join(".incoming"), folder) {
        Ok(x) => x,
        Err(e) => {
            sm.finish(false);
            return Err(e);
        }
    };
    let r = (|| -> Result<()> {
        std::fs::create_dir(&sm.dir).ctx(format!("cannot create {}", sm.dir.display()))?;
        fill(&sm.dir, &sm.list)?;
        interrupt::check()?;
        std::fs::create_dir(&ss.dir).ctx(format!("cannot create {}", ss.dir.display()))?;
        copy_whole_mag(&sm.dir, &ss.dir, &ss.list, entries)?;
        interrupt::check()?;
        std::fs::rename(&sm.dir, &mt).ctx("cannot move the mag into place on the master")?;
        if let Err(e) = std::fs::rename(&ss.dir, &stt) {
            let _ = std::fs::rename(&mt, &sm.dir);
            return Err(Error(format!(
                "cannot move the mag into place on the slave: {e}"
            )));
        }
        if let Err(e) = fieldset::add_pending(set, folder) {
            let _ = std::fs::rename(&mt, &sm.dir);
            let _ = std::fs::rename(&stt, &ss.dir);
            return Err(e);
        }
        Ok(())
    })();
    sm.finish(r.is_ok());
    ss.finish(r.is_ok());
    r
}

/// Direct route: originals straight from the card, magcard + MHL from a small folder in
/// local staging, then commit.
#[allow(clippy::too_many_arguments)]
fn send_direct(
    t: &Transport,
    cc: &ClientConfig,
    probe: &Probe,
    folder: &str,
    rec: &crate::catalog::MagRecord,
    cam_name: &str,
    entries: &[FileEntry],
    c4s: &[String],
) -> Result<()> {
    let st = Staged::create(&cc.staging.join(".incoming"), folder)?;
    let r = (|| -> Result<()> {
        std::fs::create_dir(&st.dir).ctx(format!("cannot create {}", st.dir.display()))?;
        ingest::write_cards(&st.dir, rec, cam_name, entries, c4s)?;
        let rels: Vec<&str> = probe.files.iter().map(|f| f.rel.as_str()).collect();
        t.upload(&probe.root, &rels, folder, &st.list)?;
        let meta = internal_files(&st.dir)?;
        let meta: Vec<&str> = meta.iter().map(String::as_str).collect();
        t.upload(&st.dir, &meta, folder, &st.list)?;
        t.commit(folder)?;
        t.wait_committed(folder)
    })();
    st.finish(false);
    r
}
