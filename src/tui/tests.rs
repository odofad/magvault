//! Key-script tests on ratatui's TestBackend: keys go into `App::on_key`, the screen is
//! drawn into a test buffer and searched. No terminal, no drives, no sudo.
//! `MAGVAULT_TUI_DUMP=<dir>` writes every checked screen as text for review.

use super::app::{Action, App, Pane, Popup, Screen};
use super::data::{DriveRow, Preview, ServerState, Snapshot};
use super::view;
use crate::client_ingest::Via;
use crate::discover::{Card, Volume};
use crate::fieldset::{Candidate, Disk, FieldSet, MagEntry, MagState, SetStatus};
use crate::profiles::parse_camera;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use std::path::{Path, PathBuf};

const F1: &str = "20261126_aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
const GB: u64 = 1_000_000_000;

fn key(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

fn ch(c: char) -> KeyEvent {
    key(KeyCode::Char(c))
}

fn press(app: &mut App, keys: &[KeyEvent]) -> Vec<Action> {
    keys.iter().filter_map(|k| app.on_key(*k)).collect()
}

fn screen(app: &App, w: u16, h: u16, name: &str) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| view::draw(f, app)).unwrap();
    let buf = t.backend().buffer();
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            out.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
        }
        out.push('\n');
    }
    if let Ok(dir) = std::env::var("MAGVAULT_TUI_DUMP") {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(Path::new(&dir).join(format!("{name}-{w}x{h}.txt")), &out);
    }
    out
}

fn camera() -> crate::profiles::CameraProfile {
    parse_camera(
        Path::new("sony-a7siii.cam"),
        "name: Sony A7S III\nid: sony-a7siii\nvendor: Sony\ncard_layout: DCIM, PRIVATE\nfile_formats: XAVC S, ProRes RAW\n\
         default_format: XAVC S\ngamma: S-Log3, Rec.709\ndefault_gamma: S-Log3\ngamut: S-Gamut3.Cine\ndefault_gamut: S-Gamut3.Cine\n",
    )
    .unwrap()
}

fn card(name: &str) -> Card {
    Card {
        volume: Volume { mount: PathBuf::from(format!("/media/op/{name}")), device: "/dev/sdd1".into(), fstype: "exfat".into() },
        layout: "Sony XAVC (PRIVATE/M4ROOT)".into(),
        camera: Some(("sony-a7siii".into(), "Sony A7S III".into())),
        model_hint: None,
        files: 22,
        bytes: 106_800_000,
        problem: None,
        fingerprint: Some("f".repeat(64)),
        identity: Some(format!("disk:{name}-UUID:64000000000")),
    }
}

fn disk(path: &str, size: u64, label: &str) -> DriveRow {
    DriveRow::Disk(Candidate {
        path: path.into(),
        disk: Ok(Disk {
            path: path.into(),
            size,
            label: label.into(),
            model: "ST500LM021".into(),
            mounts: vec![PathBuf::from(format!("/media/op/{label}"))],
            backup: None,
        }),
    })
}

fn set(label: &str, state: Option<MagState>) -> FieldSet {
    FieldSet {
        set_id: "11111111-2222-4333-8444-555555555555".into(),
        label: label.into(),
        set_size: 320 * GB,
        master: Some(PathBuf::from(format!("/media/op/{label}_M"))),
        slave: Some(PathBuf::from(format!("/media/op/{label}_S"))),
        mags: state
            .map(|s| vec![MagEntry { folder: F1.into(), state: s, updated: "2026-11-27T00:00:00Z".into() }])
            .unwrap_or_default(),
        status: SetStatus::Valid,
    }
}

fn snapshot() -> Snapshot {
    let mut s = Snapshot::empty(PathBuf::from("/home/op/mvclient/staging"));
    s.server = ServerState::Ready("magvaultclient@courage".into());
    s.cards = vec![card("A7SIII_128")];
    s.dump = vec![crate::staging::DumpStatus::NotDumped];
    s.cameras = vec![camera()];
    s.projects = vec![("cce28b9f-0000-4000-8000-000000000001".into(), "CardTestFix".into())];
    s.drives = vec![
        disk("/dev/sdb", 64 * GB, "NEW_64"),
        disk("/dev/sdc", 128 * GB, "NEW_128"),
        disk("/dev/sde", 32 * GB, "NEW_32"),
        DriveRow::Disk(Candidate {
            path: "/dev/sdf".into(),
            disk: Err("/dev/sdf has an existing ZFS signature; wipe it first (wipefs), then Prepare".into()),
        }),
    ];
    s
}

