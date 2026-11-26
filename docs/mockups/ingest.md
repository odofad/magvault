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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                                                    │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [ o Prepare ]                                                                      │
│ [!] Prepare opens a popup and needs sudo. It is not offered on a set while any mag is pending sync.                                                │
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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                                                    │
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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                      │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Prepare: popup, needs sudo. Not offered on a set while any mag is pending sync.                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Enter pick  ·  n new project  ·  r rescan  ·  p PULL  ·  click pane to focus  ·  Esc dashboard
```

## Field backup sets and the Prepare popup

Source of truth is `PLAN.md` ("Client ingest and field backups", "Field backup sets", "Prepare popup"). This is field USB only; it has nothing to do with zmag primary/backup roles.

**Targets pane (5).** A field backup is a matched set (one master, one slave, same mags, set size = the smaller drive). The set is the target: it is one row (`USB set`, label, `master + slave`, set size, valid, pending / synced), and the individual drives are not listed or selectable. A drive that is not a valid set member shows `USB drive · not valid · [o Prepare]`. Prepare opens the popup below; it is never inline. A set with pending mags stays selectable as a target but is not offered Prepare until every mag has synced.

**Prepare popup** (`o` on a not-valid drive, drawn over the ingest screen, which is dimmed behind it):

1. Warning: "Mount the smaller drive first and prepare it as master."
2. Mounted drives with sizes. The operator picks the master.
3. Master size confirmed, sudo password asked, then format ext4 and write `magvault-backup.txt`.
4. Slave step: the drive list again. A smaller drive is refused with the reason.
5. A larger slave: "Will use X GB to match master. Rest unused." Confirm, sudo, then partition or limit to the master size and write `magvault-backup.txt`.
6. Esc cancels. Nothing is written until each sudo password is accepted.

A slave cannot be prepared before its master exists. Esc closes the popup and returns to the ingest screen.

### Step 1 and 2: warning, mounted drives, pick the master, 150x42

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
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════════╗III S-Log3                      ▾  │
║   /media/magvault/RED_256      ║│ ║                                                                            ║y S-Gamut3.Cine                 ▾  │
║   ● ready                      ║│ ║  Mount the smaller drive first and prepare it as master.                   ║5-05-20   guessed                  │
║                                ║│ ║                                                                            ║tional)                            │
║ ▶ Generic DCIM           64 GB ║│ ║  Mounted drives                                                            ║                                   │
║   /dev/sdd1 · FAT32            ║│ ║  ▶ NEW_64      /dev/sde       64 GB                                        ║                                   │
║   /media/magvault/GENERIC_64   ║│ ║    NEW_128     /dev/sdf      128 GB                                        ║                                   │
║   ● selected                   ║│ ║    NEW_32      /dev/sdg       32 GB                                        ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║                                ║│ ║  Enter  pick as master                                                     ║hoot day  ✓ target                 │
║                                ║│ ║                                                                            ║e? Admin only.                     │
║                                ║│ ║  Pick the master first. Its size becomes the set size.                     ║                                   │
║ r rescan    ↑↓ choose          ║│ ║  Nothing is written until the sudo password is accepted.                   ║                                   │
║                                ║│ ║                                                                            ║                                   │
╚════════════════════════════════╝└─║                                                                            ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║                                                                            ║───────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░║                                                                            ║0 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ [!] Prepare opens a popup and needs sudo. It is not offered on a set while any mag is pending sync.                                                │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 3: master size confirmed, sudo prompt, 150x42

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
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════════╗III S-Log3                      ▾  │
║   /media/magvault/RED_256      ║│ ║                                                                            ║y S-Gamut3.Cine                 ▾  │
║   ● ready                      ║│ ║  Mount the smaller drive first and prepare it as master.                   ║5-05-20   guessed                  │
║                                ║│ ║                                                                            ║tional)                            │
║ ▶ Generic DCIM           64 GB ║│ ║  Mounted drives                                                            ║                                   │
║   /dev/sdd1 · FAT32            ║│ ║  ▶ NEW_64      /dev/sde       64 GB                                        ║                                   │
║   /media/magvault/GENERIC_64   ║│ ║    NEW_128     /dev/sdf      128 GB                                        ║                                   │
║   ● selected                   ║│ ║    NEW_32      /dev/sdg       32 GB                                        ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║                                ║│ ║  Master   NEW_64 · /dev/sde · 64 GB                                        ║hoot day  ✓ target                 │
║                                ║│ ║  Set size 64 GB                                                            ║e? Admin only.                     │
║                                ║│ ║  [ Enter  Confirm size ]                                                   ║                                   │
║ r rescan    ↑↓ choose          ║│ ║                                                                            ║                                   │
║                                ║│ ║  Sudo password  ••••••••▏                                                  ║                                   │
╚════════════════════════════════╝└─║  Then: format ext4, write magvault-backup.txt (master).                    ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║  Nothing is written until the password is accepted.                        ║───────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░║                                                                            ║0 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ [!] Prepare opens a popup and needs sudo. It is not offered on a set while any mag is pending sync.                                                │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4 and 5: slave is larger, "Will use 64 GB to match master. Rest unused.", 150x42

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
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════════╗III S-Log3                      ▾  │
║   /media/magvault/RED_256      ║│ ║                                                                            ║y S-Gamut3.Cine                 ▾  │
║   ● ready                      ║│ ║  The slave must be the same size or larger than the master (64 GB).        ║5-05-20   guessed                  │
║                                ║│ ║                                                                            ║tional)                            │
║ ▶ Generic DCIM           64 GB ║│ ║  Mounted drives                                                            ║                                   │
║   /dev/sdd1 · FAT32            ║│ ║    NEW_64      /dev/sde       64 GB   master of this set                   ║                                   │
║   /media/magvault/GENERIC_64   ║│ ║  ▶ NEW_128     /dev/sdf      128 GB                                        ║                                   │
║   ● selected                   ║│ ║    NEW_32      /dev/sdg       32 GB                                        ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║                                ║│ ║  Slave    NEW_128 · /dev/sdf · 128 GB                                      ║hoot day  ✓ target                 │
║                                ║│ ║  Will use 64 GB to match master. Rest unused.                              ║e? Admin only.                     │
║                                ║│ ║  [ Enter  Confirm ]                                                        ║                                   │
║ r rescan    ↑↓ choose          ║│ ║                                                                            ║                                   │
║                                ║│ ║  Sudo password  ••••••••▏                                                  ║                                   │
╚════════════════════════════════╝└─║  Then: limit to 64 GB, write magvault-backup.txt (slave).                  ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║  Nothing is written until the password is accepted.                        ║───────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░║                                                                            ║0 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ [!] Prepare opens a popup and needs sudo. It is not offered on a set while any mag is pending sync.                                                │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4: slave is smaller, refused, 150x42

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
║   /dev/sdc1 · exFAT            ║│ ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════════╗III S-Log3                      ▾  │
║   /media/magvault/RED_256      ║│ ║                                                                            ║y S-Gamut3.Cine                 ▾  │
║   ● ready                      ║│ ║  The slave must be the same size or larger than the master (64 GB).        ║5-05-20   guessed                  │
║                                ║│ ║                                                                            ║tional)                            │
║ ▶ Generic DCIM           64 GB ║│ ║  Mounted drives                                                            ║                                   │
║   /dev/sdd1 · FAT32            ║│ ║    NEW_64      /dev/sde       64 GB   master of this set                   ║                                   │
║   /media/magvault/GENERIC_64   ║│ ║    NEW_128     /dev/sdf      128 GB                                        ║                                   │
║   ● selected                   ║│ ║  ▶ NEW_32      /dev/sdg       32 GB                                        ║ pull                              │
║                                ║│ ║                                                                            ║mma/gamut                          │
║                                ║│ ║  Slave    NEW_32 · /dev/sdg · 32 GB                                        ║hoot day  ✓ target                 │
║                                ║│ ║  ✖ Refused: this drive (32 GB) is smaller than the master (64 GB).         ║e? Admin only.                     │
║                                ║│ ║    A slave must be the same size or larger.                                ║                                   │
║ r rescan    ↑↓ choose          ║│ ║    To use this drive, start a new set with it as master.                   ║                                   │
║                                ║│ ║                                                                            ║                                   │
╚════════════════════════════════╝└─║  Pick a larger drive, or Esc to cancel.                                    ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║                                                                            ║───────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░║                                                                            ║0 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║                                                                            ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-║                                                                            ║                                   │
│ [x] Server          server:/mags  ║                                                                            ║                                   │
│ [ ] USB set         FIELD-A  maste║  Esc  Cancel · nothing is written until sudo is accepted                   ║                                   │
│ [ ] USB drive       /media/odofad/╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ [!] Prepare opens a popup and needs sudo. It is not offered on a set while any mag is pending sync.                                                │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 1 and 2: warning, mounted drives, pick the master, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Files ──────────────────────────────────────┐┌─ 3 Project ──────────────────────────┐
║                            ║│                                                ││ Project  ACME_Campaign_2025     ▾    │
║   Sony A7S III      128 GB ║│ /media/magvault/GENERIC_64                     ││ (open projects only)                 │
║   /dev/sdb1 · exFAT        ║│ ├─ DCIM                                        ││ n new project · UUID on create       │
║   ● ready                  ║│ ├─ MP_ROOT                                     │└──────────────────────────────────────┘
║                      ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════╗──────────────────────┐
║   RED Komodo        2║                                                                        ║ A7S III           ▾  │
║   /dev/sdc1 · exFAT  ║  Mount the smaller drive first and prepare it as master.               ║es RAW             ▾  │
║   ● ready            ║                                                                        ║II S-Log3          ▾  │
║                      ║  Mounted drives                                                        ║ S-Gamut3.Cine     ▾  │
║ ▶ Generic DCIM       ║  ▶ NEW_64      /dev/sde       64 GB                                    ║-05-20   guessed      │
║   /dev/sdd1 · FAT32  ║    NEW_128     /dev/sdf      128 GB                                    ║ional)                │
║   ● selected         ║    NEW_32      /dev/sdg       32 GB                                    ║                      │
║                      ║                                                                        ║                      │
║                      ║  Enter  pick as master                                                 ║pull                  │
║                      ║                                                                        ║ma/gamut              │
║                      ║  Pick the master first. Its size becomes the set size.                 ║oot day  ✓ target     │
║                      ║  Nothing is written until the sudo password is accepted.               ║? Admin only.         │
║                      ║                                                                        ║                      │
║                      ║                                                                        ║                      │
║ r rescan    ↑↓ choose║                                                                        ║                      │
║                      ║                                                                        ║                      │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│ [ p  PULL ]  ░░░░░░░░║                                                                        ║  (0%)                │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags                    reachable                                                        │
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                      │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Prepare: popup, needs sudo. Not offered on a set while any mag is pending sync.                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 3: master size confirmed, sudo prompt, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Files ──────────────────────────────────────┐┌─ 3 Project ──────────────────────────┐
║                            ║│                                                ││ Project  ACME_Campaign_2025     ▾    │
║   Sony A7S III      128 GB ║│ /media/magvault/GENERIC_64                     ││ (open projects only)                 │
║   /dev/sdb1 · exFAT        ║│ ├─ DCIM                                        ││ n new project · UUID on create       │
║   ● ready                  ║│ ├─ MP_ROOT                                     │└──────────────────────────────────────┘
║                      ╔═ Prepare USB drive · step 1 of 2 · master ═════════════════════════════╗──────────────────────┐
║   RED Komodo        2║                                                                        ║ A7S III           ▾  │
║   /dev/sdc1 · exFAT  ║  Mount the smaller drive first and prepare it as master.               ║es RAW             ▾  │
║   ● ready            ║                                                                        ║II S-Log3          ▾  │
║                      ║  Mounted drives                                                        ║ S-Gamut3.Cine     ▾  │
║ ▶ Generic DCIM       ║  ▶ NEW_64      /dev/sde       64 GB                                    ║-05-20   guessed      │
║   /dev/sdd1 · FAT32  ║    NEW_128     /dev/sdf      128 GB                                    ║ional)                │
║   ● selected         ║    NEW_32      /dev/sdg       32 GB                                    ║                      │
║                      ║                                                                        ║                      │
║                      ║  Master   NEW_64 · /dev/sde · 64 GB                                    ║pull                  │
║                      ║  Set size 64 GB                                                        ║ma/gamut              │
║                      ║  [ Enter  Confirm size ]                                               ║oot day  ✓ target     │
║                      ║                                                                        ║? Admin only.         │
║                      ║  Sudo password  ••••••••▏                                              ║                      │
║                      ║  Then: format ext4, write magvault-backup.txt (master).                ║                      │
║ r rescan    ↑↓ choose║  Nothing is written until the password is accepted.                    ║                      │
║                      ║                                                                        ║                      │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│ [ p  PULL ]  ░░░░░░░░║                                                                        ║  (0%)                │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags                    reachable                                                        │
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                      │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Prepare: popup, needs sudo. Not offered on a set while any mag is pending sync.                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4 and 5: slave is larger, "Will use 64 GB to match master. Rest unused.", 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Files ──────────────────────────────────────┐┌─ 3 Project ──────────────────────────┐
║                            ║│                                                ││ Project  ACME_Campaign_2025     ▾    │
║   Sony A7S III      128 GB ║│ /media/magvault/GENERIC_64                     ││ (open projects only)                 │
║   /dev/sdb1 · exFAT        ║│ ├─ DCIM                                        ││ n new project · UUID on create       │
║   ● ready                  ║│ ├─ MP_ROOT                                     │└──────────────────────────────────────┘
║                      ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════╗──────────────────────┐
║   RED Komodo        2║                                                                        ║ A7S III           ▾  │
║   /dev/sdc1 · exFAT  ║  The slave must be the same size or larger than the master (64 GB).    ║es RAW             ▾  │
║   ● ready            ║                                                                        ║II S-Log3          ▾  │
║                      ║  Mounted drives                                                        ║ S-Gamut3.Cine     ▾  │
║ ▶ Generic DCIM       ║    NEW_64      /dev/sde       64 GB   master of this set               ║-05-20   guessed      │
║   /dev/sdd1 · FAT32  ║  ▶ NEW_128     /dev/sdf      128 GB                                    ║ional)                │
║   ● selected         ║    NEW_32      /dev/sdg       32 GB                                    ║                      │
║                      ║                                                                        ║                      │
║                      ║  Slave    NEW_128 · /dev/sdf · 128 GB                                  ║pull                  │
║                      ║  Will use 64 GB to match master. Rest unused.                          ║ma/gamut              │
║                      ║  [ Enter  Confirm ]                                                    ║oot day  ✓ target     │
║                      ║                                                                        ║? Admin only.         │
║                      ║  Sudo password  ••••••••▏                                              ║                      │
║                      ║  Then: limit to 64 GB, write magvault-backup.txt (slave).              ║                      │
║ r rescan    ↑↓ choose║  Nothing is written until the password is accepted.                    ║                      │
║                      ║                                                                        ║                      │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│ [ p  PULL ]  ░░░░░░░░║                                                                        ║  (0%)                │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags                    reachable                                                        │
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                      │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Prepare: popup, needs sudo. Not offered on a set while any mag is pending sync.                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```

