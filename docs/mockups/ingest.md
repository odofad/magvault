# Ingest screen mockup (ratatui)

> **Mockup only.** Design reference for the ingest screen. There is no code yet.
> Drawn on character grids at the two sizes from the GUI technical spec. Colours and glyphs are placeholders until the shared theme is defined (after layouts are locked).

Source of truth for behaviour is `PLAN.md` (ingest flow, mag rules, targets, time display, client staging cache, card dump status, project requirements warning, shared key convention). This file shows layout, focus order and keys only. Sample values (names, sizes, dates, times) are placeholders.

**Display defaults mocked here:** 24-hour clock and DD/MM/YYYY dates (both can be changed in config).

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

## Ingest screen

### Ready, card selected, set pending sync, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ Frame rate   25 fps                                          ││ Not a hard stop. Logged to the project.          │
║   /media/magvault/RED_256      ║│ Resolution   4K  (project requires HD)                       ││                                                  │
║   ● dumped · not yet safe      ║│                                                              │└──────────────────────────────────────────────────┘
║                                ║│ Dump status  ● not dumped                                    │┌─ 4 Details ──────────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│                                                              ││ Camera     Sony A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ One frame rate and one resolution for the whole card. Shows  ││ Format     ProRes RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ "mixed" when clips differ. No file list.                     ││ Gamma      A7SIII S-Log3                      ▾  │
║   ● not dumped                 ║│                                                              ││ Gamut      Sony S-Gamut3.Cine                 ▾  │
║                                ║│                                                              ││ Shoot day  27/11/2026  guessed                   │
║                                ║│                                                              ││ Notes      (optional)                            │
║                                ║│                                                              ││                                                  │
║                                ║│                                                              ││ Confirm before pull                              │
║                                ║│                                                              ││ ✓ camera  ✓ gamma/gamut                          │
║ r rescan    ↑↓ item            ║│                                                              ││ ✓ project  ✓ shoot day  ✓ target                 │
╚════════════════════════════════╝└──────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
┌─ Ingest ───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                                                   │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                                                         │
│ [x] Server          server:/mags            reachable                                                                                              │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                                                           │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                                                │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ←→ pane · ↑↓ item · Enter pick · n new project · r rescan · p PULL · o prepare · c clean · F1 help · Esc dashboard
```

### Pulling, inputs locked, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
┌─ 1 Detected ───────────────────┐┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
│                                ││                                                              ││ 28/11/2026  01:12:07                             │
│   Sony A7S III        128 GB   ││ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
│   /dev/sdb1 · exFAT            ││              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
│   /media/magvault/A7SIII_128   ││                                                              ││ Project    ACME_Campaign_2025                 ▾  │
│   ● 2 copies · on server       ││ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
│                                ││ Clips        6                                               ││ Camera footage does not meet project             │
│   RED Komodo          256 GB   ││ Total size   42.7 GB                                         ││ requirements.                                    │
│   /dev/sdc1 · exFAT            ││ Frame rate   25 fps                                          ││ Not a hard stop. Logged to the project.          │
│   /media/magvault/RED_256      ││ Resolution   4K  (project requires HD)                       ││                                                  │
│   ● dumped · not yet safe      ││                                                              │└──────────────────────────────────────────────────┘
│                                ││ Dump status  ● not dumped                                    │┌─ 4 Details ──────────────────────────────────────┐
│ ▶ Generic DCIM         64 GB   ││                                                              ││ Camera     Sony A7S III                       ▾  │
│   /dev/sdd1 · FAT32            ││ One frame rate and one resolution for the whole card. Shows  ││ Format     ProRes RAW                         ▾  │
│   /media/magvault/GENERIC_64   ││ "mixed" when clips differ. No file list.                     ││ Gamma      A7SIII S-Log3                      ▾  │
│   ● not dumped                 ││                                                              ││ Gamut      Sony S-Gamut3.Cine                 ▾  │
│                                ││                                                              ││ Shoot day  27/11/2026  guessed                   │
│                                ││                                                              ││ Notes      (optional)                            │
│                                ││                                                              ││                                                  │
│                                ││                                                              ││ Confirm before pull                              │
│                                ││                                                              ││ ✓ camera  ✓ gamma/gamut                          │
│ r rescan    ↑↓ item            ││                                                              ││ ✓ project  ✓ shoot day  ✓ target                 │
└────────────────────────────────┘└──────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
╔═ Ingesting ════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║  x ABORT   ███████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  18.2 GB / 42.7 GB  (43%)          ║
║ ● rsync copy   ○ hash + verify  ○ magcard + MHL  ○ database  ○ queue 320p                                                                          ║
╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
┌─ 5 Targets ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                                                         │
│ [x] Server          server:/mags            queued · waits for staging                                                                             │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                                                           │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                                                │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:14:20  pull started    Generic DCIM → local staging                                                                                             │
│ 01:14:20  hashing         SHA-256 after rsync finishes                                                                                             │
│ 01:14:21  next            hash + verify, magcard.txt + ascmhl/, database, 320p queue                                                               │
│ 01:14:21  locked          details and project cannot change until commit                                                                           │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 x abort ingest  ·  l log  ·  inputs locked until verified  ·  Esc blocked while copying
```