/// Two 25 fps UHD XAVC clips, 11:30 and 15:43 local time on 30/04/2011.
fn summary() -> crate::cardinfo::Summary {
    use crate::probe::{Clip, MediaInfo, Probe, ProbedFile, Role};
    let local = |h: i64, m: i64| {
        // 2011-04-30 hh:mm in the machine's zone (the test reads it back in local time).
        let utc = crate::time::days_from_civil(2011, 4, 30) * 86_400 + h * 3600 + m * 60;
        let (_, _, _, lh, lm, _) = crate::time::local(utc);
        utc - ((lh as i64 * 60 + lm as i64) - (h * 60 + m)) * 60
    };
    let info = || MediaInfo { fps: Some("25/1".into()), width: Some(3840), height: Some(2160), codec: Some("h264".into()), profile: Some("High".into()), ..Default::default() };
    let f = |rel: &str, t: i64, clip: usize| ProbedFile { rel: rel.into(), abs: PathBuf::new(), size: 25_001_000, mtime: t, role: Role::Media, clip: Some(clip) };
    let p = Probe {
        root: PathBuf::new(),
        files: vec![f("PRIVATE/M4ROOT/CLIP/C0001.MP4", local(11, 30), 0), f("PRIVATE/M4ROOT/CLIP/C0002.MP4", local(15, 43), 1)],
        clips: vec![Clip { file: 0, info: info() }, Clip { file: 1, info: info() }],
        total_bytes: 50_002_000,
    };
    crate::cardinfo::summarize(&p)
}

fn preview(path: &Path) -> Preview {
    let _ = path;
    Preview {
        summary: summary(),
        total: 50_002_000,
        camera: Ok("sony-a7siii".into()),
        shoot: Some(("2011-04-30".into(), "earliest file modification time".into())),
        model_hint: Some("ILCE-7M3 (clip sidecar XML)".into()),
        mixed: Vec::new(),
        ffprobe: true,
    }
}

/// Dashboard → Ingest, the card picked and probed.
fn in_ingest_with_card() -> App {
    let mut app = App::new(snapshot(), Vec::new());
    press(&mut app, &[ch('i')]);
    let acts = press(&mut app, &[key(KeyCode::Enter)]);
    let path = PathBuf::from("/media/op/A7SIII_128");
    assert!(matches!(acts.as_slice(), [Action::Probe(p)] if *p == path), "{acts:?}");
    app.set_preview(path.clone(), Ok(preview(&path)));
    app
}

fn to_targets_row(app: &mut App, row: usize) {
    while app.pane != Pane::Targets {
        press(app, &[key(KeyCode::Tab)]);
    }
    for _ in 0..10 {
        press(app, &[key(KeyCode::Up)]);
    }
    for _ in 0..row {
        press(app, &[key(KeyCode::Down)]);
    }
}

#[test]
fn dashboard_then_ingest_and_back() {
    let mut app = App::new(snapshot(), Vec::new());
    let s = screen(&app, 150, 42, "dashboard");
    for want in ["MAGVAULT  ·  Dashboard", "1 Modules", "[i] Ingest", "[h] Server dashboard", "[s] Settings", "2 Server", "● server ready", "3 Local media", "4 Recent activity", "Next", "1 card(s) detected. Press i to ingest.", "q quit", "F1 help"] {
        assert!(s.contains(want), "dashboard misses {want:?}\n{s}");
    }
    let acts = press(&mut app, &[ch('i')]);
    assert!(matches!(acts.as_slice(), [Action::Refresh]));
    assert_eq!(app.screen, Screen::Ingest);
    let s = screen(&app, 150, 42, "ingest-empty");
    for want in ["1 Detected", "2 Card", "System time", "3 Project", "4 Details", "5 Targets", "[ p  PULL ]", "Sony A7S III", "A7SIII_128", "Next: Pick a card", "←→ pane", "F1 help", "Esc dashboard"] {
        assert!(s.contains(want), "ingest misses {want:?}\n{s}");
    }
    // q does not quit outside the dashboard; Esc returns; q quits there.
    assert!(press(&mut app, &[ch('q')]).is_empty());
    assert_eq!(app.screen, Screen::Ingest);
    press(&mut app, &[key(KeyCode::Esc)]);
    assert_eq!(app.screen, Screen::Dashboard);
    assert!(matches!(press(&mut app, &[ch('q')]).as_slice(), [Action::Quit]));
    // Enter on the menu opens the selected module; stubs return with Esc.
    press(&mut app, &[key(KeyCode::Down), key(KeyCode::Enter)]);
    assert!(screen(&app, 120, 36, "stub-logging").contains("Logging comes in Phase 3"));
    press(&mut app, &[key(KeyCode::Esc)]);
    assert_eq!(app.screen, Screen::Dashboard);
}

