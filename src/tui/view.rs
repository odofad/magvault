//! Drawing (layouts from docs/mockups/dashboard.md and docs/mockups/ingest.md). Reads the
//! `App` only. Focused pane: double border. Below 120x36 a "resize terminal" message.

use super::app::{set_state, App, Field, InputKind, Pane, Popup, PrepStep, ProjectSel, Screen, Stub, TargetRow, FIELDS, MENU};
use super::data::{DriveRow, ServerState};
use crate::util::human;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub const MIN_W: u16 = 120;
pub const MIN_H: u16 = 36;

fn pane(title: &str, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(if focused { BorderType::Double } else { BorderType::Plain })
        .title(format!(" {title} "))
}

fn sel(on: bool) -> &'static str {
    if on {
        "▶ "
    } else {
        "  "
    }
}

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn warn() -> Style {
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
}

fn server_line(app: &App) -> (String, Style) {
    match &app.snap.server {
        ServerState::Online(h) => (format!("● server connected · {h}"), Style::default().fg(Color::Green)),
        ServerState::Offline(h, _) => (format!("○ server offline · {h}"), warn()),
        ServerState::NotConfigured => ("○ no server configured".into(), warn()),
    }
}

fn header(f: &mut Frame, area: Rect, title: &str, app: &App) {
    let (s, st) = server_line(app);
    let refreshing = if app.refreshing { "  · refreshing…" } else { "" };
    let left = format!(" MAGVAULT  ·  {title}{refreshing}");
    let pad = (area.width as usize).saturating_sub(left.chars().count() + s.chars().count() + 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" ".repeat(pad)),
            Span::styled(s, st),
        ])),
        area,
    );
}

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    if area.width < MIN_W || area.height < MIN_H {
        f.render_widget(
            Paragraph::new(format!(
                "Magvault needs at least {MIN_W}x{MIN_H} cells (now {}x{}). Resize the terminal. q quits.",
                area.width, area.height
            ))
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    match app.screen {
        Screen::Dashboard => dashboard(f, area, app),
        Screen::Ingest => ingest(f, area, app),
        Screen::Stub(s) => stub(f, area, app, s),
    }
    if let Some(p) = &app.popup {
        popup(f, area, app, p);
    }
}

// ---- dashboard ------------------------------------------------------------------------------

fn dashboard(f: &mut Frame, area: Rect, app: &App) {
    let [head, body, foot] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    header(f, head, "Dashboard  ·  client", app);
    let menu_w = if area.width >= 150 { 30 } else { 26 };
    let [menu, right] = Layout::horizontal([Constraint::Length(menu_w), Constraint::Fill(1)]).areas(body);
    let mut lines = vec![Line::raw("")];
    for (i, (k, name)) in MENU.iter().enumerate() {
        lines.push(Line::raw(format!("{}[{k}] {name}", sel(i == app.dash_sel))));
        lines.push(Line::raw(""));
    }
    lines.push(Line::styled(" g l s: later phases", dim()));
    lines.push(Line::raw(""));
    lines.push(Line::styled(" ↑↓ move   Enter open", dim()));
    lines.push(Line::styled(" r refresh   q quit", dim()));
    f.render_widget(Paragraph::new(lines).block(pane("1 Modules", true)), menu);

    let [top, note, recent] =
        Layout::vertical([Constraint::Length(9), Constraint::Length(3), Constraint::Fill(1)]).areas(right);
    let [srv, local] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(top);
    let (state, host, why) = match &app.snap.server {
        ServerState::Online(h) => ("● online", h.clone(), String::new()),
        ServerState::Offline(h, w) => ("○ offline", h.clone(), w.clone()),
        ServerState::NotConfigured => ("○ not configured", "-".into(), "server_host is empty in path.conf".into()),
    };
    let mut s = vec![
        Line::raw(""),
        Line::raw(format!(" Connection   {state}")),
        Line::raw(format!(" Host         {host}")),
        Line::raw(" Queue depth  not reported yet (Phase 2)"),
    ];
    if !why.is_empty() {
        s.push(Line::styled(format!(" {why}"), dim()));
    }
    f.render_widget(Paragraph::new(s).wrap(Wrap { trim: false }).block(pane("2 Server", false)), srv);

    let mut l = vec![
        Line::raw(""),
        Line::raw(format!(" Staging  {}", app.snap.staging.display())),
        Line::raw(format!("          {} mag(s) pending sync", app.snap.staging_pending.len())),
    ];
    for set in &app.snap.sets {
        l.push(Line::raw(format!(" USB set  {}  {} · {}", set.label, human(set.set_size), set_state(set))));
    }
    for d in &app.snap.drives {
        l.push(Line::raw(format!(" USB      {}", drive_name(d))));
    }
    if app.snap.sets.is_empty() && app.snap.drives.is_empty() {
        l.push(Line::styled(" USB      no field drive mounted", dim()));
    }
    f.render_widget(Paragraph::new(l).block(pane("3 Local media", false)), local);
    f.render_widget(
        Paragraph::new(" Encoder progress and zmag health stay on the server. s will open the server dashboard over SSH (Phase 2).")
            .block(pane("Server-only status", false)),
        note,
    );
    let lines: Vec<Line> = if app.snap.activity.is_empty() {
        vec![Line::styled(" no local activity logged yet", dim())]
    } else {
        app.snap.activity.iter().map(|a| Line::raw(format!(" {a}"))).collect()
    };
    f.render_widget(Paragraph::new(lines).block(pane("4 Recent local activity", false)), recent);
    f.render_widget(
        Paragraph::new(" ↑↓ Enter open  ·  i ingest  ·  g logging  ·  l library  ·  s server dashboard  ·  r refresh  ·  q quit")
            .style(dim()),
        foot,
    );
}

fn stub(f: &mut Frame, area: Rect, app: &App, s: Stub) {
    let [head, body, foot] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    let (title, text) = match s {
        Stub::Logging => ("Logging", "Logging comes in Phase 3 (PLAN.md roadmap)."),
        Stub::Library => ("Library", "Library comes in Phase 3 (PLAN.md roadmap)."),
        Stub::ServerDashboard => (
            "Server dashboard",
            "The server dashboard comes in Phase 2. It will open on the server over SSH.",
        ),
    };
    header(f, head, title, app);
    f.render_widget(Paragraph::new(format!("\n  {text}")).block(pane(title, true)), body);
    f.render_widget(Paragraph::new(" Esc dashboard").style(dim()), foot);
}

fn drive_name(d: &DriveRow) -> String {
    match d {
        DriveRow::Disk(c) => match &c.disk {
            Ok(d) => {
                let m = d.mounts.first().map(|m| m.display().to_string()).unwrap_or_else(|| "not mounted".into());
                format!("{}  {}  {m}  not valid", d.path, human(d.size))
            }
            Err(why) => why.clone(),
        },
        DriveRow::Folder(p) => format!("{}  not valid (folder)", p.display()),
    }
}

// ---- ingest ---------------------------------------------------------------------------------

fn ingest(f: &mut Frame, area: Rect, app: &App) {
    let targets = app.targets();
    let target_rows = targets.len().min(8);
    let target_h = target_rows as u16 + 3;
    let log_h = if area.height >= 42 { 6 } else { 0 };
    let [head, mid, bar, tgt, logp, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(4),
        Constraint::Length(target_h),
        Constraint::Length(log_h),
        Constraint::Length(1),
    ])
    .areas(area);
    header(f, head, "Ingest", app);
    let (det_w, right_w) = if area.width >= 150 { (34, 52) } else { (30, 40) };
    let [det, files, right] =
        Layout::horizontal([Constraint::Length(det_w), Constraint::Fill(1), Constraint::Length(right_w)]).areas(mid);
    let [proj, details] = Layout::vertical([Constraint::Length(8), Constraint::Fill(1)]).areas(right);

    // 1 Detected
    let mut l = vec![Line::raw("")];
    for (i, c) in app.snap.cards.iter().enumerate() {
        let name = c.camera.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| c.layout.clone());
        let chosen = app.source.as_ref() == Some(&c.volume.mount);
        l.push(Line::raw(format!("{}{name}", sel(app.pane == Pane::Detected && i == app.card_sel))));
        let label = c.volume.mount.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        l.push(Line::raw(format!("  {label} · {}", human(c.bytes))));
        l.push(Line::raw(format!("  {} · {}", c.volume.device, c.volume.fstype)));
        let state = match (&c.problem, chosen) {
            (Some(p), _) => format!("  ✖ {p}"),
            (None, true) => "  ● selected".into(),
            (None, false) => "  ● ready".into(),
        };
        l.push(Line::raw(state));
        l.push(Line::raw(""));
    }
    if app.snap.cards.is_empty() {
        l.push(Line::styled("  no camera card detected", dim()));
        l.push(Line::raw(""));
    }
    let manual = app.card_sel >= app.snap.cards.len();
    l.push(Line::raw(format!("{}type a path…", sel(app.pane == Pane::Detected && manual))));
    if let Some(s) = &app.source {
        if !app.snap.cards.iter().any(|c| &c.volume.mount == s) {
            l.push(Line::raw(format!("  ● {}", s.display())));
        }
    }
    l.push(Line::raw(""));
    l.push(Line::styled(" r rescan · Enter pick", dim()));
    f.render_widget(Paragraph::new(l).block(pane("1 Detected", app.pane == Pane::Detected)), det);

    // 2 Files
    let mut l = Vec::new();
    match (&app.source, &app.preview) {
        (None, _) => l.push(Line::styled(" pick a card in 1 Detected", dim())),
        (Some(s), None) => l.push(Line::raw(format!(" {}  · probing…", s.display()))),
        (Some(_), Some(Err(e))) => l.push(Line::styled(format!(" cannot ingest: {e}"), warn())),
        (Some(s), Some(Ok(p))) => {
            l.push(Line::raw(format!(" {}", s.display())));
            let rows = (files.height as usize).saturating_sub(6);
            for (rel, role, size) in p.files.iter().skip(app.files_scroll).take(rows) {
                l.push(Line::raw(format!(" {rel}  [{role}]  {}", human(*size))));
            }
            l.push(Line::raw(""));
            l.push(Line::raw(format!(" {} files · {}", p.files.len(), human(p.total))));
            if !p.skipped.is_empty() {
                l.push(Line::styled(format!(" {} junk file(s) skipped", p.skipped.len()), dim()));
            }
        }
    }
    f.render_widget(Paragraph::new(l).block(pane("2 Files", app.pane == Pane::Files)), files);

    // 3 Project
    let mut l = Vec::new();
    let chosen = match &app.project {
        Some(ProjectSel::Existing { name, .. }) => name.clone(),
        Some(ProjectSel::New { name }) => format!("{name}  (new · UUID on create)"),
        None => "(pick one)".into(),
    };
    l.push(Line::raw(format!(" Project  {chosen}")));
    let list = app.projects();
    let rows = (proj.height as usize).saturating_sub(4);
    let start = app.project_sel.saturating_sub(rows.saturating_sub(1));
    for (i, p) in list.iter().enumerate().skip(start).take(rows) {
        let name = match p {
            ProjectSel::Existing { name, .. } => name.clone(),
            ProjectSel::New { name } => format!("{name} (new)"),
        };
        l.push(Line::raw(format!(" {}{name}", sel(app.pane == Pane::Project && i == app.project_sel))));
    }
    if list.is_empty() {
        l.push(Line::styled(" no open projects known", dim()));
    }
    l.push(Line::styled(" n new project · open projects only", dim()));
    f.render_widget(Paragraph::new(l).block(pane("3 Project", app.pane == Pane::Project)), proj);

    // 4 Details
    let d = &app.details;
    let cam = app.camera().map(|c| c.name.clone()).unwrap_or_else(|| "(pick one)".into());
    let shoot = if d.shoot_date.is_empty() {
        "(pick one)".to_string()
    } else {
        format!("{}   {}", d.shoot_date, if d.shoot_edited { "given" } else { "guessed" })
    };
    let notes = if d.notes.is_empty() { "(optional)".to_string() } else { d.notes.clone() };
    let vals = [cam, d.format.clone(), d.gamma.clone(), d.gamut.clone(), shoot, notes];
    let names = ["Camera", "Format", "Gamma", "Gamut", "Shoot day", "Notes"];
    let mut l = vec![Line::raw("")];
    for (i, f_) in FIELDS.iter().enumerate() {
        let v = if vals[i].is_empty() { "-".to_string() } else { vals[i].clone() };
        let _ = f_;
        l.push(Line::raw(format!("{}{:<10} {v}", sel(app.pane == Pane::Details && i == app.detail_sel), names[i])));
    }
    l.push(Line::raw(""));
    let missing = app.missing();
    if missing.is_empty() {
        l.push(Line::styled(" ✓ ready to pull", Style::default().fg(Color::Green)));
    } else {
        l.push(Line::raw(" Confirm before pull"));
        l.push(Line::styled(format!(" missing: {}", missing.join(", ")), warn()));
    }
    l.push(Line::styled(" Missing profile? Admin only.", dim()));
    f.render_widget(
        Paragraph::new(l).wrap(Wrap { trim: false }).block(pane("4 Details", app.pane == Pane::Details)),
        details,
    );

    // Ingest bar
    let dest = match app.route() {
        Ok((v, m)) => {
            let set = app.use_set.clone().unwrap_or_default();
            match (v, m) {
                (crate::client_ingest::Via::Staging, false) => "card → local staging → server".to_string(),
                (crate::client_ingest::Via::Staging, true) => format!("card → local staging + field set {set} → server"),
                (crate::client_ingest::Via::Set, _) => format!("card → field set {set} (master, then slave) → server"),
                (crate::client_ingest::Via::Direct, _) => "card → server (no local copy)".to_string(),
            }
        }
        Err(e) => e,
    };
    let server = if app.use_server { "" } else { "  · server not contacted now (mag stays pending)" };
    f.render_widget(
        Paragraph::new(vec![
            Line::raw(format!(" [ p  PULL ]  {dest}{server}")),
            Line::styled(
                " → <YYYYMMDD>_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified · keep the card in until verified",
                dim(),
            ),
        ])
        .block(pane("Ingest", false)),
        bar,
    );

    // 5 Targets
    let mut l = Vec::new();
    let first = if app.pane == Pane::Targets { app.target_sel.saturating_sub(target_rows.saturating_sub(1)) } else { 0 };
    for (i, t) in targets.iter().enumerate().skip(first).take(target_rows) {
        let cur = sel(app.pane == Pane::Targets && i == app.target_sel);
        let check = |b: bool| if b { "[x]" } else { "[ ]" };
        let line = match t {
            TargetRow::Staging => format!("{cur}{} Local staging   {}", check(app.use_staging), app.snap.staging.display()),
            TargetRow::Server => {
                let st = match &app.snap.server {
                    ServerState::Online(h) => format!("{h}  reachable"),
                    ServerState::Offline(h, _) => format!("{h}  NOT reachable (mag stays pending)"),
                    ServerState::NotConfigured => "none configured".into(),
                };
                format!("{cur}{} Server          {st}", check(app.use_server))
            }
            TargetRow::Set(k) => {
                let s = &app.snap.sets[*k];
                let action = if fieldset_clean_ok(s) {
                    "  ·  [ c Clean ]"
                } else if s.master.is_some() && s.slave.is_none() && s.mags.is_empty() {
                    "  ·  [ o Prepare slave ]"
                } else {
                    ""
                };
                format!(
                    "{cur}{} USB set         {}  master + slave  {} · {}{action}",
                    check(app.use_set.as_deref() == Some(&s.label)),
                    s.label,
                    human(s.set_size),
                    set_state(s)
                )
            }
            TargetRow::Drive(k) => match &app.snap.drives[*k] {
                DriveRow::Disk(c) => match &c.disk {
                    Ok(_) => format!("{cur}[ ] USB drive       {}  ·  [ o Prepare ]", drive_name(&app.snap.drives[*k])),
                    Err(why) => format!("{cur}[ ] USB drive       {why}"),
                },
                DriveRow::Folder(p) => format!("{cur}[ ] USB drive       {}  not valid (folder)", p.display()),
            },
        };
        l.push(Line::raw(line));
    }
    match app.target_warning() {
        Some(w) => l.push(Line::styled(format!("[!] {w}"), warn())),
        None => l.push(Line::styled("    Prepare needs sudo; not offered on a set while any mag is pending sync.", dim())),
    }
    f.render_widget(Paragraph::new(l).block(pane("5 Targets", app.pane == Pane::Targets)), tgt);

    if log_h > 0 {
        let n = (log_h as usize).saturating_sub(2);
        let lines: Vec<Line> = app.log.iter().rev().take(n).rev().map(|s| Line::raw(format!(" {s}"))).collect();
        f.render_widget(Paragraph::new(lines).block(pane("Log", false)), logp);
    }
    f.render_widget(
        Paragraph::new(
            " Tab pane · ↑↓ move · Enter pick · n new project · r rescan · p PULL · o Prepare · c Clean · Esc dashboard",
        )
        .style(dim()),
        foot,
    );
}

