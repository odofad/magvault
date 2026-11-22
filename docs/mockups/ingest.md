# Ingest screen mockup (ratatui)

> **Mockup only.** Design reference for the ingest screen. There is no code yet.
> Drawn on character grids at the two sizes from the GUI technical spec. Colours and glyphs are placeholders until the shared theme is defined (after layouts are locked).

Source of truth for behaviour is `PLAN.md` (ingest flow, mag rules, targets). This file shows layout, focus order and keys only.

## GUI technical boundary

- ratatui + crossterm. Pure terminal, works locally and over SSH. No X11/Wayland, no HTML, no web server, no browser, no CUDA.
- Comfortable at **150x42** cells, usable down to **120x36** (1366x768 class machines). Character cells only, no pixels.
- The layout engine distributes extra space. Panes have fixed minimum sizes.
- Keyboard first. Mouse optional where useful (for example clicking a pane to focus it).
- The UI is a pure view. All data comes from files, SQLite and the server over SSH. No GUI-specific database.
- Everything must stay understandable with the TUI not running: files are the source of truth.
- One shared theme (colours, spacing, key bindings) for every screen, defined after the layouts are locked.
- Modules: Ingest screen (this file), Zmag manager (two-pane), Dashboard (reads the telemetry file), Profile Editor (admin only), Library / export selection (later).
- An optional egui/Vulkan layer may come later on the same core logic and files.

## Comfortable size, 150x42 (ready state)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Files ────────────────────────────────────────────────────┐┌─ 3 Project ──────────────────────────────────────┐
║                                ║│                                                              ││ Project  ACME_Campaign_2025                 ▾    │
║   Sony A7S III          128 GB ║│ /media/magvault/GENERIC_64                                   ││ (open projects only)                             │
║   /dev/sdb1 · exFAT            ║│ ├─ DCIM                                                      ││ n new project · UUID on create                   │
║   /media/magvault/A7SIII_128   ║│ ├─ MP_ROOT                                                   │└──────────────────────────────────────────────────┘
║   ● ready                      ║│ ├─ CLIP                                                      │┌─ 4 Details ──────────────────────────────────────┐
║                                ║│ │  ├─ A001_C001_0101R7.mov    [clip]                         ││ Camera     Sony A7S III                       ▾  │
║   RED Komodo            256 GB ║│ │  ├─ A001_C001_0102R7.mov    [clip]                         ││ Format     ProRes RAW                         ▾  │
║   /dev/sdc1 · exFAT            ║│ │  ├─ A001_C002_0101R7.mov    [clip]                         ││ Gamma      A7SIII S-Log3                      ▾  │
║   /media/magvault/RED_256      ║│ │  └─ A001_C002_0102R7.mov    [clip]                         ││ Gamut      Sony S-Gamut3.Cine                 ▾  │
║   ● ready                      ║│ ├─ MISC                                                      ││ Shoot day  2025-05-20   guessed                  │
║                                ║│ │  └─ .DS_Store               [skipped]                      ││ Notes      (optional)                            │
║ ▶ Generic DCIM           64 GB ║│ └─ INFO.TXT                   [sidecar]                      ││                                                  │
║   /dev/sdd1 · FAT32            ║│                                                              ││                                                  │
║   /media/magvault/GENERIC_64   ║│                                                              ││                                                  │
║   ● selected                   ║│                                                              ││ Confirm before pull                              │
║                                ║│                                                              ││ ✓ camera  ✓ gamma/gamut                          │
║                                ║│                                                              ││ ✓ project  ✓ shoot day  ✓ target                 │
║                                ║│                                                              ││ Missing profile? Admin only.                     │
║                                ║│                                                              ││                                                  │
║                                ║│                                                              ││                                                  │
║ r rescan    ↑↓ choose          ║│ 6 files · 42.7 GB                                            ││                                                  │
║                                ║│ 1 junk file skipped                                          ││                                                  │
╚════════════════════════════════╝└──────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
┌─ Ingest ───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                                                   │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging           writable                                                                                       │
│ [x] Server          server:/mags                    reachable                                                                                      │
│ [ ] USB backup      /media/odofad/BACKUP1 (master)  valid · connected · pending · not selected                                                     │
│ [ ] USB backup      /media/odofad/NEW_64            not valid · [ o Prepare ]                                                                      │
│ [!] Prepare needs sudo. Disabled while pending sync mags remain. Valid drives stay selectable.                                                     │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Enter pick  ·  n new project  ·  r rescan  ·  p PULL  ·  click pane to focus  ·  Esc dashboard
```

## Comfortable size, 150x42 (pulling state, inputs locked)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
┌─ 1 Detected ───────────────────┐┌─ 2 Files ────────────────────────────────────────────────────┐┌─ 3 Project ──────────────────────────────────────┐
│                                ││                                                              ││ Project  ACME_Campaign_2025                 ▾    │
│   Sony A7S III          128 GB ││ /media/magvault/GENERIC_64                                   ││ (open projects only)                             │
│   /dev/sdb1 · exFAT            ││ ├─ DCIM                                                      ││ n new project · UUID on create                   │
│   /media/magvault/A7SIII_128   ││ ├─ MP_ROOT                                                   │└──────────────────────────────────────────────────┘
│   ● ready                      ││ ├─ CLIP                                                      │┌─ 4 Details ──────────────────────────────────────┐
│                                ││ │  ├─ A001_C001_0101R7.mov    [clip]                         ││ Camera     Sony A7S III                       ▾  │
│   RED Komodo            256 GB ││ │  ├─ A001_C001_0102R7.mov    [clip]                         ││ Format     ProRes RAW                         ▾  │
│   /dev/sdc1 · exFAT            ││ │  ├─ A001_C002_0101R7.mov    [clip]                         ││ Gamma      A7SIII S-Log3                      ▾  │
│   /media/magvault/RED_256      ││ │  └─ A001_C002_0102R7.mov    [clip]                         ││ Gamut      Sony S-Gamut3.Cine                 ▾  │
│   ● ready                      ││ ├─ MISC                                                      ││ Shoot day  2025-05-20   guessed                  │
│                                ││ │  └─ .DS_Store               [skipped]                      ││ Notes      (optional)                            │
│ ▶ Generic DCIM           64 GB ││ └─ INFO.TXT                   [sidecar]                      ││                                                  │
│   /dev/sdd1 · FAT32            ││                                                              ││                                                  │
│   /media/magvault/GENERIC_64   ││                                                              ││                                                  │
│   ● selected                   ││                                                              ││ Confirm before pull                              │
│                                ││                                                              ││ ✓ camera  ✓ gamma/gamut                          │
│                                ││                                                              ││ ✓ project  ✓ shoot day  ✓ target                 │
│                                ││                                                              ││ Missing profile? Admin only.                     │
│                                ││                                                              ││                                                  │
│                                ││                                                              ││                                                  │
│ r rescan    ↑↓ choose          ││ 6 files · 42.7 GB                                            ││                                                  │
│                                ││ 1 junk file skipped                                          ││                                                  │
└────────────────────────────────┘└──────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
╔═ Ingesting ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║ [ x ABORT ]  ████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  18.2 GB / 42.7 GB  (43%)              ║
║ ● rsync copy    ○ hash + verify  ○ magcard + MHL   ○ database    ○ queue 320p                                                                      ║
╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
┌─ 5 Targets ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging           █████░░░░░░░ 43%                                                                               │
│ [x] Server          server:/mags                    queued · waits for staging                                                                     │
│ [ ] USB backup      /media/odofad/BACKUP1 (master)  valid · pending · not selected                                                                 │
│ [!] Unsafe until verified. Keep the card inserted until hashes match.                                                                              │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ pull started    Generic DCIM → local staging                                                                                                       │
│ hashing         SHA-256 after rsync finishes                                                                                                       │
│ next            hash + verify, magcard.txt + ascmhl/, database, 320p queue                                                                         │
│ locked          details and project cannot change until commit                                                                                     │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 x abort ingest  ·  l log  ·  Esc dashboard  ·  inputs locked until verified
```