### Ready, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                            ║│ Shoot date   27/11/2026  11:30 – 15:43         ││ Requires   25 fps · HD               │
║   RED Komodo      256 GB   ║│ Clips        6                                 ││ Camera footage does not meet project │
║   /dev/sdc1 · exFAT        ║│ Total size   42.7 GB                           ││ requirements.                        │
║   ● dumped · not yet safe  ║│ Frame rate   25 fps                            ││ Not a hard stop. Logged to the       │
║                            ║│ Resolution   4K  (project: HD)                 ││ project.                             │
║ ▶ Generic DCIM     64 GB   ║│                                                │└──────────────────────────────────────┘
║   /dev/sdd1 · FAT32        ║│ Dump status  ● not dumped                      │┌─ 4 Details ──────────────────────────┐
║   ● not dumped             ║│                                                ││ Camera     Sony A7S III           ▾  │
║                            ║│ One frame rate and one resolution for the      ││ Format     ProRes RAW             ▾  │
║                            ║│ whole card. Shows "mixed" when clips differ.   ││ Gamma      A7SIII S-Log3          ▾  │
║                            ║│ No file list.                                  ││ Gamut      Sony S-Gamut3.Cine     ▾  │
║                            ║│                                                ││ Shoot day  27/11/2026  guessed       │
║                            ║│                                                ││ Notes      (optional)                │
║                            ║│                                                ││                                      │
║                            ║│                                                ││ Confirm before pull                  │
║                            ║│                                                ││ ✓ camera  ✓ gamma/gamut              │
║ r rescan    ↑↓ item        ║│                                                ││ ✓ project  ✓ shoot day  ✓ target     │
╚════════════════════════════╝└────────────────────────────────────────────────┘└──────────────────────────────────────┘
┌─ Ingest ─────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                           │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                             │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ←→ pane · ↑↓ item · Enter pick · n new project · r rescan · p PULL · o prepare · c clean · F1 help · Esc dashboard
```

## What changed in this revision

- **Time.** Everything is 24-hour and DD/MM/YYYY. A **System time** block (top right) shows the wall clock. Block 2 shows the shoot date and the clip time range (`27/11/2026  11:30 – 15:43`) instead of a file list.
- **Block 2 (Card)** has no file list. It shows: shoot date and clip time range, number of clips, total size, frame rate for the card (one value, "mixed" if clips differ) and resolution for the card (same rule).
- **Card detect (pane 1)** shows a dump status dot per card: red = not dumped (no Magvault copy), yellow = dumped but not yet two copies and uploaded to the server, green = two copies and uploaded. Status comes from hashes / mag identity, not the volume label.
- **Staging.** The Local staging row has a space bar (used / free). A note says staging auto-cleans once two backups exist, can be disabled, and is never cleaned if it is the only copy.
- **Requirements.** If the project's fixed requirements do not match the card, pane 3 shows "Camera footage does not meet project requirements." It is not a hard stop: the operator can still confirm and pull, and the mismatch is logged to the project. With no requirements set, no check is done.
- **Keys** follow the shared convention: Left / Right between panes, Up / Down for the item, F1 help, Esc dashboard. There is no F2.

## Field backup sets and the Prepare popup

Field USB only; it has nothing to do with zmag primary/backup roles. A field backup is a matched set (master + slave, set size = the smaller drive). The set is the target and is **one row** in Targets: `USB set`, label, `master + slave`, set size, valid, pending / synced (or empty after a clean). Individual drives are not selectable. A drive that is not valid shows `USB drive · not valid · [o Prepare]`.

**Prepare popup** (`o` on a not-valid drive; drawn over the dimmed ingest screen):

1. Warning: "Mount the smaller drive first and prepare it as master."
2. Mounted drives with sizes. The operator picks the master.
3. Master size confirmed, sudo password asked, then format ext4 and write `magvault-backup.txt`.
4. Slave step: the drive list again. A smaller drive is refused.
5. A larger slave: "Will use X GB to match master. Rest unused." Confirm, sudo, then limit to the master size and write `magvault-backup.txt`.
6. Esc cancels. Nothing is written until sudo is accepted.

### Step 1 and 2: warning, mounted drives, pick the master, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════════╗p. Logged to the project.          │
║   /media/magvault/RED_256      ║│ ║                                                                            ║                                   │
║   ● dumped · not yet safe      ║│ ║  Mount the smaller drive first and prepare it as master.                   ║───────────────────────────────────┘
║                                ║│ ║                                                                            ║───────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│ ║  Mounted drives                                                            ║y A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ ║  ▶ NEW_64      /dev/sde       64 GB                                        ║Res RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ ║    NEW_128     /dev/sdf      128 GB                                        ║III S-Log3                      ▾  │
║   ● not dumped                 ║│ ║    NEW_32      /dev/sdg       32 GB                                        ║y S-Gamut3.Cine                 ▾  │
║                                ║│ ║                                                                            ║11/2026  guessed                   │
║                                ║│ ║  Enter  pick as master                                                     ║tional)                            │
║                                ║│ ║                                                                            ║                                   │
║                                ║│ ║  Pick the master first. Its size becomes the set size.                     ║ pull                              │
║                                ║│ ║  Nothing is written until the sudo password is accepted.                   ║mma/gamut                          │
║ r rescan    ↑↓ item            ║│ ║                                                                            ║hoot day  ✓ target                 │
╚════════════════════════════════╝└─║                                                                            ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║                                                                            ║───────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░║                                                                            ║ 0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 3: master size confirmed, sudo prompt, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════════╗p. Logged to the project.          │
║   /media/magvault/RED_256      ║│ ║                                                                            ║                                   │
║   ● dumped · not yet safe      ║│ ║  Mount the smaller drive first and prepare it as master.                   ║───────────────────────────────────┘
║                                ║│ ║                                                                            ║───────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│ ║  Mounted drives                                                            ║y A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ ║  ▶ NEW_64      /dev/sde       64 GB                                        ║Res RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ ║    NEW_128     /dev/sdf      128 GB                                        ║III S-Log3                      ▾  │
║   ● not dumped                 ║│ ║    NEW_32      /dev/sdg       32 GB                                        ║y S-Gamut3.Cine                 ▾  │
║                                ║│ ║                                                                            ║11/2026  guessed                   │
║                                ║│ ║  Master   NEW_64 · /dev/sde · 64 GB                                        ║tional)                            │
║                                ║│ ║  Set size 64 GB                                                            ║                                   │
║                                ║│ ║  [ Enter  Confirm size ]                                                   ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║ r rescan    ↑↓ item            ║│ ║  Sudo password  ••••••••▏                                                  ║hoot day  ✓ target                 │
╚════════════════════════════════╝└─║  Then: format ext4, write magvault-backup.txt (master).                    ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║  Nothing is written until the password is accepted.                        ║───────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░║                                                                            ║ 0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4 and 5: slave is larger, "Will use 64 GB to match master. Rest unused.", 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════════╗p. Logged to the project.          │
║   /media/magvault/RED_256      ║│ ║                                                                            ║                                   │
║   ● dumped · not yet safe      ║│ ║  The slave must be the same size or larger than the master (64 GB).        ║───────────────────────────────────┘
║                                ║│ ║                                                                            ║───────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│ ║  Mounted drives                                                            ║y A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ ║    NEW_64      /dev/sde       64 GB   master of this set                   ║Res RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ ║  ▶ NEW_128     /dev/sdf      128 GB                                        ║III S-Log3                      ▾  │
║   ● not dumped                 ║│ ║    NEW_32      /dev/sdg       32 GB                                        ║y S-Gamut3.Cine                 ▾  │
║                                ║│ ║                                                                            ║11/2026  guessed                   │
║                                ║│ ║  Slave    NEW_128 · /dev/sdf · 128 GB                                      ║tional)                            │
║                                ║│ ║  Will use 64 GB to match master. Rest unused.                              ║                                   │
║                                ║│ ║  [ Enter  Confirm ]                                                        ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║ r rescan    ↑↓ item            ║│ ║  Sudo password  ••••••••▏                                                  ║hoot day  ✓ target                 │
╚════════════════════════════════╝└─║  Then: limit to 64 GB, write magvault-backup.txt (slave).                  ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║  Nothing is written until the password is accepted.                        ║───────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░║                                                                            ║ 0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4: slave is smaller, refused, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════════╗p. Logged to the project.          │
║   /media/magvault/RED_256      ║│ ║                                                                            ║                                   │
║   ● dumped · not yet safe      ║│ ║  The slave must be the same size or larger than the master (64 GB).        ║───────────────────────────────────┘
║                                ║│ ║                                                                            ║───────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│ ║  Mounted drives                                                            ║y A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ ║    NEW_64      /dev/sde       64 GB   master of this set                   ║Res RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ ║    NEW_128     /dev/sdf      128 GB                                        ║III S-Log3                      ▾  │
║   ● not dumped                 ║│ ║  ▶ NEW_32      /dev/sdg       32 GB                                        ║y S-Gamut3.Cine                 ▾  │
║                                ║│ ║                                                                            ║11/2026  guessed                   │
║                                ║│ ║  Slave    NEW_32 · /dev/sdg · 32 GB                                        ║tional)                            │
║                                ║│ ║  ✖ Refused: this drive (32 GB) is smaller than the master (64 GB).         ║                                   │
║                                ║│ ║    A slave must be the same size or larger.                                ║ pull                              │
║                                ║│ ║    To use this drive, start a new set with it as master.                   ║mma/gamut                          │
║ r rescan    ↑↓ item            ║│ ║                                                                            ║hoot day  ✓ target                 │
╚════════════════════════════════╝└─║  Pick a larger drive, or Esc to cancel.                                    ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║                                                                            ║───────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░║                                                                            ║ 0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 1 and 2: warning, mounted drives, pick the master, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                      ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════╗ps · HD               │
║   RED Komodo      256║                                                                        ║does not meet project │
║   /dev/sdc1 · exFAT  ║  Mount the smaller drive first and prepare it as master.               ║                      │
║   ● dumped · not yet ║                                                                        ║. Logged to the       │
║                      ║  Mounted drives                                                        ║                      │
║ ▶ Generic DCIM     64║  ▶ NEW_64      /dev/sde       64 GB                                    ║──────────────────────┘
║   /dev/sdd1 · FAT32  ║    NEW_128     /dev/sdf      128 GB                                    ║──────────────────────┐
║   ● not dumped       ║    NEW_32      /dev/sdg       32 GB                                    ║ A7S III           ▾  │
║                      ║                                                                        ║es RAW             ▾  │
║                      ║  Enter  pick as master                                                 ║II S-Log3          ▾  │
║                      ║                                                                        ║ S-Gamut3.Cine     ▾  │
║                      ║  Pick the master first. Its size becomes the set size.                 ║1/2026  guessed       │
║                      ║  Nothing is written until the sudo password is accepted.               ║ional)                │
║                      ║                                                                        ║                      │
║                      ║                                                                        ║pull                  │
║                      ║                                                                        ║ma/gamut              │
║ r rescan    ↑↓ item  ║                                                                        ║oot day  ✓ target     │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│  p  PULL   ░░░░░░░░░░║                                                                        ║7 GB  (0%)            │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                             │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 3: master size confirmed, sudo prompt, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                      ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════╗ps · HD               │
║   RED Komodo      256║                                                                        ║does not meet project │
║   /dev/sdc1 · exFAT  ║  Mount the smaller drive first and prepare it as master.               ║                      │
║   ● dumped · not yet ║                                                                        ║. Logged to the       │
║                      ║  Mounted drives                                                        ║                      │
║ ▶ Generic DCIM     64║  ▶ NEW_64      /dev/sde       64 GB                                    ║──────────────────────┘
║   /dev/sdd1 · FAT32  ║    NEW_128     /dev/sdf      128 GB                                    ║──────────────────────┐
║   ● not dumped       ║    NEW_32      /dev/sdg       32 GB                                    ║ A7S III           ▾  │
║                      ║                                                                        ║es RAW             ▾  │
║                      ║  Master   NEW_64 · /dev/sde · 64 GB                                    ║II S-Log3          ▾  │
║                      ║  Set size 64 GB                                                        ║ S-Gamut3.Cine     ▾  │
║                      ║  [ Enter  Confirm size ]                                               ║1/2026  guessed       │
║                      ║                                                                        ║ional)                │
║                      ║  Sudo password  ••••••••▏                                              ║                      │
║                      ║  Then: format ext4, write magvault-backup.txt (master).                ║pull                  │
║                      ║  Nothing is written until the password is accepted.                    ║ma/gamut              │
║ r rescan    ↑↓ item  ║                                                                        ║oot day  ✓ target     │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│  p  PULL   ░░░░░░░░░░║                                                                        ║7 GB  (0%)            │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                             │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4 and 5: slave is larger, "Will use 64 GB to match master. Rest unused.", 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                      ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════╗ps · HD               │
║   RED Komodo      256║                                                                        ║does not meet project │
║   /dev/sdc1 · exFAT  ║  The slave must be the same size or larger than the master (64 GB).    ║                      │
║   ● dumped · not yet ║                                                                        ║. Logged to the       │
║                      ║  Mounted drives                                                        ║                      │
║ ▶ Generic DCIM     64║    NEW_64      /dev/sde       64 GB   master of this set               ║──────────────────────┘
║   /dev/sdd1 · FAT32  ║  ▶ NEW_128     /dev/sdf      128 GB                                    ║──────────────────────┐
║   ● not dumped       ║    NEW_32      /dev/sdg       32 GB                                    ║ A7S III           ▾  │
║                      ║                                                                        ║es RAW             ▾  │
║                      ║  Slave    NEW_128 · /dev/sdf · 128 GB                                  ║II S-Log3          ▾  │
║                      ║  Will use 64 GB to match master. Rest unused.                          ║ S-Gamut3.Cine     ▾  │
║                      ║  [ Enter  Confirm ]                                                    ║1/2026  guessed       │
║                      ║                                                                        ║ional)                │
║                      ║  Sudo password  ••••••••▏                                              ║                      │
║                      ║  Then: limit to 64 GB, write magvault-backup.txt (slave).              ║pull                  │
║                      ║  Nothing is written until the password is accepted.                    ║ma/gamut              │
║ r rescan    ↑↓ item  ║                                                                        ║oot day  ✓ target     │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│  p  PULL   ░░░░░░░░░░║                                                                        ║7 GB  (0%)            │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                             │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4: slave is smaller, refused, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                      ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════╗ps · HD               │
║   RED Komodo      256║                                                                        ║does not meet project │
║   /dev/sdc1 · exFAT  ║  The slave must be the same size or larger than the master (64 GB).    ║                      │
║   ● dumped · not yet ║                                                                        ║. Logged to the       │
║                      ║  Mounted drives                                                        ║                      │
║ ▶ Generic DCIM     64║    NEW_64      /dev/sde       64 GB   master of this set               ║──────────────────────┘
║   /dev/sdd1 · FAT32  ║    NEW_128     /dev/sdf      128 GB                                    ║──────────────────────┐
║   ● not dumped       ║  ▶ NEW_32      /dev/sdg       32 GB                                    ║ A7S III           ▾  │
║                      ║                                                                        ║es RAW             ▾  │
║                      ║  Slave    NEW_32 · /dev/sdg · 32 GB                                    ║II S-Log3          ▾  │
║                      ║  ✖ Refused: this drive (32 GB) is smaller than the master (64 GB).     ║ S-Gamut3.Cine     ▾  │
║                      ║    A slave must be the same size or larger.                            ║1/2026  guessed       │
║                      ║    To use this drive, start a new set with it as master.               ║ional)                │
║                      ║                                                                        ║                      │
║                      ║  Pick a larger drive, or Esc to cancel.                                ║pull                  │
║                      ║                                                                        ║ma/gamut              │
║ r rescan    ↑↓ item  ║                                                                        ║oot day  ✓ target     │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│  p  PULL   ░░░░░░░░░░║                                                                        ║7 GB  (0%)            │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                             │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

## Clean action for field backup sets

- **Offered only on a fully synced set.** The set row shows `[c Clean]`. A set with any pending mag does not offer Clean (see the pending row on the main screen).
- **Clean popup** (`c`): "All mags on FIELD-A are on the server. Wipe master and slave?" Sudo password required; nothing is deleted until it is accepted. Esc cancels.
- **What it does:** deletes the mag folders on both drives. Keeps `magvault-backup.txt`, the set id, the size limit and the master / slave roles. Logged.
- **After clean** the set is empty, still valid and ready for new mags. It does not become "not valid" and is not re-prepared. Prepare is only for a new set.

### Set fully synced: Clean offered, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ Frame rate   25 fps                                          ││ Not a hard stop. Logged to the project.          │
║   /media/magvault/RED_256      ║│ Resolution   4K  (project requires HD)                       ││                                                  │
║   ● dumped · not yet safe      ║│                                                              │└──────────────────────────────────────────────────┘
║                                ║│ Dump status  ● not dumped                                    │┌─ 4 Details ──────────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│                                                              ││ Camera     Sony A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ One frame rate and one resolution for the whole card. Shows  ││ Format     ProRes RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ "mixed" when clips differ. No file list.                     ││ Gamma      A7SIII S-Log3                      ▾  │
║   ● not dumped                 ║│                                                              ││ Gamut      Sony S-Gamut3.Cine                 ▾  │
║                                ║│                                                              ││ Shoot day  27/11/2026  guessed                   │
║                                ║│                                                              ││ Notes      (optional)                            │
║                                ║│                                                              ││                                                  │
║                                ║│                                                              ││ Confirm before pull                              │
║                                ║│                                                              ││ ✓ camera  ✓ gamma/gamut                          │
║ r rescan    ↑↓ item            ║│                                                              ││ ✓ project  ✓ shoot day  ✓ target                 │
╚════════════════════════════════╝└──────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
┌─ Ingest ───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                                                   │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                                                         │
│ [x] Server          server:/mags            reachable                                                                                              │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · synced [c Clean]                                                                       │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                                                │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ←→ pane · ↑↓ item · Enter pick · n new project · r rescan · p PULL · o prepare · c clean · F1 help · Esc dashboard
```

