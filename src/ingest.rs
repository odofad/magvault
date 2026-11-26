//! Manual-path ingest (PLAN.md "Ingest flow", Phase 0):
//!
//! 1. Probe the source: walk files and sidecars, skip junk, ffprobe media, guess camera
//!    and shoot day.
//! 2. Operator confirms camera, format, gamma, gamut, shoot day, project (flags or prompt).
//! 3. Hash at ingest: SHA-256 (and C4 for the ASC MHL) of every file, in the first read
//!    of the card (PLAN.md "Hashing").
//! 4. Copy with rsync into `incoming/<YYYYMMDD>_<uuid>/` (same filesystem as `mags/`,
//!    never /tmp).
//! 5. Verify every copied file against the stored SHA-256; any mismatch is an integrity
//!    error and nothing is committed.
//! 6. Write `magcard.txt` (header + manifest + empty logging section) and ASC MHL.
//! 7. One database transaction: project (if new), mag, clips, files, 320p encoder job.
//!    The folder is renamed into `mags/` inside that transaction and the transaction is
//!    committed only after the rename succeeded.
//!
//! From the confirmation on, every failure (including Ctrl+C / SIGTERM) removes the staging
//! folder, its `.files` list and its `.lock` file and is logged as ERROR. The `.lock` file is
//! held (flock) for the whole ingest so that a second ingest can tell a live staging folder
//! from one left by a killed ingest.

use crate::catalog::{card_from_record, MagRecord};
use crate::config::{self, Config};
use crate::db::{self, Db, Value};
use crate::error::{Context, Error, Result};
use crate::hash;
use crate::interrupt;
use crate::log::{Level, Log};
use crate::magcard::{self, ManifestRow};
use crate::mhl;
use crate::probe::{self, Probe, Role};
use crate::profiles::{self, CameraProfile, EncoderProfile};
use crate::textfile::{atomic_write, check_card_field, sync_dir};
use crate::time;
use crate::util::{self, esc, human, Progress};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct Args {
    pub source: PathBuf,
    pub project: Option<String>,
    pub project_id: Option<String>,
    pub camera: Option<String>,
    pub format: Option<String>,
    pub gamma: Option<String>,
    pub gamut: Option<String>,
    pub shoot_date: Option<String>,
    pub notes: Option<String>,
    pub yes: bool,
}

enum ProjectChoice {
    Existing { id: String, name: String, status: String },
    New { id: String, name: String },
}

impl ProjectChoice {
    fn id(&self) -> &str {
        match self {
            ProjectChoice::Existing { id, .. } | ProjectChoice::New { id, .. } => id,
        }
    }
    fn name(&self) -> &str {
        match self {
            ProjectChoice::Existing { name, .. } | ProjectChoice::New { name, .. } => name,
        }
    }
}

struct Hashed {
    sha256: String,
    c4: String,
}

pub struct Outcome {
    pub folder: String,
    pub mag_dir: PathBuf,
    pub files: usize,
    pub clips: usize,
    pub bytes: u64,
}