#[test]
fn one_card_selected_then_pull_uses_the_cli_arguments() {
    let mut app = in_ingest_with_card();
    let s = screen(&app, 150, 42, "ingest-card");
    assert!(s.contains("· selected"), "{s}");
    for want in ["Shoot date   30/04/2011  11:30 – 15:43", "Clips        2", "Total size   50.0 MB", "Frame rate   25 fps", "Resolution   4K", "GOP          long-GOP", "Dump status  ● not dumped"] {
        assert!(s.contains(want), "card summary misses {want:?}\n{s}");
    }
    assert!(!s.contains("C0001.MP4"), "no file list\n{s}");
    assert!(!s.contains("/media/op/"), "mount paths only with d\n{s}");
    assert!(s.contains("Shoot day  30/04/2011  guessed"), "{s}");
    assert!(s.contains("✗ project"), "{s}");
    // Without a project, p explains what is missing and starts nothing.
    assert!(press(&mut app, &[ch('p')]).is_empty());
    assert!(matches!(&app.popup, Some(Popup::Message(m)) if m.join(" ").contains("project")));
    press(&mut app, &[key(KeyCode::Esc)]);
    // Project pane (→ →): Enter opens the dropdown, Enter picks the open project.
    press(&mut app, &[key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Enter)]);
    assert!(matches!(&app.popup, Some(Popup::Picker { items, .. }) if items[0] == "CardTestFix" && items[1] == "+ new project…"), "{:?}", app.popup);
    press(&mut app, &[key(KeyCode::Enter)]);
    assert!(app.missing().is_empty(), "{:?}", app.missing());
    let s = screen(&app, 120, 36, "ingest-ready");
    assert!(s.contains("Ready: press p to pull.") && s.contains("✓ project  ✓ shoot day  ✓ target"), "{s}");
    let acts = press(&mut app, &[ch('p')]);
    let [Action::Pull(a)] = acts.as_slice() else { panic!("{acts:?}") };
    assert_eq!(a.base.source, PathBuf::from("/media/op/A7SIII_128"));
    assert_eq!(a.base.project_id.as_deref(), Some("cce28b9f-0000-4000-8000-000000000001"));
    assert_eq!(a.base.camera.as_deref(), Some("sony-a7siii"));
    assert_eq!(a.base.format.as_deref(), Some("XAVC S"));
    assert_eq!(a.base.shoot_date, None, "a guessed shoot day is guessed again by ingest");
    assert!(a.base.yes);
    assert_eq!((a.via, a.mirror_set, a.no_sync), (Via::Staging, false, false));
}

#[test]
fn targets_map_to_ingest_routes_and_warn_without_backup() {
    let mut s = snapshot();
    s.sets = vec![set("TESTSET1", None)];
    let mut app = App::new(s, Vec::new());
    press(&mut app, &[ch('i')]);
    assert!(screen(&app, 150, 42, "targets").contains("[!] No field backup selected: keep the card until the server confirms."));
    to_targets_row(&mut app, 2); // the set row
    press(&mut app, &[key(KeyCode::Enter)]);
    assert_eq!(app.use_set.as_deref(), Some("TESTSET1"));
    assert!(app.target_warning().is_none());
    assert_eq!(app.route().unwrap(), (Via::Staging, true), "staging + set = --mirror-set");
    to_targets_row(&mut app, 0);
    press(&mut app, &[key(KeyCode::Enter)]); // staging off
    assert_eq!(app.route().unwrap(), (Via::Set, false), "set only = --via set");
    to_targets_row(&mut app, 2);
    press(&mut app, &[key(KeyCode::Enter)]); // set off
    assert_eq!(app.route().unwrap(), (Via::Direct, false), "server only = --via direct");
    to_targets_row(&mut app, 1);
    press(&mut app, &[key(KeyCode::Enter)]); // server off
    assert!(app.route().is_err());
    assert!(app.missing().iter().any(|m| m.contains("no target")));
}

