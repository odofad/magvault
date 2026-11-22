# Zmag manager mockup (ratatui)

> **Mockup only.** Design reference for the Zmag manager. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Zmag manager"). The module runs on the server. Anything that touches `zpool` / `zfs` asks for the sudo password. Remote zmag stations appear as additional zmags but their pool health is not managed from here.

## Main two-pane view, 150x42

```text
 MAGVAULT  ·  Zmag manager  ·  two-pane                                                               zmag-01 ● zmag-02 ● zmag-03 (remote) ● · odofad
╔═ 1 Server mags ════════════════════════════════════════════╗┌─ 2 Zmag contents ────────────────────────────────────────────────────────────────────┐
║                                                            ║│                                                                                      │
║ project ACME_Campaign_2025 ▾                               ║│  zmag-01    [zmag-02]    zmag-03                                                     │
║                                                            ║│                                                                                      │
║ mag                 size      copies  01  02  03           ║│ zmag-02  backup · local · ONLINE                                                     │
║   20250520_a3f9c2   84 GB      0      ·   ·   ·            ║│ 3.2 TB used of 14.5 TB · 48 mags                                                     │
║   20250519_c41d77   212 GB     1      ●   ·   ·            ║│                                                                                      │
║ ▶ 20250518_7be410   96 GB      2      ●   ●   ·            ║│ mag                 size                                                             │
║   20250517_0cd8e1   140 GB     2      ●   ●   ·            ║│   20250518_7be410   96 GB                                                            │
║   20250412_55d0aa   61 GB      2      ●   ·   ●            ║│   20250517_0cd8e1   140 GB                                                           │
║   20250410_11ab90   118 GB     1      ·   ●   ·            ║│   20250408_9e0042   77 GB                                                            │
║   20250408_9e0042   77 GB      2      ●   ●   ·            ║│   20250301_aa31f0   203 GB                                                           │
║                                                            ║│   20250227_71c2de   58 GB                                                            │
║ copies = placements across zmags. 0 or 1 shown in amber.   ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║                                                            ║│                                                                                      │
║ c copy to selected zmag (whole mag)                        ║│ ← → switch zmag                                                                      │
║ s snapshots   p pool health                                ║│ zmag-03 is a remote station (SSH)                                                    │
╚════════════════════════════════════════════════════════════╝└──────────────────────────────────────────────────────────────────────────────────────┘
┌─ 3 Operations ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Auto-backup   ON  zmag-02 inserted: copying everything that needs a second copy                                                                    │
│ ▶ 20250519_c41d77 → zmag-02                                                                                                                        │
│ ██████████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  41%  87 GB / 212 GB            │
│ queued  20250410_11ab90 · 20250412_55d0aa → zmag-02                                                                                                │
│                                                                                                                                                    │
│ Cold pull requests  1 pending: 20250412_55d0aa → Bamba_Cutaways (zmag-03 remote, dock needed)                                                      │
│                                                                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · ←→ zmag · c copy · f project · a auto-backup · s snapshots · p pool health · Esc dashboard
```

## Main two-pane view, 120x36 (minimum)