pub fn run(cfg: &Config, a: Args) -> Result<Outcome> {
    let log = Log::new(&cfg.paths.logs);
    require_tool("rsync", &["--version"], "rsync is required for every copy (PLAN.md \"Copy method\")")?;
    config::ensure_roots(&cfg.paths)?;
    config::check_incoming(&cfg.paths)?;
    let db = Db::open(&cfg.paths.db_file())?;
    db::ensure_schema(&db)?;

    // ---- source -------------------------------------------------------------------
    let src = std::fs::canonicalize(&a.source).ctx(format!("source {}", a.source.display()))?;
    if !src.is_dir() {
        crate::bail!("source {} is not a directory", src.display());
    }
    for root in cfg.paths.data_roots() {
        if let Ok(r) = std::fs::canonicalize(root) {
            if src.starts_with(&r) || r.starts_with(&src) {
                crate::bail!("source {} overlaps the Magvault root {}; ingest from a card or a separate folder", esc(&src.display().to_string()), r.display());
            }
        }
    }
    if src.join(magcard::FILE_NAME).exists() || src.join(mhl::DIR).is_dir() {
        crate::bail!(
            "source {} already contains {} or {}/ — it looks like a Magvault mag folder, not a card. \
             Importing existing mag folders (field backups) is part of Phase 1.",
            src.display(),
            magcard::FILE_NAME,
            mhl::DIR
        );
    }
    warn_stale_incoming(&cfg.paths.incoming, &log);

    // ---- profiles -----------------------------------------------------------------
    let cams = profiles::load_cameras(&cfg.paths.cameras);
    for (p, e) in &cams.invalid {
        eprintln!("warning: camera profile {} ignored: {e}", p.display());
        log.warn("ingest", &format!("camera profile {} ignored: {e}", p.display()));
    }
    let encs = profiles::load_encoders(&cfg.paths.encoders);
    for (p, e) in &encs.invalid {
        eprintln!("warning: encoder profile {} ignored: {e}", p.display());
    }
    let proxy_profile = resolve_320p_profile(cfg, &encs.profiles)?;

    // ---- probe --------------------------------------------------------------------
    let src_shown = esc(&src.display().to_string());
    eprintln!("probing {src_shown} …");
    let use_ffprobe = probe::ffprobe_available();
    if !use_ffprobe {
        eprintln!("warning: ffprobe not found; media details (duration, fps, size, audio) will be empty");
    }
    let probe = probe::probe(&src, use_ffprobe)?;
    if probe.files.is_empty() {
        crate::bail!("nothing to ingest: {} contains no files", src.display());
    }
    for c in &probe.clips {
        if let Some(e) = &c.info.probe_error {
            eprintln!("warning: ffprobe could not read {}: {}", esc(&probe.files[c.file].rel), esc(e));
        }
    }

    // ---- operator choices (flags), validated against the profile ----------------------
    let cam = choose_camera(&a, &probe, &cams.profiles)?;
    let pick = |what: &str, given: &Option<String>, list: &[String], default: &str| -> Result<String> {
        let v = given.clone().unwrap_or_else(|| default.to_string());
        if !list.contains(&v) {
            crate::bail!("{what} {v:?} is not offered by camera profile {} (choose one of: {})", cam.id, list.join(", "));
        }
        Ok(v)
    };
    let file_format = pick("format", &a.format, &cam.file_formats, &cam.default_format)?;
    let gamma = pick("gamma", &a.gamma, &cam.gamma, &cam.default_gamma)?;
    let gamut = pick("gamut", &a.gamut, &cam.gamut, &cam.default_gamut)?;

    let (shoot_date, shoot_src) = match &a.shoot_date {
        Some(d) => {
            if time::parse_date(d).is_none() {
                crate::bail!("--shoot-date {d:?} must be a real date in the form YYYY-MM-DD");
            }
            (d.clone(), "given")
        }
        None => probe::guess_shoot_day(&probe).ok_or_else(|| Error("cannot guess the shoot day; pass --shoot-date YYYY-MM-DD".into()))?,
    };
    let notes = a.notes.clone().unwrap_or_default();
    check_card_field("--notes", &notes)?;
    let project = choose_project(&db, &a)?;

    // ---- confirm --------------------------------------------------------------------
    let media_count = probe.clips.len();
    let sidecars = probe.files.iter().filter(|f| f.role == Role::Sidecar).count();
    let source_label = src.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    check_card_field("source folder name", &source_label)?;
    eprintln!();
    eprintln!("  source      {src_shown}");
    eprintln!(
        "  files       {} ({} clips, {} sidecars, {} other) · {}",
        probe.files.len(),
        media_count,
        sidecars,
        probe.files.len() - media_count - sidecars,
        human(probe.total_bytes)
    );
    let skipped: Vec<String> = probe.skipped.iter().map(|s| esc(s)).collect();
    if !skipped.is_empty() {
        eprintln!("  skipped     {} junk item(s): {}", skipped.len(), skipped.join(", "));
    }
    let guessed = |given: bool| if given { "" } else { "  (guessed)" };
    eprintln!("  camera      {} [{}]{}", cam.name, cam.id, guessed(a.camera.is_some()));
    eprintln!("  format      {file_format}{}", guessed(a.format.is_some()));
    eprintln!("  gamma       {gamma}{}", guessed(a.gamma.is_some()));
    eprintln!("  gamut       {gamut}{}", guessed(a.gamut.is_some()));
    eprintln!("  shoot day   {shoot_date}{}", if shoot_src == "given" { String::new() } else { format!("  (guessed: {shoot_src})") });
    match &project {
        ProjectChoice::Existing { name, id, .. } => eprintln!("  project     {name} [{id}]"),
        ProjectChoice::New { name, .. } => eprintln!("  project     {name}  (NEW project, UUID assigned on commit)"),
    }
    if !notes.is_empty() {
        eprintln!("  notes       {notes}");
    }
    eprintln!("  proxy       320p job queued with profile {} (not encoded in Phase 0)", proxy_profile.id);
    eprintln!("  destination {}/{}_<new uuid>/", cfg.paths.mags.display(), shoot_date.replace('-', ""));
    eprintln!();
    if !a.yes && !util::confirm("Pull (verified copy) with these values?")? {
        crate::bail!("cancelled; nothing was written");
    }

    // ---- after confirmation: every failure is cleaned up and logged as ERROR ------------
    interrupt::install();
    let mut staged: Option<Staged> = None;
    let result = (|| -> Result<String> {
        let hashes = hash_source(&probe)?;
        check_space(cfg, &probe)?;
        interrupt::check()?;
        let mag_uuid = util::uuid_v4()?;
        let folder = format!("{}_{}", shoot_date.replace('-', ""), mag_uuid);
        if cfg.paths.mags.join(&folder).exists() {
            crate::bail!("{folder} already exists in mags/ (UUID collision?); nothing was written");
        }
        let st = staged.insert(Staged::create(&cfg.paths.incoming, &folder)?);
        let ctx = Commit {
            cfg,
            db: &db,
            probe: &probe,
            hashes: &hashes,
            cam,
            proxy_profile,
            project: &project,
            mag_uuid: &mag_uuid,
            folder: &folder,
            staging: &st.dir,
            final_dir: &cfg.paths.mags.join(&folder),
            list_file: &st.list,
            shoot_date: &shoot_date,
            file_format: &file_format,
            gamma: &gamma,
            gamut: &gamut,
            notes: &notes,
            source_label: &source_label,
        };
        ctx.run()?;
        Ok(folder)
    })();
    if let Some(st) = staged {
        st.finish(result.is_ok());
    }
    match result {
        Ok(folder) => {
            let msg = format!(
                "Mag {folder} ingested ({}) · {} files, {} clips · project {}",
                human(probe.total_bytes),
                probe.files.len(),
                media_count,
                project.name()
            );
            let context: Vec<String> =
                if probe.skipped.is_empty() { Vec::new() } else { vec![format!("skipped junk: {}", probe.skipped.join(", "))] };
            log.write(Level::Info, "ingest", &msg, &context);
            if media_count > 0 {
                log.info("encoder", &format!("320p proxy job queued for {folder} ({} clips, profile {})", media_count, proxy_profile.id));
            }
            if interrupt::requested() {
                eprintln!("note: a stop signal arrived while the mag was being committed; the ingest completed");
            }
            let final_dir = cfg.paths.mags.join(&folder);
            Ok(Outcome { folder, mag_dir: final_dir, files: probe.files.len(), clips: media_count, bytes: probe.total_bytes })
        }
        Err(e) => {
            let context: Vec<String> = e.0.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
            log.error("ingest", &format!("Ingest of {} failed; nothing committed", src.display()), &context);
            Err(e)
        }
    }
}