### Clean popup, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ Frame rate   25 fps                                          ││ Not a hard stop. Logged to the project.          │
║   /media/magvault/RED_256      ║│ Resolution   4K  (project requires HD)                       ││                                                  │
║   ● dumped · not yet safe      ║│                                                              │└──────────────────────────────────────────────────┘
║                                ║│ ╔═ Clean field backup set · FIELD-A ═════════════════════════════════════════╗───────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│ ║                                                                            ║y A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ ║  All mags on FIELD-A are on the server. Wipe master and slave?             ║Res RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ ║                                                                            ║III S-Log3                      ▾  │
║   ● not dumped                 ║│ ║  Deletes the mag folders on both drives.                                   ║y S-Gamut3.Cine                 ▾  │
║                                ║│ ║  Keeps the set: magvault-backup.txt, set id, size limit, and               ║11/2026  guessed                   │
║                                ║│ ║  the master / slave roles. No re-prepare.                                  ║tional)                            │
║                                ║│ ║  After clean the set is empty, still valid, ready for new mags.            ║                                   │
║                                ║│ ║  This action is logged.                                                    ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║ r rescan    ↑↓ item            ║│ ║  Sudo password  ••••••••▏                                                  ║hoot day  ✓ target                 │
╚════════════════════════════════╝└─║                                                                            ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║  [ Enter  Wipe master and slave ]                                          ║───────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░║                                                                            ║ 0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║  Esc  Cancel · nothing is deleted until sudo is accepted                   ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ [x] Server          server:/mags            reachable                                                                                              │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · synced [c Clean]                                                                       │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                                                │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Enter confirm · Esc cancel (back to ingest)
```

### After clean: set empty, still valid, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ Frame rate   25 fps                                          ││ Not a hard stop. Logged to the project.          │
║   /media/magvault/RED_256      ║│ Resolution   4K  (project requires HD)                       ││                                                  │
║   ● dumped · not yet safe      ║│                                                              │└──────────────────────────────────────────────────┘
║                                ║│ Dump status  ● not dumped                                    │┌─ 4 Details ──────────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│                                                              ││ Camera     Sony A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ One frame rate and one resolution for the whole card. Shows  ││ Format     ProRes RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ "mixed" when clips differ. No file list.                     ││ Gamma      A7SIII S-Log3                      ▾  │
║   ● not dumped                 ║│                                                              ││ Gamut      Sony S-Gamut3.Cine                 ▾  │
║                                ║│                                                              ││ Shoot day  27/11/2026  guessed                   │
║                                ║│                                                              ││ Notes      (optional)                            │
║                                ║│                                                              ││                                                  │
║                                ║│                                                              ││ Confirm before pull                              │
║                                ║│                                                              ││ ✓ camera  ✓ gamma/gamut                          │
║ r rescan    ↑↓ item            ║│                                                              ││ ✓ project  ✓ shoot day  ✓ target                 │
╚════════════════════════════════╝└──────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
┌─ Ingest ───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                                                   │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                                                         │
│ [x] Server          server:/mags            reachable                                                                                              │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · empty                                                                                  │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                                                │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ←→ pane · ↑↓ item · Enter pick · n new project · r rescan · p PULL · o prepare · c clean · F1 help · Esc dashboard
```

