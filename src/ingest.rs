//! Ingest building blocks and the server-side (Phase 0) manual-path ingest.
//!
//! Server ingest (PLAN.md "Ingest flow"):
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
//!
//! The same blocks are used by client ingest (`client_ingest.rs`, Phase 1) and by the
//! server commit of client uploads (`serve.rs`).

use crate::catalog::{card_from_record, MagRecord};
use crate::config::{self, Config};
use crate::db::{self, Db, Value};
use crate::error::{Context, Error, Result};
use crate::hash;
use crate::interrupt;
use crate::log::{Level, Log};
use crate::magcard::{self, ManifestRow};
use crate::mhl;
use crate::probe::{self, Clip, Probe, Role};
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

#[derive(Debug, Clone)]
pub enum ProjectChoice {
    Existing {
        id: String,
        name: String,
        status: String,
    },
    New {
        id: String,
        name: String,
    },
}

impl ProjectChoice {
    pub fn id(&self) -> &str {
        match self {
            ProjectChoice::Existing { id, .. } | ProjectChoice::New { id, .. } => id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ProjectChoice::Existing { name, .. } | ProjectChoice::New { name, .. } => name,
        }
    }
    pub fn status(&self) -> &str {
        match self {
            ProjectChoice::Existing { status, .. } => status,
            ProjectChoice::New { .. } => "open",
        }
    }
    pub fn summary_line(&self) -> String {
        match self {
            ProjectChoice::Existing { name, id, .. } => format!("{name} [{id}]"),
            ProjectChoice::New { name, .. } => {
                format!("{name}  (NEW project, UUID assigned on commit)")
            }
        }
    }
}

pub struct Hashed {
    pub sha256: String,
    pub c4: String,
}

/// What the operator confirmed (or what was guessed and accepted).
pub struct Details {
    pub cam: CameraProfile,
    pub file_format: String,
    pub gamma: String,
    pub gamut: String,
    pub shoot_date: String,
    pub shoot_src: &'static str,
    pub notes: String,
    pub source_label: String,
}

/// One file of a new mag as it goes into the manifest, the database and the MHL.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub rel: String,
    pub size: u64,
    pub mtime: i64,
    pub sha256: String,
    pub role: Role,
    pub clip: Option<usize>,
}

pub struct Outcome {
    pub folder: String,
    pub mag_dir: PathBuf,
    pub files: usize,
    pub clips: usize,
    pub bytes: u64,
}

// ---- building blocks -----------------------------------------------------------------

pub fn require_tool(name: &str, args: &[&str], why: &str) -> Result<()> {
    match Command::new(name).args(args).output() {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(Error(format!("`{name}` not found or not working: {why}"))),
    }
}

/// Canonical source directory. Refuses sources that overlap a Magvault root and folders
/// that already are Magvault mags (they hold `magcard.txt` / `ascmhl/`).
pub fn check_source(source: &Path, roots: &[&Path]) -> Result<PathBuf> {
    let src = std::fs::canonicalize(source)
        .ctx(format!("source {}", esc(&source.display().to_string())))?;
    if !src.is_dir() {
        crate::bail!(
            "source {} is not a directory",
            esc(&src.display().to_string())
        );
    }
    for root in roots {
        if let Ok(r) = std::fs::canonicalize(root) {
            if src.starts_with(&r) || r.starts_with(&src) {
                crate::bail!(
                    "source {} overlaps the Magvault root {}; ingest from a card or a separate folder",
                    esc(&src.display().to_string()),
                    r.display()
                );
            }
        }
    }
    if src.join(magcard::FILE_NAME).exists() || src.join(mhl::DIR).is_dir() {
        crate::bail!(
            "source {} already contains {} or {}/ — it looks like a Magvault mag folder, not a card. \
             Mags on a field backup set are sent to the server with `magvault sync`.",
            esc(&src.display().to_string()),
            magcard::FILE_NAME,
            mhl::DIR
        );
    }
    if src.join(crate::fieldset::FILE).exists() {
        crate::bail!(
            "source {} is a Magvault field backup drive, not a card",
            esc(&src.display().to_string())
        );
    }
    Ok(src)
}