/// First read of the card: SHA-256 (and C4) of every file (PLAN.md "Hashing").
fn hash_source(probe: &Probe) -> Result<Vec<Hashed>> {
    let mut prog = Progress::new("hashing ", probe.total_bytes);
    let mut hashes: Vec<Hashed> = Vec::with_capacity(probe.files.len());
    for f in &probe.files {
        let h = hash::hash_file(&f.abs, true, &mut |n| {
            prog.add(n);
            !interrupt::requested()
        });
        interrupt::check()?;
        let h = h.ctx(format!("cannot read {}", esc(&f.rel)))?;
        if h.bytes != f.size {
            crate::bail!(
                "integrity error: {} changed size while it was being read ({} → {} bytes); is the card still being written?",
                esc(&f.rel),
                f.size,
                h.bytes
            );
        }
        hashes.push(Hashed { sha256: h.sha256, c4: h.c4.unwrap_or_default() });
    }
    prog.finish();
    Ok(hashes)
}

fn check_space(cfg: &Config, probe: &Probe) -> Result<()> {
    let need = probe.total_bytes + probe.total_bytes / 100 + (1 << 30);
    let avail = util::free_bytes(&cfg.paths.incoming)?;
    if avail < need {
        crate::bail!(
            "not enough space in {}: need {} (card {} + margin), {} free",
            cfg.paths.incoming.display(),
            human(need),
            human(probe.total_bytes),
            human(avail)
        );
    }
    Ok(())
}

