# Logging module mockup (ratatui)

> **Mockup only.** Design reference for the Logging module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Logging module", "Logging fields (locked)", "Media playback"). Logging is per clip, separate from ingest, and available on both server and client.

## Server logging, 150x42

```text
 MAGVAULT  ·  Logging  ·  20250520_a3f9c2  ·  ACME_Campaign_2025                                           server · all online mags · synced · odofad
╔═ 1 Clips ════════════════════════════╗┌─ 2 Player ─────────────────────────────────────────────┐┌─ 3 Log clip ─────────────────────────────────────┐
║                                      ║│                                                        ││                                                  │
║ filter / type to filter              ║│                                                        ││ Description                                      │
║                                      ║│                                                        ││  Sun setting behind the stage                    │
║   ✓ A001_C001.mov  ★★★★☆             ║│                                                        ││                                                  │
║   ✓ A001_C002.mov  ★★★☆☆  SM         ║│                                                        ││ Tags                                             │
║   ✓ A001_C003.mov  ★★★★★             ║│                                                        ││ [sunset] [crowd] [+]                             │
║   ✓ A001_C004.mov  ★★☆☆☆             ║│                                                        ││                                                  │
║ ▶ ○ A001_C005.mov                    ║│                                                        ││ Rating                                           │
║   ○ A001_C006.mov                    ║│                                                        ││ ★★★★☆   1-5 sets                                 │
║   ○ A001_C007.mov                    ║│                       video area                       ││                                                  │
║   ○ A001_C008.mov                    ║│               kitty / sixel if supported               ││ Slow motion   [x] 120 fps                        │
║   ○ A001_C009.mov                    ║│                else audio + status only                ││                                                  │
║   ○ A001_C010.mov                    ║│                                                        ││ Technical note                                   │
║   ○ A001_C011.mov                    ║│                                                        ││  Wind noise on ch 1                              │
║                                      ║│                                                        ││                                                  │
║ 4 of 11 logged                       ║│                                                        ││ Shot type                                        │
║                                      ║│                                                        ││ wide  medium  close  aerial  [+]                 │
║                                      ║│                                                        ││                                                  │
║                                      ║│                                                        ││ Camera operator                                  │
║                                      ║│                                                        ││ Thabo  Lerato  Sam  [+]                          │
║                                      ║│                                                        ││                                                  │
║                                      ║│                                                        ││ Director / Producer                              │
║                                      ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch                  ││ Naledi  Pieter  [+]                              │
║                                      ║│ █████████░░░░░░░░░░░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                                  │
║                                      ║│ Space play · ←/→ seek 5s · Shift 30s                   ││                                                  │
║                                      ║└────────────────────────────────────────────────────────┘│                                                  │
║                                      ║┌─ 4 Tag picker ─────────────────────────────────────────┐│                                                  │
║                                      ║│ pick, or type to create new                            ││                                                  │
║                                      ║│ > sun                                                  ││                                                  │
║                                      ║│                                                        ││                                                  │
║                                      ║│ [ sunset ]  [ sunrise ]  [ sunglasses ]                ││                                                  │
║                                      ║│                                                        ││                                                  │
║                                      ║│ [ interview ]  [ b-roll ]  [ crowd ]  [ drone ]        ││                                                  │
║                                      ║│                                                        ││                                                  │
║                                      ║│ [ detail ]  [ night ]                                  ││                                                  │
║                                      ║│                                                        ││                                                  │
║ ] next unlogged   [ previous         ║│ Enter on no match creates new tag.                     ││ saved to database on field exit                  │
║                                      ║│ Shared with everyone.                                  ││                                                  │
╚══════════════════════════════════════╝└────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · 1-5 rating · m slow-mo · t tags · / filter · ][ unlogged · Space play · Esc dashboard
```

## Server logging, 120x36 (minimum)