pub fn load_cameras_logged(dir: &Path, log: &Log) -> Vec<CameraProfile> {
    let cams = profiles::load_cameras(dir);
    for (p, e) in &cams.invalid {
        eprintln!("warning: camera profile {} ignored: {e}", p.display());
        log.warn(
            "ingest",
            &format!("camera profile {} ignored: {e}", p.display()),
        );
    }
    cams.profiles
}

pub fn probe_source(src: &Path) -> Result<Probe> {
    eprintln!("probing {} …", esc(&src.display().to_string()));
    let use_ffprobe = probe::ffprobe_available();
    if !use_ffprobe {
        eprintln!(
            "warning: ffprobe not found; media details (duration, fps, size, audio) will be empty"
        );
    }
    let probe = probe::probe(src, use_ffprobe)?;
    if probe.files.is_empty() {
        crate::bail!(
            "nothing to ingest: {} contains no files",
            esc(&src.display().to_string())
        );
    }
    for c in &probe.clips {
        if let Some(e) = &c.info.probe_error {
            eprintln!(
                "warning: ffprobe could not read {}: {}",
                esc(&probe.files[c.file].rel),
                esc(e)
            );
        }
    }
    Ok(probe)
}

/// Camera, format, gamma, gamut, shoot day and notes: from flags, else guessed; values
/// only from the camera profile (PLAN.md "Camera profiles").
pub fn choose_details(
    a: &Args,
    probe: &Probe,
    cams: &[CameraProfile],
    src: &Path,
) -> Result<Details> {
    let cam = choose_camera(a, probe, cams)?.clone();
    let pick =
        |what: &str, given: &Option<String>, list: &[String], default: &str| -> Result<String> {
            let v = given.clone().unwrap_or_else(|| default.to_string());
            if !list.contains(&v) {
                crate::bail!(
                    "{what} {v:?} is not offered by camera profile {} (choose one of: {})",
                    cam.id,
                    list.join(", ")
                );
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
        None => probe::guess_shoot_day(probe).ok_or_else(|| {
            Error("cannot guess the shoot day; pass --shoot-date YYYY-MM-DD".into())
        })?,
    };
    let notes = a.notes.clone().unwrap_or_default();
    check_card_field("--notes", &notes)?;
    let source_label = src
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    check_card_field("source folder name", &source_label)?;
    Ok(Details {
        cam,
        file_format,
        gamma,
        gamut,
        shoot_date,
        shoot_src,
        notes,
        source_label,
    })
}

/// The confirmation summary (stderr). `extra` lines go after the project line.
pub fn print_summary(
    src: &Path,
    probe: &Probe,
    a: &Args,
    d: &Details,
    project: &ProjectChoice,
    extra: &[String],
) {
    let media_count = probe.clips.len();
    let sidecars = probe
        .files
        .iter()
        .filter(|f| f.role == Role::Sidecar)
        .count();
    eprintln!();
    eprintln!("  source      {}", esc(&src.display().to_string()));
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
        eprintln!(
            "  skipped     {} junk item(s): {}",
            skipped.len(),
            skipped.join(", ")
        );
    }
    let guessed = |given: bool| if given { "" } else { "  (guessed)" };
    eprintln!(
        "  camera      {} [{}]{}",
        d.cam.name,
        d.cam.id,
        guessed(a.camera.is_some())
    );
    eprintln!(
        "  format      {}{}",
        d.file_format,
        guessed(a.format.is_some())
    );
    eprintln!("  gamma       {}{}", d.gamma, guessed(a.gamma.is_some()));
    eprintln!("  gamut       {}{}", d.gamut, guessed(a.gamut.is_some()));
    eprintln!(
        "  shoot day   {}{}",
        d.shoot_date,
        if d.shoot_src == "given" {
            String::new()
        } else {
            format!("  (guessed: {})", d.shoot_src)
        }
    );
    eprintln!("  project     {}", project.summary_line());
    if !d.notes.is_empty() {
        eprintln!("  notes       {}", d.notes);
    }
    for l in extra {
        eprintln!("  {l}");
    }
    eprintln!();
}

