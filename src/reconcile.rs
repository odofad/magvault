//! Conflict handling (PLAN.md "Conflict handling", tech spec §9.3):
//!
//! - Database is authoritative for metadata. If `magcard.txt` differs from the database
//!   (header fields or `# logging` section), the old card is copied to
//!   `.magvault/magcard.YYYYMMDD-HHMMSS.backup` (UTC) and a fresh card is written from
//!   the database. Only the newest backup is kept ("single .magvault/magcard.*.backup",
//!   PLAN.md "Logging on the mag card").
//! - The manifest is fixed at ingest. A manifest difference (card vs database vs files on
//!   disk) is an integrity error: it is reported and nothing is rewritten.

use crate::catalog::{self, card_from_record, MagRecord, COMPARED_KEYS};
use crate::config::Config;
use crate::db::{self, Db};
use crate::error::{Context, Result};
use crate::hash;
use crate::log::Log;
use crate::magcard::{self, MagCard, HEADER_KEYS};
use crate::mhl;
use crate::probe;
use crate::profiles;
use crate::textfile::atomic_write;
use crate::time;
use std::collections::BTreeMap;
use std::path::Path;

pub const BACKUP_DIR: &str = ".magvault";

#[derive(Debug, Default)]
pub struct Report {
    pub folder: String,
    pub rewritten: Vec<String>,
    pub integrity: Vec<String>,
    pub notes: Vec<String>,
}

pub fn run(cfg: &Config, targets: &[String], all: bool, verify_hashes: bool) -> Result<Vec<Report>> {
    let db = Db::open(&cfg.paths.db_file())?;
    db::ensure_schema(&db)?;
    let log = Log::new(&cfg.paths.logs);
    let cams = profiles::load_cameras(&cfg.paths.cameras).profiles;

    let mut ids = Vec::new();
    if all {
        for r in db.query("SELECT id FROM mags WHERE status = 'online' ORDER BY folder", &[])? {
            ids.push(r[0].s());
        }
    }
    for t in targets {
        match catalog::find_mag_id(&db, t)? {
            Some(id) => ids.push(id),
            None => crate::bail!("no mag with UUID or folder {t:?} in the database"),
        }
    }
    let mut reports = Vec::new();
    for id in ids {
        let rec = catalog::load_record(&db, &id)?.expect("id came from the database");
        let model = rec.camera_profile_id.as_ref().and_then(|p| cams.iter().find(|c| &c.id == p)).map(|c| c.name.clone());
        let rep = check_one(cfg, &rec, model.as_deref(), verify_hashes)?;
        if !rep.integrity.is_empty() {
            log.error("system", &format!("Integrity problem in mag {}", rep.folder), &rep.integrity);
        }
        if !rep.rewritten.is_empty() {
            log.info("system", &format!("magcard.txt of {} rewritten from database ({})", rep.folder, rep.rewritten.join(", ")));
        }
        for n in &rep.notes {
            log.warn("system", &format!("{}: {n}", rep.folder));
        }
        reports.push(rep);
    }
    Ok(reports)
}

fn internal(rel: &str) -> bool {
    rel == magcard::FILE_NAME
        || rel == mhl::DIR
        || rel.starts_with(&format!("{}/", mhl::DIR))
        || rel.starts_with(&format!("{BACKUP_DIR}/"))
}

