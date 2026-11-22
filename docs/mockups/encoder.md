# Encoder module mockup (ratatui)

> **Mockup only.** Design reference for the Encoder module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Encoder module"). Encoding is mostly automatic (a successful ingest queues a 320p job on the server). This module is for watching, managing and overriding.

## Queue, 150x42

```text
 MAGVAULT  ·  Encoder  ·  queue                                                                             queue RUNNING  ·  depth 6  ·  odofad
╔═ 1 Active jobs ════════════════════════════════════════════════════════════════════════════════════════════╗┌─ 4 Selected job ─────────────────────┐
║                                                                                                            ║│                                      │
║ ▶ 320p 20250520_a3f9c2  ACME_Campaign_2025  default-320p                                         00:04:12  ║│ Job      custom #3                   │
║   ████████████████████████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 62%     ║│ Mag      20250412_55d0aa             │
║ ▶ HQ   20250519_c41d77  ACME_Campaign_2025  hevc-hq                                              00:11:40  ║│ Project  Bamba_Cutaways              │
║   ███████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 28%     ║│ Files    4 of 11                     │
╚════════════════════════════════════════════════════════════════════════════════════════════════════════════╝│ Profile  client-review               │
┌─ 2 Waiting ────────────────────────────────────────────────────────────────────────────────────────────────┐│ Output   custom_encode/              │
│ #  type  mag              project / file set            profile        prio                                ││            20250412_55d0aa/          │
│ 1  320p  20250519_c41d77   ACME_Campaign_2025            default-320p   norm                               ││            client-review/            │
│ 2  320p  20250517_0cd8e1   Bamba_Cutaways                default-320p   norm                               ││ Priority high                        │
│ 3  cust  20250412_55d0aa   Bamba_Cutaways · 4 files      client-review  high                               ││ Queued   18:40                       │
│ 4  HQ    20250517_0cd8e1   Bamba_Cutaways                hevc-hq        low                                ││ Status   waiting                     │
│                                                                                                            ││                                      │
│                                                                                                            ││ Not a Magvault proxy.                │
│                                                                                                            ││ Not logged as an asset.              │
│                                                                                                            ││                                      │
│                                                                                                            ││                                      │
│                                                                                                            ││                                      │
│                                                                                                            ││                                      │
│                                                                                                            ││                                      │
│                                                                                                            │└──────────────────────────────────────┘
│                                                                                                            │┌─ 5 Profiles ─────────────────────────┐
│                                                                                                            ││ Active defaults                      │
│                                                                                                            ││ 320p  default-320p                   │
│                                                                                                            ││ HQ    hevc-hq                        │
│                                                                                                            ││                                      │
│                                                                                                            ││ Rules: aspect kept,                  │
│                                                                                                            ││ exact fps, no audio                  │
│                                                                                                            ││ downmix.                             │
│                                                                                                            ││                                      │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────┘│ e  edit profiles                     │
┌─ 3 Failed ─────────────────────────────────────────────────────────────────────────────────────────────────┐│                                      │
│ ✖ 320p  20250518_7be410   GX010412.MP4    source file missing                                              ││                                      │
│ ✖ cust  20250410_11ab90   5 files         bad profile: client-review-v0                                    ││                                      │
│ ✖ 320p  20250408_9e0042   A004_C012.mov   ffmpeg exit 1 · no space left                                    ││                                      │
│                                                                                                            ││                                      │
│                                                                                                            ││                                      │
│ R retry   C cancel   shows first error line; full ffmpeg log in Logs                                       ││                                      │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────┘└──────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  n new job  ·  p pause/resume  ·  r retry  ·  c cancel  ·  u re-queue  ·  +/- priority  ·  e profiles  ·  Esc dashboard
```

## Queue, 120x36 (minimum)