```text
 MAGVAULT  ·  Logging  ·  20250520_a3f9c2  ·  ACME_Campaign_2025             server · all online mags · synced · odofad
╔═ 1 Clips ════════════════════╗┌─ 2 Player ─────────────────────────────────┐┌─ 3 Log clip ───────────────────────────┐
║                              ║│                                            ││                                        │
║ filter / type to filter      ║│                                            ││ Description                            │
║                              ║│                                            ││  Sun setting behind the stage          │
║   ✓ A001_C001.mov  ★★★★☆     ║│                                            ││                                        │
║   ✓ A001_C002.mov  ★★★☆☆  SM ║│                                            ││ Tags                                   │
║   ✓ A001_C003.mov  ★★★★★     ║│                                            ││ [sunset] [crowd] [+]                   │
║   ✓ A001_C004.mov  ★★☆☆☆     ║│                 video area                 ││                                        │
║ ▶ ○ A001_C005.mov            ║│         kitty / sixel if supported         ││ Rating                                 │
║   ○ A001_C006.mov            ║│          else audio + status only          ││ ★★★★☆   1-5 sets                       │
║   ○ A001_C007.mov            ║│                                            ││                                        │
║   ○ A001_C008.mov            ║│                                            ││ Slow motion   [x] 120 fps              │
║   ○ A001_C009.mov            ║│                                            ││                                        │
║   ○ A001_C010.mov            ║│                                            ││ Technical note                         │
║   ○ A001_C011.mov            ║│                                            ││  Wind noise on ch 1                    │
║                              ║│                                            ││                                        │
║ 4 of 11 logged               ║│                                            ││ Shot type                              │
║                              ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch      ││ wide  medium  close  aerial  [+]       │
║                              ║│ ██████░░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                        │
║                              ║│ Space play · ←/→ seek 5s · Shift 30s       ││ Camera operator                        │
║                              ║└────────────────────────────────────────────┘│ Thabo  Lerato  Sam  [+]                │
║                              ║┌─ 4 Tag picker ─────────────────────────────┐│                                        │
║                              ║│ pick, or type to create new                ││ Director / Producer                    │
║                              ║│ > sun                                      ││ Naledi  Pieter  [+]                    │
║                              ║│                                            ││                                        │
║                              ║│ [ sunset ]  [ sunrise ]  [ sunglasses ]    ││                                        │
║                              ║│                                            ││                                        │
║                              ║│ [ interview ]  [ b-roll ]  [ crowd ]       ││                                        │
║                              ║│                                            ││                                        │
║                              ║│ [ drone ]  [ detail ]  [ night ]           ││                                        │
║                              ║│                                            ││                                        │
║ ] next unlogged   [ previous ║│ Enter on no match creates new tag.         ││ saved to database on field exit        │
║                              ║│ Shared with everyone.                      ││                                        │
╚══════════════════════════════╝└────────────────────────────────────────────┘└────────────────────────────────────────┘
 Tab · ↑↓ · Enter edit · 1-5 rate · m slow-mo · t tags · / filter · ][ unlogged · Esc dash
```

## Client logging (offline, local media), 150x42

