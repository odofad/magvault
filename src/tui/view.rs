//! Drawing (layouts from docs/mockups/dashboard.md and docs/mockups/ingest.md). Reads the
//! `App` only. Focused pane: double border. Below 120x36 a "resize terminal" message.
//!
//! Plain by default: card label, project, set name, pending / synced / empty, server
//! status, next action. Paths, UUIDs, device nodes and internal codes only with `d`.

use super::app::{set_state, App, Field, InputKind, Job, Pane, Popup, PrepStep, ProjectSel, Screen, Stub, SudoOp, TargetRow, FIELDS, MENU};
use super::data::{DriveRow, ServerState};
use crate::fieldset::Disk;
use crate::ingest::is_mag_folder;
use crate::util::{human, uuid_ok};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap};
use ratatui::Frame;
use std::path::Path;

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

fn good() -> Style {
    Style::default().fg(Color::Green)
}

// ---- plain wording ----------------------------------------------------------------------

fn mag_day(folder: &str) -> String {
    format!("mag {}-{}-{}", &folder[0..4], &folder[4..6], &folder[6..8])
}

/// Hide technical tokens unless details are on: a mag folder becomes "mag 2026-11-20",
/// a UUID "…", a device node "drive", an absolute path its last part.
pub fn scrub(s: &str, tech: bool) -> String {
    if tech {
        return s.to_string();
    }
    let mut out = Vec::new();
    for tok in s.split(' ') {
        let core = tok.trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ';' | ':' | '"' | '\'' | '[' | ']' | '.'));
        let start = tok.find(core).unwrap_or(0);
        let (lead, trail) = (&tok[..start], &tok[start + core.len()..]);
        let plain = if core.is_empty() {
            None
        } else if is_mag_folder(core) {
            Some(mag_day(core))
        } else if uuid_ok(core) {
            Some("…".to_string())
        } else if core.starts_with("/dev/") {
            Some("drive".to_string())
        } else if core.starts_with('/') && core.len() > 1 {
            let base = Path::new(core).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            Some(if is_mag_folder(&base) { mag_day(&base) } else { base })
        } else {
            None
        };
        match plain {
            Some(p) => out.push(format!("{lead}{p}{trail}")),
            None => out.push(tok.to_string()),
        }
    }
    out.join(" ")
}

/// A log line `2026-11-26T23:33:41Z  INFO   ingest    message` for the screen.
fn activity(raw: &str, tech: bool) -> Line<'static> {
    if tech {
        return Line::raw(format!(" {raw}"));
    }
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() < 4 || parts[0].len() < 16 || !parts[0].ends_with('Z') {
        return Line::raw(format!(" {}", scrub(raw, false)));
    }
    let time = &parts[0][11..16];
    let level = parts[1];
    let after_level = raw.find(level).map(|i| i + level.len()).unwrap_or(0);
    let after_module = raw[after_level..].find(parts[2]).map(|i| after_level + i + parts[2].len()).unwrap_or(after_level);
    let msg = scrub(raw[after_module..].trim(), false);
    let mark = match level {
        "WARN" => "⚠ ",
        "ERROR" => "✖ ",
        "DEBUG" => "· ",
        _ => "",
    };
    let st = if matches!(level, "WARN" | "ERROR") { warn() } else { Style::default() };
    Line::styled(format!(" {time}  {mark}{msg}"), st)
}

fn server_style(s: &ServerState) -> Style {
    match s {
        ServerState::Ready(_) => good(),
        _ => warn(),
    }
}

fn server_text(s: &ServerState, tech: bool) -> String {
    let dot = match s {
        ServerState::Ready(_) => "●",
        ServerState::CommitterDown(..) => "◐",
        _ => "○",
    };
    let host = s.host();
    if tech && !host.is_empty() {
        format!("{dot} {} · {host}", s.plain())
    } else {
        format!("{dot} {}", s.plain())
    }
}

/// "ST320LM010 · 320.1 GB · NEW_64" (plus the device with details).
fn disk_name(d: &Disk, tech: bool) -> String {
    let model = if d.model.is_empty() { "USB drive".to_string() } else { d.model.clone() };
    let mut s = format!("{model} · {}", human(d.size));
    if !d.label.is_empty() {
        s.push_str(&format!(" · {}", d.label));
    }
    if tech {
        s.push_str(&format!(" · {}", d.path));
        if let Some(m) = d.mounts.first() {
            s.push_str(&format!(" · {}", m.display()));
        }
    }
    s
}