#[test]
fn prepare_popup_opens_and_esc_returns_to_ingest() {
    let mut app = in_ingest_with_card();
    to_targets_row(&mut app, 2); // first not-valid drive (NEW_64)
    press(&mut app, &[ch('o')]);
    let s = screen(&app, 150, 42, "prepare-master");
    for want in ["Prepare USB drive · step 1 of 2 · master", "Mount the smaller drive first and prepare it as master.", "ST500LM021 · 64.0 GB · NEW_64", "ST500LM021 · 128.0 GB · NEW_128", "this drive has an existing ZFS signature", "Esc  Cancel · nothing is written until sudo is accepted"] {
        assert!(s.contains(want), "prepare popup misses {want:?}\n{s}");
    }
    assert!(!s.contains("/dev/sd"), "device nodes only with d\n{s}");
    press(&mut app, &[ch('d')]);
    let t = screen(&app, 150, 42, "prepare-master-details");
    assert!(t.contains("ST500LM021 · 64.0 GB · NEW_64 · /dev/sdb · /media/op/NEW_64"), "{t}");
    press(&mut app, &[ch('d')]);
    // Esc closes the popup and stays on Ingest; nothing was asked of the runtime.
    assert!(press(&mut app, &[key(KeyCode::Esc)]).is_empty());
    assert!(app.popup.is_none());
    assert_eq!(app.screen, Screen::Ingest);

    // Enter picks and shows the confirmation; Esc goes back to the list; Enter twice confirms.
    press(&mut app, &[ch('o'), key(KeyCode::Enter)]);
    let s = screen(&app, 120, 36, "prepare-master-confirm");
    assert!(s.contains("ALL DATA ON THIS DRIVE (ST500LM021 · 64.0 GB · NEW_64) WILL BE DESTROYED."), "{s}");
    assert!(s.contains("label FIELD-A"), "{s}");
    press(&mut app, &[key(KeyCode::Esc)]);
    assert!(matches!(&app.popup, Some(Popup::Prepare(p)) if !p.confirm));
    // Confirm → the sudo password, in the TUI; Esc cancels (reported), Enter runs.
    assert!(press(&mut app, &[key(KeyCode::Enter), key(KeyCode::Enter)]).is_empty());
    assert!(matches!(&app.popup, Some(Popup::Password { .. })));
    for c in "s3cret".chars() {
        press(&mut app, &[ch(c)]);
    }
    let s = screen(&app, 150, 42, "sudo-prompt");
    assert!(s.contains("Prepare the master of FIELD-A needs your sudo password."), "{s}");
    assert!(s.contains("Password  ••••••"), "{s}");
    assert!(!s.contains("s3cret"), "the password is never shown");
    let acts = press(&mut app, &[key(KeyCode::Esc)]);
    assert!(matches!(acts.as_slice(), [Action::SudoCancelled(w)] if w == "Prepare master of FIELD-A"), "{acts:?}");
    assert!(app.popup.is_none());
    press(&mut app, &[ch('o'), key(KeyCode::Enter), key(KeyCode::Enter)]);
    for c in "s3cret".chars() {
        press(&mut app, &[ch(c)]);
    }
    let acts = press(&mut app, &[key(KeyCode::Enter)]);
    let [Action::PrepareMaster { disk, label, password: Some(pw) }] = acts.as_slice() else { panic!("{acts:?}") };
    assert_eq!((disk.path.as_str(), label.as_str(), pw.0.as_str()), ("/dev/sdb", "FIELD-A", "s3cret"));
    assert_eq!(format!("{pw:?}"), "Secret(***)", "Debug never prints the password");
}

#[test]
fn prepare_refuses_signature_disk_and_smaller_slave() {
    let mut app = in_ingest_with_card();
    app.needs_sudo = false; // as root: no password prompt
    to_targets_row(&mut app, 0);
    press(&mut app, &[ch('o'), key(KeyCode::Down), key(KeyCode::Down), key(KeyCode::Down)]);
    let s = screen(&app, 150, 42, "prepare-zfs");
    assert!(s.contains("has an existing ZFS signature; wipe it first (wipefs), then Prepare"), "{s}");
    assert!(press(&mut app, &[key(KeyCode::Enter), key(KeyCode::Enter)]).is_empty(), "a refused disk is never prepared");

    // The runtime prepared the master on /dev/sdb (64 GB): the popup moves to the slave step.
    let mut s = snapshot();
    let mut m = set("FIELD-A", None);
    m.set_size = 64 * GB;
    m.master = Some(PathBuf::from("/media/op/NEW_64"));
    m.slave = None;
    m.status = SetStatus::Incomplete("slave not mounted".into());
    s.sets = vec![m];
    app.set_snapshot(s);
    app.master_prepared("FIELD-A");
    let shown = screen(&app, 150, 42, "prepare-slave");
    assert!(shown.contains("step 2 of 2 · slave of FIELD-A"), "{shown}");
    assert_eq!(app.prepare_rows(), vec![1, 2, 3], "the master (/dev/sdb) is not offered as its own slave");
    assert!(shown.contains("▶ ST500LM021 · 128.0 GB · NEW_128"), "{shown}");
    // Rows: NEW_128, NEW_32, ZFS disk. The larger one: "Will use …".
    let p = |app: &App| match &app.popup {
        Some(Popup::Prepare(p)) => p.note.clone().unwrap_or_default(),
        _ => String::new(),
    };
    assert_eq!(p(&app), "Will use 64.0 GB to match master. Rest unused.");
    press(&mut app, &[key(KeyCode::Down)]);
    assert!(p(&app).starts_with("Refused: the slave (32.0 GB) is smaller than the master (64.0 GB)"), "{}", p(&app));
    assert!(press(&mut app, &[key(KeyCode::Enter), key(KeyCode::Enter)]).is_empty(), "a smaller slave is refused");
    press(&mut app, &[key(KeyCode::Up)]);
    let acts = press(&mut app, &[key(KeyCode::Enter), key(KeyCode::Enter)]);
    assert!(
        matches!(acts.as_slice(), [Action::PrepareSlave { disk, label, password: None }] if disk.path == "/dev/sdc" && label == "FIELD-A"),
        "{acts:?}"
    );
}