/// First read of the card: SHA-256 (and C4) of every file (PLAN.md "Hashing").
pub fn hash_source(probe: &Probe) -> Result<Vec<Hashed>> {
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
        hashes.push(Hashed {
            sha256: h.sha256,
            c4: h.c4.unwrap_or_default(),
        });
    }
    prog.finish();
    Ok(hashes)
}

/// Free space on the filesystem of `dir`: `bytes` plus 1 % plus `extra`.
pub fn check_space(dir: &Path, bytes: u64, extra: u64) -> Result<()> {
    let need = bytes + bytes / 100 + extra;
    let avail = util::free_bytes(dir)?;
    if avail < need {
        crate::bail!(
            "not enough space in {}: need {} (card {} + margin), {} free",
            dir.display(),
            human(need),
            human(bytes),
            human(avail)
        );
    }
    Ok(())
}

pub fn file_entries(probe: &Probe, hashes: &[Hashed]) -> Vec<FileEntry> {
    probe
        .files
        .iter()
        .zip(hashes)
        .map(|(f, h)| FileEntry {
            rel: f.rel.clone(),
            size: f.size,
            mtime: f.mtime,
            sha256: h.sha256.clone(),
            role: f.role,
            clip: f.clip,
        })
        .collect()
}

pub fn new_folder_name(shoot_date: &str) -> Result<(String, String)> {
    let mag_uuid = util::uuid_v4()?;
    Ok((
        format!("{}_{}", shoot_date.replace('-', ""), mag_uuid),
        mag_uuid,
    ))
}

/// Mag folder name: `YYYYMMDD_<full lowercase uuid>`.
pub fn is_mag_folder(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 45
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'_'
        && util::uuid_ok(&name[9..])
}

pub fn build_record(
    folder: &str,
    mag_uuid: &str,
    project: &ProjectChoice,
    d: &Details,
    entries: &[FileEntry],
) -> MagRecord {
    let now_iso = time::iso_utc(time::now_secs());
    MagRecord {
        mag_uuid: mag_uuid.into(),
        folder: folder.into(),
        project_uuid: project.id().into(),
        project_name: project.name().into(),
        project_status: project.status().into(),
        shoot_date: d.shoot_date.clone(),
        camera_profile_id: Some(d.cam.id.clone()),
        file_format: d.file_format.clone(),
        gamma: d.gamma.clone(),
        gamut: d.gamut.clone(),
        source_label: d.source_label.clone(),
        ingested_by: util::identity(),
        ingested_at: now_iso.clone(),
        updated_at: now_iso,
        notes: d.notes.clone(),
        status: "online".into(),
        manifest: entries
            .iter()
            .map(|e| ManifestRow {
                rel: e.rel.clone(),
                size: e.size,
                sha256: e.sha256.clone(),
                mtime: time::iso_utc(e.mtime),
            })
            .collect(),
        logging: Vec::new(),
    }
}

/// Copy `rels` from `src_root` into `dest` with rsync (PLAN.md "Copy method"). Stops on
/// Ctrl+C / SIGTERM (a SIGTERM to Magvault alone is passed on to rsync).
pub fn rsync_copy(src_root: &Path, rels: &[&str], dest: &Path, list_file: &Path) -> Result<()> {
    rsync_to(
        src_root,
        rels,
        &dest.as_os_str().to_string_lossy(),
        list_file,
        &[],
    )
}

/// rsync into a local directory or a remote `host:path` destination (`extra` = e.g.
/// `-e ssh …`). The file list is written to `list_file` and NUL-separated.
pub fn rsync_to(
    src_root: &Path,
    rels: &[&str],
    dest: &str,
    list_file: &Path,
    extra: &[String],
) -> Result<()> {
    let mut list = Vec::new();
    for r in rels {
        list.extend_from_slice(r.as_bytes());
        list.push(0);
    }
    std::fs::write(list_file, &list).ctx(format!("cannot write {}", list_file.display()))?;
    let mut src_arg = src_root.as_os_str().to_owned();
    src_arg.push("/");
    let dst_arg = if dest.ends_with('/') {
        dest.to_string()
    } else {
        format!("{dest}/")
    };
    eprintln!("copying with rsync → {}", esc(&dst_arg));
    let mut child = Command::new("rsync")
        .arg("-t")
        .arg("--no-links")
        .arg("--no-devices")
        .arg("--no-specials")
        .arg("--from0")
        .arg(format!("--files-from={}", list_file.display()))
        .arg("--info=progress2")
        .arg("--outbuf=L")
        .args(extra)
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
        crate::bail!(
            "rsync failed ({}); nothing committed",
            status
                .code()
                .map_or("killed".into(), |c| format!("exit {c}"))
        );
    }
    Ok(())
}

