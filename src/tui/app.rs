//! TUI state and keys. Pure: no terminal, no files. Keys change the state and may return an
//! `Action`; the runtime (`tui::run`) performs actions with the same functions as the CLI
//! (`client_ingest::run`, `usb::prepare_master`, `usb::prepare_slave`, `usb::clean`).
//!
//! Keys follow PLAN.md "Shared key convention": Esc back to the dashboard (or closes a
//! popup), q quits only on the dashboard, Tab / Shift+Tab panes, arrows move, Enter
//! selects. Ingest letters: n new project, r rescan, p pull, o Prepare, c Clean. Both
//! screens: y sync pending mags, d show / hide technical details (paths, UUIDs, devices).
//! While a step runs (pull, sync, Prepare, Clean) only Ctrl-C (cancel) and d work.

use super::data::{DriveRow, Preview, ServerState, Snapshot};
use crate::client_ingest::{ClientArgs, Via};
use crate::fieldset::{self, Disk, FieldSet, SetStatus};
use crate::ingest;
use crate::profiles::CameraProfile;
use crate::util::human;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

/// The sudo password on its way from the prompt to `sudo -S`; never printed.
#[derive(Clone)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// A step that needs sudo, waiting for the password prompt.
#[derive(Debug, Clone)]
pub enum SudoOp {
    PrepareMaster { disk: Disk, label: String },
    PrepareSlave { disk: Disk, label: String },
    Clean { label: String },
}

impl SudoOp {
    fn what(&self) -> String {
        match self {
            SudoOp::PrepareMaster { label, .. } => format!("Prepare master of {label}"),
            SudoOp::PrepareSlave { label, .. } => format!("Prepare slave of {label}"),
            SudoOp::Clean { label } => format!("Clean {label}"),
        }
    }
    fn into_action(self, password: Option<Secret>) -> Action {
        match self {
            SudoOp::PrepareMaster { disk, label } => Action::PrepareMaster { disk, label, password },
            SudoOp::PrepareSlave { disk, label } => Action::PrepareSlave { disk, label, password },
            SudoOp::Clean { label } => Action::Clean { label, password },
        }
    }
}

/// A running step (pull, sync, Prepare, Clean): progress and the lines it printed.
#[derive(Debug, Clone, Default)]
pub struct Job {
    pub title: String,
    pub stage: String,
    pub done: u64,
    pub total: u64,
    pub lines: Vec<String>,
    pub cancelling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Ingest,
    Stub(Stub),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stub {
    Logging,
    Library,
    ServerDashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Detected,
    Files,
    Project,
    Details,
    Targets,
}

const PANES: [Pane; 5] = [Pane::Detected, Pane::Files, Pane::Project, Pane::Details, Pane::Targets];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Camera,
    Format,
    Gamma,
    Gamut,
    ShootDate,
    Notes,
    Model,
}

pub const FIELDS: [Field; 7] =
    [Field::Camera, Field::Format, Field::Gamma, Field::Gamut, Field::ShootDate, Field::Notes, Field::Model];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    ManualPath,
    NewProject,
    ShootDate,
    Notes,
    SetLabel,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepStep {
    Master,
    /// The master of set `label` exists; pick the slave.
    Slave { label: String, set_size: u64, master_mount: PathBuf },
}