### Step 4: slave is smaller, refused, 120x36 (minimum)

```text
 MAGVAULT INGEST  ·  ingest pipeline                                                      ● server connected  ·  odofad
╔═ 1 Detected ═══════════════╗┌─ 2 Files ──────────────────────────────────────┐┌─ 3 Project ──────────────────────────┐
║                            ║│                                                ││ Project  ACME_Campaign_2025     ▾    │
║   Sony A7S III      128 GB ║│ /media/magvault/GENERIC_64                     ││ (open projects only)                 │
║   /dev/sdb1 · exFAT        ║│ ├─ DCIM                                        ││ n new project · UUID on create       │
║   ● ready                  ║│ ├─ MP_ROOT                                     │└──────────────────────────────────────┘
║                      ╔═ Prepare USB drive · step 2 of 2 · slave ══════════════════════════════╗──────────────────────┐
║   RED Komodo        2║                                                                        ║ A7S III           ▾  │
║   /dev/sdc1 · exFAT  ║  The slave must be the same size or larger than the master (64 GB).    ║es RAW             ▾  │
║   ● ready            ║                                                                        ║II S-Log3          ▾  │
║                      ║  Mounted drives                                                        ║ S-Gamut3.Cine     ▾  │
║ ▶ Generic DCIM       ║    NEW_64      /dev/sde       64 GB   master of this set               ║-05-20   guessed      │
║   /dev/sdd1 · FAT32  ║    NEW_128     /dev/sdf      128 GB                                    ║ional)                │
║   ● selected         ║  ▶ NEW_32      /dev/sdg       32 GB                                    ║                      │
║                      ║                                                                        ║                      │
║                      ║  Slave    NEW_32 · /dev/sdg · 32 GB                                    ║pull                  │
║                      ║  ✖ Refused: this drive (32 GB) is smaller than the master (64 GB).     ║ma/gamut              │
║                      ║    A slave must be the same size or larger.                            ║oot day  ✓ target     │
║                      ║    To use this drive, start a new set with it as master.               ║? Admin only.         │
║                      ║                                                                        ║                      │
║                      ║  Pick a larger drive, or Esc to cancel.                                ║                      │
║ r rescan    ↑↓ choose║                                                                        ║                      │
║                      ║                                                                        ║                      │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│ [ p  PULL ]  ░░░░░░░░║                                                                        ║  (0%)                │
│ → mags/20250520_<mag-║                                                                        ║                      │
└──────────────────────║                                                                        ║──────────────────────┘
┌─ 5 Targets ──────────║  Esc  Cancel · nothing is written until sudo is accepted               ║──────────────────────┐
│ [x] Local staging   /╚════════════════════════════════════════════════════════════════════════╝                      │
│ [x] Server          server:/mags                    reachable                                                        │
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · pending sync · not selected                      │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Prepare: popup, needs sudo. Not offered on a set while any mag is pending sync.                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ choose drive · Enter confirm · Esc cancel (back to ingest)
```


