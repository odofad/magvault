//! `magvault serve` — runs on the server as `magvault` and commits mags that clients have
//! uploaded into their spool (`<incoming>/.clients/<client>/<folder>/`) after the gate's
//! `commit <folder>` request. This is the first piece of `magvaultd` (PLAN.md
//! Implementation defaults); in Phase 1 it only handles client commits and publishes the
//! open-project list for clients.
//!
//! For each request: the uploaded folder must be exactly the files listed in its
//! `magcard.txt` manifest (plus `magcard.txt` and `ascmhl/`), every file must match its
//! stored SHA-256, the camera values must exist in the server's camera profile, the
//! project must be open (or new). Then the same database transaction as a server ingest
//! (mag, clips, files, queued 320p job) and an atomic rename into `mags/` (the spool is on
//! the mags dataset). The result (`committed` or `error: …`) is written where the client's
//! gate can read it but the client cannot write it.

use crate::catalog;
use crate::config::{self, Config};
use crate::db::{self, Db};
use crate::error::{Context, Error, Result};
use crate::ingest::{self, expected_of, is_mag_folder, FileEntry, ProjectChoice};
use crate::interrupt;
use crate::log::Log;
use crate::magcard::{self, MagCard};
use crate::probe;
use crate::profiles;
use crate::textfile::atomic_write;
use crate::time;
use crate::util::{esc, human};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn spool_dir(cfg: &Config) -> PathBuf {
    cfg.paths.incoming.join(".clients")
}

pub fn results_dir(cfg: &Config) -> PathBuf {
    cfg.paths.incoming.join(".results")
}

fn mkdir_mode(p: &Path, mode: u32) -> Result<()> {
    if !p.exists() {
        std::fs::create_dir_all(p).ctx(format!("cannot create {}", p.display()))?;
        let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode));
    }
    Ok(())
}

fn write_readable(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).ctx(format!("cannot write {}", tmp.display()))?;
    let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o640));
    std::fs::rename(&tmp, path).ctx(format!("cannot write {}", path.display()))?;
    Ok(())
}

pub fn run(cfg: &Config, once: bool) -> Result<()> {
    let log = Log::new(&cfg.paths.logs);
    config::ensure_roots(&cfg.paths)?;
    config::check_incoming(&cfg.paths)?;
    let db = Db::open(&cfg.paths.db_file())?;
    db::ensure_schema(&db)?;
    let spool = spool_dir(cfg);
    let results = results_dir(cfg);
    mkdir_mode(&spool, 0o2770)?;
    mkdir_mode(&results, 0o2750)?;
    interrupt::install();
    if !once {
        eprintln!(
            "serving client commits from {} (Ctrl+C to stop)",
            spool.display()
        );
        log.info("system", "serve started (client commits)");
    }
    loop {
        export_projects(&db, &results)?;
        for (client, folder) in ready_requests(&spool) {
            handle(cfg, &db, &log, &spool, &results, &client, &folder);
            if interrupt::requested() {
                break;
            }
        }
        if once || interrupt::requested() {
            break;
        }
        for _ in 0..20 {
            if interrupt::requested() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if !once {
        log.info("system", "serve stopped");
    }
    Ok(())
}

/// Open projects for clients: `uuid | name` per line.
fn export_projects(db: &Db, results: &Path) -> Result<()> {
    let rows = db.query(
        "SELECT id, name FROM projects WHERE status = 'open' ORDER BY name, created_at",
        &[],
    )?;
    let text: String = rows
        .iter()
        .map(|r| format!("{} | {}\n", r[0].s(), r[1].s()))
        .collect();
    let path = results.join("projects.txt");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
        write_readable(&path, &text)?;
    }
    Ok(())
}

fn ready_requests(spool: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(clients) = std::fs::read_dir(spool) else {
        return out;
    };
    for c in clients.flatten() {
        let client = c.file_name().to_string_lossy().into_owned();
        if !crate::gate::valid_client(&client) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(c.path()) else {
            continue;
        };
        for e in entries.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if let Some(folder) = n.strip_prefix('.').and_then(|s| s.strip_suffix(".ready")) {
                if is_mag_folder(folder) {
                    out.push((client.clone(), folder.to_string()));
                }
            }
        }
    }
    out.sort();
    out
}