### Set fully synced: Clean offered, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                            ║│ Shoot date   27/11/2026  11:30 – 15:43         ││ Requires   25 fps · HD               │
║   RED Komodo      256 GB   ║│ Clips        6                                 ││ Camera footage does not meet project │
║   /dev/sdc1 · exFAT        ║│ Total size   42.7 GB                           ││ requirements.                        │
║   ● dumped · not yet safe  ║│ Frame rate   25 fps                            ││ Not a hard stop. Logged to the       │
║                            ║│ Resolution   4K  (project: HD)                 ││ project.                             │
║ ▶ Generic DCIM     64 GB   ║│                                                │└──────────────────────────────────────┘
║   /dev/sdd1 · FAT32        ║│ Dump status  ● not dumped                      │┌─ 4 Details ──────────────────────────┐
║   ● not dumped             ║│                                                ││ Camera     Sony A7S III           ▾  │
║                            ║│ One frame rate and one resolution for the      ││ Format     ProRes RAW             ▾  │
║                            ║│ whole card. Shows "mixed" when clips differ.   ││ Gamma      A7SIII S-Log3          ▾  │
║                            ║│ No file list.                                  ││ Gamut      Sony S-Gamut3.Cine     ▾  │
║                            ║│                                                ││ Shoot day  27/11/2026  guessed       │
║                            ║│                                                ││ Notes      (optional)                │
║                            ║│                                                ││                                      │
║                            ║│                                                ││ Confirm before pull                  │
║                            ║│                                                ││ ✓ camera  ✓ gamma/gamut              │
║ r rescan    ↑↓ item        ║│                                                ││ ✓ project  ✓ shoot day  ✓ target     │
╚════════════════════════════╝└────────────────────────────────────────────────┘└──────────────────────────────────────┘
┌─ Ingest ─────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                           │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · synced [c Clean]                                         │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ←→ pane · ↑↓ item · Enter pick · n new project · r rescan · p PULL · o prepare · c clean · F1 help · Esc dashboard
```

### Clean popup, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                            ║│ Shoot date   27/11/2026  11:30 – 15:43         ││ Requires   25 fps · HD               │
║   RED Komodo      256 GB   ║│ Clips        6                                 ││ Camera footage does not meet project │
║   /dev/sdc1 · exFAT        ║│ Total size   42.7 GB                           ││ requirements.                        │
║   ● dumped · not yet ╔═ Clean field backup set · FIELD-A ═════════════════════════════════════╗. Logged to the       │
║                      ║                                                                        ║                      │
║ ▶ Generic DCIM     64║  All mags on FIELD-A are on the server. Wipe master and slave?         ║──────────────────────┘
║   /dev/sdd1 · FAT32  ║                                                                        ║──────────────────────┐
║   ● not dumped       ║  Deletes the mag folders on both drives.                               ║ A7S III           ▾  │
║                      ║  Keeps the set: magvault-backup.txt, set id, size limit, and           ║es RAW             ▾  │
║                      ║  the master / slave roles. No re-prepare.                              ║II S-Log3          ▾  │
║                      ║  After clean the set is empty, still valid, ready for new mags.        ║ S-Gamut3.Cine     ▾  │
║                      ║  This action is logged.                                                ║1/2026  guessed       │
║                      ║                                                                        ║ional)                │
║                      ║  Sudo password  ••••••••▏                                              ║                      │
║                      ║                                                                        ║pull                  │
║                      ║  [ Enter  Wipe master and slave ]                                      ║ma/gamut              │
║ r rescan    ↑↓ item  ║                                                                        ║oot day  ✓ target     │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│  p  PULL   ░░░░░░░░░░║  Esc  Cancel · nothing is deleted until sudo is accepted               ║7 GB  (0%)            │
│ → mags/20250520_<mag-╚════════════════════════════════════════════════════════════════════════╝                      │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                           │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · synced [c Clean]                                         │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Enter confirm · Esc cancel (back to ingest)
```