/// One ingest's footprint in `incoming/`: the staging folder, its rsync list and its lock.
/// The lock file is created and flock-ed before the folder exists and held until the end,
/// so `warn_stale_incoming` never reports a folder whose ingest is still running.
struct Staged {
    dir: PathBuf,
    list: PathBuf,
    lock_path: PathBuf,
    lock: File,
}

impl Staged {
    fn create(incoming: &Path, folder: &str) -> Result<Staged> {
        let dir = incoming.join(folder);
        let list = incoming.join(format!(".{folder}.files"));
        let lock_path = incoming.join(format!(".{folder}.lock"));
        if dir.exists() || list.exists() || lock_path.exists() {
            crate::bail!("{folder} already exists in incoming/ (UUID collision?); nothing was written");
        }
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .ctx(format!("cannot create {}", lock_path.display()))?;
        lock.lock().ctx(format!("cannot lock {}", lock_path.display()))?;
        Ok(Staged { dir, list, lock_path, lock })
    }

    /// Remove what is left in `incoming/`. On success the folder has already been renamed
    /// into `mags/`; on failure it is deleted. The lock file goes last.
    fn finish(self, committed: bool) {
        if !committed && self.dir.exists() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
        let _ = std::fs::remove_file(&self.list);
        let _ = std::fs::remove_file(&self.lock_path);
        let _ = self.lock.unlock();
    }
}

struct Commit<'a> {
    cfg: &'a Config,
    db: &'a Db,
    probe: &'a Probe,
    hashes: &'a [Hashed],
    cam: &'a CameraProfile,
    proxy_profile: &'a EncoderProfile,
    project: &'a ProjectChoice,
    mag_uuid: &'a str,
    folder: &'a str,
    staging: &'a Path,
    final_dir: &'a Path,
    list_file: &'a Path,
    shoot_date: &'a str,
    file_format: &'a str,
    gamma: &'a str,
    gamut: &'a str,
    notes: &'a str,
    source_label: &'a str,
}