fn fieldset_clean_ok(s: &crate::fieldset::FieldSet) -> bool {
    crate::fieldset::clean_allowed(s).is_ok()
}

// ---- popups ---------------------------------------------------------------------------------

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

fn popup(f: &mut Frame, area: Rect, app: &App, p: &Popup) {
    let (title, lines, w, h): (String, Vec<Line>, u16, u16) = match p {
        Popup::Message(m) => {
            let mut l: Vec<Line> = vec![Line::raw("")];
            l.extend(m.iter().map(|s| Line::raw(format!("  {s}"))));
            l.push(Line::raw(""));
            l.push(Line::styled("  any key closes", dim()));
            ("Magvault".into(), l, 80, m.len() as u16 + 6)
        }
        Popup::Input { kind, text } => {
            let what = match kind {
                InputKind::ManualPath => "Card path (a mounted card or a folder)",
                InputKind::NewProject => "New project name (an open project with this name is reused)",
                InputKind::ShootDate => "Shoot day (YYYY-MM-DD)",
                InputKind::Notes => "Mag notes (optional)",
                InputKind::SetLabel => "Set label (A-Z, 0-9, '-', up to 12)",
            };
            let l = vec![
                Line::raw(""),
                Line::raw(format!("  {what}")),
                Line::raw(""),
                Line::raw(format!("  > {text}▏")),
                Line::raw(""),
                Line::styled("  Enter accept · Esc cancel", dim()),
            ];
            ("Input".into(), l, 80, 9)
        }
        Popup::Picker { field, items, sel: s } => {
            let name = match field {
                Field::Camera => "Camera profile",
                Field::Format => "File format",
                Field::Gamma => "Gamma",
                Field::Gamut => "Gamut",
                _ => "",
            };
            let mut l = vec![Line::raw("")];
            for (i, it) in items.iter().enumerate() {
                l.push(Line::raw(format!("  {}{it}", sel(i == *s))));
            }
            l.push(Line::raw(""));
            l.push(Line::styled("  values from existing profiles only · Enter pick · Esc cancel", dim()));
            (name.into(), l, 70, items.len() as u16 + 5)
        }
        Popup::Clean { label, mags } => {
            let l = vec![
                Line::raw(""),
                Line::raw(format!("  All mags on {label} are on the server. Wipe master and slave?")),
                Line::raw(""),
                Line::raw(format!("  Deletes the {mags} mag folder(s) on both drives.")),
                Line::raw("  Keeps the set: magvault-backup.txt, set id, size limit, and"),
                Line::raw("  the master / slave roles. No re-prepare."),
                Line::raw("  After clean the set is empty, still valid, ready for new mags."),
                Line::raw("  This action is logged."),
                Line::raw(""),
                Line::raw("  [ Enter  Wipe master and slave ]   sudo asks for the password next"),
                Line::raw(""),
                Line::styled("  Esc  Cancel · nothing is deleted until sudo is accepted", dim()),
            ];
            (format!("Clean field backup set · {label}"), l, 78, 15)
        }
        Popup::Prepare(ps) => {
            let rows = app.prepare_rows();
            let (title, intro) = match &ps.step {
                PrepStep::Master => (
                    "Prepare USB drive · step 1 of 2 · master".to_string(),
                    "Mount the smaller drive first and prepare it as master.".to_string(),
                ),
                PrepStep::Slave { label, set_size, .. } => (
                    format!("Prepare USB drive · step 2 of 2 · slave of {label}"),
                    format!("The slave must be the same size or larger than the master ({}).", human(*set_size)),
                ),
            };
            let mut l = vec![Line::raw(""), Line::styled(format!("  {intro}"), warn()), Line::raw(""), Line::raw("  Mounted drives")];
            if rows.is_empty() {
                l.push(Line::styled("    no USB drive found (camera cards are never prepared)", dim()));
            }
            for (pos, &i) in rows.iter().enumerate() {
                l.push(Line::raw(format!("  {}{}", sel(pos == ps.sel), drive_name(&app.snap.drives[i]))));
            }
            l.push(Line::raw(""));
            let disk = rows.get(ps.sel).and_then(|&i| match &app.snap.drives[i] {
                DriveRow::Disk(c) => c.disk.as_ref().ok(),
                _ => None,
            });
            if let Some(n) = &ps.note {
                let st = if n.starts_with("Will use") { Style::default() } else { warn() };
                l.push(Line::styled(format!("  {n}"), st));
            }
            match (&ps.step, disk, ps.confirm) {
                (PrepStep::Master, Some(d), true) => {
                    l.push(Line::raw(format!("  Master   {} · {}", d.path, human(d.size))));
                    l.push(Line::raw(format!("  Set size {}   label {}  (e edit)", human(d.size / 4096 * 4096), ps.label)));
                    l.push(Line::styled(format!("  ALL DATA ON {} WILL BE DESTROYED.", d.path), warn()));
                    l.push(Line::raw("  [ Enter  Confirm size ]  then sudo asks for the password;"));
                    l.push(Line::raw("  then: format ext4, write magvault-backup.txt (master)."));
                }
                (PrepStep::Slave { .. }, Some(d), true) => {
                    l.push(Line::raw(format!("  Slave    {} · {}", d.path, human(d.size))));
                    l.push(Line::styled(format!("  ALL DATA ON {} WILL BE DESTROYED.", d.path), warn()));
                    l.push(Line::raw("  [ Enter  Confirm ]  then sudo asks for the password;"));
                    l.push(Line::raw("  then: limit to the set size, write magvault-backup.txt (slave)."));
                }
                (PrepStep::Master, _, _) => {
                    l.push(Line::raw("  Enter  pick as master"));
                    l.push(Line::raw("  Pick the master first. Its size becomes the set size."));
                }
                (PrepStep::Slave { .. }, _, _) => l.push(Line::raw("  Enter  pick as slave · a smaller drive is refused")),
            }
            l.push(Line::raw("  Nothing is written until the sudo password is accepted."));
            l.push(Line::raw(""));
            l.push(Line::styled("  Esc  Cancel · nothing is written until sudo is accepted", dim()));
            (title, l, 90, 0)
        }
    };
    // Height from the wrapped line count (0 = compute).
    let inner = (w.min(area.width) as usize).saturating_sub(2).max(1);
    let wrapped: u16 = lines.iter().map(|l| (l.width().max(1)).div_ceil(inner) as u16).sum();
    let h = h.max(wrapped + 2);
    let r = centered(area, w, h);
    f.render_widget(Clear, r);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(pane(&title, true)), r);
}