```text
 MAGVAULT  ·  Encoder  ·  queue                                               queue RUNNING  ·  depth 6  ·  odofad
╔═ 1 Active jobs ════════════════════════════════════════════════════════════════════╗┌─ 4 Selected job ───────────────┐
║                                                                                    ║│                                │
║ ▶ 320p 20250520_a3f9c2  default-320p                                     00:04:12  ║│ Job      custom #3             │
║   █████████████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░ 62%     ║│ Mag      20250412_55d0aa       │
║ ▶ HQ   20250519_c41d77  hevc-hq                                          00:11:40  ║│ Project  Bamba_Cutaways        │
║   ████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 28%     ║│ Files    4 of 11               │
╚════════════════════════════════════════════════════════════════════════════════════╝│ Profile  client-review         │
┌─ 2 Waiting ────────────────────────────────────────────────────────────────────────┐│ Output   custom_encode/        │
│ #  type  mag              project / file set            profile        prio        ││            20250412_55d0aa/    │
│ 1  320p  20250519_c41d77   ACME_Campaign_2025            default-320p   norm       ││            client-review/      │
│ 2  320p  20250517_0cd8e1   Bamba_Cutaways                default-320p   norm       ││ Priority high                  │
│ 3  cust  20250412_55d0aa   Bamba_Cutaways · 4 files      client-review  high       ││ Queued   18:40                 │
│ 4  HQ    20250517_0cd8e1   Bamba_Cutaways                hevc-hq        low        ││ Status   waiting               │
│                                                                                    ││                                │
│                                                                                    ││ Not a Magvault proxy.          │
│                                                                                    ││ Not logged as an asset.        │
│                                                                                    ││                                │
│                                                                                    │└────────────────────────────────┘
│                                                                                    │┌─ 5 Profiles ───────────────────┐
│                                                                                    ││ Active defaults                │
│                                                                                    ││ 320p  default-320p             │
│                                                                                    ││ HQ    hevc-hq                  │
│                                                                                    ││                                │
│                                                                                    ││ Rules: aspect kept,            │
│                                                                                    ││ exact fps, no audio            │
│                                                                                    ││ downmix.                       │
└────────────────────────────────────────────────────────────────────────────────────┘│                                │
┌─ 3 Failed ─────────────────────────────────────────────────────────────────────────┐│ e  edit profiles               │
│ ✖ 320p  20250518_7be410   GX010412.MP4    source file missing                      ││                                │
│ ✖ cust  20250410_11ab90   5 files         bad profile: client-review-v0            ││                                │
│ ✖ 320p  20250408_9e0042   A004_C012.mov   ffmpeg exit 1 · no space left            ││                                │
│                                                                                    ││                                │
│ R retry   C cancel   shows first error line; full ffmpeg log in Logs               ││                                │
└────────────────────────────────────────────────────────────────────────────────────┘└────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  n new job  ·  p pause/resume  ·  r retry  ·  c cancel  ·  u re-queue  ·  +/- priority  ·  e pr
```

## New job (ad-hoc), 150x42

```text
 MAGVAULT  ·  Encoder  ·  new job (ad-hoc)                                                                              step 1-3 of 3  ·  odofad
╔═ 1 Mag ══════════════════════════════════╗┌─ 2 Files ──────────────────────────────────────────────────────┐┌─ 3 Profile ──────────────────────────┐
║                                          ║│                                                                ││                                      │
║ filter project                           ║│ [x] whole mag   [ ] pick files                                 ││   default-320p                       │
║  ACME_Campaign_2025                      ║│                                                                ││   hevc-hq                            │
║  Bamba_Cutaways                          ║│ [x] A001_C001_0412.mov                                1.8 GB   ││ ▶ client-review                      │
║                                          ║│ [x] A001_C002_0412.mov                                2.1 GB   ││   prores-proxy                       │
║ online mags                              ║│ [x] A001_C003_0412.mov                                1.1 GB   ││                                      │
║   20250520_a3f9c2  38 files              ║│ [x] A001_C004_0412.mov                                3.4 GB   ││ client-review                        │
║   20250519_c41d77  22 files              ║│ [ ] A001_C005_0412.mov                                0.9 GB   ││ H.264 · 1080p · 8 Mb/s               │
║   20250518_7be410  12 files              ║│ [ ] A001_C006_0412.mov                                1.3 GB   ││ aspect kept · fps exact              │
║   20250517_0cd8e1  11 files              ║│                                                                ││ audio: all channels kept             │
║ ▶ 20250412_55d0aa  11 files              ║│ 4 of 11 selected · 8.4 GB                                      ││                                      │
║                                          ║│                                                                ││ e  edit in Profile Editor            │
║ Online mags only.                        ║│ Space toggle   a all   n none                                  ││                                      │
║ Closed projects hidden.                  ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
║                                          ║│                                                                ││                                      │
╚══════════════════════════════════════════╝└────────────────────────────────────────────────────────────────┘└──────────────────────────────────────┘
┌─ Destination (fixed) and summary ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                                                                                                                                    │
│ Destination   custom_encode/20250412_55d0aa/client-review/                                                                                         │
│ Job           4 files · client-review · priority normal                                                                        Enter  START JOB    │
│               Pure encoder output. Not written to proxies/, not linked, not logged as an asset.                                                    │
│               [!] 8.4 GB source · about 2.1 GB output · 214 GB free                                                            Esc  cancel         │
│                                                                                                                                                    │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Space toggle  ·  Enter start job (queued)  ·  e edit profiles  ·  Esc back to queue
```