fn internal(rel: &str) -> bool {
    rel == magcard::FILE_NAME
        || rel == mhl::DIR
        || rel.starts_with(&format!("{}/", mhl::DIR))
        || rel.starts_with(".magvault/")
}

/// Every later copy is verified against the hashes stored at ingest (PLAN.md "Hashing"):
/// exactly the expected files (Magvault's own `magcard.txt`, `ascmhl/`, `.magvault/` are
/// allowed alongside), each with the expected size and SHA-256.
pub fn verify_tree(dir: &Path, expected: &[(&str, u64, &str)], label: &str) -> Result<()> {
    let (copied, _) = probe::walk(dir).ctx("cannot list the copied files")?;
    let copied: Vec<_> = copied.into_iter().filter(|f| !internal(&f.rel)).collect();
    let want: BTreeMap<&str, (u64, &str)> =
        expected.iter().map(|(r, s, h)| (*r, (*s, *h))).collect();
    let got: BTreeSet<&str> = copied.iter().map(|f| f.rel.as_str()).collect();
    let mut problems = Vec::new();
    for rel in want.keys().filter(|r| !got.contains(*r)) {
        problems.push(format!("missing after copy: {}", esc(rel)));
    }
    for rel in got.iter().filter(|r| !want.contains_key(*r)) {
        problems.push(format!("unexpected file after copy: {}", esc(rel)));
    }
    let total: u64 = expected.iter().map(|e| e.1).sum();
    let mut prog = Progress::new(label, total);
    for f in &copied {
        let Some(&(size, sha)) = want.get(f.rel.as_str()) else {
            continue;
        };
        if f.size != size {
            problems.push(format!(
                "size differs: {} ({} vs {} bytes)",
                esc(&f.rel),
                f.size,
                size
            ));
            continue;
        }
        let h = hash::hash_file(&f.abs, false, &mut |n| {
            prog.add(n);
            !interrupt::requested()
        });
        interrupt::check()?;
        let h = h.ctx(format!("cannot read copied {}", esc(&f.rel)))?;
        if h.sha256 != sha {
            problems.push(format!(
                "SHA-256 mismatch: {} (expected {}, copy {})",
                esc(&f.rel),
                sha,
                h.sha256
            ));
        }
    }
    prog.finish();
    if !problems.is_empty() {
        crate::bail!(
            "integrity error: the copy does not match the stored SHA-256; nothing committed\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(())
}

pub fn expected_of(entries: &[FileEntry]) -> Vec<(&str, u64, &str)> {
    entries
        .iter()
        .map(|e| (e.rel.as_str(), e.size, e.sha256.as_str()))
        .collect()
}

/// Write `magcard.txt` and the ASC MHL into `dir`.
pub fn write_cards(
    dir: &Path,
    rec: &MagRecord,
    camera_model: &str,
    entries: &[FileEntry],
    c4s: &[String],
) -> Result<()> {
    let card = card_from_record(rec, Some(camera_model));
    atomic_write(&dir.join(magcard::FILE_NAME), card.render()?.as_bytes())?;
    let mhl_entries: Vec<mhl::Entry> = entries
        .iter()
        .zip(c4s)
        .map(|(e, c4)| mhl::Entry {
            rel: &e.rel,
            size: e.size,
            mtime: e.mtime,
            c4,
        })
        .collect();
    let now = time::now_secs();
    mhl::write_initial(dir, &rec.folder, &mhl_entries, now, now, &util::hostname())?;
    Ok(())
}

/// One database transaction for a new mag: project (if new), mag, clips, files, queued 320p
/// job. `staging` is renamed to `final_dir` inside the transaction; the transaction is
/// committed only after the rename succeeded, and the rename is undone if the commit fails.
#[allow(clippy::too_many_arguments)]
pub fn commit_db(
    db: &Db,
    rec: &MagRecord,
    entries: &[FileEntry],
    clips: &[Clip],
    proxy_profile: &str,
    project: &ProjectChoice,
    staging: &Path,
    final_dir: &Path,
) -> Result<()> {
    let total: u64 = entries.iter().map(|e| e.size).sum();
    db.begin()?;
    let res = (|| -> Result<()> {
        let seq = db.next_seq()?;
        let now_iso = rec.ingested_at.as_str();
        if let ProjectChoice::New { id, name } = project {
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
                total.into(),
                (entries.len() as i64).into(),
                rec.source_label.as_str().into(),
                rec.ingested_by.as_str().into(),
                now_iso.into(),
                Value::from(Some(rec.notes.as_str()).filter(|n| !n.is_empty())),
                seq.into(),
            ],
        )?;
        let queue_proxy = !clips.is_empty();
        let mut clip_ids = Vec::with_capacity(clips.len());
        for c in clips {
            let f = &entries[c.file];
            let id = util::uuid_v4()?;
            let name = Path::new(&f.rel)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
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
                    f.sha256.as_str().into(),
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
        for f in entries {
            let clip = f.clip.map(|i| clip_ids[i].clone());
            db.execute(
                "INSERT INTO files(mag_id, relative_path, size, mtime, sha256, role, clip_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                &[
                    rec.mag_uuid.as_str().into(),
                    f.rel.as_str().into(),
                    f.size.into(),
                    time::iso_utc(f.mtime).into(),
                    f.sha256.as_str().into(),
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
                    proxy_profile.into(),
                    (clip_ids.len() as i64).into(),
                    format!("proxies/db/{}", rec.folder).into(),
                    rec.ingested_by.as_str().into(),
                    now_iso.into(),
                ],
            )?;
        }
        // Publish the folder, then commit. If the commit fails, take the folder back out.
        std::fs::rename(staging, final_dir).ctx(format!(
            "cannot move {} into {}",
            staging.display(),
            final_dir.display()
        ))?;
        if let Err(e) = db.commit() {
            let _ = std::fs::rename(final_dir, staging);
            return Err(e);
        }
        if let Some(parent) = final_dir.parent() {
            sync_dir(parent);
        }
        Ok(())
    })();
    if res.is_err() {
        db.rollback();
    }
    res
}

// ---- server (Phase 0) ingest ------------------------------------------------------------

pub fn run(cfg: &Config, a: Args) -> Result<Outcome> {
    let log = Log::new(&cfg.paths.logs);
    require_tool("rsync", &["--version"], "rsync is required for every copy (PLAN.md \"Copy method\")")?;
    config::ensure_roots(&cfg.paths)?;
    config::check_incoming(&cfg.paths)?;
    let db = Db::open(&cfg.paths.db_file())?;
    db::ensure_schema(&db)?;

    let src = check_source(&a.source, &cfg.paths.data_roots())?;
    warn_stale_incoming(&cfg.paths.incoming, &log);

    let cams = load_cameras_logged(&cfg.paths.cameras, &log);
    let encs = profiles::load_encoders(&cfg.paths.encoders);
    for (p, e) in &encs.invalid {
        eprintln!("warning: encoder profile {} ignored: {e}", p.display());
    }
    let proxy_profile = resolve_320p_profile(cfg, &encs.profiles)?;

    let probe = probe_source(&src)?;
    let d = choose_details(&a, &probe, &cams, &src)?;
    let project = choose_project(&db, &a)?;
    print_summary(
        &src,
        &probe,
        &a,
        &d,
        &project,
        &[
            format!(
                "proxy       320p job queued with profile {} (not encoded in Phase 0)",
                proxy_profile.id
            ),
            format!(
                "destination {}/{}_<new uuid>/",
                cfg.paths.mags.display(),
                d.shoot_date.replace('-', "")
            ),
        ],
    );
    if !a.yes && !util::confirm("Pull (verified copy) with these values?")? {
        crate::bail!("cancelled; nothing was written");
    }

    // ---- after confirmation: every failure is cleaned up and logged as ERROR ------------
    interrupt::install();
    let mut staged: Option<Staged> = None;
    let result = (|| -> Result<String> {
        let hashes = hash_source(&probe)?;
        check_space(&cfg.paths.incoming, probe.total_bytes, 1 << 30)?;
        interrupt::check()?;
        let (folder, mag_uuid) = new_folder_name(&d.shoot_date)?;
        let final_dir = cfg.paths.mags.join(&folder);
        if final_dir.exists() {
            crate::bail!("{folder} already exists in mags/ (UUID collision?); nothing was written");
        }
        let st = staged.insert(Staged::create(&cfg.paths.incoming, &folder)?);
        std::fs::create_dir(&st.dir).ctx(format!("cannot create {}", st.dir.display()))?;
        let rels: Vec<&str> = probe.files.iter().map(|f| f.rel.as_str()).collect();
        rsync_copy(&probe.root, &rels, &st.dir, &st.list)?;
        let entries = file_entries(&probe, &hashes);
        verify_tree(&st.dir, &expected_of(&entries), "verifying")?;
        interrupt::check()?;
        let rec = build_record(&folder, &mag_uuid, &project, &d, &entries);
        let c4s: Vec<String> = hashes.iter().map(|h| h.c4.clone()).collect();
        write_cards(&st.dir, &rec, &d.cam.name, &entries, &c4s)?;
        // Last point where a stop signal is honoured. From here the rename into mags/ and the
        // database commit run to the end; a signal arriving now is held off (PLAN.md Threat model).
        interrupt::check()?;
        commit_db(
            &db,
            &rec,
            &entries,
            &probe.clips,
            &proxy_profile.id,
            &project,
            &st.dir,
            &final_dir,
        )?;
        Ok(folder)
    })();
    if let Some(st) = staged {
        st.finish(result.is_ok());
    }
    let media_count = probe.clips.len();
    match result {
        Ok(folder) => {
            let msg = format!(
                "Mag {folder} ingested ({}) · {} files, {} clips · project {}",
                human(probe.total_bytes),
                probe.files.len(),
                media_count,
                project.name()
            );
            log.write(Level::Info, "ingest", &msg, &skipped_context(&probe));
            if media_count > 0 {
                log.info(
                    "encoder",
                    &format!(
                        "320p proxy job queued for {folder} ({} clips, profile {})",
                        media_count, proxy_profile.id
                    ),
                );
            }
            if interrupt::requested() {
                eprintln!("note: a stop signal arrived while the mag was being committed; the ingest completed");
            }
            let mag_dir = cfg.paths.mags.join(&folder);
            Ok(Outcome {
                folder,
                mag_dir,
                files: probe.files.len(),
                clips: media_count,
                bytes: probe.total_bytes,
            })
        }
        Err(e) => {
            log_failure(&log, &src, &e);
            Err(e)
        }
    }
}

pub fn skipped_context(probe: &Probe) -> Vec<String> {
    if probe.skipped.is_empty() {
        Vec::new()
    } else {
        vec![format!("skipped junk: {}", probe.skipped.join(", "))]
    }
}

pub fn log_failure(log: &Log, src: &Path, e: &Error) {
    let context: Vec<String> =
        e.0.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    log.error(
        "ingest",
        &format!("Ingest of {} failed; nothing committed", src.display()),
        &context,
    );
}

/// One ingest's footprint in an `incoming/` folder: the staging folder, its rsync list and
/// its lock. The lock file is created and flock-ed before the folder exists and held until
/// the end, so `warn_stale_incoming` never reports a folder whose ingest is still running.
pub struct Staged {
    pub dir: PathBuf,
    pub list: PathBuf,
    lock_path: PathBuf,
    lock: File,
}

impl Staged {
    pub fn create(incoming: &Path, folder: &str) -> Result<Staged> {
        std::fs::create_dir_all(incoming).ctx(format!("cannot create {}", incoming.display()))?;
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
    /// out; on failure it is deleted. The lock file goes last.
    pub fn finish(self, committed: bool) {
        if !committed && self.dir.exists() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
        let _ = std::fs::remove_file(&self.list);
        let _ = std::fs::remove_file(&self.lock_path);
        let _ = self.lock.unlock();
    }
}

/// Report staging folders left by an ingest that was killed (SIGKILL, power loss): a folder
/// in `incoming/` whose `.lock` is not held by a running ingest. Printed and logged as WARN;
/// cleanup is manual. Folders of ingests still running are skipped.
pub fn warn_stale_incoming(incoming: &Path, log: &Log) {
    let Ok(rd) = std::fs::read_dir(incoming) else {
        return;
    };
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
            many.iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn proxy_profile_id(cfg: &Config) -> Result<String> {
    let encs = profiles::load_encoders(&cfg.paths.encoders);
    Ok(resolve_320p_profile(cfg, &encs.profiles)?.id.clone())
}

fn choose_camera<'a>(
    a: &Args,
    probe: &Probe,
    cams: &'a [CameraProfile],
) -> Result<&'a CameraProfile> {
    let available = || {
        cams.iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if cams.is_empty() {
        crate::bail!("no valid camera profiles; ingest can only offer values from existing profiles (missing profile? admin only)");
    }
    match &a.camera {
        Some(id) => cams.iter().find(|c| &c.id == id).ok_or_else(|| {
            Error(format!(
                "camera profile {id:?} not found (available: {})",
                available()
            ))
        }),
        None => probe::guess_camera(probe, cams).ok_or_else(|| {
            Error(format!(
                "could not guess the camera; pass --camera <profile id> (available: {})",
                available()
            ))
        }),
    }
}

/// Server: pick an open project from the database by `--project-id` or `--project`.
pub fn choose_project(db: &Db, a: &Args) -> Result<ProjectChoice> {
    if let Some(id) = &a.project_id {
        let row = db
            .query_one(
                "SELECT id, name, status FROM projects WHERE id = ?1",
                &[id.as_str().into()],
            )?
            .ok_or_else(|| Error(format!("project {id} not found")))?;
        if row[2].s() != "open" {
            crate::bail!(
                "project {} [{id}] is closed; closed projects do not receive new mags",
                row[1].s()
            );
        }
        if let Some(name) = &a.project {
            if &row[1].s() != name {
                crate::bail!(
                    "--project {name:?} does not match the name of project {id} ({:?})",
                    row[1].s()
                );
            }
        }
        return Ok(ProjectChoice::Existing {
            id: row[0].s(),
            name: row[1].s(),
            status: row[2].s(),
        });
    }
    let name = project_name_arg(a)?;
    let rows = db.query(
        "SELECT id, name, status FROM projects WHERE name = ?1 ORDER BY created_at",
        &[name.as_str().into()],
    )?;
    let rows: Vec<(String, String, String)> = rows
        .into_iter()
        .map(|r| (r[0].s(), r[1].s(), r[2].s()))
        .collect();
    pick_project(&name, &rows)
}

pub fn project_name_arg(a: &Args) -> Result<String> {
    let Some(name) = a
        .project
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    else {
        crate::bail!("choose a project: --project <name> (an open project, or a new one) or --project-id <uuid>");
    };
    check_card_field("project name", name)?;
    Ok(name.to_string())
}

/// From all projects with this name (id, name, status): reuse the single open one, create a
/// new one if none exists, refuse if only closed ones or several open ones match.
pub fn pick_project(name: &str, rows: &[(String, String, String)]) -> Result<ProjectChoice> {
    let open: Vec<_> = rows.iter().filter(|r| r.2 == "open").collect();
    match open.as_slice() {
        [one] => Ok(ProjectChoice::Existing { id: one.0.clone(), name: one.1.clone(), status: "open".into() }),
        [] if rows.is_empty() => Ok(ProjectChoice::New { id: util::uuid_v4()?, name: name.to_string() }),
        [] => Err(Error(format!(
            "project {name:?} exists but is closed; closed projects are hidden from ingest. Use another name or reopen it first."
        ))),
        many => Err(Error(format!(
            "{} open projects are named {name:?}; pick one with --project-id ({})",
            many.len(),
            many.iter().map(|r| r.0.clone()).collect::<Vec<_>>().join(", ")
        ))),
    }
}