### After clean: set empty, still valid, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                            ║│ Shoot date   27/11/2026  11:30 – 15:43         ││ Requires   25 fps · HD               │
║   RED Komodo      256 GB   ║│ Clips        6                                 ││ Camera footage does not meet project │
║   /dev/sdc1 · exFAT        ║│ Total size   42.7 GB                           ││ requirements.                        │
║   ● dumped · not yet safe  ║│ Frame rate   25 fps                            ││ Not a hard stop. Logged to the       │
║                            ║│ Resolution   4K  (project: HD)                 ││ project.                             │
║ ▶ Generic DCIM     64 GB   ║│                                                │└──────────────────────────────────────┘
║   /dev/sdd1 · FAT32        ║│ Dump status  ● not dumped                      │┌─ 4 Details ──────────────────────────┐
║   ● not dumped             ║│                                                ││ Camera     Sony A7S III           ▾  │
║                            ║│ One frame rate and one resolution for the      ││ Format     ProRes RAW             ▾  │
║                            ║│ whole card. Shows "mixed" when clips differ.   ││ Gamma      A7SIII S-Log3          ▾  │
║                            ║│ No file list.                                  ││ Gamut      Sony S-Gamut3.Cine     ▾  │
║                            ║│                                                ││ Shoot day  27/11/2026  guessed       │
║                            ║│                                                ││ Notes      (optional)                │
║                            ║│                                                ││                                      │
║                            ║│                                                ││ Confirm before pull                  │
║                            ║│                                                ││ ✓ camera  ✓ gamma/gamut              │
║ r rescan    ↑↓ item        ║│                                                ││ ✓ project  ✓ shoot day  ✓ target     │
╚════════════════════════════╝└────────────────────────────────────────────────┘└──────────────────────────────────────┘
┌─ Ingest ─────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                           │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · empty                                                    │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ←→ pane · ↑↓ item · Enter pick · n new project · r rescan · p PULL · o prepare · c clean · F1 help · Esc dashboard
```

## Help (F1)

F1 opens help for the current screen. F1 or Esc closes it. F2 is deferred and not drawn.

### Ingest help, 150x42

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                                                    ● server connected  ·  odofad
╔═ 1 Detected ═══════════════════╗┌─ 2 Card ─────────────────────────────────────────────────────┐┌─ System time ────────────────────────────────────┐
║                                ║│                                                              ││ 28/11/2026  01:12:07                             │
║   Sony A7S III        128 GB   ║│ Card         Generic DCIM · 64 GB                            │└──────────────────────────────────────────────────┘
║   /dev/sdb1 · exFAT            ║│              /media/magvault/GENERIC_64                      │┌─ 3 Project ──────────────────────────────────────┐
║   /media/magvault/A7SIII_128   ║│                                                              ││ Project    ACME_Campaign_2025                 ▾  │
║   ● 2 copies · on server       ║│ Shoot date   27/11/2026  11:30 – 15:43                       ││ Requires   25 fps · HD                           │
║                                ║│ Clips        6                                               ││ Camera footage does not meet project             │
║   RED Komodo          256 GB   ║│ Total size   42.7 GB                                         ││ requirements.                                    │
║   /dev/sdc1 · exFAT            ║│ Frame rate   25 fps                                          ││ Not a hard stop. Logged to the project.          │
║   /media/magvault/RED_256      ║│ Resolution   4K  (project requires HD)                       ││                                                  │
║   ● dumped · not yet safe      ║│                                                              │└──────────────────────────────────────────────────┘
║                                ║│ ╔═ Help · Ingest ════════════════════════════════════════════════════════════╗───────────────────────────────────┐
║ ▶ Generic DCIM         64 GB   ║│ ║                                                                            ║y A7S III                       ▾  │
║   /dev/sdd1 · FAT32            ║│ ║  ←  →      move between panes (same as Tab)                                ║Res RAW                         ▾  │
║   /media/magvault/GENERIC_64   ║│ ║  ↑  ↓      select the item in the pane                                     ║III S-Log3                      ▾  │
║   ● not dumped                 ║│ ║  Enter     confirm / pick                                                  ║y S-Gamut3.Cine                 ▾  │
║                                ║│ ║  n         new project           r   rescan cards                          ║11/2026  guessed                   │
║                                ║│ ║  p         pull (start verified copy)                                      ║tional)                            │
║                                ║│ ║  x         abort (while pulling only)                                      ║                                   │
║                                ║│ ║  o         prepare a not-valid USB drive                                   ║ pull                              │
║                                ║│ ║  c         clean a fully synced set                                        ║mma/gamut                          │
║ r rescan    ↑↓ item            ║│ ║  l         show log (overlay at 120x36)                                    ║hoot day  ✓ target                 │
╚════════════════════════════════╝└─║  F1        this help             Esc   dashboard (not while copying)       ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║                                                                            ║───────────────────────────────────┐
│  p  PULL   ░░░░░░░░░░░░░░░░░░░░░░░║                                                                            ║ 0.00 B / 42.7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║  F1 or Esc  Close help                                                     ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────╚════════════════════════════════════════════════════════════════════════════╝───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                                                         │
│ [x] Server          server:/mags            reachable                                                                                              │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                                                           │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                                                │
│ Staging auto-cleans once two backups exist. Can be disabled. Never cleaned if it is the only copy.                                                 │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 01:12:04  card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                │
│ 01:12:05  probe done      6 clips · 42.7 GB · 27/11/2026  11:30 – 15:43                                                                            │
│ 01:12:05  guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3                                                                                │
│ 01:12:06  waiting         confirm details, then p to pull                                                                                          │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 F1 or Esc close help
```