impl Commit<'_> {
    fn run(&self) -> Result<()> {
        std::fs::create_dir(self.staging).ctx(format!("cannot create {}", self.staging.display()))?;
        self.rsync()?;
        interrupt::check()?;
        self.verify()?;
        interrupt::check()?;
        let now = time::now_secs();
        let now_iso = time::iso_utc(now);
        self.write_cards(now, &now_iso)?;
        // Last point where a stop signal is honoured. From here the rename into mags/ and the
        // database commit run to the end; a signal arriving now is held off (PLAN.md Threat model).
        interrupt::check()?;
        self.commit_db(&now_iso)
    }

    fn rsync(&self) -> Result<()> {
        let mut list = Vec::new();
        for f in &self.probe.files {
            list.extend_from_slice(f.rel.as_bytes());
            list.push(0);
        }
        std::fs::write(self.list_file, &list).ctx(format!("cannot write {}", self.list_file.display()))?;
        let mut src_arg = self.probe.root.as_os_str().to_owned();
        src_arg.push("/");
        let mut dst_arg = self.staging.as_os_str().to_owned();
        dst_arg.push("/");
        eprintln!("copying with rsync → {}", self.staging.display());
        let mut child = Command::new("rsync")
            .arg("-t")
            .arg("--no-links")
            .arg("--no-devices")
            .arg("--no-specials")
            .arg("--from0")
            .arg(format!("--files-from={}", self.list_file.display()))
            .arg("--info=progress2")
            .arg("--outbuf=L")
            .arg(&src_arg)
            .arg(&dst_arg)
            .env("LC_ALL", "C")
            .spawn()
            .ctx("cannot start rsync")?;
        // Ctrl+C reaches rsync directly (same process group). A SIGTERM sent to Magvault
        // alone does not, so rsync is stopped here when a signal has been recorded.
        let status = loop {
            if let Some(st) = child.try_wait().ctx("waiting for rsync")? {
                break st;
            }
            if interrupt::requested() {
                interrupt::stop_child(&mut child);
                return Err(interrupt::error());
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        interrupt::check()?;
        if !status.success() {
            crate::bail!("rsync failed ({}); nothing committed", status.code().map_or("killed".into(), |c| format!("exit {c}")));
        }
        Ok(())
    }

    /// Every later copy is verified against the hashes stored at ingest (PLAN.md "Hashing").
    fn verify(&self) -> Result<()> {
        let (copied, _) = probe::walk(self.staging).ctx("cannot list the copied files")?;
        let expected: BTreeMap<&str, usize> = self.probe.files.iter().enumerate().map(|(i, f)| (f.rel.as_str(), i)).collect();
        let got: BTreeSet<&str> = copied.iter().map(|f| f.rel.as_str()).collect();
        let mut problems = Vec::new();
        for rel in expected.keys().filter(|r| !got.contains(*r)) {
            problems.push(format!("missing after copy: {rel}"));
        }
        for rel in got.iter().filter(|r| !expected.contains_key(*r)) {
            problems.push(format!("unexpected file after copy: {rel}"));
        }
        let total: u64 = self.probe.total_bytes;
        let mut prog = Progress::new("verifying", total);
        for f in &copied {
            let Some(&i) = expected.get(f.rel.as_str()) else { continue };
            let want = &self.probe.files[i];
            if f.size != want.size {
                problems.push(format!("size differs: {} ({} vs {} bytes)", f.rel, f.size, want.size));
                continue;
            }
            let h = hash::hash_file(&f.abs, false, &mut |n| {
                prog.add(n);
                !interrupt::requested()
            });
            interrupt::check()?;
            let h = h.ctx(format!("cannot read copied {}", esc(&f.rel)))?;
            if h.sha256 != self.hashes[i].sha256 {
                problems.push(format!("SHA-256 mismatch: {} (card {}, copy {})", f.rel, self.hashes[i].sha256, h.sha256));
            }
        }
        prog.finish();
        if !problems.is_empty() {
            crate::bail!("integrity error: the copy does not match the card; nothing committed\n  {}", problems.join("\n  "));
        }
        Ok(())
    }

    fn record(&self, now_iso: &str) -> MagRecord {
        let (project_status, project_name) = match self.project {
            ProjectChoice::Existing { status, name, .. } => (status.clone(), name.clone()),
            ProjectChoice::New { name, .. } => ("open".to_string(), name.clone()),
        };
        MagRecord {
            mag_uuid: self.mag_uuid.into(),
            folder: self.folder.into(),
            project_uuid: self.project.id().into(),
            project_name,
            project_status,
            shoot_date: self.shoot_date.into(),
            camera_profile_id: Some(self.cam.id.clone()),
            file_format: self.file_format.into(),
            gamma: self.gamma.into(),
            gamut: self.gamut.into(),
            source_label: self.source_label.into(),
            ingested_by: util::identity(),
            ingested_at: now_iso.into(),
            updated_at: now_iso.into(),
            notes: self.notes.into(),
            status: "online".into(),
            manifest: self
                .probe
                .files
                .iter()
                .zip(self.hashes)
                .map(|(f, h)| ManifestRow { rel: f.rel.clone(), size: f.size, sha256: h.sha256.clone(), mtime: time::iso_utc(f.mtime) })
                .collect(),
            logging: Vec::new(),
        }
    }

    fn write_cards(&self, now: i64, now_iso: &str) -> Result<()> {
        let card = card_from_record(&self.record(now_iso), Some(&self.cam.name));
        atomic_write(&self.staging.join(magcard::FILE_NAME), card.render()?.as_bytes())?;
        let entries: Vec<mhl::Entry> = self
            .probe
            .files
            .iter()
            .zip(self.hashes)
            .map(|(f, h)| mhl::Entry { rel: &f.rel, size: f.size, mtime: f.mtime, c4: &h.c4 })
            .collect();
        mhl::write_initial(self.staging, self.folder, &entries, now, now, &util::hostname())?;
        Ok(())
    }

    fn commit_db(&self, now_iso: &str) -> Result<()> {
        let db = self.db;
        let rec = self.record(now_iso);
        db.begin()?;
        let res = (|| -> Result<()> {
            let seq = db.next_seq()?;
            if let ProjectChoice::New { id, name } = self.project {
                db.execute(
                    "INSERT INTO projects(id, name, status, created_at, updated_at, seq) VALUES (?1, ?2, 'open', ?3, ?3, ?4)",
                    &[id.into(), name.into(), now_iso.into(), seq.into()],
                )?;
            }
            db.execute(
                "INSERT INTO mags(id, project_id, folder, shoot_date, camera_profile_id, file_format, gamma, gamut, status,
                                  logging_complete, size_bytes, file_count, source_label, ingested_by, ingested_at, notes,
                                  updated_at, seq)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'online', 0, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?15)",
                &[
                    rec.mag_uuid.as_str().into(),
                    rec.project_uuid.as_str().into(),
                    rec.folder.as_str().into(),
                    rec.shoot_date.as_str().into(),
                    rec.camera_profile_id.clone().into(),
                    rec.file_format.as_str().into(),
                    rec.gamma.as_str().into(),
                    rec.gamut.as_str().into(),
                    self.probe.total_bytes.into(),
                    (self.probe.files.len() as i64).into(),
                    rec.source_label.as_str().into(),
                    rec.ingested_by.as_str().into(),
                    now_iso.into(),
                    Value::from(Some(rec.notes.as_str()).filter(|n| !n.is_empty())),
                    seq.into(),
                ],
            )?;
            let queue_proxy = !self.probe.clips.is_empty();
            let mut clip_ids = Vec::with_capacity(self.probe.clips.len());
            for c in &self.probe.clips {
                let f = &self.probe.files[c.file];
                let id = util::uuid_v4()?;
                let name = Path::new(&f.rel).file_name().unwrap().to_string_lossy().into_owned();
                db.execute(
                    "INSERT INTO clips(id, mag_id, relative_path, original_filename, size, checksum, duration_ms, fps, width, height,
                                       audio_streams, audio_channels, proxy_status, created_at, updated_at, seq)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15)",
                    &[
                        id.as_str().into(),
                        rec.mag_uuid.as_str().into(),
                        f.rel.as_str().into(),
                        name.into(),
                        f.size.into(),
                        self.hashes[c.file].sha256.as_str().into(),
                        c.info.duration_ms.into(),
                        c.info.fps.clone().into(),
                        c.info.width.into(),
                        c.info.height.into(),
                        c.info.audio_streams.into(),
                        c.info.audio_channels.clone().into(),
                        (if queue_proxy { "pending" } else { "none" }).into(),
                        now_iso.into(),
                        seq.into(),
                    ],
                )?;
                clip_ids.push(id);
            }
            for (f, h) in self.probe.files.iter().zip(self.hashes) {
                let clip = f.clip.map(|i| clip_ids[i].clone());
                db.execute(
                    "INSERT INTO files(mag_id, relative_path, size, mtime, sha256, role, clip_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    &[
                        rec.mag_uuid.as_str().into(),
                        f.rel.as_str().into(),
                        f.size.into(),
                        time::iso_utc(f.mtime).into(),
                        h.sha256.as_str().into(),
                        f.role.as_str().into(),
                        clip.into(),
                    ],
                )?;
            }
            if queue_proxy {
                db.execute(
                    "INSERT INTO encoder_jobs(id, type, mag_id, profile_id, priority, status, clips_total, output_root,
                                              requested_by, created_at)
                     VALUES (?1, '320p', ?2, ?3, 0, 'queued', ?4, ?5, ?6, ?7)",
                    &[
                        util::uuid_v4()?.into(),
                        rec.mag_uuid.as_str().into(),
                        self.proxy_profile.id.as_str().into(),
                        (clip_ids.len() as i64).into(),
                        format!("proxies/db/{}", self.folder).into(),
                        rec.ingested_by.as_str().into(),
                        now_iso.into(),
                    ],
                )?;
            }
            // Publish the folder, then commit. If the commit fails, take the folder back out.
            std::fs::rename(self.staging, self.final_dir)
                .ctx(format!("cannot move {} into {}", self.staging.display(), self.cfg.paths.mags.display()))?;
            if let Err(e) = db.commit() {
                let _ = std::fs::rename(self.final_dir, self.staging);
                return Err(e);
            }
            sync_dir(&self.cfg.paths.mags);
            Ok(())
        })();
        if res.is_err() {
            db.rollback();
        }
        res
    }
}