## New job (ad-hoc), 120x36 (minimum)

```text
 MAGVAULT  ·  Encoder  ·  new job (ad-hoc)                                                step 1-3 of 3  ·  odofad
╔═ 1 Mag ════════════════════════╗┌─ 2 Files ──────────────────────────────────────────┐┌─ 3 Profile ──────────────────┐
║                                ║│                                                    ││                              │
║ filter project                 ║│ [x] whole mag   [ ] pick files                     ││   default-320p               │
║  ACME_Campaign_2025            ║│                                                    ││   hevc-hq                    │
║  Bamba_Cutaways                ║│ [x] A001_C001_0412.mov                    1.8 GB   ││ ▶ client-review              │
║                                ║│ [x] A001_C002_0412.mov                    2.1 GB   ││   prores-proxy               │
║ online mags                    ║│ [x] A001_C003_0412.mov                    1.1 GB   ││                              │
║   20250520_a3f9c2  38 files    ║│ [x] A001_C004_0412.mov                    3.4 GB   ││ client-review                │
║   20250519_c41d77  22 files    ║│ [ ] A001_C005_0412.mov                    0.9 GB   ││ H.264 · 1080p · 8 Mb/s       │
║   20250518_7be410  12 files    ║│ [ ] A001_C006_0412.mov                    1.3 GB   ││ aspect kept · fps exact      │
║   20250517_0cd8e1  11 files    ║│                                                    ││ audio: all channels kept     │
║ ▶ 20250412_55d0aa  11 files    ║│ 4 of 11 selected · 8.4 GB                          ││                              │
║                                ║│                                                    ││ e  edit in Profile Editor    │
║ Online mags only.              ║│ Space toggle   a all   n none                      ││                              │
║ Closed projects hidden.        ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
║                                ║│                                                    ││                              │
╚════════════════════════════════╝└────────────────────────────────────────────────────┘└──────────────────────────────┘
┌─ Destination (fixed) and summary ────────────────────────────────────────────────────────────────────────────────────┐
│                                                                                                                      │
│ Destination   custom_encode/20250412_55d0aa/client-review/                                                           │
│ Job           4 files · client-review · priority normal                                          Enter  START JOB    │
│               Pure encoder output. Not written to proxies/, not linked, not logged as an asset.                      │
│               [!] 8.4 GB source · about 2.1 GB output · 214 GB free                              Esc  cancel         │
│                                                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
 Tab pane  ·  ↑↓ move  ·  Space toggle  ·  Enter start job (queued)  ·  e edit profiles  ·  Esc back to queue
```

## Queue panes

| # | Pane | Notes |
|---|------|-------|
| 1 | Active jobs | Progress bar, mag, type (320p / HQ / custom), profile, elapsed. |
| 2 | Waiting | Order, type, mag, project or file set, profile, priority. |
| 3 | Failed | One clear error reason per job (ffmpeg exit, disk full, missing source, bad profile). Full ffmpeg log lives in Logs. |
| 4 | Selected job | Detail for the highlighted row. For custom jobs it shows the fixed output folder and states the output is not a Magvault proxy and not a logged asset. |
| 5 | Profiles | Active default profile per type, proxy rules reminder, jump to the Profile Editor. |

## New job screen

Three steps shown as panes, left to right, with a fixed summary below.

1. **Mag**: online mags only, filterable by project (closed projects hidden).
2. **Files**: whole mag or pick files, with selected count and size.
3. **Profile**: choose from existing encoder profiles; shows what the profile does.

Destination is **fixed** to `custom_encode/<YYYYMMDD>_<mag-uuid>/<profile-name>/` (root from `path.conf`). Output is not written to `proxies/db/` or `proxies/hq/`, not linked as an official proxy and not logged as a Magvault asset. The summary shows source size, estimated output size and free space, with a warning if space is tight.

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list |
| `n` | New job screen |
| `p` | Pause / resume the queue |
| `r` | Retry the selected failed job |
| `c` | Cancel the selected job |
| `u` | Re-queue |
| `+` / `-` | Raise / lower priority |
| `e` | Jump to the Profile Editor for encoder profiles |
| `Space` | (new job) toggle a file |
| `Enter` | (new job) start job, which is queued |
| `Esc` | Back (new job to queue, queue to dashboard) |

`q` is deliberately not used for re-queue because `Esc`/`q` means back everywhere else.

## Behaviour notes

- Strict proxy rules still apply: aspect preserved, exact frame rate, no audio downmix.
- The dashboard only shows queue depth and active progress. Full management lives here.
- Queue state (running / paused) and depth are in the header.
- All sample values (mags, projects, profiles, sizes, times) are placeholders.
