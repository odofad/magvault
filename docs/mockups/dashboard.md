# Dashboard mockup (ratatui)

> **Mockup only.** Design reference for the dashboard (landing screen). There is no code yet.
> Drawn on character grids at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, shortcut letters and all sample values are placeholders until the shared theme is defined.

Source of truth for behaviour is `PLAN.md` ("Dashboard (landing screen)", "Telemetry", "Media playback"). This file shows layout, focus order and keys only.

## What the dashboard is

The first screen after launch. It shows status and is the way into every other module. There are two versions, both ratatui.

- **Server (or standalone) dashboard:** full status from the local telemetry file plus light local checks.
- **Client dashboard:** thin status only. It does **not** show full encoder progress or zmag drive health. Those stay on the server and can be opened over SSH.

Keyboard first (arrows + Enter, or a single-letter shortcut). Mouse optional. `Esc` or `q` returns to the dashboard from any module. On the dashboard itself, `q` quits.

## Server dashboard, 150x42

```text
 MAGVAULT  ·  Dashboard  ·  server                                                                               telemetry updated 12s ago  ·  odofad
╔═ 1 Modules ════════════════╗┌─ 2 Encoder queue ────────────────────────────────────────┐┌─ 3 Zmag health ──────────────────────────────────────────┐
║                            ║│                                                          ││                                                          │
║ ▶ [i] Ingest               ║│ Depth 3   Active 1                                       ││ zmag-01  primary   ONLINE                                │
║                            ║│                                                          ││   sda ●   sdb ●   sdc ●   sdd ●                          │
║   [g] Logging              ║│ ▶ 320p  20250520_a3f9c2                                  ││                                                          │
║                            ║│   ACME_Campaign_2025 · clip 3 of 6                       ││ zmag-02  backup    DEGRADED                              │
║   [l] Library              ║│   ██████████████████████████████░░░░░░░░░░░░░░░░░░ 62%   ││   sde ●   sdf ●   sdg ✖   sdh ●                          │
║                            ║│                                                          ││   [!] sdg FAULTED · replace drive                        │
║   [e] Export bins          ║│ next                                                     ││                                                          │
║                            ║│ · 320p  20250519_c41d77  ACME_Campaign_2025              ││ 2 mounted · only recognised zmags shown                  │
║   [z] Zmag manager         ║│ · 320p  20250517_0cd8e1  Bamba_Cutaways                  ││                                                          │
║                            ║│                                                          ││                                                          │
║   [p] Profile Editor *     ║│                                                          ││                                                          │
║                            ║│                                                          ││                                                          │
║   [v] Logs                 ║└──────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────────────┘
║                            ║┌─ Status ─────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
║   [s] Settings             ║│ Open projects                         Online mags                           Pending requests                         │
║                            ║│ 4                                     37                                    2                                        │
║                            ║│                                                                                                                      │
║                            ║└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
║                            ║┌─ 4 Recent events ────────────────────────────────────────────────────────────────────────────────────────────────────┐
║                            ║│ 19:21  proxy started       320p · 20250520_a3f9c2                                                                    │
║                            ║│ 19:20  ingest committed    20250520_a3f9c2 · ACME_Campaign_2025                                                      │
║                            ║│ 19:18  proxy done          320p · 20250518_7be410                                                                    │
║                            ║│ 19:02  zmag-02 degraded    sdg FAULTED                                                                               │
║                            ║│ 18:40  request pending     cold pull · 20250412_55d0aa → Bamba-Cutaways                                              │
║                            ║│ 18:12  export created      Bamba-Cutaways · 6 clips                                                                  │
║                            ║│ 17:55  zmag-01 mounted     4/4 drives ONLINE                                                                         │
║                            ║│ 17:31  project opened      Bamba_Cutaways                                                                            │
║                            ║│ 16:48  ingest committed    20250518_7be410 · ACME_Campaign_2025                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║ * admin only               ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║ ↑↓ move   Enter open       ║│                                                                                                                      │
║ Esc / q: back here         ║│                                                                                                                      │
╚════════════════════════════╝└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ Enter open  ·  letter = module  ·  Tab pane  ·  r refresh  ·  q quit  ·  Esc/q in a module returns here
```

## Server dashboard, 120x36 (minimum)

