# Logging module mockup (ratatui)

> **Mockup only.** Design reference for the Logging module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Logging module", "Logging fields (locked)", "Logging screen behaviour", "Media playback"). Logging is per clip, separate from ingest, and available on both server and client.

The screens show one moment: clip `A001_C005.mov` selected and the Tags field being typed into, so the tag popup is open.

When the typed text matches nothing, the popup shows a single line `[create "typed text"]`. Enter on that line adds the new tag for everyone.

## Server logging, 150x42

```text
 MAGVAULT  ·  Logging  ·  ACME_Campaign_2025                                                                       server · synced · saved ✓ · odofad
┌─ 1 Mags ─────────────────────────┐┌─ 3 Player ─────────────────────────────────────────────────┐┌─ 4 Log clip ─────────────────────────────────────┐
│                                  ││                                              ● proxy ready ││                                                  │
│ mag               logged         ││                                                            ││ Description                                      │
│ ▶ 20250520_a3f9c2  4/11          ││                                                            ││ ┌──────────────────────────────────────────────┐ │
│   20250519_c41d77 22/22 ✓ done   ││                                                            ││ │ Sun setting behind the stage                 │ │
│   20250518_7be410 12/12 ✓ done   ││                                                            ││ └──────────────────────────────────────────────┘ │
│   20250517_0cd8e1 0/11           ││                                                            ││ Tags                                             │
│                                  ││                                                            ││ [sunset ×] [crowd ×]                             │
│                                  ││                                                            ││ ┌──────────────────────────────────────────────┐ │
│                                  ││                                                            ││ │ sun▏                                         │ │
│ c  mark mag complete             ││                                                            ││ └──────────────────────────────────────────────┘ │
└──────────────────────────────────┘│                                                            ││   ┌────────────────────────────────────────┐     │
╔═ 2 Clips ════════════════════════╗│                                                            ││   │ sunset  (already added)                │     │
║                                  ║│                                                            ││   │ sunrise                                │     │
║   ✓ A001_C001.mov  ★★★★☆         ║│                                                            ││   │ sunglasses                             │     │
║   ✓ A001_C002.mov  ★★★☆☆  SM     ║│                                                            ││   └────────────────────────────────────────┘     │
║   ✓ A001_C003.mov  ★★★★★         ║│                         video area                         ││   Enter picks · ↑↓ move · [create "sun"] if no match                      │
║   ✓ A001_C004.mov  ★★☆☆☆         ║│                 kitty / sixel if supported                 ││                                                  │
║ ▶ ○ A001_C005.mov                ║│                  else audio + status only                  ││ Rating    ★★★★☆    1-5 sets                      │
║   ○ A001_C006.mov                ║│                                                            ││ Slow motion   [x] 120 fps                        │
║   ○ A001_C007.mov                ║│                                                            ││ Technical note                                   │
║   ○ A001_C008.mov                ║│                                                            ││ ┌──────────────────────────────────────────────┐ │
║   ○ A001_C009.mov                ║│                                                            ││ │ audio, focus, exposure…                      │ │
║   ○ A001_C010.mov                ║│                                                            ││ └──────────────────────────────────────────────┘ │
║   ○ A001_C011.mov                ║│                                                            ││ Shot type           wide  medium  close  aerial  │
║                                  ║│                                                            ││ Camera operator     [Thabo ×]                    │
║                                  ║│                                                            ││ Director / Producer [Naledi ×]                   │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch                      ││                                                  │
║                                  ║│ ███████████░░░░░░░░░░░░░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                                  │
║ 4 of 11 logged                   ║│ Space play · ←/→ seek 5s · Shift 30s                       ││ Ctrl+Z undo field · v revert from mag file       │
║ ] next unlogged  [ previous      ║│ icon: ready · pending · pulling · fallback                 ││ saved ✓ on field exit                            │
╚══════════════════════════════════╝└────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · 1-5 rate · m slow-mo · t tags · ][ unlogged · c complete · v revert · Ctrl+Z undo · Esc dashboard
```

## Server logging, 120x36 (minimum)

