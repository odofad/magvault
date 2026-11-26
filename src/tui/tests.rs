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
        files: 22,
        bytes: 106_800_000,
        problem: None,
    }
}

fn disk(path: &str, size: u64, label: &str) -> DriveRow {
    DriveRow::Disk(Candidate {
        path: path.into(),
        disk: Ok(Disk {
            path: path.into(),
            size,
            label: label.into(),
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
    s.server = ServerState::Online("magvaultclient@courage".into());
    s.cards = vec![card("A7SIII_128")];
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

fn preview(path: &Path) -> Preview {
    let _ = path;
    Preview {
        files: vec![
            ("PRIVATE/M4ROOT/CLIP/C0001.MP4".into(), "clip", 50_000_000),
            ("PRIVATE/M4ROOT/CLIP/C0001M01.XML".into(), "sidecar", 2_000),
        ],
        skipped: vec![".DS_Store".into()],
        total: 50_002_000,
        camera: Ok("sony-a7siii".into()),
        shoot: Some(("2011-04-30".into(), "earliest file modification time".into())),
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
    for want in ["MAGVAULT  ·  Dashboard  ·  client", "1 Modules", "[i] Ingest", "[s] Server dashboard", "2 Server", "● online", "3 Local media", "4 Recent local activity", "q quit"] {
        assert!(s.contains(want), "dashboard misses {want:?}\n{s}");
    }
    let acts = press(&mut app, &[ch('i')]);
    assert!(matches!(acts.as_slice(), [Action::Refresh]));
    assert_eq!(app.screen, Screen::Ingest);
    let s = screen(&app, 150, 42, "ingest-empty");
    for want in ["1 Detected", "2 Files", "3 Project", "4 Details", "5 Targets", "[ p  PULL ]", "Sony A7S III", "Esc dashboard"] {
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
    assert!(s.contains("● selected"), "{s}");
    assert!(s.contains("C0001.MP4  [clip]"), "{s}");
    assert!(s.contains("2011-04-30   guessed"), "{s}");
    assert!(s.contains("missing: project"), "{s}");
    // Without a project, p explains what is missing and starts nothing.
    assert!(press(&mut app, &[ch('p')]).is_empty());
    assert!(matches!(&app.popup, Some(Popup::Message(m)) if m.join(" ").contains("project")));
    press(&mut app, &[key(KeyCode::Esc)]);
    // Project pane: pick the open project.
    press(&mut app, &[key(KeyCode::Tab), key(KeyCode::Tab), key(KeyCode::Enter)]);
    assert!(app.missing().is_empty(), "{:?}", app.missing());
    assert!(screen(&app, 120, 36, "ingest-ready").contains("✓ ready to pull"));
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
    for want in ["Prepare USB drive · step 1 of 2 · master", "Mount the smaller drive first and prepare it as master.", "/dev/sdb  64.0 GB", "/dev/sdc  128.0 GB", "Esc  Cancel · nothing is written until sudo is accepted"] {
        assert!(s.contains(want), "prepare popup misses {want:?}\n{s}");
    }
    assert!(!s.contains("A7SIII_128  ·"), "the camera card is not a Prepare target");
    // Esc closes the popup and stays on Ingest; nothing was asked of the runtime.
    assert!(press(&mut app, &[key(KeyCode::Esc)]).is_empty());
    assert!(app.popup.is_none());
    assert_eq!(app.screen, Screen::Ingest);

    // Enter picks and shows the confirmation; Esc goes back to the list; Enter twice confirms.
    press(&mut app, &[ch('o'), key(KeyCode::Enter)]);
    let s = screen(&app, 120, 36, "prepare-master-confirm");
    assert!(s.contains("ALL DATA ON /dev/sdb WILL BE DESTROYED."), "{s}");
    assert!(s.contains("label FIELD-A"), "{s}");
    press(&mut app, &[key(KeyCode::Esc)]);
    assert!(matches!(&app.popup, Some(Popup::Prepare(p)) if !p.confirm));
    let acts = press(&mut app, &[key(KeyCode::Enter), key(KeyCode::Enter)]);
    assert!(
        matches!(acts.as_slice(), [Action::PrepareMaster { disk, label }] if disk.path == "/dev/sdb" && label == "FIELD-A"),
        "{acts:?}"
    );
}

#[test]
fn prepare_refuses_signature_disk_and_smaller_slave() {
    let mut app = in_ingest_with_card();
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
    assert!(shown.contains("▶ /dev/sdc  128.0 GB"), "{shown}");
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
        matches!(acts.as_slice(), [Action::PrepareSlave { disk, label }] if disk.path == "/dev/sdc" && label == "FIELD-A"),
        "{acts:?}"
    );
}

#[test]
fn clean_not_offered_while_pending_offered_when_synced() {
    let mut s = snapshot();
    s.sets = vec![set("TESTSET1", Some(MagState::Pending))];
    let mut app = App::new(s, Vec::new());
    press(&mut app, &[ch('i')]);
    let shown = screen(&app, 150, 42, "clean-pending");
    assert!(shown.contains("TESTSET1  master + slave  320.0 GB · valid · 1 pending sync"), "{shown}");
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
    assert!(matches!(acts.as_slice(), [Action::Clean { label }] if label == "TESTSET1"), "{acts:?}");
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
    let acts = press(&mut app, &[ch('p')]);
    let [Action::Pull(a)] = acts.as_slice() else { panic!("{acts:?}") };
    assert_eq!((a.base.project.as_deref(), a.base.project_id.as_deref()), (Some("Night"), None));
    assert_eq!(a.base.shoot_date.as_deref(), Some("2026-11-20"));
    assert_eq!(a.base.format.as_deref(), Some("ProRes RAW"));
}

#[test]
fn small_terminal_asks_to_resize() {
    let app = App::new(snapshot(), Vec::new());
    assert!(screen(&app, 100, 30, "small").contains("Resize the terminal"));
}