fn require_tool(name: &str, args: &[&str], why: &str) -> Result<()> {
    match Command::new(name).args(args).output() {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(Error(format!("`{name}` not found or not working: {why}"))),
    }
}

/// Report staging folders left by an ingest that was killed (SIGKILL, power loss): a folder
/// in `incoming/` whose `.lock` is not held by a running ingest. Printed and logged as WARN;
/// cleanup is manual. Folders of ingests still running are skipped.
fn warn_stale_incoming(incoming: &Path, log: &Log) {
    let Ok(rd) = std::fs::read_dir(incoming) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    let mut items = Vec::new();
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let lock = incoming.join(format!(".{name}.lock"));
        let live = match File::open(&lock) {
            Ok(f) => matches!(f.try_lock(), Err(TryLockError::WouldBlock)),
            Err(_) => false,
        };
        // Re-check: a live ingest may have just finished and moved its folder away.
        if live || !e.path().is_dir() {
            continue;
        }
        let mut item = format!("{}/", incoming.join(&name).display());
        for extra in [format!(".{name}.files"), format!(".{name}.lock")] {
            let p = incoming.join(&extra);
            if p.exists() {
                item.push_str(&format!(" + {}", p.display()));
            }
        }
        items.push(item);
    }
    if items.is_empty() {
        return;
    }
    let msg = format!(
        "{} holds {} folder(s) from an interrupted ingest (not in mags/, not in the database; remove by hand)",
        incoming.display(),
        items.len()
    );
    eprintln!("warning: {}", esc(&msg));
    for i in &items {
        eprintln!("  {}", esc(i));
    }
    log.write(Level::Warn, "ingest", &msg, &items);
}