### Ingest help, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Card ───────────────────────────────────────┐┌─ System time ────────────────────────┐
║                            ║│                                                ││ 28/11/2026  01:12:07                 │
║   Sony A7S III    128 GB   ║│ Card         Generic DCIM · 64 GB              │└──────────────────────────────────────┘
║   /dev/sdb1 · exFAT        ║│              /media/magvault/GENERIC_64        │┌─ 3 Project ──────────────────────────┐
║   ● 2 copies · on server   ║│                                                ││ Project    ACME_Campaign_2025     ▾  │
║                            ║│ Shoot date   27/11/2026  11:30 – 15:43         ││ Requires   25 fps · HD               │
║   RED Komodo      256 GB   ║│ Clips        6                                 ││ Camera footage does not meet project │
║   /dev/sdc1 · exFAT        ║│ Total size   42.7 GB                           ││ requirements.                        │
║   ● dumped · not yet ╔═ Help · Ingest ════════════════════════════════════════════════════════╗. Logged to the       │
║                      ║                                                                        ║                      │
║ ▶ Generic DCIM     64║  ←  →      move between panes (same as Tab)                            ║──────────────────────┘
║   /dev/sdd1 · FAT32  ║  ↑  ↓      select the item in the pane                                 ║──────────────────────┐
║   ● not dumped       ║  Enter     confirm / pick                                              ║ A7S III           ▾  │
║                      ║  n         new project           r   rescan cards                      ║es RAW             ▾  │
║                      ║  p         pull (start verified copy)                                  ║II S-Log3          ▾  │
║                      ║  x         abort (while pulling only)                                  ║ S-Gamut3.Cine     ▾  │
║                      ║  o         prepare a not-valid USB drive                               ║1/2026  guessed       │
║                      ║  c         clean a fully synced set                                    ║ional)                │
║                      ║  l         show log (overlay at 120x36)                                ║                      │
║                      ║  F1        this help             Esc   dashboard (not while copying)   ║pull                  │
║                      ║                                                                        ║ma/gamut              │
║ r rescan    ↑↓ item  ║                                                                        ║oot day  ✓ target     │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║  F1 or Esc  Close help                                                 ║──────────────────────┐
│  p  PULL   ░░░░░░░░░░╚════════════════════════════════════════════════════════════════════════╝7 GB  (0%)            │
│ → mags/20250520_<mag-uuid>  originals + magcard.txt + ascmhl/ · SHA-256 verified                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging   ██████░░░░░░░░░░░░░░ 122 GB used / 258 GB free                           │
│ [x] Server          server:/mags            reachable                                                                │
│ [ ] USB set         FIELD-A  master + slave 64 GB · valid · pending sync                                             │
│ [ ] USB drive       /media/odofad/NEW_64    not valid · [o Prepare]                                                  │
│ Staging auto-cleans after two backups (can be disabled).                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 F1 or Esc close help
```

## How the layout flexes

| Part | 120x36 (minimum) | 150x42 (comfortable) |
|------|------------------|----------------------|
| Detected (pane 1) | 30 columns, 3 lines per card (name, device, dump status) | 34 columns, adds the mount path to each card |
| Card (pane 2) | takes all remaining width (50 columns) | takes all remaining width (64 columns) |
| System time, Project, Details (right column) | 40 columns | 52 columns |
| Middle row height | 23 rows | 23 rows (same) |
| Log strip | hidden (`l` opens it as an overlay) | shown, 6 rows |
| Ingest bar, Targets | full width, progress and staging bars scale | full width |

Below 120x36, show a "resize terminal" message instead of a broken layout.

## Panes and focus order

`Left` / `Right` (same as `Tab` / `Shift+Tab`) move between the numbered panes. `Up` / `Down` select the item inside the focused pane. The focused pane uses a double-line border. A mouse click on a pane also focuses it.

| # | Pane | Suggested widget | Notes |
|---|------|------------------|-------|
| - | System time | `Paragraph` | Wall clock, 24-hour, DD/MM/YYYY. Not focusable. |
| 1 | Detected | `List` + `ListState` | Cards with label, size, fs and a red / yellow / green dump status. `r` rescans. Manual path entry stays available. |
| 2 | Card | `Paragraph` | Read-only summary of the probed card: shoot date and clip time range, clips, total size, frame rate, resolution. No file list. |
| 3 | Project | popup `List` over `Clear` | Open projects only. Shows the project's fixed requirements and the mismatch warning. `n` creates a project. |
| 4 | Details | field rows + popup pickers | Camera, format, gamma, gamut from existing profiles only. Shoot day is guessed and confirmed. Notes optional. |
| 5 | Targets | checkbox `List` | Local staging (with used / free space bar), server, one USB backup set row, and any not-valid USB drive with Prepare. Staging auto-clean note. |
| - | Ingest bar | `Gauge` + stage line | `p` starts the pull. Shows destination `mags/<YYYYMMDD>_<mag-uuid>`. |
| - | Log | `List` | Shown only when there are spare rows (40+). 24-hour timestamps. |

## Keys

| Key | Action |
|-----|--------|
| `Left` / `Right` (or `Tab` / `Shift+Tab`) | Previous / next pane |
| `Up` / `Down` | Select the item in the pane |
| `Enter` | Open picker or confirm field |
| `n` | New project |
| `r` | Rescan for cards |
| `p` | Pull (start verified copy; disabled until the confirm checklist is complete) |
| `o` | Open the Prepare popup for a not-valid USB drive |
| `c` | Open the Clean popup for a fully synced set |
| `x` | Abort (pulling state only) |
| `l` | Show log (overlay at 120x36, strip at 150x42) |
| `F1` | Help for the current screen |
| `Esc` | Close a popup; otherwise return to the dashboard (blocked while a copy is writing) |

`q` quits the whole program only on the dashboard, so it is not an ingest key. F2 is deferred.

## Behaviour notes

- **Pull is gated.** `p` only works once camera, format, gamma, gamut, project, shoot day and at least one target are confirmed (the "Confirm before pull" lines in pane 4). A requirements mismatch does not block it.
- **Pipeline stages** match `PLAN.md`: rsync copy, then hash and verify the destination, write `magcard.txt` and ASC MHL, record in database, queue 320p proxies.
- **Unsafe until verified.** The card must stay inserted until hashes match.
- **Inputs lock** during a pull. Panes 1-4 ignore keys.
- **USB validity:** only a valid matched set can be selected as a backup target, and it is selected as one set. Any other drive is "not valid" and offers Prepare.
- **Prepare:** master first (smaller drive), then slave (same size or larger; smaller is refused). A sudo password is asked and nothing is written until it is accepted. Not offered on a set while any mag is pending sync.
- **Clean:** only on a fully synced set; wipes the mag folders on both drives after sudo, keeps the set, leaves it empty and valid.
- **Staging:** auto-clean removes a staged mag once at least two backups exist. It can be disabled. If staging is the only copy it is never cleaned.
- **Pull** is the single action that starts the verified copy into the mag folder. No separate "populate" step.