## Clean action for field backup sets

Source of truth is `PLAN.md` ("Clean" under "USB drive diagnostics and prepare"). Field USB only; not related to zmag roles.

- **Offered only on a fully synced set.** When every mag on the set is synced (no pending), the set row shows `[c Clean]`. A set with any pending mag does not offer Clean (see the pending row in the Prepare section above).
- **Clean popup** (`c` on a fully synced set): "All mags on FIELD-A are on the server. Wipe master and slave?" Sudo password is required and nothing is deleted until it is accepted. Esc cancels.
- **What it does:** deletes the mag folders on both drives. It keeps `magvault-backup.txt`, the set id, the size limit and the master / slave roles.
- **After clean** the set is empty, still valid and ready for new mags. It does not become "not valid" and is not re-prepared. Prepare is only for a new set (different drives, different size, or a drive that was never prepared). The action is logged.

### Set fully synced: Clean offered, 150x42

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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · synced · [ c Clean ]                                                           │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [ o Prepare ]                                                                      │
│ [!] Clean needs sudo. Offered only when every mag on the set is synced.                                                                            │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Enter pick  ·  n new project  ·  r rescan  ·  p PULL  ·  click pane to focus  ·  Esc dashboard
```

### Clean popup, 150x42

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
║                                ║│ ╔═ Clean field backup set · FIELD-A ═════════════════════════════════════════╗tional)                            │
║ ▶ Generic DCIM           64 GB ║│ ║                                                                            ║                                   │
║   /dev/sdd1 · FAT32            ║│ ║  All mags on FIELD-A are on the server. Wipe master and slave?             ║                                   │
║   /media/magvault/GENERIC_64   ║│ ║                                                                            ║                                   │
║   ● selected                   ║│ ║  Deletes the mag folders on both drives.                                   ║ pull                              │
║                                ║│ ║  Keeps the set: magvault-backup.txt, set id, size limit, and               ║mma/gamut                          │
║                                ║│ ║  the master / slave roles. No re-prepare.                                  ║hoot day  ✓ target                 │
║                                ║│ ║  After clean the set is empty, still valid, ready for new mags.            ║e? Admin only.                     │
║                                ║│ ║  This action is logged.                                                    ║                                   │
║ r rescan    ↑↓ choose          ║│ ║                                                                            ║                                   │
║                                ║│ ║  Sudo password  ••••••••▏                                                  ║                                   │
╚════════════════════════════════╝└─║                                                                            ║───────────────────────────────────┘
┌─ Ingest ──────────────────────────║  [ Enter  Wipe master and slave ]                                          ║───────────────────────────────────┐
│ [ p  PULL ]  ░░░░░░░░░░░░░░░░░░░░░║                                                                            ║0 B / 42.7 GB  (0%)                │
│ → mags/20250520_<mag-uuid>  origin║                                                                            ║                                   │
└───────────────────────────────────║                                                                            ║───────────────────────────────────┘
┌─ 5 Targets ───────────────────────║  Esc  Cancel · nothing is deleted until sudo is accepted                   ║───────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-╚════════════════════════════════════════════════════════════════════════════╝                                   │
│ [x] Server          server:/mags                    reachable                                                                                      │
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · synced · [ c Clean ]                                                           │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [ o Prepare ]                                                                      │
│ [!] Clean needs sudo. Offered only when every mag on the set is synced.                                                                            │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Enter confirm · Esc cancel (back to ingest)
```