fn handle(
    cfg: &Config,
    db: &Db,
    log: &Log,
    spool: &Path,
    results: &Path,
    client: &str,
    folder: &str,
) {
    let dir = spool.join(client).join(folder);
    let marker = spool.join(client).join(format!(".{folder}.ready"));
    let rdir = results.join(client);
    let result_path = rdir.join(format!("{folder}.result"));
    let _ = mkdir_mode(&rdir, 0o2750);
    let _ = write_readable(&result_path, "verifying\n");
    match commit_upload(cfg, db, &dir, folder) {
        Ok(msg) => {
            let _ = write_readable(&result_path, "committed\n");
            log.info(
                "ingest",
                &format!("Mag {folder} committed from client {client} · {msg}"),
            );
        }
        Err(_) if interrupt::requested() => {
            // Server stopping: leave the request queued; it is picked up again on restart.
            let _ = std::fs::remove_file(&result_path);
            return;
        }
        Err(e) => {
            let first = e.0.lines().next().unwrap_or("").to_string();
            let _ = write_readable(&result_path, &format!("error: {}\n", esc(&first)));
            let context: Vec<String> =
                e.0.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            log.error(
                "ingest",
                &format!("Upload {folder} from client {client} refused; nothing committed"),
                &context,
            );
            if dir.exists() {
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }
    let _ = std::fs::remove_file(&marker);
}

/// Verify an uploaded mag folder and commit it. Idempotent: a mag that is already committed
/// reports success again (the client may have missed the first answer).
pub fn commit_upload(cfg: &Config, db: &Db, dir: &Path, folder: &str) -> Result<String> {
    let final_dir = cfg.paths.mags.join(folder);
    if db
        .query_one("SELECT id FROM mags WHERE folder = ?1", &[folder.into()])?
        .is_some()
    {
        if final_dir.is_dir() {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(dir);
            }
            return Ok("already committed".into());
        }
        crate::bail!("{folder} is in the database but missing from mags/");
    }
    if final_dir.exists() {
        crate::bail!("mags/{folder} exists but is not in the database");
    }
    if !std::fs::symlink_metadata(dir)
        .map(|m| m.is_dir())
        .unwrap_or(false)
    {
        crate::bail!("upload {folder} is missing");
    }
    let text = std::fs::read_to_string(dir.join(magcard::FILE_NAME))
        .ctx("upload has no readable magcard.txt")?;
    let card = MagCard::parse(&text).map_err(|e| Error(format!("magcard.txt: {e}")))?;
    let rec = catalog::record_from_card(&card)?;
    if rec.folder != folder
        || folder[9..] != rec.mag_uuid
        || folder[..8] != rec.shoot_date.replace('-', "")
    {
        crate::bail!("magcard does not match the folder name {folder}");
    }
    if !dir.join(crate::mhl::DIR).is_dir() {
        crate::bail!("upload has no ascmhl/");
    }
    let expected: Vec<(&str, u64, &str)> = rec
        .manifest
        .iter()
        .map(|m| (m.rel.as_str(), m.size, m.sha256.as_str()))
        .collect();
    ingest::verify_tree(dir, &expected, "verifying upload")?;

    // Camera values must exist in the server's profile (server is authoritative).
    let cams = profiles::load_cameras(&cfg.paths.cameras).profiles;
    let cam_id = rec.camera_profile_id.clone().unwrap_or_default();
    let cam = cams
        .iter()
        .find(|c| c.id == cam_id)
        .ok_or_else(|| Error(format!("camera profile {cam_id:?} is not on the server")))?;
    for (what, v, list) in [
        ("format", &rec.file_format, &cam.file_formats),
        ("gamma", &rec.gamma, &cam.gamma),
        ("gamut", &rec.gamut, &cam.gamut),
    ] {
        if !list.contains(v) {
            crate::bail!("{what} {v:?} is not offered by the server's camera profile {cam_id}");
        }
    }
    let project = project_for_upload(db, &rec.project_uuid, &rec.project_name)?;
    let proxy = ingest::proxy_profile_id(cfg)?;

    let probe = probe::probe_mag(dir)?;
    let by_rel: BTreeMap<&str, &magcard::ManifestRow> =
        rec.manifest.iter().map(|m| (m.rel.as_str(), m)).collect();
    let mut entries = Vec::with_capacity(probe.files.len());
    for f in &probe.files {
        let m = by_rel
            .get(f.rel.as_str())
            .ok_or_else(|| Error(format!("{} is not in the manifest", esc(&f.rel))))?;
        entries.push(FileEntry {
            rel: f.rel.clone(),
            size: m.size,
            mtime: time::parse_iso(&m.mtime).unwrap_or(f.mtime),
            sha256: m.sha256.clone(),
            role: f.role,
            clip: f.clip,
        });
    }
    debug_assert_eq!(expected_of(&entries).len(), rec.manifest.len());
    let mut rec = rec;
    if rec.project_uuid != project.id() || rec.project_name != project.name() {
        // The server's project wins: rewrite the uploaded magcard before it moves into mags/
        // (magcard.txt is an MHL ignore pattern, so the ASC MHL history stays valid).
        let mut card = card;
        card.set("project_uuid", project.id());
        card.set("project_name", project.name());
        atomic_write(&dir.join(magcard::FILE_NAME), card.render()?.as_bytes())?;
    }
    rec.project_uuid = project.id().to_string();
    rec.project_name = project.name().to_string();
    ingest::commit_db(
        db,
        &rec,
        &entries,
        &probe.clips,
        &proxy,
        &project,
        dir,
        &final_dir,
    )?;
    Ok(format!(
        "{} · {} files, {} clips · project {}",
        human(probe.total_bytes),
        entries.len(),
        probe.clips.len(),
        project.name()
    ))
}

/// The project named in a client's magcard: the existing project with that UUID (must be
/// open); else, when the UUID is unknown, the one open project with the same name (a client
/// that was offline created its own UUID; the server's project wins); else a new project with
/// the client's UUID and name. Two or more open projects with that name are refused.
pub fn project_for_upload(db: &Db, uuid: &str, name: &str) -> Result<ProjectChoice> {
    match db.query_one(
        "SELECT id, name, status FROM projects WHERE id = ?1",
        &[uuid.into()],
    )? {
        Some(r) if r[2].s() == "open" => Ok(ProjectChoice::Existing {
            id: r[0].s(),
            name: r[1].s(),
            status: "open".into(),
        }),
        Some(r) => Err(Error(format!(
            "project {} is closed; closed projects do not receive new mags",
            r[1].s()
        ))),
        None => {
            // Same rule as a server ingest with --project (Phase 0): one open project with the
            // name is reused; a name that only matches closed projects, or several open ones,
            // is refused.
            let rows = db.query(
                "SELECT id, name, status FROM projects WHERE name = ?1",
                &[name.into()],
            )?;
            let open: Vec<_> = rows.iter().filter(|r| r[2].s() == "open").collect();
            match (open.as_slice(), rows.len()) {
                ([], 0) => Ok(ProjectChoice::New {
                    id: uuid.into(),
                    name: name.into(),
                }),
                ([], _) => Err(Error(format!(
                    "project {name:?} is closed; closed projects do not receive new mags"
                ))),
                ([r], _) => Ok(ProjectChoice::Existing {
                    id: r[0].s(),
                    name: r[1].s(),
                    status: "open".into(),
                }),
                (o, _) => Err(Error(format!(
                    "{} open projects are named {name:?}; the server cannot choose one",
                    o.len()
                ))),
            }
        }
    }
}