```text
 MAGVAULT  ·  Zmag manager  ·  two-pane                                 zmag-01 ● zmag-02 ● zmag-03 (remote) ● · odofad
╔═ 1 Server mags ════════════════════════════════╗┌─ 2 Zmag contents ──────────────────────────────────────────────────┐
║                                                ║│                                                                    │
║ project ACME_Campaign_2025 ▾                   ║│  zmag-01  [zmag-02]  zmag-03                                       │
║                                                ║│                                                                    │
║ mag                 size      copies  01  02   ║│ zmag-02  backup · local · ONLINE                                   │
║   20250520_a3f9c2   84 GB      0      ·   ·    ║│ 3.2 TB used of 14.5 TB · 48 mags                                   │
║   20250519_c41d77   212 GB     1      ●   ·    ║│                                                                    │
║ ▶ 20250518_7be410   96 GB      2      ●   ●    ║│ mag                 size                                           │
║   20250517_0cd8e1   140 GB     2      ●   ●    ║│   20250518_7be410   96 GB                                          │
║   20250412_55d0aa   61 GB      2      ●   ·    ║│   20250517_0cd8e1   140 GB                                         │
║   20250410_11ab90   118 GB     1      ·   ●    ║│   20250408_9e0042   77 GB                                          │
║   20250408_9e0042   77 GB      2      ●   ●    ║│   20250301_aa31f0   203 GB                                         │
║                                                ║│   20250227_71c2de   58 GB                                          │
║ copies = placements across zmags. 0 or 1 shown i│ amber.                                                             │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║                                                ║│                                                                    │
║ c copy to selected zmag (whole mag)            ║│ ← → switch zmag                                                    │
║ s snapshots   p pool health                    ║│ zmag-03 is a remote station (SSH)                                  │
╚════════════════════════════════════════════════╝└────────────────────────────────────────────────────────────────────┘
┌─ 3 Operations ───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Auto-backup   ON  zmag-02 inserted: copying everything that needs a second copy                                      │
│ ▶ 20250519_c41d77 → zmag-02                                                                                          │
│ ███████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  41%  87 GB / 212 GB          │
│ queued  20250410_11ab90 · 20250412_55d0aa → zmag-02                                                                  │
│                                                                                                                      │
│ Cold pull requests  1 pending: 20250412_55d0aa → Bamba_Cutaways (zmag-03 remote, dock needed)                        │
│                                                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab · ↑↓ · ←→ zmag · c copy · a auto · s snaps · p health · Esc dash
```

## Snapshots (`s`), 150x42

```text
 MAGVAULT  ·  Zmag manager  ·  snapshots                                                                                      scope: zmag-02 · odofad
┌─ 1 Scope ──────────────────┐╔═ 2 Snapshots · zmag-02 ══════════════════════════════════════════════════════════════════════════════════════════════╗
│                            │║                                                                                                                      ║
│ zmag-01                    │║ snapshot                          created           used      hold                                                   ║
│ ▶ zmag-02                  │║ zmag-02@auto-20251122-0200        2025-11-22 02:00  1.2 GB    no                                                     ║
│ zmag-03  remote            │║ zmag-02@auto-20251115-0200        2025-11-15 02:00  3.8 GB    no                                                     ║
│                            │║ zmag-02@pre-scrub-20251101        2025-11-01 18:14  0.4 GB    yes                                                    ║
│ selected mag               │║ zmag-02@auto-20251025-0200        2025-10-25 02:00  5.1 GB    no                                                     ║
│ 20250518_7be410            │║ zmag-02@first-copy-complete       2025-09-30 11:42  12 GB     yes                                                    ║
│                            │║                                                                                                                      ║
│ m mag snapshots            │║                                                                                                                      ║
│ z zmag snapshots           │║ h  hold / release      d  destroy (asks)                                                                             ║
│                            │║                                                                                                                      ║
│                            │║ ┌─ Confirm destroy ──────────────────────────────────────────────────────┐                                           ║
│                            │║ │                                                                        │                                           ║
│                            │║ │ Destroy zmag-02@auto-20251115-0200 ?                                   │                                           ║
│                            │║ │ Frees about 3.8 GB. This cannot be undone.                             │                                           ║
│                            │║ │ Needs sudo password.   y confirm   n cancel                            │                                           ║
│                            │║ └────────────────────────────────────────────────────────────────────────┘                                           ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║                                                                                                                      ║
│                            │║ Held snapshots cannot be destroyed until released.                                                                   ║
│                            │║                                                                                                                      ║
└────────────────────────────┘╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
 ↑↓ move · h hold/release · d destroy · m mag · z zmag · Tab pane · Esc back to zmag manager
```

## Snapshots (`s`), 120x36 (minimum)