```text
 MAGVAULT  ·  Dashboard  ·  server                                                 telemetry updated 12s ago  ·  odofad
╔═ 1 Modules ════════════╗┌─ 2 Encoder queue ───────────────────────────┐┌─ 3 Zmag health ─────────────────────────────┐
║                        ║│                                             ││                                             │
║ ▶ [i] Ingest           ║│ Depth 3   Active 1                          ││ zmag-01  primary   ONLINE                   │
║                        ║│                                             ││   sda ●   sdb ●   sdc ●   sdd ●             │
║   [g] Logging          ║│ ▶ 320p  20250520_a3f9c2                     ││                                             │
║                        ║│   ACME_Campaign_2025 · clip 3 of 6          ││ zmag-02  backup    DEGRADED                 │
║   [l] Library          ║│   ██████████████████████░░░░░░░░░░░░░ 62%   ││   sde ●   sdf ●   sdg ✖   sdh ●             │
║                        ║│                                             ││   [!] sdg FAULTED · replace drive           │
║   [e] Export bins      ║│ next                                        ││                                             │
║                        ║│ · 320p  20250519_c41d77  ACME_Campaign_2025 ││ 2 mounted · only recognised zmags shown     │
║   [z] Zmag manager     ║│ · 320p  20250517_0cd8e1  Bamba_Cutaways     ││                                             │
║                        ║│                                             ││                                             │
║   [p] Profile Editor * ║└─────────────────────────────────────────────┘└─────────────────────────────────────────────┘
║                        ║┌─ Status ───────────────────────────────────────────────────────────────────────────────────┐
║   [v] Logs             ║│ Open projects                 Online mags                   Pending requests               │
║                        ║│ 4                             37                            2                              │
║   [s] Settings         ║│                                                                                            │
║                        ║└────────────────────────────────────────────────────────────────────────────────────────────┘
║                        ║┌─ 4 Recent events ──────────────────────────────────────────────────────────────────────────┐
║                        ║│ 19:21  proxy started       320p · 20250520_a3f9c2                                          │
║                        ║│ 19:20  ingest committed    20250520_a3f9c2 · ACME_Campaign_2025                            │
║                        ║│ 19:18  proxy done          320p · 20250518_7be410                                          │
║                        ║│ 19:02  zmag-02 degraded    sdg FAULTED                                                     │
║                        ║│ 18:40  request pending     cold pull · 20250412_55d0aa → Bamba-Cutaways                    │
║                        ║│ 18:12  export created      Bamba-Cutaways · 6 clips                                        │
║                        ║│ 17:55  zmag-01 mounted     4/4 drives ONLINE                                               │
║                        ║│ 17:31  project opened      Bamba_Cutaways                                                  │
║                        ║│ 16:48  ingest committed    20250518_7be410 · ACME_Campaign_2025                            │
║                        ║│                                                                                            │
║ * admin only           ║│                                                                                            │
║                        ║│                                                                                            │
║ ↑↓ move   Enter open   ║│                                                                                            │
║ Esc / q: back here     ║│                                                                                            │
╚════════════════════════╝└────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ Enter open  ·  letter = module  ·  Tab pane  ·  r refresh  ·  q quit  ·  Esc/q in a module returns here
```

## Client dashboard, 150x42

```text
 MAGVAULT  ·  Dashboard  ·  client                                                                                        ● server syncing  ·  odofad
╔═ 1 Modules ════════════════╗┌─ 2 Server ───────────────────────────────────────────────┐┌─ 3 Local media ──────────────────────────────────────────┐
║                            ║│                                                          ││                                                          │
║ ▶ [i] Ingest               ║│ Connection   ● syncing                                   ││ Staging  /tmp/magvault-staging                           │
║                            ║│ Host         server (ssh)                                ││          1 mag pending sync                              │
║   [g] Logging (local)      ║│ Queue depth  3                                           ││ USB      BACKUP1 (master)  valid                         │
║                            ║│ Last sync    19:20 · 1 mag                               ││          2 mags pending sync                             │
║   [l] Library              ║│                                                          ││ USB      NEW_64  not valid                               │
║                            ║│ states: online · offline · syncing                       ││                                                          │
║   [s] Server dashboard     ║└──────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────────────┘
║                            ║┌─ Server-only status ─────────────────────────────────────────────────────────────────────────────────────────────────┐
║                            ║│ Encoder progress and zmag health stay on the server. s opens it over SSH.                                            │
║                            ║└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
║                            ║┌─ 4 Recent local activity ────────────────────────────────────────────────────────────────────────────────────────────┐
║                            ║│                                                                                                                      │
║                            ║│ 19:21  copy to staging     20250520_a3f9c2 · Generic DCIM                                                            │
║                            ║│ 19:20  sync to server      20250520_a3f9c2 · syncing                                                                 │
║                            ║│ 19:05  logging saved       20250518_7be410 · 12 clips · local                                                        │
║                            ║│ 18:52  usb synced          BACKUP1 · 1 mag                                                                           │
║                            ║│ 18:30  server connected    ssh ok                                                                                    │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║ Library: local proxies     ║│                                                                                                                      │
║ + export requests          ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║                            ║│                                                                                                                      │
║ ↑↓ move   Enter open       ║│                                                                                                                      │
║ Esc / q: back here         ║│                                                                                                                      │
╚════════════════════════════╝└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ Enter open  ·  i ingest  ·  g logging  ·  l library  ·  s server dashboard  ·  Tab pane  ·  q quit
```

## Client dashboard, 120x36 (minimum)

