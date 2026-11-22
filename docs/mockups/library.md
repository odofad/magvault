# Library module mockup (ratatui)

> **Mockup only.** Design reference for the Library module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Library module", "Export bins", "exportcard.txt"). The module is available on both server and client with the same four-pane layout.

## Server library, 150x42

```text
 MAGVAULT  ·  Library                                                                                                                 server · odofad
┌─ 1 Projects ───────────┐┌─ 2 Mags ───────────────────┐╔═ 3 Clips ════════════════════════════════╗┌─ 4 Player and export list ─────────────────────┐
│                        ││                            │║                                          ║│                                                │
│   ACME_Campaign_2025   ││ ▶ 20250517_0cd8e1    warm  │║                                          ║│                                                │
│   open                 ││   20250412_55d0aa    cold  │║   A001_C001.mov                          ║│                                                │
│ ▶ Bamba_Cutaways       ││   20250410_11ab90    warm  │║  +A001_C002.mov                          ║│                   video area                   │
│   open                 ││                            │║ ▶ A001_C003.mov                          ║│                   320p proxy                   │
│   Karoo_Doc_2024       ││                            │║  +A001_C004.mov                          ║│                                                │
│   closed               ││                            │║   A001_C005.mov                          ║│                                                │
│   Studio_Promo         ││                            │║   A001_C006.mov                          ║│                                                │
│   closed               ││                            │║   A001_C007.mov                          ║│ ▶  A001_C003.mov                               │
│                        ││                            │║   A001_C008.mov                          ║│ ████████░░░░░░░░░░░░░░░░░░  00:12 / 00:36      │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│ ────────────────────────────────────────────── │
│                        ││                            │║                                          ║│ Export list (exportcard.txt)                   │
│                        ││                            │║                                          ║│ name  Bamba-Cutaways                           │
│                        ││                            │║                                          ║│ mag / clip                        in     out   │
│                        ││                            │║                                          ║│ 20250517_0cd8e1                           warm │
│                        ││                            │║                                          ║│   A001_C002.mov                                │
│                        ││                            │║                                          ║│   A001_C004.mov                   01:12  01:45 │
│                        ││                            │║                                          ║│ 20250412_55d0aa                           cold │
│                        ││                            │║                                          ║│   B002_C010.mov                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│ card updated on every add                      │
│                        ││                            │║                                          ║│ previous version kept as .backup               │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│ e starts the media copy.                       │
│                        ││                            │║                                          ║│ Cold clips create a cold-pull                  │
│                        ││                            │║                                          ║│ request for the Zmag manager.                  │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││ warm = online on server    │║ + already on the export list             ║│                                                │
│ open + closed shown    ││ cold = on a zmag only      │║ a add to list                            ║│                                                │
└────────────────────────┘└────────────────────────────┘╚══════════════════════════════════════════╝└────────────────────────────────────────────────┘
 Status   warm 2   cold 1   pulling 0
 Tab pane · ↑↓ move · Enter select · a add to list · e export · Esc dashboard
```

## Server library, 120x36 (minimum)

```text
 MAGVAULT  ·  Library                                                                                   server · odofad
┌─ 1 Projects ─────────┐┌─ 2 Mags ─────────────────┐╔═ 3 Clips ══════════════════╗┌─ 4 Player and export list ─────────┐
│                      ││                          │║                            ║│                                    │
│   ACME_Campaign_2025 ││ ▶ 20250517_0cd8e1  warm  │║                            ║│                                    │
│   open               ││   20250412_55d0aa  cold  │║   A001_C001.mov            ║│             video area             │
│ ▶ Bamba_Cutaways     ││   20250410_11ab90  warm  │║  +A001_C002.mov            ║│             320p proxy             │
│   open               ││                          │║ ▶ A001_C003.mov            ║│                                    │
│   Karoo_Doc_2024     ││                          │║  +A001_C004.mov            ║│                                    │
│   closed             ││                          │║   A001_C005.mov            ║│ ▶  A001_C003.mov                   │
│   Studio_Promo       ││                          │║   A001_C006.mov            ║│ ████░░░░░░░░░░  00:12 / 00:36      │
│   closed             ││                          │║   A001_C007.mov            ║│                                    │
│                      ││                          │║   A001_C008.mov            ║│ ────────────────────────────────── │
│                      ││                          │║                            ║│ Export list (exportcard.txt)       │
│                      ││                          │║                            ║│ name  Bamba-Cutaways               │
│                      ││                          │║                            ║│ mag / clip            in     out   │
│                      ││                          │║                            ║│ 20250517_0cd8e1               warm │
│                      ││                          │║                            ║│   A001_C002.mov                    │
│                      ││                          │║                            ║│   A001_C004.mov       01:12  01:45 │
│                      ││                          │║                            ║│ 20250412_55d0aa               cold │
│                      ││                          │║                            ║│   B002_C010.mov                    │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│ card updated on every add          │
│                      ││                          │║                            ║│ previous version kept as .backup   │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│ e starts the media copy.           │
│                      ││                          │║                            ║│ Cold clips create a cold-pull      │
│                      ││                          │║                            ║│ request for the Zmag manager.      │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│                                    │
│                      ││ warm = online on server  │║ + already on the export list│                                    │
│ open + closed shown  ││ cold = on a zmag only    │║ a add to list              ║│                                    │
└──────────────────────┘└──────────────────────────┘╚════════════════════════════╝└────────────────────────────────────┘
 Status   warm 2   cold 1   pulling 0
 Tab pane · ↑↓ move · Enter select · a add to list · e export · Esc dashboard
```