```text
 MAGVAULT  ·  Logging  ·  ACME_Campaign_2025                                         server · synced · saved ✓ · odofad
┌─ 1 Mags ─────────────────────┐┌─ 3 Player ───────────────────────────────┐┌─ 4 Log clip ─────────────────────────────┐
│                              ││                            ● proxy ready ││                                          │
│ mag               logged     ││                                          ││ Description                              │
│ ▶ 20250520_a3f9c2  4/11      ││                                          ││ ┌──────────────────────────────────────┐ │
│   20250519_c41d77 22/22✓     ││                                          ││ │ Sun setting behind the stage         │ │
│   20250518_7be410 12/12✓     ││                                          ││ └──────────────────────────────────────┘ │
│   20250517_0cd8e1 0/11       ││                                          ││ Tags                                     │
│                              ││                                          ││ [sunset ×] [crowd ×]                     │
│ c  mark mag complete         ││                                          ││ ┌──────────────────────────────────────┐ │
└──────────────────────────────┘│                                          ││ │ sun▏                                 │ │
╔═ 2 Clips ════════════════════╗│                                          ││ └──────────────────────────────────────┘ │
║                              ║│                                          ││   ┌────────────────────────────────┐     │
║   ✓ A001_C001.mov  ★★★★☆     ║│                                          ││   │ sunset  (already added)        │     │
║   ✓ A001_C002.mov  ★★★☆☆  SM ║│                video area                ││   │ sunrise                        │     │
║   ✓ A001_C003.mov  ★★★★★     ║│        kitty / sixel if supported        ││   │ sunglasses                     │     │
║   ✓ A001_C004.mov  ★★☆☆☆     ║│         else audio + status only         ││   └────────────────────────────────┘     │
║ ▶ ○ A001_C005.mov            ║│                                          ││   Enter picks · ↑↓ move · [create "sun"] if no match              │
║   ○ A001_C006.mov            ║│                                          ││                                          │
║   ○ A001_C007.mov            ║│                                          ││ Rating    ★★★★☆    1-5 sets              │
║   ○ A001_C008.mov            ║│                                          ││ Slow motion   [x] 120 fps                │
║   ○ A001_C009.mov            ║│                                          ││ Technical note                           │
║   ○ A001_C010.mov            ║│                                          ││ ┌──────────────────────────────────────┐ │
║   ○ A001_C011.mov            ║│                                          ││ │ audio, focus, exposure…              │ │
║                              ║│                                          ││ └──────────────────────────────────────┘ │
║                              ║│                                          ││ Shot type           wide  medium  close  │
║                              ║│                                          ││ Camera operator     [Thabo ×]            │
║                              ║│                                          ││ Director / Producer [Naledi ×]           │
║                              ║│                                          ││                                          │
║                              ║│                                          ││                                          │
║                              ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch    ││                                          │
║                              ║│ █████░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                          │
║ 4 of 11 logged               ║│ Space play · ←/→ seek 5s · Shift 30s     ││ Ctrl+Z undo · v revert from mag file     │
║ ] next unlogged  [ previous  ║│ icon: ready · pending · pulling · fallba ││ saved ✓ on field exit                    │
╚══════════════════════════════╝└──────────────────────────────────────────┘└──────────────────────────────────────────┘
 Tab · ↑↓ · Enter edit · 1-5 rate · m slow-mo · t tags · ][ unlog · c done · v revert · Esc dash
```

## Client logging (offline, local media), 150x42