#[test]
fn clean_not_offered_while_pending_offered_when_synced() {
    let mut s = snapshot();
    s.sets = vec![set("TESTSET1", Some(MagState::Pending))];
    let mut app = App::new(s, Vec::new());
    app.needs_sudo = false;
    press(&mut app, &[ch('i')]);
    let shown = screen(&app, 150, 42, "clean-pending");
    assert!(shown.contains("USB set         TESTSET1  master + slave 320.0 GB · valid · 1 pending sync"), "{shown}");
    assert!(!shown.contains("c Clean ]"), "{shown}");
    to_targets_row(&mut app, 2);
    assert!(press(&mut app, &[ch('c')]).is_empty());
    assert!(matches!(&app.popup, Some(Popup::Message(m)) if m[0].contains("pending server sync; Clean is only offered when every mag is synced")));
    press(&mut app, &[key(KeyCode::Esc)]);
    // Prepare is blocked on the set too.
    press(&mut app, &[ch('o')]);
    assert!(matches!(&app.popup, Some(Popup::Message(m)) if m[0].contains("Prepare is blocked until every mag is synced")));
    press(&mut app, &[key(KeyCode::Esc)]);

    // After sync: Clean is offered, opens the popup, Esc cancels, Enter asks the runtime.
    let mut s = snapshot();
    s.sets = vec![set("TESTSET1", Some(MagState::Synced))];
    app.set_snapshot(s);
    let shown = screen(&app, 150, 42, "clean-synced");
    assert!(shown.contains("valid · 1 synced  ·  [ c Clean ]"), "{shown}");
    press(&mut app, &[ch('c')]);
    let shown = screen(&app, 150, 42, "clean-popup");
    assert!(shown.contains("All mags on TESTSET1 are on the server. Wipe master and slave?"), "{shown}");
    assert!(shown.contains("Keeps the set: magvault-backup.txt, set id, size limit, and"), "{shown}");
    assert!(press(&mut app, &[key(KeyCode::Esc)]).is_empty());
    assert!(app.popup.is_none());
    assert_eq!(app.screen, Screen::Ingest);
    let acts = press(&mut app, &[ch('c'), key(KeyCode::Enter)]);
    assert!(matches!(acts.as_slice(), [Action::Clean { label, password: None }] if label == "TESTSET1"), "{acts:?}");
}

#[test]
fn details_pickers_offer_profile_values_only_and_new_project() {
    let mut app = in_ingest_with_card();
    while app.pane != Pane::Details {
        press(&mut app, &[key(KeyCode::Tab)]);
    }
    press(&mut app, &[key(KeyCode::Down), key(KeyCode::Enter)]); // Format
    assert!(matches!(&app.popup, Some(Popup::Picker { items, .. }) if items == &vec!["XAVC S".to_string(), "ProRes RAW".to_string()]));
    press(&mut app, &[key(KeyCode::Down), key(KeyCode::Enter)]);
    assert_eq!(app.details.format, "ProRes RAW");
    // Shoot day typed: passed as --shoot-date.
    press(&mut app, &[key(KeyCode::Down), key(KeyCode::Down), key(KeyCode::Down), key(KeyCode::Enter)]);
    for _ in 0..10 {
        press(&mut app, &[key(KeyCode::Backspace)]);
    }
    for c in "2026-11-20".chars() {
        press(&mut app, &[ch(c)]);
    }
    press(&mut app, &[key(KeyCode::Enter)]);
    assert_eq!(app.details.shoot_date, "2026-11-20");
    // n: new project by name (an open project with the same name would be reused).
    press(&mut app, &[ch('n')]);
    for c in "Night".chars() {
        press(&mut app, &[ch(c)]);
    }
    press(&mut app, &[key(KeyCode::Enter)]);
    assert!(matches!(&app.popup, Some(Popup::Requirements { .. })), "a new project asks for its requirements");
    press(&mut app, &[key(KeyCode::Enter)]); // none
    let acts = press(&mut app, &[ch('p')]);
    let [Action::Pull(a)] = acts.as_slice() else { panic!("{acts:?}") };
    assert_eq!((a.base.project.as_deref(), a.base.project_id.as_deref()), (Some("Night"), None));
    assert!(a.base.requirements.is_none());
    assert_eq!(a.base.shoot_date.as_deref(), Some("2026-11-20"));
    assert_eq!(a.base.format.as_deref(), Some("ProRes RAW"));
}

#[test]
fn small_terminal_asks_to_resize() {
    let app = App::new(snapshot(), Vec::new());
    assert!(screen(&app, 100, 30, "small").contains("Resize the terminal"));
}

