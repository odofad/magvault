# Export bins browser mockup (ratatui)

> **Mockup only.** Design reference for the Export bins browser. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values (export names, projects, dates, clips) are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Export bins", "exportcard.txt", "Export bins browser"). This is a **read-only** viewer for existing exports under the `bins/` path. There is no editing and no delete in this screen. Exports are found by walking `bins/` and reading each `exportcard.txt`; the folders are the record.

Panes: 1 Exports (name, project, date, status), 2 Export card (card fields, then clips grouped by source mag folder with optional in/out and a per-clip status), 3 On disk (the export folder as it exists: `exportcard.txt`, `exportcard.txt.backup`, and the original-named mag subfolders with copied clips).

Status values are the four in PLAN.md: **warm, cold, copying, done**. The sample export `Bamba-Cutaways` has a mix: two clips done or copying from a warm mag, one warm clip and two cold clips in the second mag, so a cold pull is still needed for it.

## Keys

| key | action |
|---|---|
| Tab / Shift+Tab | cycle panes |
| Up / Down | move in the list |
| Enter | open the selected export (focus the card and file panes) |
| `e` or `l` | jump to the Library module with this export open for editing |
| Esc | back to dashboard |

`e` / `l` come from PLAN.md (`l` matches the dashboard letter for Library; `e` matches the dashboard letter for Export bins itself, so here it means "edit"). The other keys follow the shared convention.

## Export bins, 150x42

```text
 MAGVAULT  ·  Export bins                                                                                                          read-only · server
╔═ 1 Exports (bins/) ══════════════════════════════════════════════╗┌─ 2 Export card · Bamba-Cutaways ───────────────────────────────────────────────┐
║                                                                  ║│                                                                                │
║   name              project             date       status        ║│ name          Bamba-Cutaways                                                   │
║ ▶ Bamba-Cutaways    Bamba_Cutaways      2026-11-22 cold          ║│ project_name  Bamba_Cutaways                                                   │
║   Karoo-Selects     Karoo_Doc_2024      2026-11-20 copying       ║│ created       2026-11-22T14:30:00 by odofad                                    │
║   ACME-Teaser-v2    ACME_Campaign_2025  2026-11-18 done          ║│ updated       2026-11-22T16:15:00                                              │
║   ACME-Alt-Takes    ACME_Campaign_2025  2026-11-10 warm          ║│ notes         rough cut cutaways                                               │
║   Studio-Promo-Cut  Studio_Promo        2026-11-02 done          ║│                                                                                │
║                                                                  ║│ Clips                                                                          │
║ status: warm · cold · copying · done                             ║│   original_filename in        out       status                                 │
║                                                                  ║│ 20261118_abc123                                                                │
║                                                                  ║│   A001_C001.mov     ·         ·         done                                   │
║                                                                  ║│   A001_C002.mov     00:01:12  00:01:45  done                                   │
║                                                                  ║│   A001_C003.mov     ·         ·         copying                                │
║                                                                  ║│ 20261120_f4e219                                                                │
║                                                                  ║│   B002_C004.mov     ·         ·         warm                                   │
║                                                                  ║│   B002_C005.mov     00:00:20  00:00:48  cold                                   │
║                                                                  ║│   B002_C006.mov     ·         ·         cold                                   │
║                                                                  ║│ Export status: cold                                                            │
║                                                                  ║│ clips: 2 done · 1 copying · 1 warm · 2 cold                                    │
║                                                                  ║│                                                                                │
║                                                                  ║│                                                                                │
║                                                                  ║│                                                                                │
║                                                                  ║│                                                                                │
║                                                                  ║│                                                                                │
║                                                                  ║└────────────────────────────────────────────────────────────────────────────────┘
║                                                                  ║┌─ 3 On disk ────────────────────────────────────────────────────────────────────┐
║                                                                  ║│ bins/Bamba-Cutaways/                                                           │
║                                                                  ║│   exportcard.txt                                                               │
║                                                                  ║│   exportcard.txt.backup                                                        │
║                                                                  ║│   20261118_abc123/                                                             │
║                                                                  ║│     A001_C001.mov                                                              │
║                                                                  ║│     A001_C002.mov                                                              │
║                                                                  ║│     A001_C003.mov (copying)                                                    │
║                                                                  ║│   20261120_f4e219/  (no files yet)                                             │
║ Read-only. No editing and no delete here.                        ║│                                                                                │
║ Exports are found by walking bins/ and                           ║│                                                                                │
║ reading each exportcard.txt.                                     ║│                                                                                │
║                                                                  ║│ e / l  open in Library for editing                                             │
╚══════════════════════════════════════════════════════════════════╝└────────────────────────────────────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter open · e / l edit in Library · Esc dashboard
```

