# Logs module mockup (ratatui)

> **Mockup only.** Design reference for the Logs module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample entries (including the multi-line context) are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Logs module"). The viewer reads the daily files `logs/YYYY-MM-DD.log` from the `logs/` path in `path.conf`, never the database. One entry per line: `timestamp (UTC)  LEVEL  module  message`. Newest first. Filter by module or by level (INFO / WARN / ERROR). The detail pane shows the full selected line plus any multi-line context that follows it. Optional follow mode with `f`. Old day files are removed by the `log_retention_days` setting (default 30); that is a note only, there is no retention control in this screen.

Panes: 1 Filter (module row, level row), 2 Log list (time, level, module, short message; a divider per day file), 3 Entry detail.

## Keys

| key | action |
|---|---|
| Tab / Shift+Tab | cycle panes |
| Up / Down | move in the list (or in the focused filter row: Left / Right) |
| Enter | select entry / apply focused filter option |
| `f` | toggle follow mode |
| `m` | cycle module filter (all, ingest, encoder, zmag, library, logging, settings, system) |
| `l` | cycle level filter (all, INFO, WARN, ERROR) |
| Esc | back to dashboard |

`m` and `l` are proposals; `f` is from PLAN.md. Logs is a server module (dashboard key `v`); the client menu in PLAN.md does not list it.

## Logs, 150x42 (server)

```text
 MAGVAULT  ·  Logs  ·  server                                                                                                             follow: off
┌─ 1 Filter ─────────────────────────────────────────────────────────────────────────────────────┐┌─ 3 Entry ────────────────────────────────────────┐
│ module  all  ingest  encoder  zmag  library  logging  settings  system                         ││                                                  │
│ level   all  INFO  WARN  ERROR                                                                 ││ time     2026-11-22T21:15:05Z                    │
└────────────────────────────────────────────────────────────────────────────────────────────────┘│ level    ERROR                                   │
╔═ 2 Log (newest first) ═════════════════════════════════════════════════════════════════════════╗│ module   zmag                                    │
║ time      level  module    message                                                             ║│ file     logs/2026-11-22.log                     │
║ ── logs/2026-11-22.log ─────────────────────────────────────────────────────────────────────── ║│                                                  │
║   21:16:40  INFO   library   Export list updated: 3 clips (exportcard.txt)                     ║│ Message                                          │
║   21:16:12  INFO   settings  path.conf saved (logs changed)                                    ║│   zmag-02 degraded: sdg faulted                  │
║   21:15:58  INFO   logging   Log entry saved for mag 20250520_a3f9c2                           ║│                                                  │
║   21:15:40  INFO   encoder   Proxy done for A001_C004.mov                                      ║│ Context (lines that follow)                      │
║   21:15:22  INFO   zmag      Snapshot created for zmag-01                                      ║│    pool: zmag-02                                 │
║ ▶ 21:15:05  ERROR  zmag      zmag-02 degraded: sdg faulted                                     ║│    state: DEGRADED                               │
║   21:15:04  WARN   encoder   Proxy failed for A001_C005.mov: timeout                           ║│    sdg   FAULTED  too many errors                │
║   21:15:03  INFO   ingest    Mag 20250520_a3f9c2 ingested (84 GB)                              ║│                                                  │
║   21:14:51  INFO   ingest    rsync finished, MHL verified                                      ║│                                                  │
║   21:14:02  INFO   encoder   Proxy started for A001_C004.mov                                   ║│                                                  │
║   21:10:15  WARN   ingest    USB drive slow to mount                                           ║│                                                  │
║   21:00:00  INFO   system    magvault started                                                  ║│                                                  │
║ ── logs/2026-11-21.log ─────────────────────────────────────────────────────────────────────── ║│                                                  │
║   23:58:10  INFO   system    magvault stopped                                                  ║│                                                  │
║   23:12:44  INFO   library   Export started: 3 clips                                           ║│                                                  │
║   22:40:09  ERROR  encoder   Proxy failed for B002_C011.mov: no profile match                  ║│                                                  │
║   22:31:30  INFO   ingest    Mag 20250520_b71e04 ingested (61 GB)                              ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│ Reads logs/*.log files only,                     │
║                                                                                                ║│ never the database.                              │
║                                                                                                ║│ Old day files are removed per                    │
║                                                                                                ║│ log_retention_days.                              │
║                                                                                                ║│                                                  │
╚════════════════════════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter select · f follow · m module · l level · Esc dashboard
```

## Logs, module filter + follow on, 150x42

```text
 MAGVAULT  ·  Logs  ·  server                                                                                                              follow: ON
┌─ 1 Filter ─────────────────────────────────────────────────────────────────────────────────────┐┌─ 3 Entry ────────────────────────────────────────┐
│ module  all  ingest  encoder  zmag  library  logging  settings  system                         ││                                                  │
│ level   all  INFO  WARN  ERROR                                                                 ││ time     2026-11-22T21:15:40Z                    │
└────────────────────────────────────────────────────────────────────────────────────────────────┘│ level    INFO                                    │
╔═ 2 Log (newest first) ═════════════════════════════════════════════════════════════════════════╗│ module   encoder                                 │
║ time      level  module    message                                                             ║│ file     logs/2026-11-22.log                     │
║ ── logs/2026-11-22.log ─────────────────────────────────────────────────────────────────────── ║│                                                  │
║ ▶ 21:15:40  INFO   encoder   Proxy done for A001_C004.mov                                      ║│ Message                                          │
║   21:15:04  WARN   encoder   Proxy failed for A001_C005.mov: timeout                           ║│   Proxy done for A001_C004.mov                   │
║   21:14:02  INFO   encoder   Proxy started for A001_C004.mov                                   ║│                                                  │
║ ── logs/2026-11-21.log ─────────────────────────────────────────────────────────────────────── ║│ Context (lines that follow)                      │
║   22:40:09  ERROR  encoder   Proxy failed for B002_C011.mov: no profile match                  ║│   (none)                                         │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│                                                  │
║                                                                                                ║│ Reads logs/*.log files only,                     │
║                                                                                                ║│ never the database.                              │
║                                                                                                ║│ Old day files are removed per                    │
║                                                                                                ║│ log_retention_days.                              │
║                                                                                                ║│                                                  │
╚════════════════════════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter select · f follow · m module · l level · Esc dashboard
```