#[test]
fn running_step_shows_progress_and_ctrl_c_cancels() {
    use crate::out::Msg;
    let mut app = in_ingest_with_card();
    app.start_job("Pull A7SIII_128");
    app.job_msg(Msg::Line {
        err: true,
        text: format!("copying with rsync → /home/op/mvclient/staging/.incoming/{F1}/"),
    });
    app.job_msg(Msg::Progress { label: "copying".into(), done: 25_000_000, total: 50_000_000 });
    let s = screen(&app, 150, 42, "job-progress");
    for want in ["Pull A7SIII_128", " copying", "25.0 MB / 50.0 MB  (50 %)", "copying with rsync → mag 2026-11-26", "Ctrl-C cancels"] {
        assert!(s.contains(want), "job popup misses {want:?}\n{s}");
    }
    assert!(!s.contains("/home/op") && !s.contains("aaaaaaaa-bbbb"), "paths and UUIDs only with d\n{s}");
    // While it runs, other keys do nothing; Ctrl-C asks once to stop.
    assert!(press(&mut app, &[ch('p'), key(KeyCode::Esc), ch('o')]).is_empty());
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(matches!(press(&mut app, &[ctrl_c]).as_slice(), [Action::Cancel]));
    assert!(press(&mut app, &[ctrl_c]).is_empty());
    assert!(screen(&app, 150, 42, "job-cancelling").contains("cancelling…"));
    // The step ends: back on Ingest (no "press Enter"), the failure in a popup.
    app.job_done(Err("interrupted by SIGINT; nothing committed".into()));
    assert!(app.job.is_none());
    assert_eq!(app.screen, Screen::Ingest);
    assert!(matches!(&app.popup, Some(Popup::Message(m)) if m[0] == "Pull A7SIII_128 failed:"));
}

#[test]
fn committer_down_is_never_shown_as_reachable() {
    let mut s = snapshot();
    s.server = ServerState::CommitterDown("magvaultclient@courage".into(), "no heartbeat for 40 s".into());
    s.staging_pending = vec![F1.into()];
    let mut app = App::new(s, Vec::new());
    let d = screen(&app, 150, 42, "dashboard-committer-down");
    for want in ["◐ server up, committer down", "1 mag(s) wait for the server. Its committer is down", "The server answers but does not commit"] {
        assert!(d.contains(want), "dashboard misses {want:?}\n{d}");
    }
    assert!(!d.contains("reachable") && !d.contains("● server ready"), "{d}");
    press(&mut app, &[ch('i')]);
    let i = screen(&app, 150, 42, "ingest-committer-down");
    assert!(i.contains("Server          server up, committer down — the mag waits locally"), "{i}");
    assert!(i.contains("[!] No backup and server sync not confirmed"), "{i}");
    assert!(!i.contains("reachable"), "{i}");
    press(&mut app, &[ch('d')]);
    assert!(screen(&app, 150, 42, "ingest-committer-down-details").contains("(magvaultclient@courage)"));
}

#[test]
fn plain_wording_hides_paths_uuids_devices() {
    use super::view::scrub;
    let raw = format!("Mag {F1} copied to /home/op/mvclient/staging/{F1} from /dev/sdd1 (project cce28b9f-0000-4000-8000-000000000001).");
    assert_eq!(scrub(&raw, false), "Mag mag 2026-11-26 copied to mag 2026-11-26 from drive (project …).");
    assert_eq!(scrub(&raw, true), raw);
}

#[test]
fn arrows_move_panes_and_f1_helps_on_every_screen() {
    let mut app = App::new(snapshot(), Vec::new());
    press(&mut app, &[key(KeyCode::F(1))]);
    let s = screen(&app, 150, 42, "help-dashboard");
    assert!(s.contains("Help · Dashboard") && s.contains("h Server dashboard") && s.contains("←→ / Tab"), "{s}");
    press(&mut app, &[key(KeyCode::Esc)]);
    assert!(app.popup.is_none());
    // h: the server dashboard (its own key); s: Settings.
    press(&mut app, &[ch('h')]);
    assert_eq!(app.screen, Screen::Stub(super::app::Stub::ServerDashboard));
    press(&mut app, &[key(KeyCode::Esc), ch('s')]);
    assert_eq!(app.screen, Screen::Stub(super::app::Stub::Settings));
    press(&mut app, &[key(KeyCode::Esc), ch('i')]);
    assert_eq!(app.pane, Pane::Detected);
    press(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(app.pane, Pane::Card);
    press(&mut app, &[key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Right)]);
    assert_eq!(app.pane, Pane::Targets);
    press(&mut app, &[key(KeyCode::Right)]);
    assert_eq!(app.pane, Pane::Detected, "wraps");
    press(&mut app, &[key(KeyCode::Left)]);
    assert_eq!(app.pane, Pane::Targets);
    press(&mut app, &[key(KeyCode::F(1))]);
    let s = screen(&app, 150, 42, "help-ingest");
    assert!(s.contains("Help · Ingest") && s.contains("red not dumped") && s.contains("While pulling: x aborts"), "{s}");
}