#[derive(Debug, Clone)]
pub struct PrepareState {
    pub step: PrepStep,
    /// Cursor in the drive list.
    pub sel: usize,
    pub label: String,
    /// Operator pressed Enter on a drive: show the confirmation (size, label, warning).
    pub confirm: bool,
    /// Refusal or note for the drive under the cursor.
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Popup {
    Prepare(PrepareState),
    Clean { label: String, mags: usize },
    Picker { field: Field, items: Vec<String>, sel: usize },
    Input { kind: InputKind, text: String },
    Message(Vec<String>),
    /// Sudo password for a Prepare / Clean step (shown as dots).
    Password { op: SudoOp, text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSel {
    Existing { id: String, name: String },
    New { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRow {
    Staging,
    Server,
    Set(usize),
    Drive(usize),
}

#[derive(Debug)]
pub enum Action {
    Quit,
    Refresh,
    Probe(PathBuf),
    Pull(Box<ClientArgs>),
    Sync,
    PrepareMaster { disk: Disk, label: String, password: Option<Secret> },
    PrepareSlave { disk: Disk, label: String, password: Option<Secret> },
    Clean { label: String, password: Option<Secret> },
    /// Ctrl-C while a step runs: stop it (nothing committed / written).
    Cancel,
    /// The operator cancelled a sudo prompt (logged; reported to the server log).
    SudoCancelled(String),
}

#[derive(Debug, Clone, Default)]
pub struct DetailsState {
    pub camera: Option<String>,
    pub format: String,
    pub gamma: String,
    pub gamut: String,
    pub shoot_date: String,
    pub shoot_src: String,
    /// The operator changed the shoot day (passed as --shoot-date; otherwise ingest guesses
    /// it again with ffprobe, as on the command line).
    pub shoot_edited: bool,
    pub notes: String,
    /// Model hint read from the card (with its source).
    pub model_hint: Option<String>,
    /// The operator's own model name (passed as --model).
    pub model: Option<String>,
}

pub struct App {
    pub snap: Snapshot,
    pub scan_roots: Vec<PathBuf>,
    pub screen: Screen,
    pub popup: Option<Popup>,
    pub refreshing: bool,
    pub dash_sel: usize,
    pub pane: Pane,
    pub card_sel: usize,
    pub source: Option<PathBuf>,
    pub preview: Option<Result<Preview, String>>,
    pub probing: bool,
    pub files_scroll: usize,
    pub project_sel: usize,
    pub project: Option<ProjectSel>,
    pub details: DetailsState,
    pub detail_sel: usize,
    pub target_sel: usize,
    pub use_staging: bool,
    pub use_server: bool,
    pub use_set: Option<String>,
    pub log: Vec<String>,
    /// The Prepare popup while its set label is being edited.
    pub prepare_stash: Option<Popup>,
    /// The step running in the background, if any.
    pub job: Option<Job>,
    /// d: show paths, UUIDs and device names.
    pub show_tech: bool,
    /// Prepare / Clean need the sudo password (not when running as root).
    pub needs_sudo: bool,
}

pub const MENU: [(char, &str); 4] = [('i', "Ingest"), ('g', "Logging"), ('l', "Library"), ('s', "Server dashboard")];

impl App {
    pub fn new(snap: Snapshot, scan_roots: Vec<PathBuf>) -> App {
        let use_server = snap.server != ServerState::NotConfigured;
        App {
            snap,
            scan_roots,
            screen: Screen::Dashboard,
            popup: None,
            refreshing: false,
            dash_sel: 0,
            pane: Pane::Detected,
            card_sel: 0,
            source: None,
            preview: None,
            probing: false,
            files_scroll: 0,
            project_sel: 0,
            project: None,
            details: DetailsState::default(),
            detail_sel: 0,
            target_sel: 0,
            use_staging: true,
            use_server,
            use_set: None,
            log: Vec::new(),
            prepare_stash: None,
            job: None,
            show_tech: false,
            needs_sudo: true,
        }
    }

    // ---- running steps -----------------------------------------------------------------

    pub fn start_job(&mut self, title: &str) {
        self.job = Some(Job { title: title.to_string(), stage: "starting".into(), ..Default::default() });
    }

    pub fn job_msg(&mut self, m: crate::out::Msg) {
        match m {
            crate::out::Msg::Line { text, .. } => {
                if text.trim().is_empty() {
                    return;
                }
                if let Some(j) = &mut self.job {
                    j.lines.push(text.clone());
                    if j.lines.len() > 200 {
                        j.lines.remove(0);
                    }
                }
                self.log(text);
            }
            crate::out::Msg::Progress { label, done, total } => {
                if let Some(j) = &mut self.job {
                    j.stage = label;
                    j.done = done;
                    j.total = total;
                }
            }
        }
    }

    /// The step ended: one line in the log, a message on failure. Stays on the TUI.
    pub fn job_done(&mut self, r: std::result::Result<String, String>) {
        let title = self.job.take().map(|j| j.title).unwrap_or_default();
        match r {
            Ok(summary) => self.log(format!("done            {title}: {summary}")),
            Err(e) => {
                self.log(format!("failed          {title}: {}", e.lines().next().unwrap_or("")));
                self.popup = Some(Popup::Message(vec![format!("{title} failed:"), e]));
            }
        }
    }

    // ---- data from the runtime ---------------------------------------------------------

    pub fn set_snapshot(&mut self, snap: Snapshot) {
        self.snap = snap;
        self.refreshing = false;
        if self.snap.server == ServerState::NotConfigured {
            self.use_server = false;
        }
        // A chosen set that disappeared or became unusable is unchecked.
        if let Some(l) = &self.use_set {
            if !self.snap.sets.iter().any(|s| &s.label == l && s.status == SetStatus::Valid) {
                self.use_set = None;
            }
        }
        self.card_sel = self.card_sel.min(self.snap.cards.len());
        self.target_sel = self.target_sel.min(self.targets().len().saturating_sub(1));
        self.dash_sel = self.dash_sel.min(MENU.len() - 1);
        if let Some(Popup::Prepare(p)) = &mut self.popup {
            p.sel = p.sel.min(prepare_rows(&self.snap, &p.step).len().saturating_sub(1));
        }
        self.update_prepare_note();
    }

    pub fn set_preview(&mut self, path: PathBuf, r: Result<Preview, String>) {
        if self.source.as_ref() != Some(&path) {
            return; // an older probe
        }
        self.probing = false;
        self.files_scroll = 0;
        match &r {
            Ok(p) => {
                self.details.camera = None;
                self.details.model_hint = p.model_hint.clone();
                self.details.model = None;
                if let Some(h) = &p.model_hint {
                    self.log(format!("model hint      {h}"));
                }
                match &p.camera {
                    Ok(id) => {
                        self.set_camera(id);
                        self.log(format!("guessed         camera {id}"));
                    }
                    Err(why) => self.log(format!("no camera guess {why}; pick one in Details")),
                }
                if !self.details.shoot_edited {
                    match &p.shoot {
                        Some((d, src)) => {
                            self.details.shoot_date = d.clone();
                            self.details.shoot_src = src.clone();
                        }
                        None => {
                            self.details.shoot_date.clear();
                            self.details.shoot_src = "not guessed".into();
                        }
                    }
                }
                self.log(format!("probe done      {} files · {}", p.files.len(), human(p.total)));
            }
            Err(e) => self.log(format!("probe failed    {e}")),
        }
        self.preview = Some(r);
    }

    pub fn log(&mut self, line: String) {
        self.log.push(line);
        if self.log.len() > 50 {
            self.log.remove(0);
        }
    }

    pub fn camera(&self) -> Option<&CameraProfile> {
        let id = self.details.camera.as_ref()?;
        self.snap.cameras.iter().find(|c| &c.id == id)
    }

    fn set_camera(&mut self, id: &str) {
        let Some(c) = self.snap.cameras.iter().find(|c| c.id == id).cloned() else { return };
        self.details.camera = Some(c.id.clone());
        self.details.format = c.default_format.clone();
        self.details.gamma = c.default_gamma.clone();
        self.details.gamut = c.default_gamut.clone();
    }

    // ---- derived -----------------------------------------------------------------------

    pub fn targets(&self) -> Vec<TargetRow> {
        let mut v = vec![TargetRow::Staging, TargetRow::Server];
        v.extend((0..self.snap.sets.len()).map(TargetRow::Set));
        v.extend((0..self.snap.drives.len()).map(TargetRow::Drive));
        v
    }

    pub fn projects(&self) -> Vec<ProjectSel> {
        let mut v: Vec<ProjectSel> = Vec::new();
        if let Some(ProjectSel::New { name }) = &self.project {
            v.push(ProjectSel::New { name: name.clone() });
        }
        v.extend(
            self.snap
                .projects
                .iter()
                .map(|(id, name)| ProjectSel::Existing { id: id.clone(), name: name.clone() }),
        );
        v
    }

    /// Route the target checkboxes select (same options as `ingest --via / --mirror-set`).
    pub fn route(&self) -> Result<(Via, bool), String> {
        match (self.use_staging, self.use_set.is_some(), self.use_server) {
            (true, false, _) => Ok((Via::Staging, false)),
            (true, true, _) => Ok((Via::Staging, true)),
            (false, true, _) => Ok((Via::Set, false)),
            (false, false, true) => Ok((Via::Direct, false)),
            (false, false, false) => Err("no target: pick local staging, a field set or the server".into()),
        }
    }

    /// Warning for the Targets pane (PLAN.md: no backup and server sync not confirmed).
    pub fn target_warning(&self) -> Option<String> {
        if self.use_set.is_some() {
            return None;
        }
        if self.use_server && self.snap.ready() {
            Some("No field backup selected: keep the card until the server confirms.".into())
        } else {
            Some("No backup and server sync not confirmed: keep the card until `magvault sync` succeeds.".into())
        }
    }

    /// What is still missing before Pull.
    pub fn missing(&self) -> Vec<String> {
        let mut m = Vec::new();
        match (&self.source, &self.preview) {
            (None, _) => m.push("card".to_string()),
            (Some(_), None) => m.push("card probe (running)".to_string()),
            (Some(_), Some(Err(e))) => m.push(format!("a readable card ({e})")),
            _ => {}
        }
        if self.camera().is_none() {
            m.push("camera".into());
        }
        if self.project.is_none() {
            m.push("project".into());
        }
        if self.details.shoot_date.is_empty() {
            m.push("shoot day".into());
        }
        match self.route() {
            Err(e) => m.push(e),
            Ok((Via::Direct, _)) if !self.snap.ready() => m.push("server ready (direct needs it)".into()),
            _ => {}
        }
        m
    }

    pub fn pull_args(&self) -> Result<ClientArgs, Vec<String>> {
        let missing = self.missing();
        if !missing.is_empty() {
            return Err(missing);
        }
        let (via, mirror_set) = self.route().map_err(|e| vec![e])?;
        let (project, project_id) = match self.project.clone().expect("checked") {
            ProjectSel::Existing { id, .. } => (None, Some(id)),
            ProjectSel::New { name } => (Some(name), None),
        };
        let d = &self.details;
        Ok(ClientArgs {
            base: ingest::Args {
                source: self.source.clone().expect("checked"),
                project,
                project_id,
                camera: d.camera.clone(),
                format: Some(d.format.clone()),
                gamma: Some(d.gamma.clone()),
                gamut: Some(d.gamut.clone()),
                shoot_date: d.shoot_edited.then(|| d.shoot_date.clone()),
                notes: (!d.notes.is_empty()).then(|| d.notes.clone()),
                model: d.model.clone(),
                yes: true,
            },
            via,
            mirror_set,
            set_label: self.use_set.clone(),
            no_sync: !self.use_server,
            scan_roots: self.scan_roots.clone(),
        })
    }

    // ---- keys --------------------------------------------------------------------------

    pub fn on_key(&mut self, k: KeyEvent) -> Option<Action> {
        let ctrl_c = k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c');
        if let Some(j) = &mut self.job {
            // A step is running: Ctrl-C stops it, d toggles details, nothing else.
            if ctrl_c && !j.cancelling {
                j.cancelling = true;
                return Some(Action::Cancel);
            }
            if k.code == KeyCode::Char('d') {
                self.show_tech = !self.show_tech;
            }
            return None;
        }
        if let Some(Popup::Password { .. }) = &self.popup {
            return self.password_key(k, ctrl_c);
        }
        if ctrl_c {
            return Some(Action::Quit);
        }
        if self.popup.is_some() {
            return self.popup_key(k);
        }
        if matches!(self.screen, Screen::Dashboard | Screen::Ingest) {
            match k.code {
                KeyCode::Char('d') => {
                    self.show_tech = !self.show_tech;
                    return None;
                }
                KeyCode::Char('y') => return self.sync_key(),
                _ => {}
            }
        }
        match self.screen {
            Screen::Dashboard => self.dashboard_key(k),
            Screen::Stub(_) => {
                if k.code == KeyCode::Esc {
                    self.screen = Screen::Dashboard;
                }
                None
            }
            Screen::Ingest => self.ingest_key(k),
        }
    }

    fn password_key(&mut self, k: KeyEvent, ctrl_c: bool) -> Option<Action> {
        let Some(Popup::Password { op, mut text }) = self.popup.take() else { return None };
        match k.code {
            _ if ctrl_c => {}
            KeyCode::Esc => {}
            KeyCode::Enter => {
                return Some(op.into_action(Some(Secret(text))));
            }
            KeyCode::Backspace => {
                text.pop();
                self.popup = Some(Popup::Password { op, text });
                return None;
            }
            KeyCode::Char(c) if !c.is_control() => {
                text.push(c);
                self.popup = Some(Popup::Password { op, text });
                return None;
            }
            _ => {
                self.popup = Some(Popup::Password { op, text });
                return None;
            }
        }
        // Esc / Ctrl-C: cancelled before anything was written.
        let what = op.what();
        self.log(format!("sudo cancelled  {what}; nothing written"));
        Some(Action::SudoCancelled(what))
    }

    /// A Prepare / Clean step confirmed: ask for the sudo password first (not as root).
    fn sudo_step(&mut self, op: SudoOp) -> Option<Action> {
        if self.needs_sudo {
            self.popup = Some(Popup::Password { op, text: String::new() });
            None
        } else {
            Some(op.into_action(None))
        }
    }

    fn sync_key(&mut self) -> Option<Action> {
        let pending = self.snap.staging_pending.len() + self.snap.sets.iter().map(|s| s.pending().len()).sum::<usize>();
        if self.snap.server == ServerState::NotConfigured {
            self.popup = Some(Popup::Message(vec!["No server is configured; mags stay pending here.".into()]));
            None
        } else if pending == 0 {
            self.popup = Some(Popup::Message(vec!["Nothing to sync: no mag is waiting for the server.".into()]));
            None
        } else {
            Some(Action::Sync)
        }
    }

    /// Mags waiting for the server (staging + field sets).
    pub fn pending_total(&self) -> usize {
        self.snap.staging_pending.len() + self.snap.sets.iter().map(|s| s.pending().len()).sum::<usize>()
    }

    fn open_module(&mut self, c: char) -> Option<Action> {
        match c {
            'i' => {
                self.screen = Screen::Ingest;
                self.pane = Pane::Detected;
                Some(Action::Refresh)
            }
            'g' => {
                self.screen = Screen::Stub(Stub::Logging);
                None
            }
            'l' => {
                self.screen = Screen::Stub(Stub::Library);
                None
            }
            's' => {
                self.screen = Screen::Stub(Stub::ServerDashboard);
                None
            }
            _ => None,
        }
    }

    fn dashboard_key(&mut self, k: KeyEvent) -> Option<Action> {
        match k.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') => {
                self.refreshing = true;
                Some(Action::Refresh)
            }
            KeyCode::Up => {
                self.dash_sel = self.dash_sel.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                self.dash_sel = (self.dash_sel + 1).min(MENU.len() - 1);
                None
            }
            KeyCode::Enter => self.open_module(MENU[self.dash_sel].0),
            KeyCode::Char(c) => self.open_module(c),
            _ => None,
        }
    }

    fn ingest_key(&mut self, k: KeyEvent) -> Option<Action> {
        match k.code {
            KeyCode::Esc => {
                self.screen = Screen::Dashboard;
                None
            }
            KeyCode::Tab => {
                self.pane = PANES[(PANES.iter().position(|p| *p == self.pane).unwrap() + 1) % PANES.len()];
                None
            }
            KeyCode::BackTab => {
                self.pane = PANES[(PANES.iter().position(|p| *p == self.pane).unwrap() + PANES.len() - 1) % PANES.len()];
                None
            }
            KeyCode::Up => {
                self.move_cursor(-1);
                None
            }
            KeyCode::Down => {
                self.move_cursor(1);
                None
            }
            KeyCode::Enter => self.enter(),
            KeyCode::Char(' ') if self.pane == Pane::Targets => self.enter(),
            KeyCode::Char('r') => {
                self.refreshing = true;
                self.log("rescan".into());
                Some(Action::Refresh)
            }
            KeyCode::Char('n') => {
                self.popup = Some(Popup::Input { kind: InputKind::NewProject, text: String::new() });
                None
            }
            KeyCode::Char('p') => match self.pull_args() {
                Ok(a) => Some(Action::Pull(Box::new(a))),
                Err(missing) => {
                    self.popup = Some(Popup::Message(vec![
                        "Pull needs:".to_string(),
                        format!("  {}", missing.join(", ")),
                    ]));
                    None
                }
            },
            KeyCode::Char('o') => {
                self.open_prepare();
                None
            }
            KeyCode::Char('c') => {
                self.open_clean();
                None
            }
            _ => None,
        }
    }

    fn move_cursor(&mut self, d: i32) {
        let step = |v: &mut usize, len: usize| {
            if len == 0 {
                *v = 0;
            } else if d < 0 {
                *v = v.saturating_sub(1);
            } else {
                *v = (*v + 1).min(len - 1);
            }
        };
        match self.pane {
            Pane::Detected => step(&mut self.card_sel, self.snap.cards.len() + 1),
            Pane::Files => {
                let n = match &self.preview {
                    Some(Ok(p)) => p.files.len(),
                    _ => 0,
                };
                step(&mut self.files_scroll, n)
            }
            Pane::Project => {
                let n = self.projects().len();
                step(&mut self.project_sel, n)
            }
            Pane::Details => step(&mut self.detail_sel, FIELDS.len()),
            Pane::Targets => {
                let n = self.targets().len();
                step(&mut self.target_sel, n)
            }
        }
    }

    fn enter(&mut self) -> Option<Action> {
        match self.pane {
            Pane::Detected => {
                if self.card_sel >= self.snap.cards.len() {
                    self.popup = Some(Popup::Input { kind: InputKind::ManualPath, text: String::new() });
                    return None;
                }
                let c = self.snap.cards[self.card_sel].clone();
                if let Some(p) = &c.problem {
                    self.popup = Some(Popup::Message(vec![format!("That card cannot be ingested: {p}")]));
                    return None;
                }
                self.choose_source(c.volume.mount)
            }
            Pane::Files => None,
            Pane::Project => {
                if let Some(p) = self.projects().get(self.project_sel).cloned() {
                    self.project = Some(p);
                }
                None
            }
            Pane::Details => {
                let field = FIELDS[self.detail_sel];
                let items: Vec<String> = match field {
                    Field::Camera => self.snap.cameras.iter().map(|c| c.id.clone()).collect(),
                    Field::Format => self.camera().map(|c| c.file_formats.clone()).unwrap_or_default(),
                    Field::Gamma => self.camera().map(|c| c.gamma.clone()).unwrap_or_default(),
                    Field::Gamut => self.camera().map(|c| c.gamut.clone()).unwrap_or_default(),
                    Field::ShootDate => {
                        self.popup = Some(Popup::Input { kind: InputKind::ShootDate, text: self.details.shoot_date.clone() });
                        return None;
                    }
                    Field::Notes => {
                        self.popup = Some(Popup::Input { kind: InputKind::Notes, text: self.details.notes.clone() });
                        return None;
                    }
                    Field::Model => {
                        let text = self.details.model.clone().unwrap_or_default();
                        self.popup = Some(Popup::Input { kind: InputKind::Model, text });
                        return None;
                    }
                };
                if items.is_empty() {
                    self.popup = Some(Popup::Message(vec![if field == Field::Camera {
                        "No camera profiles. Missing profile? Admin only.".into()
                    } else {
                        "Pick the camera first.".into()
                    }]));
                    return None;
                }
                let current = match field {
                    Field::Camera => self.details.camera.clone().unwrap_or_default(),
                    Field::Format => self.details.format.clone(),
                    Field::Gamma => self.details.gamma.clone(),
                    _ => self.details.gamut.clone(),
                };
                let sel = items.iter().position(|i| *i == current).unwrap_or(0);
                self.popup = Some(Popup::Picker { field, items, sel });
                None
            }
            Pane::Targets => {
                match self.targets().get(self.target_sel).cloned() {
                    Some(TargetRow::Staging) => self.use_staging = !self.use_staging,
                    Some(TargetRow::Server) => {
                        if self.snap.server == ServerState::NotConfigured {
                            self.popup = Some(Popup::Message(vec!["No server is configured in path.conf.".into()]));
                        } else {
                            self.use_server = !self.use_server;
                        }
                    }
                    Some(TargetRow::Set(i)) => {
                        let s = &self.snap.sets[i];
                        if s.status != SetStatus::Valid {
                            self.popup = Some(Popup::Message(vec![format!("{} cannot be a target: {}", s.label, set_state(s))]));
                        } else if self.use_set.as_deref() == Some(&s.label) {
                            self.use_set = None;
                        } else {
                            self.use_set = Some(s.label.clone());
                        }
                    }
                    Some(TargetRow::Drive(_)) => {
                        self.popup = Some(Popup::Message(vec![
                            "Only a valid field set (master + slave) can be a target.".into(),
                            "o opens Prepare for a drive that is not valid.".into(),
                        ]))
                    }
                    None => {}
                }
                None
            }
        }
    }

    fn choose_source(&mut self, path: PathBuf) -> Option<Action> {
        self.source = Some(path.clone());
        self.preview = None;
        self.probing = true;
        self.log(format!("card selected   {}", path.display()));
        Some(Action::Probe(path))
    }

    fn open_prepare(&mut self) {
        // On a set row: a set waiting for its slave continues at step 2; a set with pending
        // mags or a complete set is not prepared again.
        if self.pane == Pane::Targets {
            if let Some(TargetRow::Set(i)) = self.targets().get(self.target_sel).cloned() {
                let s = self.snap.sets[i].clone();
                if let Err(e) = fieldset::prepare_allowed_set(&s) {
                    self.popup = Some(Popup::Message(vec![e.0]));
                    return;
                }
                match (&s.master, &s.slave) {
                    (Some(m), None) => {
                        self.popup = Some(Popup::Prepare(PrepareState {
                            step: PrepStep::Slave { label: s.label.clone(), set_size: s.set_size, master_mount: m.clone() },
                            sel: 0,
                            label: s.label.clone(),
                            confirm: false,
                            note: None,
                        }));
                    }
                    _ => {
                        self.popup = Some(Popup::Message(vec![format!(
                            "{} is already a set. Prepare is only for a new set (Clean empties a synced set).",
                            s.label
                        )]))
                    }
                }
                return;
            }
        }
        let mut st = PrepareState { step: PrepStep::Master, sel: 0, label: "FIELD-A".into(), confirm: false, note: None };
        // Preselect the drive under the cursor in Targets.
        if let Some(TargetRow::Drive(i)) = self.targets().get(self.target_sel).cloned() {
            let rows = prepare_rows(&self.snap, &st.step);
            if let Some(pos) = rows.iter().position(|r| *r == i) {
                st.sel = pos;
            }
        }
        self.popup = Some(Popup::Prepare(st));
        self.update_prepare_note();
    }

    fn open_clean(&mut self) {
        let set = match self.targets().get(self.target_sel).cloned() {
            Some(TargetRow::Set(i)) if self.pane == Pane::Targets => Some(self.snap.sets[i].clone()),
            _ if self.snap.sets.len() == 1 => Some(self.snap.sets[0].clone()),
            _ => None,
        };
        let Some(set) = set else {
            self.popup = Some(Popup::Message(vec!["Select a field set row in Targets (Tab to Targets, ↑↓), then c.".into()]));
            return;
        };
        match fieldset::clean_allowed(&set) {
            Ok(()) => self.popup = Some(Popup::Clean { label: set.label.clone(), mags: set.mags.len() }),
            Err(e) => self.popup = Some(Popup::Message(vec![format!("Clean is not offered: {}", e.0)])),
        }
    }

    /// Drives offered in the Prepare popup (indices into `snap.drives`).
    pub fn prepare_rows(&self) -> Vec<usize> {
        match &self.popup {
            Some(Popup::Prepare(p)) => prepare_rows(&self.snap, &p.step),
            _ => Vec::new(),
        }
    }

    fn update_prepare_note(&mut self) {
        let rows = self.prepare_rows();
        let snap = self.snap.clone();
        let Some(Popup::Prepare(p)) = &mut self.popup else { return };
        p.note = rows.get(p.sel).and_then(|&i| match &snap.drives[i] {
            DriveRow::Folder(_) => Some("a folder, not a drive: Prepare needs a real USB disk".into()),
            DriveRow::Disk(c) => match (&c.disk, &p.step) {
                (Err(why), _) => Some(why.clone()),
                (Ok(_), PrepStep::Master) => None,
                (Ok(d), PrepStep::Slave { set_size, .. }) => match fieldset::slave_plan(*set_size, d.size) {
                    Err(e) => Some(format!("Refused: {}", e.0)),
                    Ok((_, 0)) => None,
                    Ok(_) => Some(format!("Will use {} to match master. Rest unused.", human(*set_size))),
                },
            },
        });
    }

    fn popup_key(&mut self, k: KeyEvent) -> Option<Action> {
        let popup = self.popup.take().expect("popup");
        match popup {
            Popup::Message(_) => None, // any key closes
            Popup::Password { .. } => None,   // handled in password_key
            Popup::Input { kind, mut text } => {
                match k.code {
                    KeyCode::Esc => {}
                    KeyCode::Enter => return self.input_done(kind, text.trim().to_string()),
                    KeyCode::Backspace => {
                        text.pop();
                        self.popup = Some(Popup::Input { kind, text });
                    }
                    KeyCode::Char(c) if !c.is_control() => {
                        text.push(c);
                        self.popup = Some(Popup::Input { kind, text });
                    }
                    _ => self.popup = Some(Popup::Input { kind, text }),
                }
                None
            }
            Popup::Picker { field, items, mut sel } => {
                match k.code {
                    KeyCode::Esc => {}
                    KeyCode::Up => {
                        sel = sel.saturating_sub(1);
                        self.popup = Some(Popup::Picker { field, items, sel });
                    }
                    KeyCode::Down => {
                        sel = (sel + 1).min(items.len().saturating_sub(1));
                        self.popup = Some(Popup::Picker { field, items, sel });
                    }
                    KeyCode::Enter => {
                        let v = items[sel].clone();
                        match field {
                            Field::Camera => self.set_camera(&v),
                            Field::Format => self.details.format = v,
                            Field::Gamma => self.details.gamma = v,
                            Field::Gamut => self.details.gamut = v,
                            _ => {}
                        }
                    }
                    _ => self.popup = Some(Popup::Picker { field, items, sel }),
                }
                None
            }
            Popup::Clean { label, mags } => match k.code {
                KeyCode::Esc => {
                    self.log(format!("clean cancelled {label}; nothing deleted"));
                    None
                }
                KeyCode::Enter => self.sudo_step(SudoOp::Clean { label }),
                _ => {
                    self.popup = Some(Popup::Clean { label, mags });
                    None
                }
            },
            Popup::Prepare(mut p) => {
                let rows = prepare_rows(&self.snap, &p.step);
                match k.code {
                    KeyCode::Esc if p.confirm => {
                        p.confirm = false;
                        self.popup = Some(Popup::Prepare(p));
                    }
                    KeyCode::Esc => self.log("prepare cancelled; nothing written".into()),
                    KeyCode::Up if !p.confirm => {
                        p.sel = p.sel.saturating_sub(1);
                        self.popup = Some(Popup::Prepare(p));
                        self.update_prepare_note();
                    }
                    KeyCode::Down if !p.confirm => {
                        p.sel = (p.sel + 1).min(rows.len().saturating_sub(1));
                        self.popup = Some(Popup::Prepare(p));
                        self.update_prepare_note();
                    }
                    KeyCode::Char('d') => {
                        self.show_tech = !self.show_tech;
                        self.popup = Some(Popup::Prepare(p));
                    }
                    KeyCode::Char('e') if p.confirm && p.step == PrepStep::Master => {
                        let text = p.label.clone();
                        self.popup = Some(Popup::Prepare(p));
                        // Label editing returns to this popup (see input_done).
                        self.prepare_stash = self.popup.take();
                        self.popup = Some(Popup::Input { kind: InputKind::SetLabel, text });
                    }
                    KeyCode::Enter => {
                        let disk = rows.get(p.sel).and_then(|&i| match &self.snap.drives[i] {
                            DriveRow::Disk(c) => c.disk.clone().ok(),
                            DriveRow::Folder(_) => None,
                        });
                        self.popup = Some(Popup::Prepare(p.clone()));
                        self.update_prepare_note();
                        let note = match &self.popup {
                            Some(Popup::Prepare(q)) => q.note.clone(),
                            _ => None,
                        };
                        let Some(disk) = disk else {
                            return None; // nothing to pick, or a refused drive (note shows why)
                        };
                        let refused = note.as_deref().is_some_and(|n| n.starts_with("Refused"));
                        if refused {
                            return None;
                        }
                        if !p.confirm {
                            if let Some(Popup::Prepare(q)) = &mut self.popup {
                                q.confirm = true;
                            }
                            return None;
                        }
                        // Confirmed: the sudo password next (in the TUI), then the step runs.
                        let op = match &p.step {
                            PrepStep::Master => SudoOp::PrepareMaster { disk, label: p.label.clone() },
                            PrepStep::Slave { label, .. } => SudoOp::PrepareSlave { disk, label: label.clone() },
                        };
                        return self.sudo_step(op);
                    }
                    _ => self.popup = Some(Popup::Prepare(p)),
                }
                None
            }
        }
    }

    fn input_done(&mut self, kind: InputKind, text: String) -> Option<Action> {
        match kind {
            InputKind::ManualPath => {
                if text.is_empty() {
                    return None;
                }
                let p = PathBuf::from(&text);
                if !p.is_dir() {
                    self.popup = Some(Popup::Message(vec![format!("{text} is not a directory")]));
                    return None;
                }
                return self.choose_source(p);
            }
            InputKind::NewProject => {
                if text.is_empty() {
                    return None;
                }
                // The same rule as `--project <name>`: an open project with this name is reused.
                let existing: Vec<&(String, String)> = self.snap.projects.iter().filter(|(_, n)| *n == text).collect();
                self.project = Some(match existing.as_slice() {
                    [(id, name)] => ProjectSel::Existing { id: id.clone(), name: name.clone() },
                    _ => ProjectSel::New { name: text },
                });
                self.project_sel = 0;
            }
            InputKind::ShootDate => {
                if crate::time::parse_date(&text).is_none() {
                    self.popup = Some(Popup::Message(vec![format!("{text:?} is not a date (YYYY-MM-DD)")]));
                    return None;
                }
                self.details.shoot_date = text;
                self.details.shoot_src = "given".into();
                self.details.shoot_edited = true;
            }
            InputKind::Model => {
                if text.contains('|') || text.chars().any(char::is_control) {
                    self.popup = Some(Popup::Message(vec!["The model name cannot contain '|'.".into()]));
                    return None;
                }
                self.details.model = (!text.is_empty()).then_some(text);
            }
            InputKind::Notes => {
                if text.contains('|') || text.chars().any(char::is_control) {
                    self.popup = Some(Popup::Message(vec!["Notes cannot contain '|'.".into()]));
                    return None;
                }
                self.details.notes = text;
            }
            InputKind::SetLabel => {
                let label = text.to_uppercase();
                let mut back = self.prepare_stash.take();
                if !fieldset::valid_label(&label) {
                    self.popup = Some(Popup::Message(vec![format!(
                        "set label {label:?}: use A-Z, 0-9 and '-', at most 12 characters"
                    )]));
                    return None;
                }
                if let Some(Popup::Prepare(p)) = &mut back {
                    p.label = label;
                }
                self.popup = back;
            }
        }
        None
    }

    /// After a successful master prepare the popup moves on to the slave step.
    pub fn master_prepared(&mut self, label: &str) {
        let set = self.snap.sets.iter().find(|s| s.label == label && s.slave.is_none()).cloned();
        match set.and_then(|s| s.master.clone().map(|m| (s, m))) {
            Some((s, m)) => {
                self.popup = Some(Popup::Prepare(PrepareState {
                    step: PrepStep::Slave { label: s.label.clone(), set_size: s.set_size, master_mount: m },
                    sel: 0,
                    label: s.label.clone(),
                    confirm: false,
                    note: None,
                }));
                self.update_prepare_note();
            }
            None => self.popup = Some(Popup::Message(vec![format!("Master of {label} prepared; mount it to add the slave.")])),
        }
    }
}

/// Drives listed in the Prepare popup: every drive that is not part of a valid set (refused
/// ones too, so the operator sees why); in the slave step the master's own disk is left out.
pub fn prepare_rows(snap: &Snapshot, step: &PrepStep) -> Vec<usize> {
    (0..snap.drives.len())
        .filter(|&i| match (&snap.drives[i], step) {
            (DriveRow::Disk(c), PrepStep::Slave { master_mount, .. }) => {
                !matches!(&c.disk, Ok(d) if d.mounts.iter().any(|m| m == master_mount))
            }
            _ => true,
        })
        .collect()
}

pub fn set_state(s: &FieldSet) -> String {
    match &s.status {
        SetStatus::Valid => {
            let p = s.pending().len();
            if s.mags.is_empty() {
                "valid · empty".into()
            } else if p > 0 {
                format!("valid · {p} pending sync")
            } else {
                format!("valid · {} synced", s.mags.len())
            }
        }
        SetStatus::Incomplete(r) | SetStatus::Inconsistent(r) => format!("not usable: {r}"),
    }
}