```text
 MAGVAULT  ·  Logging (local)  ·  USB BACKUP1  ·  20250520_a3f9c2                                         ● offline · 3 changes pending sync · odofad
╔═ 1 Clips ════════════════════════════╗┌─ 2 Player ─────────────────────────────────────────────┐┌─ 3 Log clip ─────────────────────────────────────┐
║                                      ║│                                                        ││                                                  │
║ filter / type to filter              ║│                                                        ││ Description                                      │
║                                      ║│                                                        ││  Sun setting behind the stage                    │
║   ✓ A001_C001.mov  ★★★★☆             ║│                                                        ││                                                  │
║   ✓ A001_C002.mov  ★★★☆☆  SM         ║│                                                        ││ Tags                                             │
║   ✓ A001_C003.mov  ★★★★★             ║│                                                        ││ [sunset] [crowd] [+]                             │
║   ✓ A001_C004.mov  ★★☆☆☆             ║│                                                        ││                                                  │
║ ▶ ○ A001_C005.mov                    ║│                                                        ││ Rating                                           │
║   ○ A001_C006.mov                    ║│                                                        ││ ★★★★☆   1-5 sets                                 │
║   ○ A001_C007.mov                    ║│                       video area                       ││                                                  │
║   ○ A001_C008.mov                    ║│               kitty / sixel if supported               ││ Slow motion   [x] 120 fps                        │
║   ○ A001_C009.mov                    ║│                else audio + status only                ││                                                  │
║   ○ A001_C010.mov                    ║│                                                        ││ Technical note                                   │
║   ○ A001_C011.mov                    ║│                                                        ││  Wind noise on ch 1                              │
║                                      ║│                                                        ││                                                  │
║ 4 of 11 logged                       ║│                                                        ││ Shot type                                        │
║                                      ║│                                                        ││ wide  medium  close  aerial  [+]                 │
║                                      ║│                                                        ││                                                  │
║                                      ║│                                                        ││ Camera operator                                  │
║                                      ║│                                                        ││ Thabo  Lerato  Sam  [+]                          │
║                                      ║│                                                        ││                                                  │
║                                      ║│                                                        ││ Director / Producer                              │
║                                      ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch                  ││ Naledi  Pieter  [+]                              │
║                                      ║│ █████████░░░░░░░░░░░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                                  │
║                                      ║│ Space play · ←/→ seek 5s · Shift 30s                   ││                                                  │
║                                      ║└────────────────────────────────────────────────────────┘│                                                  │
║                                      ║┌─ 4 Tag picker ─────────────────────────────────────────┐│                                                  │
║                                      ║│ pick, or type to create new                            ││                                                  │
║                                      ║│ > sun                                                  ││                                                  │
║                                      ║│                                                        ││                                                  │
║                                      ║│ [ sunset ]  [ sunrise ]  [ sunglasses ]                ││                                                  │
║                                      ║│                                                        ││                                                  │
║                                      ║│ [ interview ]  [ b-roll ]  [ crowd ]  [ drone ]        ││                                                  │
║                                      ║│                                                        ││                                                  │
║                                      ║│ [ detail ]  [ night ]                                  ││                                                  │
║                                      ║│                                                        ││ saved to database on field exit                  │
║ ] next unlogged   [ previous         ║│ Enter on no match creates new tag.                     ││ offline: syncs when server is back               │
║                                      ║│ Shared with everyone.                                  ││                                                  │
╚══════════════════════════════════════╝└────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · 1-5 rating · m slow-mo · t tags · / filter · ][ unlogged · Space play · Esc dashboard
```

## Client logging (offline, local media), 120x36 (minimum)