## Logs, 120x36 (minimum)

```text
 MAGVAULT  ·  Logs  ·  server                                                                               follow: off
┌─ 1 Filter ───────────────────────────────────────────────────────────────────┐┌─ 3 Entry ────────────────────────────┐
│ module  all  ingest  encoder  zmag  library  logging  settings  system       ││                                      │
│ level   all  INFO  WARN  ERROR                                               ││ time     2026-11-22T21:15:05Z        │
└──────────────────────────────────────────────────────────────────────────────┘│ level    ERROR                       │
╔═ 2 Log (newest first) ═══════════════════════════════════════════════════════╗│ module   zmag                        │
║ time      level  module    message                                           ║│ file     logs/2026-11-22.log         │
║ ── logs/2026-11-22.log ───────────────────────────────────────────────────── ║│                                      │
║   21:16:40  INFO   library   Export list updated: 3 clips (exportcard.txt)   ║│ Message                              │
║   21:16:12  INFO   settings  path.conf saved (logs changed)                  ║│   zmag-02 degraded: sdg faulted      │
║   21:15:58  INFO   logging   Log entry saved for mag 20250520_a3f9c2         ║│                                      │
║   21:15:40  INFO   encoder   Proxy done for A001_C004.mov                    ║│ Context (lines that follow)          │
║   21:15:22  INFO   zmag      Snapshot created for zmag-01                    ║│    pool: zmag-02                     │
║ ▶ 21:15:05  ERROR  zmag      zmag-02 degraded: sdg faulted                   ║│    state: DEGRADED                   │
║   21:15:04  WARN   encoder   Proxy failed for A001_C005.mov: timeout         ║│    sdg   FAULTED  too many errors    │
║   21:15:03  INFO   ingest    Mag 20250520_a3f9c2 ingested (84 GB)            ║│                                      │
║   21:14:51  INFO   ingest    rsync finished, MHL verified                    ║│                                      │
║   21:14:02  INFO   encoder   Proxy started for A001_C004.mov                 ║│                                      │
║   21:10:15  WARN   ingest    USB drive slow to mount                         ║│                                      │
║   21:00:00  INFO   system    magvault started                                ║│                                      │
║ ── logs/2026-11-21.log ───────────────────────────────────────────────────── ║│                                      │
║   23:58:10  INFO   system    magvault stopped                                ║│                                      │
║   23:12:44  INFO   library   Export started: 3 clips                         ║│                                      │
║   22:40:09  ERROR  encoder   Proxy failed for B002_C011.mov: no profile mat… ║│                                      │
║   22:31:30  INFO   ingest    Mag 20250520_b71e04 ingested (61 GB)            ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│ Reads logs/*.log files only,         │
║                                                                              ║│ never the database.                  │
║                                                                              ║│ Old day files are removed per        │
║                                                                              ║│ log_retention_days.                  │
║                                                                              ║│                                      │
╚══════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter select · f follow · m module · l level · Esc dashboard
```

## Logs, module filter + follow on, 120x36

```text
 MAGVAULT  ·  Logs  ·  server                                                                                follow: ON
┌─ 1 Filter ───────────────────────────────────────────────────────────────────┐┌─ 3 Entry ────────────────────────────┐
│ module  all  ingest  encoder  zmag  library  logging  settings  system       ││                                      │
│ level   all  INFO  WARN  ERROR                                               ││ time     2026-11-22T21:15:40Z        │
└──────────────────────────────────────────────────────────────────────────────┘│ level    INFO                        │
╔═ 2 Log (newest first) ═══════════════════════════════════════════════════════╗│ module   encoder                     │
║ time      level  module    message                                           ║│ file     logs/2026-11-22.log         │
║ ── logs/2026-11-22.log ───────────────────────────────────────────────────── ║│                                      │
║ ▶ 21:15:40  INFO   encoder   Proxy done for A001_C004.mov                    ║│ Message                              │
║   21:15:04  WARN   encoder   Proxy failed for A001_C005.mov: timeout         ║│   Proxy done for A001_C004.mov       │
║   21:14:02  INFO   encoder   Proxy started for A001_C004.mov                 ║│                                      │
║ ── logs/2026-11-21.log ───────────────────────────────────────────────────── ║│ Context (lines that follow)          │
║   22:40:09  ERROR  encoder   Proxy failed for B002_C011.mov: no profile mat… ║│   (none)                             │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│                                      │
║                                                                              ║│ Reads logs/*.log files only,         │
║                                                                              ║│ never the database.                  │
║                                                                              ║│ Old day files are removed per        │
║                                                                              ║│ log_retention_days.                  │
║                                                                              ║│                                      │
╚══════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter select · f follow · m module · l level · Esc dashboard
```

## Open point for the plan

PLAN.md says the viewer is "newest first" and that follow mode "keeps the view at the bottom as new lines arrive". Those conflict. The mockup resolves it by pinning the selection to the newest entry (top of the list) while follow is on, and shows `follow: ON` in the header. If the intent is a classic tail (oldest at top, newest at bottom while following), the list order flips when follow is toggled. Please decide in PLAN.md.
