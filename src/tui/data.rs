//! What the TUI shows: a snapshot gathered from the same functions the CLI uses (card
//! discovery, field sets, Prepare candidates, server ping, project list, staging). The
//! screens only read it; tests build it by hand.

use crate::cardinfo::{Requirements, Summary};
use crate::client::{self, Transport};
use crate::config::ClientConfig;
use crate::discover::{self, Card};
use crate::fieldset::{self, Candidate, FieldSet};
use crate::probe;
use crate::profiles::{self, CameraProfile};
use crate::staging::{self, DumpStatus};
use crate::time::DisplayStyle;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// PLAN.md "Client ↔ server status": ready only when the committer answers.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerState {
    NotConfigured,
    /// Gate up and `magvault serve` alive (host).
    Ready(String),
    /// Gate up, committer down (host, why).
    CommitterDown(String, String),
    /// Gate not reachable (host, why).
    Offline(String, String),
}

impl ServerState {
    /// Short status for the operator.
    pub fn plain(&self) -> &'static str {
        match self {
            ServerState::Ready(_) => "server ready",
            ServerState::CommitterDown(..) => crate::client::COMMITTER_DOWN,
            ServerState::Offline(..) => "server offline",
            ServerState::NotConfigured => "no server configured",
        }
    }
    pub fn host(&self) -> &str {
        match self {
            ServerState::Ready(h) | ServerState::CommitterDown(h, _) | ServerState::Offline(h, _) => h,
            ServerState::NotConfigured => "",
        }
    }
}