```text
 MAGVAULT  ·  Logging (local)  ·  USB BACKUP1                                                   ● offline · 3 changes pending sync · saved ✓ · odofad
┌─ 1 Mags ─────────────────────────┐┌─ 3 Player ─────────────────────────────────────────────────┐┌─ 4 Log clip ─────────────────────────────────────┐
│                                  ││                                           ◐ local fallback ││                                                  │
│ mag               logged         ││                                                            ││ Description                                      │
│ ▶ 20250520_a3f9c2  4/11          ││                                                            ││ ┌──────────────────────────────────────────────┐ │
│   20250519_c41d77 22/22 ✓ done   ││                                                            ││ │ Sun setting behind the stage                 │ │
│   20250518_7be410 12/12 ✓ done   ││                                                            ││ └──────────────────────────────────────────────┘ │
│   20250517_0cd8e1 0/11           ││                                                            ││ Tags                                             │
│                                  ││                                                            ││ [sunset ×] [crowd ×]                             │
│                                  ││                                                            ││ ┌──────────────────────────────────────────────┐ │
│                                  ││                                                            ││ │ sun▏                                         │ │
│ c  mark mag complete             ││                                                            ││ └──────────────────────────────────────────────┘ │
└──────────────────────────────────┘│                                                            ││   ┌────────────────────────────────────────┐     │
╔═ 2 Clips ════════════════════════╗│                                                            ││   │ sunset  (already added)                │     │
║                                  ║│                                                            ││   │ sunrise                                │     │
║   ✓ A001_C001.mov  ★★★★☆         ║│                                                            ││   │ sunglasses                             │     │
║   ✓ A001_C002.mov  ★★★☆☆  SM     ║│                                                            ││   └────────────────────────────────────────┘     │
║   ✓ A001_C003.mov  ★★★★★         ║│                         video area                         ││   Enter picks · ↑↓ move · [create "sun"] if no match                      │
║   ✓ A001_C004.mov  ★★☆☆☆         ║│                 kitty / sixel if supported                 ││                                                  │
║ ▶ ○ A001_C005.mov                ║│                  else audio + status only                  ││ Rating    ★★★★☆    1-5 sets                      │
║   ○ A001_C006.mov                ║│                                                            ││ Slow motion   [x] 120 fps                        │
║   ○ A001_C007.mov                ║│                                                            ││ Technical note                                   │
║   ○ A001_C008.mov                ║│                                                            ││ ┌──────────────────────────────────────────────┐ │
║   ○ A001_C009.mov                ║│                                                            ││ │ audio, focus, exposure…                      │ │
║   ○ A001_C010.mov                ║│                                                            ││ └──────────────────────────────────────────────┘ │
║   ○ A001_C011.mov                ║│                                                            ││ Shot type           wide  medium  close  aerial  │
║                                  ║│                                                            ││ Camera operator     [Thabo ×]                    │
║                                  ║│                                                            ││ Director / Producer [Naledi ×]                   │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│                                                            ││                                                  │
║                                  ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch                      ││                                                  │
║                                  ║│ ███████████░░░░░░░░░░░░░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                                  │
║ 4 of 11 logged                   ║│ Space play · ←/→ seek 5s · Shift 30s                       ││ Ctrl+Z undo field · v revert from mag file       │
║ ] next unlogged  [ previous      ║│ icon: ready · pending · pulling · fallback                 ││ saved ✓ on field exit · syncs online             │
╚══════════════════════════════════╝└────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · 1-5 rate · m slow-mo · t tags · ][ unlogged · c complete · v revert · Ctrl+Z undo · Esc dashboard
```

## Client logging (offline, local media), 120x36 (minimum)

```text
 MAGVAULT  ·  Logging (local)  ·  USB BACKUP1                     ● offline · 3 changes pending sync · saved ✓ · odofad
┌─ 1 Mags ─────────────────────┐┌─ 3 Player ───────────────────────────────┐┌─ 4 Log clip ─────────────────────────────┐
│                              ││                         ◐ local fallback ││                                          │
│ mag               logged     ││                                          ││ Description                              │
│ ▶ 20250520_a3f9c2  4/11      ││                                          ││ ┌──────────────────────────────────────┐ │
│   20250519_c41d77 22/22✓     ││                                          ││ │ Sun setting behind the stage         │ │
│   20250518_7be410 12/12✓     ││                                          ││ └──────────────────────────────────────┘ │
│   20250517_0cd8e1 0/11       ││                                          ││ Tags                                     │
│                              ││                                          ││ [sunset ×] [crowd ×]                     │
│ c  mark mag complete         ││                                          ││ ┌──────────────────────────────────────┐ │
└──────────────────────────────┘│                                          ││ │ sun▏                                 │ │
╔═ 2 Clips ════════════════════╗│                                          ││ └──────────────────────────────────────┘ │
║                              ║│                                          ││   ┌────────────────────────────────┐     │
║   ✓ A001_C001.mov  ★★★★☆     ║│                                          ││   │ sunset  (already added)        │     │
║   ✓ A001_C002.mov  ★★★☆☆  SM ║│                video area                ││   │ sunrise                        │     │
║   ✓ A001_C003.mov  ★★★★★     ║│        kitty / sixel if supported        ││   │ sunglasses                     │     │
║   ✓ A001_C004.mov  ★★☆☆☆     ║│         else audio + status only         ││   └────────────────────────────────┘     │
║ ▶ ○ A001_C005.mov            ║│                                          ││   Enter picks · ↑↓ move · [create "sun"] if no match              │
║   ○ A001_C006.mov            ║│                                          ││                                          │
║   ○ A001_C007.mov            ║│                                          ││ Rating    ★★★★☆    1-5 sets              │
║   ○ A001_C008.mov            ║│                                          ││ Slow motion   [x] 120 fps                │
║   ○ A001_C009.mov            ║│                                          ││ Technical note                           │
║   ○ A001_C010.mov            ║│                                          ││ ┌──────────────────────────────────────┐ │
║   ○ A001_C011.mov            ║│                                          ││ │ audio, focus, exposure…              │ │
║                              ║│                                          ││ └──────────────────────────────────────┘ │
║                              ║│                                          ││ Shot type           wide  medium  close  │
║                              ║│                                          ││ Camera operator     [Thabo ×]            │
║                              ║│                                          ││ Director / Producer [Naledi ×]           │
║                              ║│                                          ││                                          │
║                              ║│                                          ││                                          │
║                              ║│ ▶  A001_C005.mov  320p · 24 fps · 2ch    ││                                          │
║                              ║│ █████░░░░░░░░░░░░░░░  00:12:04 / 00:41:10││                                          │
║ 4 of 11 logged               ║│ Space play · ←/→ seek 5s · Shift 30s     ││ Ctrl+Z undo · v revert from mag file     │
║ ] next unlogged  [ previous  ║│ icon: ready · pending · pulling · fallba ││ saved ✓ on field exit · syncs online     │
╚══════════════════════════════╝└──────────────────────────────────────────┘└──────────────────────────────────────────┘
 Tab · ↑↓ · Enter edit · 1-5 rate · m slow-mo · t tags · ][ unlog · c done · v revert · Esc dash
```