pub fn check_one(cfg: &Config, rec: &MagRecord, camera_model: Option<&str>, verify_hashes: bool) -> Result<Report> {
    let mut rep = Report { folder: rec.folder.clone(), ..Default::default() };
    let dir = cfg.paths.mags.join(&rec.folder);
    if !dir.is_dir() {
        if rec.status == "online" {
            rep.integrity.push(format!("mag folder {} is missing but the database says online", dir.display()));
        } else {
            rep.notes.push(format!("status {} — not on the server, nothing to check", rec.status));
        }
        return Ok(rep);
    }

    // Files on disk vs database manifest.
    let db_files: BTreeMap<&str, &magcard::ManifestRow> = rec.manifest.iter().map(|m| (m.rel.as_str(), m)).collect();
    match probe::walk(&dir) {
        Err(e) => rep.integrity.push(format!("cannot list folder: {e}")),
        Ok((disk, _)) => {
            let disk: Vec<_> = disk.into_iter().filter(|f| !internal(&f.rel)).collect();
            let on_disk: BTreeMap<&str, &probe::ProbedFile> = disk.iter().map(|f| (f.rel.as_str(), f)).collect();
            for rel in db_files.keys().filter(|r| !on_disk.contains_key(*r)) {
                rep.integrity.push(format!("file missing on disk: {rel}"));
            }
            for rel in on_disk.keys().filter(|r| !db_files.contains_key(*r)) {
                rep.integrity.push(format!("file on disk is not in the manifest: {rel}"));
            }
            for (rel, f) in &on_disk {
                let Some(m) = db_files.get(rel) else { continue };
                if f.size != m.size {
                    rep.integrity.push(format!("size differs from manifest: {rel} ({} vs {} bytes)", f.size, m.size));
                } else if verify_hashes {
                    let h = hash::hash_file(&f.abs, false, &mut |_| {}).ctx(format!("cannot read {rel}"))?;
                    if h.sha256 != m.sha256 {
                        rep.integrity.push(format!("SHA-256 differs from manifest: {rel}"));
                    }
                }
            }
        }
    }

    // Card vs database.
    let card_path = dir.join(magcard::FILE_NAME);
    let expected = card_from_record(rec, camera_model);
    let existing = match std::fs::read_to_string(&card_path) {
        Ok(text) => match MagCard::parse(&text) {
            Ok(c) => Some(c),
            Err(e) => {
                rep.notes.push(format!("magcard.txt cannot be parsed ({e}); it is replaced from the database"));
                if !rep.integrity.is_empty() {
                    return Ok(rep);
                }
                write_card(&dir, &card_path, rec, camera_model, None, true)?;
                rep.rewritten.push("unreadable card".into());
                return Ok(rep);
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => crate::bail!("cannot read {}: {e}", card_path.display()),
    };
    let Some(card) = existing else {
        if rep.integrity.is_empty() {
            write_card(&dir, &card_path, rec, camera_model, None, false)?;
            rep.rewritten.push("card was missing".into());
        }
        return Ok(rep);
    };

    if card.manifest != rec.manifest {
        let n_card = card.manifest.len();
        let n_db = rec.manifest.len();
        rep.integrity.push(format!(
            "magcard manifest differs from the database manifest ({n_card} vs {n_db} entries); manifests are fixed at ingest and are not rewritten"
        ));
    }
    let mut differs: Vec<String> = COMPARED_KEYS
        .iter()
        .filter(|k| card.get(k).unwrap_or("") != expected.get(k).unwrap_or(""))
        .map(|k| k.to_string())
        .collect();
    if let Some(m) = camera_model {
        if card.get("camera_model").unwrap_or("") != m {
            differs.push("camera_model".into());
        }
    }
    if card.logging != expected.logging {
        differs.push("logging".into());
    }
    if differs.is_empty() {
        return Ok(rep);
    }
    if !rep.integrity.is_empty() {
        rep.notes.push(format!("metadata differs ({}) but the card is left as is until the integrity problem is resolved", differs.join(", ")));
        return Ok(rep);
    }
    write_card(&dir, &card_path, rec, camera_model, Some(&card), true)?;
    rep.rewritten = differs;
    Ok(rep)
}

/// Back up the current card (if any) and write a fresh one from the database.
fn write_card(dir: &Path, card_path: &Path, rec: &MagRecord, camera_model: Option<&str>, old: Option<&MagCard>, backup: bool) -> Result<()> {
    let bdir = dir.join(BACKUP_DIR);
    std::fs::create_dir_all(&bdir).ctx(format!("cannot create {}", bdir.display()))?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(bdir.join("magcard.lock"))
        .ctx("cannot open card lock")?;
    lock.lock().ctx("cannot lock magcard")?;

    let now = time::now_secs();
    let mut card = card_from_record(rec, camera_model);
    card.set("updated_at", time::iso_utc(now));
    if let Some(old) = old {
        if camera_model.is_none() {
            card.set("camera_model", old.get("camera_model").unwrap_or(""));
        }
        for (k, v) in &old.header {
            if !HEADER_KEYS.contains(&k.as_str()) {
                card.set(k, v.clone());
            }
        }
    }
    let text = card.render()?;

    if backup && card_path.exists() {
        let name = format!("magcard.{}.backup", time::backup_stamp(now));
        let target = bdir.join(&name);
        let bytes = std::fs::read(card_path).ctx(format!("cannot read {}", card_path.display()))?;
        atomic_write(&target, &bytes)?;
        // Keep a single backup: remove older ones.
        if let Ok(rd) = std::fs::read_dir(&bdir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.starts_with("magcard.") && n.ends_with(".backup") && n != name {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }
    atomic_write(card_path, text.as_bytes())?;
    let _ = lock.unlock();
    Ok(())
}