#[test]
fn requirements_warning_is_shown_and_new_project_carries_them() {
    // Existing project requires HD; the card is 4K: warning, not a stop.
    let mut s = snapshot();
    s.requirements.insert("cce28b9f-0000-4000-8000-000000000001".into(), crate::cardinfo::Requirements::parse("fps=25 res=HD"));
    let mut app = App::new(s, Vec::new());
    press(&mut app, &[ch('i'), key(KeyCode::Enter)]);
    let path = PathBuf::from("/media/op/A7SIII_128");
    app.set_preview(path.clone(), Ok(preview(&path)));
    press(&mut app, &[key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Enter), key(KeyCode::Enter)]);
    let sc = screen(&app, 150, 42, "requirements-warning");
    for want in ["Requires   25 fps · HD", "Camera footage does not meet project", "Not a hard stop. Logged to the project.", "Resolution   4K  (project requires HD)"] {
        assert!(sc.contains(want), "misses {want:?}\n{sc}");
    }
    assert!(sc.contains("Frame rate   25 fps") && !sc.contains("project requires 25 fps"), "{sc}");
    assert!(app.missing().is_empty(), "not a hard stop");
    assert!(screen(&app, 120, 36, "requirements-warning-small").contains("Camera footage does not meet"));
    // A new project: requirements from the fixed lists (→ cycles: none, 24, 25 …).
    press(&mut app, &[ch('n')]);
    for c in "Night".chars() {
        press(&mut app, &[ch(c)]);
    }
    press(&mut app, &[key(KeyCode::Enter), key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Down), key(KeyCode::Left)]);
    let sc = screen(&app, 150, 42, "requirements-popup");
    assert!(sc.contains("Frame rate  ◀ 25 fps ▶") && sc.contains("Resolution  ◀ SD ▶"), "{sc}");
    press(&mut app, &[key(KeyCode::Enter)]);
    let acts = press(&mut app, &[ch('p')]);
    let [Action::Pull(a)] = acts.as_slice() else { panic!("{acts:?}") };
    assert_eq!(a.base.requirements.map(|r| r.render()).as_deref(), Some("fps=25 res=SD"));
}

#[test]
fn mixed_camera_card_asks_before_pull() {
    let mut app = in_ingest_with_card();
    let path = PathBuf::from("/media/op/A7SIII_128");
    let mut p = preview(&path);
    p.mixed = vec!["ILCE-7M3".into(), "FDR-AX53".into()];
    app.set_preview(path, Ok(p));
    press(&mut app, &[key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Enter), key(KeyCode::Enter)]);
    assert!(screen(&app, 150, 42, "mixed-card").contains("More than one camera on this card: ILCE-7M3, FDR-AX53"));
    assert!(press(&mut app, &[ch('p')]).is_empty(), "never pulled silently");
    let s = screen(&app, 150, 42, "mixed-popup");
    assert!(s.contains("This card holds clips from more than one camera: ILCE-7M3, FDR-AX53."), "{s}");
    assert!(press(&mut app, &[key(KeyCode::Esc)]).is_empty());
    assert!(app.popup.is_none());
    let acts = press(&mut app, &[ch('p'), key(KeyCode::Enter)]);
    let [Action::Pull(a)] = acts.as_slice() else { panic!("{acts:?}") };
    assert!(a.base.mixed_ok);
}

#[test]
fn dump_colours_staging_bar_and_clock() {
    use crate::staging::DumpStatus;
    let mut s = snapshot();
    s.cards = vec![card("A7SIII_128"), card("RED_256"), card("GENERIC_64")];
    s.dump = vec![DumpStatus::Safe { copies: 2 }, DumpStatus::Dumped { copies: 1, server: false }, DumpStatus::NotDumped];
    s.staging_usage = (122_000_000_000, 258_000_000_000);
    s.staging_synced = vec![F1.into()];
    let mut app = App::new(s, Vec::new());
    app.clock = Some(crate::time::days_from_civil(2026, 11, 28) * 86_400 + 12 * 3600);
    press(&mut app, &[ch('i')]);
    let sc = screen(&app, 150, 42, "dump-colours");
    for want in ["● 2 copies · on server", "● dumped · not yet safe", "● not dumped", "122.0 GB used / 258.0 GB free", "1 on server, kept", "No field backup selected: keep the card until the server confirms.  ·  Staging auto-cleans after two backups (can be disabled).", "28/11/2026"] {
        assert!(sc.contains(want), "misses {want:?}\n{sc}");
    }
    assert!(sc.contains("██████"), "space bar\n{sc}");
    // Colours: red, yellow, green dots.
    let mut t = Terminal::new(TestBackend::new(150, 42)).unwrap();
    t.draw(|f| view::draw(f, &app)).unwrap();
    let buf = t.backend().buffer();
    let mut dots = Vec::new();
    for y in 0..42 {
        for x in 0..34 {
            let c = buf.cell((x, y)).unwrap();
            if c.symbol() == "●" {
                dots.push(c.fg);
            }
        }
    }
    use ratatui::style::Color;
    assert_eq!(dots, vec![Color::Green, Color::Yellow, Color::Red], "{dots:?}");
    app.snap.autoclean = false;
    assert!(screen(&app, 150, 42, "autoclean-off").contains("Staging auto-clean is off."));
    app.snap.autoclean = true;
    app.use_set = Some("X".into()); // a set target: no warning, the full rule
    app.snap.sets = vec![set("X", None)];
    app.snap.drives.truncate(1);
    assert!(screen(&app, 150, 42, "autoclean-note").contains("Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy."));
    assert!(screen(&app, 150, 42, "log-visible").contains("┌ Log"), "the Log fits at 150x42 with four targets");
}