## Minimum size, 120x36 (ready state)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Files ──────────────────────────────────────┐┌─ 3 Project ──────────────────────────┐
║                            ║│                                                ││ Project  ACME_Campaign_2025     ▾    │
║   Sony A7S III      128 GB ║│ /media/magvault/GENERIC_64                     ││ (open projects only)                 │
║   /dev/sdb1 · exFAT        ║│ ├─ DCIM                                        ││ n new project · UUID on create       │
║   ● ready                  ║│ ├─ MP_ROOT                                     │└──────────────────────────────────────┘
║                            ║│ ├─ CLIP                                        │┌─ 4 Details ──────────────────────────┐
║   RED Komodo        256 GB ║│ │  ├─ A001_C001_0101R7.mov    [clip]           ││ Camera     Sony A7S III           ▾  │
║   /dev/sdc1 · exFAT        ║│ │  ├─ A001_C001_0102R7.mov    [clip]           ││ Format     ProRes RAW             ▾  │
║   ● ready                  ║│ │  ├─ A001_C002_0101R7.mov    [clip]           ││ Gamma      A7SIII S-Log3          ▾  │
║                            ║│ │  └─ A001_C002_0102R7.mov    [clip]           ││ Gamut      Sony S-Gamut3.Cine     ▾  │
║ ▶ Generic DCIM       64 GB ║│ ├─ MISC                                        ││ Shoot day  2025-05-20   guessed      │
║   /dev/sdd1 · FAT32        ║│ │  └─ .DS_Store               [skipped]        ││ Notes      (optional)                │
║   ● selected               ║│ └─ INFO.TXT                   [sidecar]        ││                                      │
║                            ║│                                                ││                                      │
║                            ║│                                                ││                                      │
║                            ║│                                                ││ Confirm before pull                  │
║                            ║│                                                ││ ✓ camera  ✓ gamma/gamut              │
║                            ║│                                                ││ ✓ project  ✓ shoot day  ✓ target     │
║                            ║│                                                ││ Missing profile? Admin only.         │
║                            ║│                                                ││                                      │
║                            ║│                                                ││                                      │
║ r rescan    ↑↓ choose      ║│ 6 files · 42.7 GB                              ││                                      │
║                            ║│ 1 junk file skipped                            ││                                      │
╚════════════════════════════╝└────────────────────────────────────────────────┘└──────────────────────────────────────┘
┌─ Ingest ─────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging           writable                                                         │
│ [x] Server          server:/mags                    reachable                                                        │
│ [ ] USB backup      /media/odofad/BACKUP1 (master)  valid · pending · not selected                                   │
│ [ ] USB backup      /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Prepare: sudo required. Blocked if pending sync.                                                                 │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Enter pick  ·  n new project  ·  r rescan  ·  p PULL  ·  click pane to focus  ·  Esc dashboard
```

## How the layout flexes

| Part | 120x36 (minimum) | 150x42 (comfortable) |
|------|------------------|----------------------|
| Detected (pane 1) | 30 columns, 3 lines per card | 34 columns, adds the mount path to each card |
| Files (pane 2) | takes all remaining width (50 columns) | takes all remaining width (64 columns) |
| Project + Details (panes 3, 4) | 40 columns | 52 columns, wider pickers |
| Middle row height | 24 rows | 24 rows (same) |
| Details notes field | 3 lines | 3 lines |
| Log strip | hidden (`l` opens it as an overlay) | shown, 6 rows |
| Ingest bar, Targets | full width, progress bar scales | full width, progress bar scales |

Below 120x36, show a "resize terminal" message instead of a broken layout.

## Panes and focus order

`Tab` / `Shift+Tab` cycles the numbered panes. The focused pane uses a double-line border. A mouse click on a pane also focuses it.

| # | Pane | Suggested widget | Notes |
|---|------|------------------|-------|
| 1 | Detected | `List` + `ListState` | Auto-discovered cards (label, size, fs, guessed camera). `r` rescans. Manual path entry stays available. |
| 2 | Files | tree view (e.g. `tui-tree-widget`) or `Paragraph` | Read-only preview of the probed card. Junk (e.g. `.DS_Store`) is marked skipped and never copied. |
| 3 | Project | popup `List` over `Clear` | Lists **open** projects only. `n` creates a project; UUID assigned on create. |
| 4 | Details | field rows + popup pickers | Camera, format, gamma, gamut come from existing profiles only. No "add new" here; new profiles belong in the admin Profile Editor. Shoot day is guessed, operator confirms. Notes optional. |
| 5 | Targets | checkbox `List` | Local staging, server, USB backups. Only valid (has magvault-backup.txt) can be selected. Not-valid show Prepare. Warning for master/pending/no sync. |
| - | Ingest bar | `Gauge` + stage line | `p` starts the pull. Shows destination `mags/<YYYYMMDD>_<mag-uuid>`. |
| - | Log | `List` | Shown only when there are spare rows (40+). Events for this ingest. |

## Keys

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `Enter` | Open picker or confirm field |
| `n` | New project |
| `r` | Rescan for cards |
| `p` | Pull (start verified copy; disabled until confirm checklist complete) |
| `o` | Prepare selected not-valid USB (sudo password prompt) |
| `x` | Abort (pulling state only) |
| `l` | Show log (overlay at 120x36, strip at 150x42) |
| `Esc` | Return to dashboard (blocked while a copy is writing) |

## Behaviour notes

- **Pull is gated.** `p` only works once camera, format, gamma, gamut, project, shoot day and at least one target are confirmed (the "Confirm before pull" lines in pane 4).
- **Pipeline stages** match `PLAN.md`: rsync copy, then hash and verify the destination, write `magcard.txt` and ASC MHL, record in database, queue 320p proxies.
- **Unsafe until verified.** The card must stay inserted until hashes match.
- **Inputs lock** during a pull. Panes 1-4 dim and ignore keys.
- **USB validity:** only drives with a valid `magvault-backup.txt` at root can be selected as backup targets. Others are “not valid” and offer Prepare.
- **Prepare:** requires system sudo password as confirmation. Nothing is written until accepted. Disabled while the drive has any mags marked pending server sync. Re-enabled only after all are synced. Valid drives with pending remain selectable; they just cannot be re-prepared.
- **Pull** is the single action that starts the verified copy into the mag folder (then magcard + MHL + DB + queue). No separate “populate” step.
- Sample values (A7S III, ProRes RAW, ACME_Campaign_2025, sizes, percentages) are placeholders.