fn resolve_320p_profile<'a>(cfg: &Config, encs: &'a [EncoderProfile]) -> Result<&'a EncoderProfile> {
    if let Some(id) = cfg.setting("default_profile_320p") {
        return encs
            .iter()
            .find(|e| e.id == id && e.kind == "320p")
            .ok_or_else(|| Error(format!("default_profile_320p {id:?} (magvault.conf) is not a valid type: 320p profile in {}", cfg.paths.encoders.display())));
    }
    let list: Vec<&EncoderProfile> = encs.iter().filter(|e| e.kind == "320p").collect();
    match list.as_slice() {
        [one] => Ok(one),
        [] => Err(Error(format!(
            "no valid encoder profile with `type: 320p` in {}; the 320p proxy job cannot be queued",
            cfg.paths.encoders.display()
        ))),
        many => Err(Error(format!(
            "several 320p encoder profiles ({}); set default_profile_320p in magvault.conf",
            many.iter().map(|e| e.id.as_str()).collect::<Vec<_>>().join(", ")
        ))),
    }
}

fn choose_camera<'a>(a: &Args, probe: &Probe, cams: &'a [CameraProfile]) -> Result<&'a CameraProfile> {
    let available = || cams.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", ");
    if cams.is_empty() {
        crate::bail!("no valid camera profiles; ingest can only offer values from existing profiles (missing profile? admin only)");
    }
    match &a.camera {
        Some(id) => cams.iter().find(|c| &c.id == id).ok_or_else(|| Error(format!("camera profile {id:?} not found (available: {})", available()))),
        None => probe::guess_camera(probe, cams)
            .ok_or_else(|| Error(format!("could not guess the camera; pass --camera <profile id> (available: {})", available()))),
    }
}