#[test]
fn inline_pull_locks_inputs_and_x_aborts() {
    use crate::out::Msg;
    let mut app = in_ingest_with_card();
    app.start_pull("Pull A7SIII_128");
    app.job_msg(Msg::Progress { label: "copying".into(), done: 18_200_000, total: 42_700_000 });
    let s = screen(&app, 150, 42, "pull-inline");
    for want in ["Ingesting", "x ABORT", "18.2 MB / 42.7 MB  (43%)", "✓ hash", "● copy", "○ verify", "Inputs locked until verified", "x abort ingest", "2 Card"] {
        assert!(s.contains(want), "misses {want:?}\n{s}");
    }
    assert!(press(&mut app, &[ch('p'), key(KeyCode::Esc), key(KeyCode::Right)]).is_empty());
    assert_eq!(app.pane, Pane::Detected, "inputs locked");
    assert!(matches!(press(&mut app, &[ch('x')]).as_slice(), [Action::Cancel]));
    assert!(screen(&app, 120, 36, "pull-aborting").contains("aborting…"));
}

#[test]
fn already_dumped_card_asks_before_pull_and_matches_are_yellow() {
    use crate::staging::DumpStatus;
    let mut app = in_ingest_with_card();
    app.snap.dump = vec![DumpStatus::Safe { copies: 2 }];
    let path = PathBuf::from("/media/op/A7SIII_128");
    app.set_preview(path, Ok(preview(&PathBuf::new())));
    press(&mut app, &[key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Enter), key(KeyCode::Enter)]);
    assert!(press(&mut app, &[ch('p')]).is_empty(), "never pulled again silently");
    let s = screen(&app, 150, 42, "redump-popup");
    assert!(s.contains("Already dumped: this card was ingested here (2 copies · on server)."), "{s}");
    assert!(press(&mut app, &[key(KeyCode::Esc)]).is_empty());
    assert!(app.log.iter().any(|l| l.contains("already dumped; nothing copied")));
    let acts = press(&mut app, &[ch('p'), key(KeyCode::Enter)]);
    let [Action::Pull(a)] = acts.as_slice() else { panic!("{acts:?}") };
    assert!(a.base.force, "the TUI yes is the --force");
    // Files that match a mag from another volume: yellow, with the note; also asks.
    app.snap.dump = vec![DumpStatus::Matches { folder: F1.into() }];
    let s = screen(&app, 150, 42, "dump-matches");
    assert!(s.contains("● files match an ingested mag"), "{s}");
    assert!(s.contains("The files match a mag ingested here from another"), "{s}");
    assert!(press(&mut app, &[ch('p')]).is_empty());
    assert!(matches!(&app.popup, Some(Popup::Redump { text, .. }) if text.contains("from another volume")));
    let mut t = Terminal::new(TestBackend::new(150, 42)).unwrap();
    app.popup = None;
    t.draw(|f| view::draw(f, &app)).unwrap();
    let buf = t.backend().buffer();
    let dot = (0..42).flat_map(|y| (0..34).map(move |x| (x, y))).map(|p| buf.cell(p).unwrap()).find(|c| c.symbol() == "●").unwrap();
    assert_eq!(dot.fg, ratatui::style::Color::Yellow, "never green on a match from another volume");
}

#[test]
fn missing_ffprobe_is_said_on_screen() {
    let mut app = in_ingest_with_card();
    let path = PathBuf::from("/media/op/A7SIII_128");
    let mut p = preview(&path);
    p.ffprobe = false;
    app.set_preview(path, Ok(p));
    let s = screen(&app, 150, 42, "no-ffprobe");
    assert!(s.contains("⚠ ffprobe not found: frame rate, resolution, GOP and") && s.contains("set ffprobe: in"), "{s}");
    assert!(app.log.iter().any(|l| l.contains("ffprobe not found")));
}