/// A refusal like "/dev/sda has an existing ZFS signature; …" for the screen.
fn refusal(path: &str, why: &str, tech: bool) -> String {
    if tech || path.is_empty() {
        why.to_string()
    } else {
        why.replacen(&format!("{path} "), "this drive ", 1)
    }
}

fn drive_text(d: &DriveRow, tech: bool) -> String {
    match d {
        DriveRow::Disk(c) => match &c.disk {
            Ok(d) => format!("{}  ·  not prepared", disk_name(d, tech)),
            Err(why) => refusal(&c.path, why, tech),
        },
        DriveRow::Folder(p) => {
            let name = if tech { p.display().to_string() } else { card_label(p) };
            format!("{name}  ·  not prepared (a folder, not a drive)")
        }
    }
}

fn card_label(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// The one thing to do next, for the dashboard.
fn dashboard_next(app: &App) -> String {
    let pending = app.pending_total();
    match (&app.snap.server, pending) {
        (ServerState::CommitterDown(..), p) if p > 0 => {
            format!("{p} mag(s) wait for the server. Its committer is down: ask for `magvault serve` to be started.")
        }
        (ServerState::CommitterDown(..), _) => "The server's committer is down: ingest still copies locally; sync waits.".into(),
        (ServerState::Offline(..), p) if p > 0 => format!("{p} mag(s) wait for the server. Connect, then press y to sync."),
        (ServerState::Ready(_), p) if p > 0 => format!("{p} mag(s) wait for the server. Press y to sync."),
        _ if !app.snap.cards.is_empty() => format!("{} card(s) detected. Press i to ingest.", app.snap.cards.len()),
        _ => "Insert a camera card, then press i to ingest.".into(),
    }
}

/// The one thing to do next, for Ingest.
fn ingest_next(app: &App) -> String {
    let m = app.missing();
    if m.is_empty() {
        "Ready: press p to pull.".into()
    } else if app.source.is_none() {
        "Pick a card in 1 Detected (Enter).".into()
    } else if m.iter().any(|x| x == "project") {
        "Pick a project in 3 Project (Enter), or n for a new one.".into()
    } else if m.iter().any(|x| x == "camera") {
        "Pick the camera in 4 Details.".into()
    } else {
        format!("Still needed: {}.", m.join(", "))
    }
}

fn header(f: &mut Frame, area: Rect, title: &str, app: &App) {
    let s = server_text(&app.snap.server, app.show_tech);
    let refreshing = if app.refreshing { "  · refreshing…" } else { "" };
    let left = format!(" MAGVAULT  ·  {title}{refreshing}");
    let pad = (area.width as usize).saturating_sub(left.chars().count() + s.chars().count() + 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" ".repeat(pad)),
            Span::styled(s, server_style(&app.snap.server)),
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
    if let Some(j) = &app.job {
        job(f, area, app, j);
    } else if let Some(p) = &app.popup {
        popup(f, area, app, p);
    }
}

// ---- dashboard ------------------------------------------------------------------------------

fn dashboard(f: &mut Frame, area: Rect, app: &App) {
    let tech = app.show_tech;
    let [head, body, foot] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(area);
    header(f, head, "Dashboard", app);
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
    lines.push(Line::styled(" y sync   r refresh", dim()));
    lines.push(Line::styled(" d details   q quit", dim()));
    f.render_widget(Paragraph::new(lines).block(pane("1 Modules", true)), menu);

    let [next, top, recent] =
        Layout::vertical([Constraint::Length(3), Constraint::Length(9), Constraint::Fill(1)]).areas(right);
    f.render_widget(
        Paragraph::new(Line::styled(format!(" {}", dashboard_next(app)), Style::default().add_modifier(Modifier::BOLD)))
            .block(pane("Next", false)),
        next,
    );
    let [srv, local] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(top);
    let mut s = vec![Line::raw(""), Line::styled(format!(" {}", server_text(&app.snap.server, false)), server_style(&app.snap.server))];
    match &app.snap.server {
        ServerState::Ready(_) => s.push(Line::raw(" Uploads are verified and committed.")),
        ServerState::CommitterDown(_, why) => {
            s.push(Line::raw(" The server answers but does not commit"));
            s.push(Line::raw(" (magvault serve is not running)."));
            if tech {
                s.push(Line::styled(format!(" {why}"), dim()));
            }
        }
        ServerState::Offline(_, why) => {
            s.push(Line::raw(" Mags wait locally until it is back."));
            if tech {
                s.push(Line::styled(format!(" {why}"), dim()));
            }
        }
        ServerState::NotConfigured => s.push(Line::raw(" Mags stay local (no server in the config).")),
    }
    if tech {
        s.push(Line::styled(format!(" host {}", app.snap.server.host()), dim()));
        s.push(Line::styled(" queue depth not reported yet (Phase 2)", dim()));
    }
    f.render_widget(Paragraph::new(s).wrap(Wrap { trim: false }).block(pane("2 Server", false)), srv);

    let n = app.snap.staging_pending.len();
    let mut l = vec![
        Line::raw(""),
        Line::raw(if n == 0 { " Local staging   empty".to_string() } else { format!(" Local staging   {n} mag(s) waiting for the server") }),
    ];
    if tech {
        l.push(Line::styled(format!("                 {}", app.snap.staging.display()), dim()));
    }
    for set in &app.snap.sets {
        l.push(Line::raw(format!(" Field set {}  {} · {}", set.label, human(set.set_size), set_state(set))));
    }
    for d in &app.snap.drives {
        l.push(Line::raw(format!(" USB  {}", drive_text(d, tech))));
    }
    if app.snap.sets.is_empty() && app.snap.drives.is_empty() {
        l.push(Line::styled(" No field drive connected.", dim()));
    }
    f.render_widget(Paragraph::new(l).block(pane("3 Local media", false)), local);
    let lines: Vec<Line> = if app.snap.activity.is_empty() {
        vec![Line::styled(" nothing logged yet", dim())]
    } else {
        app.snap.activity.iter().map(|a| activity(a, tech)).collect()
    };
    f.render_widget(Paragraph::new(lines).block(pane("4 Recent activity", false)), recent);
    f.render_widget(
        Paragraph::new(" ↑↓ Enter open · i ingest · y sync · r refresh · d details · g logging · l library · s server dashboard · q quit")
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

// ---- ingest ---------------------------------------------------------------------------------

fn ingest(f: &mut Frame, area: Rect, app: &App) {
    let tech = app.show_tech;
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
        let name = c.camera.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| "camera not recognised".into());
        let chosen = app.source.as_ref() == Some(&c.volume.mount);
        l.push(Line::raw(format!("{}{}", sel(app.pane == Pane::Detected && i == app.card_sel), card_label(&c.volume.mount))));
        l.push(Line::raw(format!("  {name} · {}", human(c.bytes))));
        if tech {
            l.push(Line::styled(format!("  {} · {} · {}", c.volume.device, c.volume.fstype, c.layout), dim()));
        }
        let state = match (&c.problem, chosen) {
            (Some(p), _) => format!("  ✖ {}", scrub(p, tech)),
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
            l.push(Line::raw(format!("  ● {}", if tech { s.display().to_string() } else { card_label(s) })));
        }
    }
    l.push(Line::raw(""));
    l.push(Line::styled(" r rescan · Enter pick", dim()));
    f.render_widget(Paragraph::new(l).block(pane("1 Detected", app.pane == Pane::Detected)), det);

    // 2 Files
    let mut l = Vec::new();
    match (&app.source, &app.preview) {
        (None, _) => l.push(Line::styled(" pick a card in 1 Detected", dim())),
        (Some(s), None) => l.push(Line::raw(format!(" {}  · reading the card…", card_label(s)))),
        (Some(_), Some(Err(e))) => l.push(Line::styled(format!(" cannot ingest: {}", scrub(e, tech)), warn())),
        (Some(s), Some(Ok(p))) => {
            l.push(Line::raw(format!(" {}", if tech { s.display().to_string() } else { card_label(s) })));
            let rows = (files.height as usize).saturating_sub(6);
            for (rel, role, size) in p.files.iter().skip(app.files_scroll).take(rows) {
                let name = if tech { rel.clone() } else { card_label(Path::new(rel)) };
                l.push(Line::raw(format!(" {name}  [{role}]  {}", human(*size))));
            }
            l.push(Line::raw(""));
            l.push(Line::raw(format!(" {} files · {}", p.files.len(), human(p.total))));
        }
    }
    f.render_widget(Paragraph::new(l).block(pane("2 Files", app.pane == Pane::Files)), files);

    // 3 Project
    let mut l = Vec::new();
    let chosen = match &app.project {
        Some(ProjectSel::Existing { name, id }) if tech => format!("{name}  ({id})"),
        Some(ProjectSel::Existing { name, .. }) => name.clone(),
        Some(ProjectSel::New { name }) => format!("{name}  (new)"),
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
    let model = match (&d.model, &d.model_hint) {
        (Some(m), _) => format!("{m}  (yours)"),
        (None, Some(h)) if tech => h.clone(),
        (None, Some(h)) => format!("{} (from card)", h.split(" (").next().unwrap_or(h)),
        (None, None) => "(not on the card)".to_string(),
    };
    let vals = [cam, d.format.clone(), d.gamma.clone(), d.gamut.clone(), shoot, notes, model];
    let names = ["Camera", "Format", "Gamma", "Gamut", "Shoot day", "Notes", "Model"];
    let mut l = vec![Line::raw("")];
    for i in 0..FIELDS.len() {
        let v = if vals[i].is_empty() { "-".to_string() } else { vals[i].clone() };
        l.push(Line::raw(format!("{}{:<10} {v}", sel(app.pane == Pane::Details && i == app.detail_sel), names[i])));
    }
    l.push(Line::raw(""));
    let missing = app.missing();
    if missing.is_empty() {
        l.push(Line::styled(" ✓ ready to pull", good()));
    } else {
        l.push(Line::raw(" Confirm before pull"));
        l.push(Line::styled(format!(" missing: {}", missing.join(", ")), warn()));
    }
    l.push(Line::styled(" Missing profile? Admin only.", dim()));
    f.render_widget(
        Paragraph::new(l).wrap(Wrap { trim: false }).block(pane("4 Details", app.pane == Pane::Details)),
        details,
    );

    // Ingest bar: route and next action
    let dest = match app.route() {
        Ok((v, m)) => {
            let set = app.use_set.clone().unwrap_or_default();
            match (v, m) {
                (crate::client_ingest::Via::Staging, false) => "card → local staging → server".to_string(),
                (crate::client_ingest::Via::Staging, true) => format!("card → local staging + field set {set} → server"),
                (crate::client_ingest::Via::Set, _) => format!("card → field set {set} → server"),
                (crate::client_ingest::Via::Direct, _) => "card → server (no local copy)".to_string(),
            }
        }
        Err(e) => e,
    };
    let server = if app.use_server { "" } else { "  · server later (mag stays pending)" };
    f.render_widget(
        Paragraph::new(vec![
            Line::raw(format!(" [ p  PULL ]  {dest}{server}")),
            Line::styled(format!(" Next: {}", ingest_next(app)), Style::default().add_modifier(Modifier::BOLD)),
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
            TargetRow::Staging => {
                let path = if tech { format!("   {}", app.snap.staging.display()) } else { String::new() };
                format!("{cur}{} Local staging{path}", check(app.use_staging))
            }
            TargetRow::Server => {
                let st = match &app.snap.server {
                    ServerState::Ready(_) => "ready".to_string(),
                    ServerState::CommitterDown(..) => format!("{} — the mag waits locally", crate::client::COMMITTER_DOWN),
                    ServerState::Offline(..) => "offline — the mag waits locally".to_string(),
                    ServerState::NotConfigured => "none configured".into(),
                };
                let host = if tech && !app.snap.server.host().is_empty() { format!("  ({})", app.snap.server.host()) } else { String::new() };
                format!("{cur}{} Server          {st}{host}", check(app.use_server))
            }
            TargetRow::Set(k) => {
                let s = &app.snap.sets[*k];
                let action = if crate::fieldset::clean_allowed(s).is_ok() {
                    "  ·  [ c Clean ]"
                } else if s.master.is_some() && s.slave.is_none() && s.mags.is_empty() {
                    "  ·  [ o Prepare slave ]"
                } else {
                    ""
                };
                format!(
                    "{cur}{} Field set       {}  {} · {}{action}",
                    check(app.use_set.as_deref() == Some(&s.label)),
                    s.label,
                    human(s.set_size),
                    set_state(s)
                )
            }
            TargetRow::Drive(k) => match &app.snap.drives[*k] {
                DriveRow::Disk(c) if c.disk.is_ok() => {
                    format!("{cur}[ ] USB drive       {}  ·  [ o Prepare ]", drive_text(&app.snap.drives[*k], tech))
                }
                other => format!("{cur}[ ] USB drive       {}", drive_text(other, tech)),
            },
        };
        l.push(Line::raw(line));
    }
    match app.target_warning() {
        Some(w) => l.push(Line::styled(format!("[!] {w}"), warn())),
        None => l.push(Line::styled(
            "    Prepare asks for the sudo password; not offered on a set while any mag waits for the server.",
            dim(),
        )),
    }
    f.render_widget(Paragraph::new(l).block(pane("5 Targets", app.pane == Pane::Targets)), tgt);

    if log_h > 0 {
        let n = (log_h as usize).saturating_sub(2);
        let lines: Vec<Line> = app.log.iter().rev().take(n).rev().map(|s| Line::raw(format!(" {}", scrub(s, tech)))).collect();
        f.render_widget(Paragraph::new(lines).block(pane("Log", false)), logp);
    }
    f.render_widget(
        Paragraph::new(" Tab pane · ↑↓ move · Enter pick · n project · r rescan · p PULL · y sync · o Prepare · c Clean · d details · Esc back")
            .style(dim()),
        foot,
    );
}

// ---- popups ---------------------------------------------------------------------------------

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

/// The running step: stage, progress bar (bytes and percent), the latest lines.
fn job(f: &mut Frame, area: Rect, app: &App, j: &Job) {
    let r = centered(area, 96, 20);
    f.render_widget(Clear, r);
    let block = pane(&j.title, true);
    let inner = block.inner(r);
    f.render_widget(block, r);
    let [stage, gauge, _, lines, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    let stage_text = if j.cancelling { "cancelling…".to_string() } else { j.stage.clone() };
    f.render_widget(Paragraph::new(format!(" {stage_text}")), stage);
    if j.total > 0 {
        let ratio = (j.done as f64 / j.total as f64).clamp(0.0, 1.0);
        let label = format!("{} / {}  ({:.0} %)", human(j.done), human(j.total), ratio * 100.0);
        f.render_widget(Gauge::default().gauge_style(good()).ratio(ratio).label(label), gauge);
    } else {
        f.render_widget(Paragraph::new(" working…").style(dim()), gauge);
    }
    let n = lines.height as usize;
    let tail: Vec<Line> = j.lines.iter().rev().take(n).rev().map(|s| Line::raw(format!(" {}", scrub(s, app.show_tech)))).collect();
    f.render_widget(Paragraph::new(tail).wrap(Wrap { trim: false }), lines);
    f.render_widget(
        Paragraph::new(" Ctrl-C cancels (nothing is committed; a mag already copied stays pending) · d details").style(dim()),
        foot,
    );
}

fn popup(f: &mut Frame, area: Rect, app: &App, p: &Popup) {
    let tech = app.show_tech;
    let (title, lines, w): (String, Vec<Line>, u16) = match p {
        Popup::Message(m) => {
            let mut l: Vec<Line> = vec![Line::raw("")];
            l.extend(m.iter().map(|s| Line::raw(format!("  {}", scrub(s, tech)))));
            l.push(Line::raw(""));
            l.push(Line::styled("  any key closes", dim()));
            ("Magvault".into(), l, 84)
        }
        Popup::Password { op, text } => {
            let what = match op {
                SudoOp::PrepareMaster { label, .. } => format!("Prepare the master of {label}"),
                SudoOp::PrepareSlave { label, .. } => format!("Prepare the slave of {label}"),
                SudoOp::Clean { label } => format!("Clean {label}"),
            };
            let l = vec![
                Line::raw(""),
                Line::raw(format!("  {what} needs your sudo password.")),
                Line::raw(""),
                Line::raw(format!("  Password  {}▏", "•".repeat(text.chars().count()))),
                Line::raw(""),
                Line::styled("  Enter run · Esc or Ctrl-C cancel (nothing is written)", dim()),
                Line::styled("  The password goes to sudo only; it is never logged.", dim()),
            ];
            ("sudo".into(), l, 70)
        }
        Popup::Input { kind, text } => {
            let what = match kind {
                InputKind::ManualPath => "Card path (a mounted card or a folder)",
                InputKind::NewProject => "New project name (an open project with this name is reused)",
                InputKind::ShootDate => "Shoot day (YYYY-MM-DD)",
                InputKind::Notes => "Mag notes (optional)",
                InputKind::SetLabel => "Set label (A-Z, 0-9, '-', up to 12)",
                InputKind::Model => "Camera model as you name it (empty = keep what the card says)",
            };
            let l = vec![
                Line::raw(""),
                Line::raw(format!("  {what}")),
                Line::raw(""),
                Line::raw(format!("  > {text}▏")),
                Line::raw(""),
                Line::styled("  Enter accept · Esc cancel", dim()),
            ];
            ("Input".into(), l, 80)
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
            (name.into(), l, 70)
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
                Line::raw("  [ Enter  Wipe master and slave ]   then the sudo password"),
                Line::raw(""),
                Line::styled("  Esc  Cancel · nothing is deleted until sudo is accepted", dim()),
            ];
            (format!("Clean field backup set · {label}"), l, 78)
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
            let mut l = vec![Line::raw(""), Line::styled(format!("  {intro}"), warn()), Line::raw(""), Line::raw("  Connected drives")];
            if rows.is_empty() {
                l.push(Line::styled("    no USB drive found (camera cards are never prepared)", dim()));
            }
            for (pos, &i) in rows.iter().enumerate() {
                l.push(Line::raw(format!("  {}{}", sel(pos == ps.sel), drive_text(&app.snap.drives[i], tech))));
            }
            l.push(Line::raw(""));
            let cur = rows.get(ps.sel).map(|&i| &app.snap.drives[i]);
            let disk = cur.and_then(|d| match d {
                DriveRow::Disk(c) => c.disk.as_ref().ok(),
                _ => None,
            });
            if let Some(n) = &ps.note {
                let path = match cur {
                    Some(DriveRow::Disk(c)) => c.path.clone(),
                    _ => String::new(),
                };
                let n = scrub(&refusal(&path, n, tech), tech);
                let st = if n.starts_with("Will use") { Style::default() } else { warn() };
                l.push(Line::styled(format!("  {n}"), st));
            }
            match (&ps.step, disk, ps.confirm) {
                (PrepStep::Master, Some(d), true) => {
                    l.push(Line::raw(format!("  Master   {}", disk_name(d, tech))));
                    l.push(Line::raw(format!("  Set size {}   label {}  (e edit)", human(d.size / 4096 * 4096), ps.label)));
                    l.push(Line::styled(format!("  ALL DATA ON THIS DRIVE ({}) WILL BE DESTROYED.", disk_name(d, tech)), warn()));
                    l.push(Line::raw("  [ Enter  Confirm size ]  then the sudo password;"));
                    l.push(Line::raw("  then: format ext4, write magvault-backup.txt (master)."));
                }
                (PrepStep::Slave { .. }, Some(d), true) => {
                    l.push(Line::raw(format!("  Slave    {}", disk_name(d, tech))));
                    l.push(Line::styled(format!("  ALL DATA ON THIS DRIVE ({}) WILL BE DESTROYED.", disk_name(d, tech)), warn()));
                    l.push(Line::raw("  [ Enter  Confirm ]  then the sudo password;"));
                    l.push(Line::raw("  then: limit to the set size, write magvault-backup.txt (slave)."));
                }
                (PrepStep::Master, _, _) => {
                    l.push(Line::raw("  Enter  pick as master"));
                    l.push(Line::raw("  Pick the master first. Its size becomes the set size."));
                }
                (PrepStep::Slave { .. }, _, _) => l.push(Line::raw("  Enter  pick as slave · a smaller drive is refused")),
            }
            l.push(Line::raw("  Nothing is written until the sudo password is accepted."));
            if !tech {
                l.push(Line::styled("  d shows device names", dim()));
            }
            l.push(Line::raw(""));
            l.push(Line::styled("  Esc  Cancel · nothing is written until sudo is accepted", dim()));
            (title, l, 96)
        }
    };
    // Height from the wrapped line count.
    let inner = (w.min(area.width) as usize).saturating_sub(2).max(1);
    let wrapped: u16 = lines.iter().map(|l| (l.width().max(1)).div_ceil(inner) as u16).sum();
    let r = centered(area, w, wrapped + 2);
    f.render_widget(Clear, r);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(pane(&title, true)), r);
}