fn choose_project(db: &Db, a: &Args) -> Result<ProjectChoice> {
    if let Some(id) = &a.project_id {
        let row = db
            .query_one("SELECT id, name, status FROM projects WHERE id = ?1", &[id.as_str().into()])?
            .ok_or_else(|| Error(format!("project {id} not found")))?;
        if row[2].s() != "open" {
            crate::bail!("project {} [{id}] is closed; closed projects do not receive new mags", row[1].s());
        }
        if let Some(name) = &a.project {
            if &row[1].s() != name {
                crate::bail!("--project {name:?} does not match the name of project {id} ({:?})", row[1].s());
            }
        }
        return Ok(ProjectChoice::Existing { id: row[0].s(), name: row[1].s(), status: row[2].s() });
    }
    let Some(name) = a.project.as_deref().map(str::trim).filter(|n| !n.is_empty()) else {
        crate::bail!("choose a project: --project <name> (an open project, or a new one) or --project-id <uuid>");
    };
    check_card_field("project name", name)?;
    let rows = db.query("SELECT id, name, status FROM projects WHERE name = ?1 ORDER BY created_at", &[name.into()])?;
    let open: Vec<_> = rows.iter().filter(|r| r[2].s() == "open").collect();
    match open.as_slice() {
        [one] => Ok(ProjectChoice::Existing { id: one[0].s(), name: one[1].s(), status: "open".into() }),
        [] if rows.is_empty() => Ok(ProjectChoice::New { id: util::uuid_v4()?, name: name.to_string() }),
        [] => Err(Error(format!(
            "project {name:?} exists but is closed; closed projects are hidden from ingest. Use another name or reopen it first."
        ))),
        many => Err(Error(format!(
            "{} open projects are named {name:?}; pick one with --project-id ({})",
            many.len(),
            many.iter().map(|r| r[0].s()).collect::<Vec<_>>().join(", ")
        ))),
    }
}