### After clean: set empty, still valid, 150x42

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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · empty · ready for new mags                                                     │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [ o Prepare ]                                                                      │
│ [!] Set is empty and still valid. No re-prepare needed. Prepare is for a new set.                                                                  │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ Log ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ card detected   /dev/sdd1 · FAT32 · 64 GB                                                                                                          │
│ probe done      6 files · 42.7 GB · 1 junk skipped                                                                                                 │
│ guessed         Sony A7S III · ProRes RAW · A7SIII S-Log3 · 2025-05-20                                                                             │
│ waiting         confirm details, then p to pull                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Enter pick  ·  n new project  ·  r rescan  ·  p PULL  ·  click pane to focus  ·  Esc dashboard
```

### Set fully synced: Clean offered, 120x36 (minimum)

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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · synced · [c Clean]                               │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Clean: needs sudo. Offered only when every mag is synced.                                                        │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Enter pick  ·  n new project  ·  r rescan  ·  p PULL  ·  click pane to focus  ·  Esc dashboard
```

### Clean popup, 120x36 (minimum)

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
║   ● ready            ╔═ Clean field backup set · FIELD-A ═════════════════════════════════════╗II S-Log3          ▾  │
║                      ║                                                                        ║ S-Gamut3.Cine     ▾  │
║ ▶ Generic DCIM       ║  All mags on FIELD-A are on the server. Wipe master and slave?         ║-05-20   guessed      │
║   /dev/sdd1 · FAT32  ║                                                                        ║ional)                │
║   ● selected         ║  Deletes the mag folders on both drives.                               ║                      │
║                      ║  Keeps the set: magvault-backup.txt, set id, size limit, and           ║                      │
║                      ║  the master / slave roles. No re-prepare.                              ║pull                  │
║                      ║  After clean the set is empty, still valid, ready for new mags.        ║ma/gamut              │
║                      ║  This action is logged.                                                ║oot day  ✓ target     │
║                      ║                                                                        ║? Admin only.         │
║                      ║  Sudo password  ••••••••▏                                              ║                      │
║                      ║                                                                        ║                      │
║ r rescan    ↑↓ choose║  [ Enter  Wipe master and slave ]                                      ║                      │
║                      ║                                                                        ║                      │
╚══════════════════════║                                                                        ║──────────────────────┘
┌─ Ingest ─────────────║                                                                        ║──────────────────────┐
│ [ p  PULL ]  ░░░░░░░░║  Esc  Cancel · nothing is deleted until sudo is accepted               ║  (0%)                │
│ → mags/20250520_<mag-╚════════════════════════════════════════════════════════════════════════╝                      │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ 5 Targets ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [x] Local staging   /tmp/magvault-staging           writable                                                         │
│ [x] Server          server:/mags                    reachable                                                        │
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · synced · [c Clean]                               │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Clean: needs sudo. Offered only when every mag is synced.                                                        │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Enter confirm · Esc cancel (back to ingest)
```

### After clean: set empty, still valid, 120x36 (minimum)

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
│ [ ] USB set         FIELD-A  master + slave         64 GB · valid · empty · ready for new mags                       │
│ [ ] USB drive       /media/odofad/NEW_64            not valid · [o Prepare]                                          │
│ [!] Set is empty and still valid. Prepare is only for a new set.                                                     │
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
| 5 | Targets | checkbox `List` | Local staging, server, USB backup **set** (one row: master + slave, set size, pending/synced). Only a valid set can be selected. A fully synced set shows `c Clean`. A not-valid drive shows `not valid · Prepare`. Warning for pending/no sync. |
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
| `o` | Open the Prepare popup for the selected not-valid USB drive (sudo password asked inside the popup) |
| `Esc` in popup | Cancel Prepare or Clean and return to ingest (nothing written or deleted) |
| `c` | Open the Clean popup for the selected set; only when every mag on it is synced (sudo password asked inside the popup) |
| `x` | Abort (pulling state only) |
| `l` | Show log (overlay at 120x36, strip at 150x42) |
| `Esc` | Return to dashboard (blocked while a copy is writing) |

## Behaviour notes

- **Pull is gated.** `p` only works once camera, format, gamma, gamut, project, shoot day and at least one target are confirmed (the "Confirm before pull" lines in pane 4).
- **Pipeline stages** match `PLAN.md`: rsync copy, then hash and verify the destination, write `magcard.txt` and ASC MHL, record in database, queue 320p proxies.
- **Unsafe until verified.** The card must stay inserted until hashes match.
- **Inputs lock** during a pull. Panes 1-4 dim and ignore keys.
- **USB validity:** only a valid matched set (master + slave, each with a valid `magvault-backup.txt` at its root) can be selected as a backup target, and it is selected as one set. Any other drive is “not valid” and offers Prepare, which opens the popup.
- **Prepare:** master first (smaller drive), then slave (same size or larger; smaller is refused; larger is limited to the master size). A sudo password is asked at each write step and nothing is written until it is accepted. Not offered on a set while any mag is pending server sync; re-enabled once every mag has synced. A valid set with pending mags remains selectable as a target.
- **Clean:** only on a fully synced set; wipes the mag folders on both drives after the sudo password is accepted, keeps `magvault-backup.txt`, set id, size limit and roles, and leaves the set empty and valid.
- **Pull** is the single action that starts the verified copy into the mag folder (then magcard + MHL + DB + queue). No separate “populate” step.
- Sample values (A7S III, ProRes RAW, ACME_Campaign_2025, sizes, percentages) are placeholders.