## Client library, 150x42

```text
 MAGVAULT  ·  Library (client)                                                                                              ● server syncing · odofad
┌─ 1 Projects ───────────┐┌─ 2 Mags ───────────────────┐╔═ 3 Clips ════════════════════════════════╗┌─ 4 Player and export list ─────────────────────┐
│                        ││                            │║                                          ║│                                                │
│   ACME_Campaign_2025   ││ ▶ 20250517_0cd8e1    warm  │║   clip                        proxy      ║│                                                │
│   open                 ││   20250412_55d0aa    cold  │║   A001_C001.mov               ● local    ║│                                                │
│ ▶ Bamba_Cutaways       ││   20250410_11ab90    warm  │║  +A001_C002.mov               ● local    ║│                   video area                   │
│   open                 ││                            │║ ▶ A001_C003.mov               ○ missing  ║│                   320p proxy                   │
│   Karoo_Doc_2024       ││                            │║  +A001_C004.mov               ◐ pulling  ║│                                                │
│   closed               ││                            │║   A001_C005.mov               ○ missing  ║│                                                │
│   Studio_Promo         ││                            │║   A001_C006.mov               ● local    ║│                                                │
│   closed               ││                            │║   A001_C007.mov               ● local    ║│ ▶  A001_C003.mov                               │
│                        ││                            │║   A001_C008.mov               ● local    ║│ ████████░░░░░░░░░░░░░░░░░░  00:12 / 00:36      │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│ ────────────────────────────────────────────── │
│                        ││                            │║                                          ║│ Export list (exportcard.txt)                   │
│                        ││                            │║                                          ║│ name  Bamba-Cutaways                           │
│                        ││                            │║                                          ║│ mag / clip                        in     out   │
│                        ││                            │║                                          ║│ 20250517_0cd8e1                           warm │
│                        ││                            │║                                          ║│   A001_C002.mov                                │
│                        ││                            │║                                          ║│   A001_C004.mov                   01:12  01:45 │
│                        ││                            │║                                          ║│ 20250412_55d0aa                        pulling │
│                        ││                            │║                                          ║│   B002_C010.mov                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│ card updated on every add                      │
│                        ││                            │║                                          ║│ previous version kept as .backup               │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│ Pending request                                │
│                        ││                            │║                                          ║│ Export sent to server.                         │
│                        ││                            │║                                          ║│ Waiting for cold pull of 55d0aa.               │
│                        ││                            │║                                          ║│ Finished files can be pulled                   │
│                        ││                            │║                                          ║│ when the server is done.                       │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││                            │║                                          ║│                                                │
│                        ││ warm = online on server    │║ + already on the export list             ║│                                                │
│ open + closed shown    ││ cold = on a zmag only      │║ a add to list                            ║│                                                │
└────────────────────────┘└────────────────────────────┘╚══════════════════════════════════════════╝└────────────────────────────────────────────────┘
 Status   warm 2   cold 0   pulling 1      proxies: pulling 1 · missing 2      export request pending
 Tab pane · ↑↓ move · Enter select · a add to list · e export · p pull proxies · Esc dashboard
```

## Client library, 120x36 (minimum)