```text
 MAGVAULT  ·  Zmag manager  ·  snapshots                                                        scope: zmag-02 · odofad
┌─ 1 Scope ──────────────┐╔═ 2 Snapshots · zmag-02 ════════════════════════════════════════════════════════════════════╗
│                        │║                                                                                            ║
│ zmag-01                │║ snapshot                          created           used      hold                         ║
│ ▶ zmag-02              │║ zmag-02@auto-20251122-0200        2025-11-22 02:00  1.2 GB    no                           ║
│ zmag-03  remote        │║ zmag-02@auto-20251115-0200        2025-11-15 02:00  3.8 GB    no                           ║
│                        │║ zmag-02@pre-scrub-20251101        2025-11-01 18:14  0.4 GB    yes                          ║
│ selected mag           │║ zmag-02@auto-20251025-0200        2025-10-25 02:00  5.1 GB    no                           ║
│ 20250518_7be410        │║ zmag-02@first-copy-complete       2025-09-30 11:42  12 GB     yes                          ║
│                        │║                                                                                            ║
│ m mag snapshots        │║                                                                                            ║
│ z zmag snapshots       │║ h  hold / release      d  destroy (asks)                                                   ║
│                        │║                                                                                            ║
│                        │║ ┌─ Confirm destroy ──────────────────────────────────────────────────────┐                 ║
│                        │║ │                                                                        │                 ║
│                        │║ │ Destroy zmag-02@auto-20251115-0200 ?                                   │                 ║
│                        │║ │ Frees about 3.8 GB. This cannot be undone.                             │                 ║
│                        │║ │ Needs sudo password.   y confirm   n cancel                            │                 ║
│                        │║ └────────────────────────────────────────────────────────────────────────┘                 ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║                                                                                            ║
│                        │║ Held snapshots cannot be destroyed until released.                                         ║
│                        │║                                                                                            ║
└────────────────────────┘╚════════════════════════════════════════════════════════════════════════════════════════════╝
 ↑↓ move · h hold/release · d destroy · m mag · z zmag · Tab pane · Esc back to zmag manager
```

## Pool / drive health (`p`), 150x42

```text
 MAGVAULT  ·  Zmag manager  ·  pool health                                                                                  local pools only · odofad
┌─ 1 Pools ──────────────────┐╔═ 2 zmag-02 status ═══════════════════════════════════════════════════════════════════════════════════════════════════╗
│                            │║                                                                                                                      ║
│ zmag-01          ONLINE    │║ state   DEGRADED    raidz1 · 4 drives · 14.5 TB · 3.2 TB used                                                        ║
│ ▶ zmag-02       DEGRADED   │║ errors  none known                                                                                                   ║
│ zmag-03  remote            │║ scrub   last finished 2025-11-01 · 0 errors                                                                          ║
│   not managed here         │║                                                                                                                      ║
│                            │║ drive       state       read/write/cksum                                                                             ║
│                            │║ sde         ONLINE      0 0 0                                                                                        ║
│ zmag-02 is DEGRADED        │║ sdf         ONLINE      0 0 0                                                                                        ║
│ sdg FAULTED                │║ sdg         FAULTED     12 4 0                                                                                       ║
│                            │║ sdh         ONLINE      0 0 0                                                                                        ║
│                            │║                                                                                                                      ║
│                            │║ [!] replace sdg, then resilver runs automatically                                                                    ║
│                            │║                                                                                                                      ║
│                            │╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
│                            │
│                            │┌─ 3 SMART · selected drive sdg ───────────────────────────────────────────────────────────────────────────────────────┐
│                            ││                                                                                                                      │
│                            ││ model                 ST4000VN006                                                                                    │
│                            ││ health                FAILING (pre-fail)                                                                             │
│                            ││ temperature           41 C                                                                                           │
│                            ││ power-on hours        31 402                                                                                         │
│                            ││ reallocated sectors   288                                                                                            │
│                            ││ pending sectors       16                                                                                             │
│                            ││ last self-test        read failure, 2025-11-20                                                                       │
│                            ││                                                                                                                      │
│                            ││ b  scrub   r  replace drive   t  SMART self-test   m  refresh SMART                                                  │
│                            ││ zpool actions need the sudo password. Local pools only.                                                              │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
│                            ││                                                                                                                      │
└────────────────────────────┘└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ move · b scrub · r replace drive · t self-test · m refresh · Tab pane · Esc back to zmag manager
```

## Pool / drive health (`p`), 120x36 (minimum)