```text
 MAGVAULT  ·  Logging (local)  ·  USB BACKUP1  ·  20250520_a3f9c2           ● offline · 3 changes pending sync · odofad
╔═ 1 Clips ════════════════════╗┌─ 2 Player ─────────────────────────────────┐┌─ 3 Log clip ───────────────────────────┐
║                              ║│                                            ││                                        │
║ filter / type to filter      ║│                                            ││ Description                            │
║                              ║│                                            ││  Sun setting behind the stage          │
║   ✓ A001_C001.mov  ★★★★☆     ║│                                            ││                                        │
║   ✓ A001_C002.mov  ★★★☆☆  SM ║│                                            ││ Tags                                   │
║   ✓ A001_C003.mov  ★★★★★     ║│                                            ││ [sunset] [crowd] [+]                   │
║   ✓ A001_C004.mov  ★★☆☆☆     ║│                 video area                 ││                                        │
║ ▶ ○ A001_C005.mov            ║│         kitty / sixel if supported         ││ Rating                                 │
║   ○ A001_C006.mov            ║│          else audio + status only          ││ ★★★★☆   1-5 sets                       │
║   ○ A001_C007.mov            ║│                                            ││                                        │
║   ○ A001_C008.mov            ║│                                            ││ Slow motion   [x] 120 fps              │
║   ○ A001_C009.mov            ║│                                            ││                                        │
║   ○ A001_C010.mov            ║│                                            ││ Technical note                         │
║   ○ A001_C011.mov            ║│                                            ││  Wind noise on ch 1                    │
║                              ║│                                            ││                                        │
║ 4 of 11 logged               ║│                                            ││ Shot type                              │
║                              ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch      ││ wide  medium  close  aerial  [+]       │
║                              ║│ ██████░░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                        │
║                              ║│ Space play · ←/→ seek 5s · Shift 30s       ││ Camera operator                        │
║                              ║└────────────────────────────────────────────┘│ Thabo  Lerato  Sam  [+]                │
║                              ║┌─ 4 Tag picker ─────────────────────────────┐│                                        │
║                              ║│ pick, or type to create new                ││ Director / Producer                    │
║                              ║│ > sun                                      ││ Naledi  Pieter  [+]                    │
║                              ║│                                            ││                                        │
║                              ║│ [ sunset ]  [ sunrise ]  [ sunglasses ]    ││                                        │
║                              ║│                                            ││                                        │
║                              ║│ [ interview ]  [ b-roll ]  [ crowd ]       ││                                        │
║                              ║│                                            ││                                        │
║                              ║│ [ drone ]  [ detail ]  [ night ]           ││                                        │
║                              ║│                                            ││ saved to database on field exit        │
║ ] next unlogged   [ previous ║│ Enter on no match creates new tag.         ││ offline: syncs when server is back     │
║                              ║│ Shared with everyone.                      ││                                        │
╚══════════════════════════════╝└────────────────────────────────────────────┘└────────────────────────────────────────┘
 Tab · ↑↓ · Enter edit · 1-5 rate · m slow-mo · t tags · / filter · ][ unlogged · Esc dash
```

## Panes

| # | Pane | Notes |
|---|------|-------|
| 1 | Clips | Clips of the open mag with logged marker, star rating and slow-motion flag. Filter, plus jump to next/previous unlogged clip. Shows the logged count. |
| 2 | Player | mpv on the 320p proxy (or HQ). Video area uses kitty/sixel when the terminal supports it; otherwise audio plus status line only. Transport: play/pause, seek, position and duration, proxy type, fps, audio channels. |
| 3 | Log clip | The locked field set, in order (see below). Saved to the database when leaving a field. |
| 4 | Tag picker | Opens for tags (and the other clickable lists). Pick an existing entry, or type to filter and press Enter to create a new one, which then becomes clickable for everyone. |

## Fields (locked in PLAN.md)

| Field | Widget |
|-------|--------|
| Description | Short free text |
| Keywords / Tags | Clickable chips plus `[+]` to add new |
| Rating | Stars 1-5 |
| Slow motion | Checkbox |
| Technical note | Optional short free text |
| Shot type | Optional clickable list plus `[+]` |
| Camera operator | Clickable list plus `[+]` |
| Director / Producer | Clickable list plus `[+]` |

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `Enter` | Edit the field, or pick the highlighted entry |
| `1`-`5` | Set rating |
| `m` | Toggle slow motion |
| `t` | Open tag picker |
| `/` | Filter clips |
| `]` / `[` | Next / previous unlogged clip |
| `Space` | Play / pause |
| `Left` / `Right` | Seek 5 s (Shift: 30 s) when the player has focus |
| `Esc` | Return to the dashboard (shared convention) |

## Server vs client

- **Server:** full access to all online mags. Header shows synced state.
- **Client:** works against local staging, USB backups or recently synced mags. Offline logging is supported on local media. The header shows connection state and the number of changes pending sync, and they upload when the server is reachable.
- **Proxy source on the client:** prefers server-generated 320p proxies pulled when ready. Falls back to a quick temporary local proxy for a single mag when offline or when server proxies are not yet available.
- Original filenames and sidecars are never touched. Log data lives in the database.

All sample values (mags, clips, tags, names, times, ratings) are placeholders.