```text
 MAGVAULT  ·  Library (client)                                                                ● server syncing · odofad
┌─ 1 Projects ─────────┐┌─ 2 Mags ─────────────────┐╔═ 3 Clips ══════════════════╗┌─ 4 Player and export list ─────────┐
│                      ││                          │║                            ║│                                    │
│   ACME_Campaign_2025 ││ ▶ 20250517_0cd8e1  warm  │║   clip          proxy      ║│                                    │
│   open               ││   20250412_55d0aa  cold  │║   A001_C001.mov ● local    ║│             video area             │
│ ▶ Bamba_Cutaways     ││   20250410_11ab90  warm  │║  +A001_C002.mov ● local    ║│             320p proxy             │
│   open               ││                          │║ ▶ A001_C003.mov ○ missing  ║│                                    │
│   Karoo_Doc_2024     ││                          │║  +A001_C004.mov ◐ pulling  ║│                                    │
│   closed             ││                          │║   A001_C005.mov ○ missing  ║│ ▶  A001_C003.mov                   │
│   Studio_Promo       ││                          │║   A001_C006.mov ● local    ║│ ████░░░░░░░░░░  00:12 / 00:36      │
│   closed             ││                          │║   A001_C007.mov ● local    ║│                                    │
│                      ││                          │║   A001_C008.mov ● local    ║│ ────────────────────────────────── │
│                      ││                          │║                            ║│ Export list (exportcard.txt)       │
│                      ││                          │║                            ║│ name  Bamba-Cutaways               │
│                      ││                          │║                            ║│ mag / clip            in     out   │
│                      ││                          │║                            ║│ 20250517_0cd8e1               warm │
│                      ││                          │║                            ║│   A001_C002.mov                    │
│                      ││                          │║                            ║│   A001_C004.mov       01:12  01:45 │
│                      ││                          │║                            ║│ 20250412_55d0aa            pulling │
│                      ││                          │║                            ║│   B002_C010.mov                    │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│ card updated on every add          │
│                      ││                          │║                            ║│ previous version kept as .backup   │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│ Pending request                    │
│                      ││                          │║                            ║│ Export sent to server.             │
│                      ││                          │║                            ║│ Waiting for cold pull of 55d0aa.   │
│                      ││                          │║                            ║│ Finished files can be pulled       │
│                      ││                          │║                            ║│ when the server is done.           │
│                      ││                          │║                            ║│                                    │
│                      ││                          │║                            ║│                                    │
│                      ││ warm = online on server  │║ + already on the export list│                                    │
│ open + closed shown  ││ cold = on a zmag only    │║ a add to list              ║│                                    │
└──────────────────────┘└──────────────────────────┘╚════════════════════════════╝└────────────────────────────────────┘
 Status   warm 2   cold 0   pulling 1      proxies: pulling 1 · missing 2      export request pending
 Tab pane · ↑↓ move · Enter select · a add to list · e export · p pull proxies · Esc dashboard
```

The server screens show an export list that includes a cold clip, so the cold-pull behaviour is visible. The client screens show the same list after Export, with a pending request and the cold clip being pulled.

## Panes (server and client)

| # | Pane | Notes |
|---|------|-------|
| 1 | Projects | Projects with their open / closed status. The library is the one place closed projects appear. |
| 2 | Mags | Mags of the selected project. Each is warm (online on the server) or cold (on a zmag only). |
| 3 | Clips | Clips of the selected mag. A `+` marks clips already on the export list. On the client each clip also shows its proxy status: local / missing / pulling. |
| 4 | Player and export list | The player (320p proxy) above the export list. The export list is the export card (`exportcard.txt`): name, then clips grouped by mag folder, with optional in / out points. |

A status strip at the bottom shows warm / cold / pulling for the clips on the export list.

## Export list (the export card)

- The list the operator sees is `exportcard.txt`. The program uses the card to know which mags and clips to pull.
- Add to list updates the card immediately. The current card is first copied to `exportcard.txt.backup` (single previous version).
- Export starts the media copy into the export bin. If any clip is cold, a cold-pull request is created for the Zmag manager.
- Card fields shown: name, mag folder, original filename, optional in / out.

## Client differences

- Keeps a local cache of 320p proxies (path in the client `path.conf`, for example `client-proxies/`).
- Clip list shows proxy status: local / missing / pulling.
- Pull proxies fetches missing 320p proxies for the mags being browsed (also automatic on connect).
- Export creates a request on the server. When the server finishes, including any cold pull, the client can pull the finished files.
- Offline: browse recently synced mags and build a list. Export is queued until the client is connected.

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `Enter` | Select the project / mag / clip |
| `a` | Add the selected clip to the export list |
| `e` | Export (start the media copy, or create the request on a client) |
| `p` | Pull proxies (client only) |
| `Esc` | Return to the dashboard (shared convention) |

All sample values (projects, mags, clips, in / out points, proxy states) are placeholders.