## Panes

| # | Pane | Notes |
|---|------|-------|
| 1 | Mags | Same pattern as the ingest Detected pane. Each mag shows its logged count and a done mark once marked complete. Selecting a mag loads its clips. |
| 2 | Clips | Clips of the selected mag with logged / unlogged marker, star rating and slow-motion flag. Next / previous unlogged clip. No filter in v1. |
| 3 | Player | mpv on the 320p proxy (or HQ). Small proxy status icon (ready / pending / pulling / local fallback). Video area uses kitty/sixel when supported, otherwise audio plus status line only. |
| 4 | Log clip | The locked field set, in order. Saved on field exit with a clear "saved" indicator. |

There is no permanent tag picker pane. Tags, camera operator and director/producer use a type-then-click popup (see below).

## Fields (locked in PLAN.md)

| Field | Widget |
|-------|--------|
| Description | Bordered text box with cursor; placeholder when empty |
| Keywords / Tags | Chips with a small x to remove; type in the field, matching tags pop up; click or Enter to add; `[create '...']` offered once when nothing matches |
| Rating | Stars 1-5 |
| Slow motion | Checkbox |
| Technical note | Bordered text box with cursor; placeholder when empty |
| Shot type | Optional clickable list |
| Camera operator | Chips with x; same popup behaviour as tags |
| Director / Producer | Chips with x; same popup behaviour as tags |

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `Enter` | Edit the field, or pick the highlighted popup entry |
| `1`-`5` | Set rating |
| `m` | Toggle slow motion |
| `t` | Jump to the tags field |
| `]` / `[` | Next / previous unlogged clip |
| `c` | Mark the whole mag "logging complete" |
| `v` | Revert from the last mag file backup |
| `Ctrl+Z` | Undo the current field |
| `Space` | Play / pause (player focused) |
| `Left` / `Right` | Seek 5 s (Shift: 30 s) when the player has focus |
| `Esc` | Return to the dashboard (shared convention) |

## Behaviour notes

- **Autosave** on field exit; explicit save when leaving the clip or the module. The header and the form show "saved".
- **Revert** loads the last backup of the mag file (`.magvault/magcard.*.backup` or equivalent log sidecar backup). The database does not keep its own backup; the mag file on disk is the recoverable copy.
- **Server:** full access to all online mags. Header shows synced state.
- **Client:** works against local staging, USB backups or recently synced mags, offline included. The header shows connection state and the number of changes pending sync, which upload when the server is reachable.
- **Proxy source on the client:** prefers server-generated 320p proxies pulled when ready; falls back to a quick temporary local proxy for a single mag when offline or when server proxies are not yet available (icon shows local fallback).
- Original filenames and sidecars are never touched. Log data lives in the database.

All sample values (mags, clips, tags, names, times, ratings) are placeholders.