```text
 MAGVAULT  ·  Zmag manager  ·  pool health                                                    local pools only · odofad
┌─ 1 Pools ──────────────┐╔═ 2 zmag-02 status ═════════════════════════════════════════════════════════════════════════╗
│                        │║                                                                                            ║
│ zmag-01      ONLINE    │║ state   DEGRADED    raidz1 · 4 drives · 14.5 TB · 3.2 TB used                              ║
│ ▶ zmag-02   DEGRADED   │║ errors  none known                                                                         ║
│ zmag-03  remote        │║ scrub   last finished 2025-11-01 · 0 errors                                                ║
│   not managed here     │║                                                                                            ║
│                        │║ drive       state       read/write/cksum                                                   ║
│                        │║ sde         ONLINE      0 0 0                                                              ║
│ zmag-02 is DEGRADED    │║ sdf         ONLINE      0 0 0                                                              ║
│ sdg FAULTED            │║ sdg         FAULTED     12 4 0                                                             ║
│                        │║ sdh         ONLINE      0 0 0                                                              ║
│                        │║                                                                                            ║
│                        │║ [!] replace sdg, then resilver runs automatically                                          ║
│                        │║                                                                                            ║
│                        │╚════════════════════════════════════════════════════════════════════════════════════════════╝
│                        │
│                        │┌─ 3 SMART · selected drive sdg ─────────────────────────────────────────────────────────────┐
│                        ││                                                                                            │
│                        ││ model                 ST4000VN006                                                          │
│                        ││ health                FAILING (pre-fail)                                                   │
│                        ││ temperature           41 C                                                                 │
│                        ││ power-on hours        31 402                                                               │
│                        ││ reallocated sectors   288                                                                  │
│                        ││ pending sectors       16                                                                   │
│                        ││ last self-test        read failure, 2025-11-20                                             │
│                        ││                                                                                            │
│                        ││ b  scrub   r  replace drive   t  SMART self-test   m  refresh SMART                        │
│                        ││ zpool actions need the sudo password. Local pools only.                                    │
│                        ││                                                                                            │
│                        ││                                                                                            │
│                        ││                                                                                            │
│                        ││                                                                                            │
│                        ││                                                                                            │
└────────────────────────┘└────────────────────────────────────────────────────────────────────────────────────────────┘
 ↑↓ move · b scrub · r replace drive · t self-test · m refresh · Tab pane · Esc back to zmag manager
```

## Main view panes

| # | Pane | Notes |
|---|------|-------|
| 1 | Server mags | Online server mags, filterable by project. A copies column shows how many zmags hold each mag (0 or 1 in amber, 2+ in green) with a dot per zmag, so the backup view is visible at a glance. |
| 2 | Zmag contents | Tabs for each mounted zmag (local and remote). Shows id, role (primary / backup), where it lives, state, usage, and the mags it holds. A remote zmag is a read-only status view here. |
| 3 | Operations | Auto-backup state and the active copy with progress, queued copies, and pending cold-pull requests (with a note when a dock is needed). |

- Copy is whole-mag only (`c` copies the selected mag to the selected zmag).
- Auto-backup: inserting the first zmag copies everything that needs a first copy; inserting the second copies everything that needs a second copy.
- Only recognised zmags (property `magvault:type=zmag`) are ever imported or mounted. Other pools are ignored.

## Secondary screens

Opened by key from the main view. `Esc` returns to the main two-pane view (not the dashboard).

- **Snapshots (`s`)**: scope list (each zmag, or the selected mag) and a snapshot table with name, created, used and hold. Actions: hold / release, destroy with confirmation (shows the space it frees). Held snapshots cannot be destroyed until released.
- **Pool / drive health (`p`)**: pool state, error counts, last scrub, drive table (state and read/write/checksum errors) and a SMART panel for the selected drive. Actions: scrub, replace drive, SMART self-test, refresh SMART. Local pools only. A remote zmag station shows "not managed here".

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `Left` / `Right` | Switch zmag |
| `c` | Copy selected mag to selected zmag |
| `f` | Cycle project filter |
| `a` | Toggle auto-backup |
| `s` | Snapshots screen |
| `p` | Pool / drive health screen |
| `Esc` | Main view: return to the dashboard. Secondary screens: back to the main view. |

**Snapshots screen:** `h` hold / release, `d` destroy (asks), `m` mag snapshots, `z` zmag snapshots, `y` / `n` answer the confirmation.
**Pool health screen:** `b` scrub, `r` replace drive, `t` SMART self-test, `m` refresh SMART.

## Not covered yet

- Creating a zmag from four raw drives (Phase 4).
- Remote zmag station management and the Key module (Phase 5).
- All sample values (mags, sizes, drive names, SMART numbers, dates) are placeholders.