/// A drive that is not part of a valid set.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)] // a handful of rows
pub enum DriveRow {
    /// A real disk from lsblk: offered for Prepare (`Ok`) or refused with the reason.
    Disk(Candidate),
    /// A `--scan-root` folder without a valid backup file (tests / demos): never prepared.
    Folder(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub server: ServerState,
    pub staging: PathBuf,
    pub staging_pending: Vec<String>,
    /// Staged mags the server confirmed (kept until a second backup exists).
    pub staging_synced: Vec<String>,
    /// Bytes used by staging and free on its filesystem.
    pub staging_usage: (u64, u64),
    /// `staging_autoclean` (PLAN.md "Client staging cache").
    pub autoclean: bool,
    pub cards: Vec<Card>,
    /// Dump status per card (same order as `cards`).
    pub dump: Vec<DumpStatus>,
    pub sets: Vec<FieldSet>,
    pub drives: Vec<DriveRow>,
    pub projects: Vec<(String, String)>,
    /// Requirements of known projects (project id → requirements).
    pub requirements: BTreeMap<String, Requirements>,
    pub cameras: Vec<CameraProfile>,
    pub activity: Vec<String>,
    /// How dates and times are shown (magvault.conf).
    pub display: DisplayStyle,
}

impl Snapshot {
    pub fn empty(staging: PathBuf) -> Snapshot {
        Snapshot {
            server: ServerState::NotConfigured,
            staging,
            staging_pending: Vec::new(),
            staging_synced: Vec::new(),
            staging_usage: (0, 0),
            autoclean: true,
            cards: Vec::new(),
            dump: Vec::new(),
            sets: Vec::new(),
            drives: Vec::new(),
            projects: Vec::new(),
            requirements: BTreeMap::new(),
            cameras: Vec::new(),
            activity: Vec::new(),
            display: DisplayStyle::default(),
        }
    }
    /// The server can commit now.
    pub fn ready(&self) -> bool {
        matches!(self.server, ServerState::Ready(_))
    }
}

/// Card preview for the Card and Details panes (the same probe and guesses as `ingest`).
/// No file list (PLAN.md "Time display": Block 2 is a card summary).
#[derive(Debug, Clone)]
pub struct Preview {
    /// Clips, size, clip time range, frame rate, resolution, GOP.
    pub summary: Summary,
    pub total: u64,
    /// Camera profile id, or why there is no guess (same rule as `ingest`).
    pub camera: Result<String, String>,
    /// Shoot day and where it came from, when it can be guessed.
    pub shoot: Option<(String, String)>,
    /// Camera model read next to live media, with its source (operator confirms).
    pub model_hint: Option<String>,
    /// Models named by live media when the card holds more than one camera.
    pub mixed: Vec<String>,
}

pub fn gather(cc: &ClientConfig, scan_roots: &[PathBuf]) -> Snapshot {
    let t = Transport::new(cc);
    let server = match &t {
        None => ServerState::NotConfigured,
        Some(t) => match t.ping() {
            Ok(crate::client::ServerStatus::Ready) => ServerState::Ready(t.describe()),
            Ok(crate::client::ServerStatus::CommitterDown(why)) => ServerState::CommitterDown(t.describe(), why),
            Err(e) => ServerState::Offline(t.describe(), e.0.lines().next().unwrap_or("").to_string()),
        },
    };
    let online = matches!(server, ServerState::Ready(_) | ServerState::CommitterDown(..));
    let t_online = if online { t.as_ref() } else { None };
    let projects = client::projects(cc, t_online);
    let requirements = client::requirements(cc, t_online);
    let cameras = profiles::load_cameras(&cc.cameras).profiles;
    let cards = discover::find_cards(&discover::volumes(scan_roots), &cameras);
    let all_drives = fieldset::drives(scan_roots);
    let sets = fieldset::sets(&all_drives);
    let ledger = staging::dumps(cc);
    let dump = cards.iter().map(|c| staging::card_status(&ledger, c.fingerprint.as_deref(), &cc.staging, &sets)).collect();
    let drives = if scan_roots.is_empty() {
        fieldset::candidates()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| !matches!(&c.disk, Ok(d) if d.backup.is_some()))
            .map(DriveRow::Disk)
            .collect()
    } else {
        all_drives
            .iter()
            .filter(|d| !matches!(d.backup, Ok(Some(_))) && scan_roots.iter().any(|r| d.mount.starts_with(r)))
            .filter(|d| discover::layout_of(&d.mount, &cameras).is_none())
            .map(|d| DriveRow::Folder(d.mount.clone()))
            .collect()
    };
    Snapshot {
        server,
        staging: cc.staging.clone(),
        staging_pending: staging::pending(&cc.staging),
        staging_synced: staging::synced(&cc.staging).into_iter().collect(),
        staging_usage: staging::usage(&cc.staging),
        autoclean: cc.staging_autoclean,
        cards,
        dump,
        sets,
        drives,
        projects,
        requirements,
        cameras,
        activity: recent_log(&cc.logs, 12),
        display: cc.display,
    }
}

/// The newest lines of the newest log file, newest first.
fn recent_log(logs: &Path, n: usize) -> Vec<String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(logs)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "log")).collect())
        .unwrap_or_default();
    files.sort();
    let Some(last) = files.last() else { return Vec::new() };
    let text = std::fs::read_to_string(last).unwrap_or_default();
    text.lines().rev().take(n).map(str::to_string).collect()
}

/// Probe a card for the preview (same probe and guesses as `ingest`, nothing printed).
pub fn preview(path: &Path, cameras: &[CameraProfile]) -> Result<Preview, String> {
    let p = probe::probe(path, probe::ffprobe_available()).map_err(|e| e.0)?;
    if p.files.is_empty() {
        return Err(format!("{} contains no files", path.display()));
    }
    let hints = crate::camera_meta::read(&p);
    let mixed = if hints.len() > 1 { hints.iter().map(|h| h.model.clone()).collect() } else { Vec::new() };
    let camera = discover::guess_camera(&p, cameras, &hints).map(|c| c.id.clone());
    let model_hint = crate::camera_meta::describe(&hints);
    let shoot = probe::guess_shoot_day(&p).map(|(d, src)| (d, src.to_string()));
    Ok(Preview { summary: crate::cardinfo::summarize(&p), total: p.total_bytes, camera, shoot, model_hint, mixed })
}