## Export bins, 120x36 (minimum)

```text
 MAGVAULT  ·  Export bins                                                                            read-only · server
╔═ 1 Exports (bins/) ════════════════════════════════╗┌─ 2 Export card · Bamba-Cutaways ───────────────────────────────┐
║                                                    ║│                                                                │
║   name             project      date       status  ║│ name          Bamba-Cutaways                                   │
║ ▶ Bamba-Cutaways   Bamba_Cutaw… 2026-11-22 cold    ║│ project_name  Bamba_Cutaways                                   │
║   Karoo-Selects    Karoo_Doc_2… 2026-11-20 copying ║│ created       2026-11-22T14:30:00 by odofad                    │
║   ACME-Teaser-v2   ACME_Campai… 2026-11-18 done    ║│ updated       2026-11-22T16:15:00                              │
║   ACME-Alt-Takes   ACME_Campai… 2026-11-10 warm    ║│ notes         rough cut cutaways                               │
║   Studio-Promo-Cut Studio_Promo 2026-11-02 done    ║│                                                                │
║                                                    ║│ Clips                                                          │
║ status: warm · cold · copying · done               ║│   original_filename in        out       status                 │
║                                                    ║│ 20261118_abc123                                                │
║                                                    ║│   A001_C001.mov     ·         ·         done                   │
║                                                    ║│   A001_C002.mov     00:01:12  00:01:45  done                   │
║                                                    ║│   A001_C003.mov     ·         ·         copying                │
║                                                    ║│ 20261120_f4e219                                                │
║                                                    ║│   B002_C004.mov     ·         ·         warm                   │
║                                                    ║│   B002_C005.mov     00:00:20  00:00:48  cold                   │
║                                                    ║│   B002_C006.mov     ·         ·         cold                   │
║                                                    ║│ Export status: cold                                            │
║                                                    ║│ clips: 2 done · 1 copying · 1 warm · 2 cold                    │
║                                                    ║│                                                                │
║                                                    ║└────────────────────────────────────────────────────────────────┘
║                                                    ║┌─ 3 On disk ────────────────────────────────────────────────────┐
║                                                    ║│ bins/Bamba-Cutaways/                                           │
║                                                    ║│   exportcard.txt                                               │
║                                                    ║│   exportcard.txt.backup                                        │
║                                                    ║│   20261118_abc123/                                             │
║                                                    ║│     A001_C001.mov                                              │
║                                                    ║│     A001_C002.mov                                              │
║ Read-only. No editing and no delete here.          ║│     A001_C003.mov (copying)                                    │
║ Exports are found by walking bins/ and             ║│   20261120_f4e219/  (no files yet)                             │
║ reading each exportcard.txt.                       ║│                                                                │
║                                                    ║│ e / l  open in Library for editing                             │
╚════════════════════════════════════════════════════╝└────────────────────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter open · e / l edit in Library · Esc dashboard
```

## Open points for the plan

- **Export-level status rule.** PLAN.md lists the four statuses "per clip or per export" but does not say how an export's status is derived from its clips. The mockup shows `cold` when any clip is cold; the rule needs deciding.
- **Date column.** The list shows the `created` date from the card. PLAN.md says "date" without naming which card field.
- **Status of finished-looking exports.** PLAN.md does not say whether `done` means copied and hash/MHL-verified; the mockup treats it as all clips copied.
- **Sidecars.** The On disk pane lists only the clip files; sidecar names are not specified in PLAN.md.