```text
 MAGVAULT  ·  Dashboard  ·  client                                                          ● server syncing  ·  odofad
╔═ 1 Modules ════════════╗┌─ 2 Server ──────────────────────────────────┐┌─ 3 Local media ─────────────────────────────┐
║                        ║│                                             ││                                             │
║ ▶ [i] Ingest           ║│ Connection   ● syncing                      ││ Staging  /tmp/magvault-staging              │
║                        ║│ Host         server (ssh)                   ││          1 mag pending sync                 │
║   [g] Logging (local)  ║│ Queue depth  3                              ││ USB      BACKUP1 (master)  valid            │
║                        ║│ Last sync    19:20 · 1 mag                  ││          2 mags pending sync                │
║   [l] Library          ║│                                             ││ USB      NEW_64  not valid                  │
║                        ║│ states: online · offline · syncing          ││                                             │
║   [s] Server dashboard ║└─────────────────────────────────────────────┘└─────────────────────────────────────────────┘
║                        ║┌─ Server-only status ───────────────────────────────────────────────────────────────────────┐
║                        ║│ Encoder progress and zmag health stay on the server. s opens it over SSH.                  │
║                        ║└────────────────────────────────────────────────────────────────────────────────────────────┘
║                        ║┌─ 4 Recent local activity ──────────────────────────────────────────────────────────────────┐
║                        ║│                                                                                            │
║                        ║│ 19:21  copy to staging     20250520_a3f9c2 · Generic DCIM                                  │
║                        ║│ 19:20  sync to server      20250520_a3f9c2 · syncing                                       │
║                        ║│ 19:05  logging saved       20250518_7be410 · 12 clips · local                              │
║                        ║│ 18:52  usb synced          BACKUP1 · 1 mag                                                 │
║                        ║│ 18:30  server connected    ssh ok                                                          │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║ Library: local proxies ║│                                                                                            │
║ + export requests      ║│                                                                                            │
║                        ║│                                                                                            │
║                        ║│                                                                                            │
║ ↑↓ move   Enter open   ║│                                                                                            │
║ Esc / q: back here     ║│                                                                                            │
╚════════════════════════╝└────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ Enter open  ·  i ingest  ·  g logging  ·  l library  ·  s server dashboard  ·  Tab pane  ·  q quit
```

## Server panes

| # | Pane | Suggested widget | Notes |
|---|------|------------------|-------|
| 1 | Modules | `List` + `ListState` | Ingest, Logging, Library, Export bins, Zmag manager, Profile Editor (admin only, marked `*`), Logs, Settings. Letter shortcuts are proposals. |
| 2 | Encoder queue | `Paragraph` + `Gauge` | Queue depth, active job with progress, next jobs. From the telemetry file. |
| 3 | Zmag health | `Paragraph` | Overall pool state for every mounted zmag and every drive. Any non-ONLINE drive is called out in a warning line. Only recognised zmags are listed. |
| - | Status | `Paragraph` | Simple counts: open projects, online mags, pending requests. |
| 4 | Recent events | `List` | Newest first. Colour marks the kind (ok, warning, neutral). |

## Client panes

| # | Pane | Suggested widget | Notes |
|---|------|------------------|-------|
| 1 | Modules | `List` + `ListState` | Ingest, Logging (works against local media), Library (local proxies + export requests), and "Server dashboard" which opens the full server dashboard over SSH. A short note under the menu spells out what Library means on the client. |
| 2 | Server | `Paragraph` | Connection state (online / offline / syncing), host, a simple queue depth number when connected, last sync. |
| 3 | Local media | `Paragraph` | Local staging and USB backup drives (valid / not valid, pending sync counts). First-class on the client. |
| - | Server-only status | `Paragraph` | Reminder that encoder progress and zmag health are on the server. |
| 4 | Recent local activity | `List` | Copies to staging, syncs, local logging saves, USB syncs, connection changes. |

## Keys

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `Enter` | Open the selected module |
| Module letter | Open that module directly (`i` Ingest, `g` Logging, `l` Library, `e` Export bins, `z` Zmag manager, `p` Profile Editor, `v` Logs, `s` Settings; on the client `i` Ingest, `g` Logging, `l` Library and `s` the server dashboard) |
| `r` | Refresh now |
| `Esc` / `q` | In a module: back to the dashboard. On the dashboard: `q` quits. |

## Behaviour notes

- **Server dashboard reads the local telemetry file** (`telemetry_live`, with the periodic disk copy for reboot survival). It needs no root. The header shows how fresh the data is.
- **Client dashboard does not pull the full telemetry file.** It may ask the server for a short status summary over SSH (connection state, queue depth).
- **Profile Editor is admin only.** Ordinary operators do not see it in the menu.
- **Degraded pools are loud.** A zmag with any non-ONLINE drive shows its state in the warning colour and names the drive.
- **Offline client.** When the server is unreachable the Server pane shows offline, queue depth is hidden, and local media and logging stay usable.
- **Layout flex:** the Modules pane keeps a fixed width (30 at 150x42, 26 at 120x36). The remaining width is split between the two top panes. The Recent events pane takes the leftover height.
- Below 120x36, show a "resize terminal" message instead of a broken layout.
- All sample values (counts, times, mag ids, project names, drive names, percentages) are placeholders.
