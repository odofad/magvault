# Profile Editor mockup (ratatui)

> **Mockup only.** Design reference for the admin-only Profile Editor. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Camera profiles", "Encoder profiles", "Profile Editor module", "User model"). The module is visible only when running as the `magvault` user. Normal operators and clients never see it. Files on disk stay the source of truth.

`Esc` returns to the dashboard. If there are unsaved changes, Magvault prompts “Discard changes?” before leaving.

## Camera profile, 150x42

```text
 MAGVAULT  ·  Profile Editor  ·  camera profiles                                                                admin (magvault) · unsaved changes
┌─ 1 Type ───────────┐┌─ 2 cameras/ ───────────────┐╔═ 3 Edit sony-a7siii.cam ═════════════════════════════╗┌─ 4 File on disk / checks ──────────────┐
│                    ││                            │║                                                      ║│ name: Sony A7S III                     │
│ ▶ Camera           ││ ▶ sony-a7siii.cam          │║ name                 Sony A7S III                    ║│ id: sony-a7siii                        │
│                    ││   red-komodo.cam           │║ id                   sony-a7siii                     ║│                                        │
│   Encoder          ││   arri-alexa-mini.cam      │║                                                      ║│ vendor: Sony                           │
│                    ││   bmpcc-6k.cam             │║ vendor               Sony                            ║│ model: A7S III                         │
│                    ││   canon-c70.cam            │║ model                A7S III                         ║│ card_layout: DCIM, PRIVATE             │
│ future types       ││   generic-dcim.cam         │║ card_layout          DCIM, PRIVATE                   ║│ filename_pattern: A001_C*.mov          │
│ appear here        ││                            │║ filename_pattern     A001_C*.mov                     ║│                                        │
│                    ││                            │║                                                      ║│ file_formats: ProRes RAW, XAVC S, XAVC │
│                    ││                            │║ file_formats         ProRes RAW, XAVC S, XAVC HS     ║│ default_format: ProRes RAW             │
│                    ││                            │║ default_format       ProRes RAW                      ║│ gamma: S-Log3, S-Log2, S-Cinetone, Rec │
│                    ││                            │║ gamma                S-Log3, S-Log2, S-Cinetone, Re  ║│ default_gamma: S-Log3                  │
│                    ││                            │║ default_gamma        S-Log3▏                         ║│ gamut: S-Gamut3.Cine, S-Gamut3, Rec.70 │
│                    ││                            │║ gamut                S-Gamut3.Cine, S-Gamut3, Rec.7  ║│ default_gamut: S-Gamut3.Cine           │
│                    ││                            │║ default_gamut        S-Gamut3.Cine                   ║│                                        │
│                    ││                            │║                                                      ║│ typical_resolutions: 3840x2160, 1920x1 │
│                    ││                            │║ typical_resolutions  3840x2160, 1920x1080            ║│ typical_fps: 24, 25, 30, 50, 60, 120   │
│                    ││                            │║ typical_fps          24, 25, 30, 50, 60, 120         ║│ audio_channels: 2, 4                   │
│                    ││                            │║ audio_channels       2, 4                            ║│ notes: Full-frame cinema camera.       │
│                    ││                            │║ notes                Full-frame cinema camera.       ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│ ────────────────────────────────────── │
│                    ││                            │║                                                      ║│ Checks                                 │
│                    ││ n new                      │║                                                      ║│ ✖ default_gamma must be one of         │
│                    ││ c duplicate                │║                                                      ║│   gamma list: S-Log3, S-Log2…          │
│                    ││ x delete (asks)            │║                                                      ║│ ✓ required fields present              │
│                    ││ Ctrl+S save                │║                                                      ║│ ✓ id matches filename                  │
│ r reload disk      ││                            │║                                                      ║│ Save blocked until valid.              │
│                    ││                            │║ Tab next field · Enter edit · Esc dashboard          ║│                                        │
└────────────────────┘└────────────────────────────┘╚══════════════════════════════════════════════════════╝└────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · n new · c duplicate · x delete · r reload · Ctrl+S save · Esc dashboard
```

## Camera profile, 120x36 (minimum)

```text
 MAGVAULT  ·  Profile Editor  ·  camera profiles                                  admin (magvault) · unsaved changes
┌─ 1 Type ──────┐┌─ 2 cameras/ ───────────┐╔═ 3 Edit sony-a7siii.cam ══════════════════╗┌─ 4 File on disk / checks ────┐
│               ││                        │║                                           ║│ name: Sony A7S III           │
│ ▶ Camera      ││ ▶ sony-a7siii.cam      │║ name                Sony A7S III          ║│ id: sony-a7siii              │
│               ││   red-komodo.cam       │║ id                  sony-a7siii           ║│                              │
│   Encoder     ││   arri-alexa-mini.cam  │║                                           ║│ vendor: Sony                 │
│               ││   bmpcc-6k.cam         │║ vendor              Sony                  ║│ model: A7S III               │
│               ││   canon-c70.cam        │║ model               A7S III               ║│ card_layout: DCIM, PRIVATE   │
│ future types  ││   generic-dcim.cam     │║ card_layout         DCIM, PRIVATE         ║│ filename_pattern: A001_C*.mo │
│ appear here   ││                        │║ filename_pattern    A001_C*.mov           ║│                              │
│               ││                        │║                                           ║│ file_formats: ProRes RAW, XA │
│               ││                        │║ file_formats        ProRes RAW, XAVC S,   ║│ default_format: ProRes RAW   │
│               ││                        │║ default_format      ProRes RAW            ║│ gamma: S-Log3, S-Log2, S-Cin │
│               ││                        │║ gamma               S-Log3, S-Log2, S-Ci  ║│ default_gamma: S-Log3        │
│               ││                        │║ default_gamma       S-Log3▏               ║│ gamut: S-Gamut3.Cine, S-Gamu │
│               ││                        │║ gamut               S-Gamut3.Cine, S-Gam  ║│ default_gamut: S-Gamut3.Cine │
│               ││                        │║ default_gamut       S-Gamut3.Cine         ║│                              │
│               ││                        │║                                           ║│ typical_resolutions: 3840x21 │
│               ││                        │║ typical_resolutions 3840x2160, 1920x1080  ║│ typical_fps: 24, 25, 30, 50, │
│               ││                        │║ typical_fps         24, 25, 30, 50, 60,   ║│ audio_channels: 2, 4         │
│               ││                        │║ audio_channels      2, 4                  ║│ notes: Full-frame cinema cam │
│               ││                        │║ notes               Full-frame cinema ca  ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│ ──────────────────────────── │
│               ││                        │║                                           ║│ Checks                       │
│               ││ n new                  │║                                           ║│ ✖ default_gamma must be one of
│               ││ c duplicate            │║                                           ║│   gamma list: S-Log3, S-Log2…│
│               ││ x delete (asks)        │║                                           ║│ ✓ required fields present    │
│               ││ Ctrl+S save            │║                                           ║│ ✓ id matches filename        │
│ r reload disk ││                        │║                                           ║│ Save blocked until valid.    │
│               ││                        │║ Tab field · Enter edit                    ║│                              │
└───────────────┘└────────────────────────┘╚═══════════════════════════════════════════╝└──────────────────────────────┘
 Tab · ↑↓ · Enter edit · n new · c dup · x del · r reload · ^S save · Esc dash
```

## Encoder profile, 150x42

```text
 MAGVAULT  ·  Profile Editor  ·  encoder profiles                                                               admin (magvault) · unsaved changes
┌─ 1 Type ───────────┐┌─ 2 encoders/ ──────────────┐╔═ 3 Edit default-320p.enc ════════════════════════════╗┌─ 4 File on disk / checks ──────────────┐
│                    ││                            │║                                                      ║│ name: default-320p                     │
│   Camera           ││ ▶ default-320p.enc         │║ name                 default-320p                    ║│ id: default-320p                       │
│                    ││   hevc-hq.enc              │║ id                   default-320p                    ║│ type: 320p                             │
│ ▶ Encoder          ││   client-review.enc        │║ type                 320p                            ║│                                        │
│                    ││   prores-proxy.enc         │║                                                      ║│ codec: libx264                         │
│                    ││                            │║ codec                libx264                         ║│ height: 320                            │
│ future types       ││                            │║ height               320                             ║│ rate_control: crf                      │
│ appear here        ││                            │║ rate_control         crf  ▾                          ║│ crf: 23                                │
│                    ││                            │║ crf                  23                              ║│ preset: veryfast                       │
│                    ││                            │║ preset               veryfast                        ║│                                        │
│                    ││                            │║                                                      ║│ audio_codec: aac                       │
│                    ││                            │║ audio_codec          aac                             ║│ audio_bitrate: 128k                    │
│                    ││                            │║ audio_bitrate        128k                            ║│                                        │
│                    ││                            │║                                                      ║│ container: mp4                         │
│                    ││                            │║ container            mp4                             ║│                                        │
│                    ││                            │║                                                      ║│ notes: Library proxy. Always queued af │
│                    ││                            │║ notes                Library proxy. Always queued a  ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║ audio_channels       source (locked)                 ║│                                        │
│                    ││                            │║ fps / aspect         source (locked)                 ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│                                        │
│                    ││                            │║                                                      ║│ ────────────────────────────────────── │
│                    ││                            │║                                                      ║│ Checks                                 │
│                    ││ n new                      │║                                                      ║│ ✓ required fields present              │
│                    ││ c duplicate                │║                                                      ║│ ✓ crf with rate_control crf            │
│                    ││ x delete (asks)            │║                                                      ║│ ✓ id matches filename                  │
│                    ││ Ctrl+S save                │║                                                      ║│ Aspect, fps, channels locked.          │
│ r reload disk      ││                            │║                                                      ║│ In use by 1 queued job.                │
│                    ││                            │║ Tab next field · Enter edit · Esc dashboard          ║│                                        │
└────────────────────┘└────────────────────────────┘╚══════════════════════════════════════════════════════╝└────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · n new · c duplicate · x delete · r reload · Ctrl+S save · Esc dashboard
```

## Encoder profile, 120x36 (minimum)

```text
 MAGVAULT  ·  Profile Editor  ·  encoder profiles                                 admin (magvault) · unsaved changes
┌─ 1 Type ──────┐┌─ 2 encoders/ ──────────┐╔═ 3 Edit default-320p.enc ═════════════════╗┌─ 4 File on disk / checks ────┐
│               ││                        │║                                           ║│ name: default-320p           │
│   Camera      ││ ▶ default-320p.enc     │║ name                default-320p          ║│ id: default-320p             │
│               ││   hevc-hq.enc          │║ id                  default-320p          ║│ type: 320p                   │
│ ▶ Encoder     ││   client-review.enc    │║ type                320p                  ║│                              │
│               ││   prores-proxy.enc     │║                                           ║│ codec: libx264               │
│               ││                        │║ codec               libx264               ║│ height: 320                  │
│ future types  ││                        │║ height              320                   ║│ rate_control: crf            │
│ appear here   ││                        │║ rate_control        crf  ▾                ║│ crf: 23                      │
│               ││                        │║ crf                 23                    ║│ preset: veryfast             │
│               ││                        │║ preset              veryfast              ║│                              │
│               ││                        │║                                           ║│ audio_codec: aac             │
│               ││                        │║ audio_codec         aac                   ║│ audio_bitrate: 128k          │
│               ││                        │║ audio_bitrate       128k                  ║│                              │
│               ││                        │║                                           ║│ container: mp4               │
│               ││                        │║ container           mp4                   ║│                              │
│               ││                        │║                                           ║│ notes: Library proxy. Always │
│               ││                        │║ notes               Library proxy. Alway  ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║ audio_channels      source (locked)       ║│                              │
│               ││                        │║ fps / aspect        source (locked)       ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│                              │
│               ││                        │║                                           ║│ ──────────────────────────── │
│               ││                        │║                                           ║│ Checks                       │
│               ││ n new                  │║                                           ║│ ✓ required fields present    │
│               ││ c duplicate            │║                                           ║│ ✓ crf with rate_control crf  │
│               ││ x delete (asks)        │║                                           ║│ ✓ id matches filename        │
│               ││ Ctrl+S save            │║                                           ║│ Aspect, fps, channels locked.│
│ r reload disk ││                        │║                                           ║│ In use by 1 queued job.      │
│               ││                        │║ Tab field · Enter edit                    ║│                              │
└───────────────┘└────────────────────────┘╚═══════════════════════════════════════════╝└──────────────────────────────┘
 Tab · ↑↓ · Enter edit · n new · c dup · x del · r reload · ^S save · Esc dash
```

The camera screens show a validation failure (`default_gamma` is not in the `gamma` list) to illustrate blocked saves. The encoder screens show a valid profile that is in use by a queued job.

## Panes

| # | Pane | Notes |
|---|------|-------|
| 1 | Type | Profile types the module owns: Camera (`.cam`), Encoder (`.enc`). Future types appear here. `r` reloads the folder from disk. |
| 2 | List | Files in the folder for the selected type (`cameras/`, `encoders/`; paths from `path.conf`). |
| 3 | Edit | One row per schema field, shown as input boxes. Required fields first; list fields are comma separated. Enum fields (for example `rate_control`) open a small dropdown. |
| 4 | File on disk / checks | The plain-text file exactly as it would be written, plus live validation results. |

## Fields

- **Camera (`.cam`)**: `name`, `id`, `vendor`, `model`, `card_layout`, `filename_pattern`, `file_formats`, `default_format`, `gamma`, `default_gamma`, `gamut`, `default_gamut`, `typical_resolutions`, `typical_fps`, `audio_channels`, `notes`.
- **Encoder (`.enc`)**: `name`, `id`, `type` (320p / hq / custom), `codec`, `height`, `rate_control` (crf / bitrate) with `crf` or `bitrate`, `preset`, `audio_codec`, `audio_bitrate`, `container`, `notes`.
- **Locked, shown but not editable on encoder profiles**: audio channels, frame rate and aspect ratio are always source.

## Validation (from the locked schemas)

- Required: `name` and `id`. Cameras also need `file_formats` + `default_format`, `gamma` + `default_gamma`, `gamut` + `default_gamut`. Encoders also need `type`, `codec`, `height`, `rate_control` with `crf` or `bitrate`, `audio_codec`.
- Each `default_*` value must be one of its list.
- `id` must match the filename (`<id>.cam` / `<id>.enc`).
- Save is blocked until all checks pass. Unknown keys in existing files are kept, not edited.

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move within a list or between fields |
| `Enter` | Edit the field, or open the dropdown |
| `n` | New profile (from a blank or the type's template) |
| `c` | Duplicate the selected profile |
| `x` | Delete the selected profile (asks to confirm; warns if in use) |
| `r` | Reload the folder from disk |
| `Ctrl+S` | Save to disk |
| `Esc` | Return to the dashboard (shared convention) |

## Behaviour notes

- After a save or reload the server serves the updated lists to clients. Clients never read the files directly.
- Deleting or renaming a profile that is in use (queued encoder job, ingest default) shows a warning first.
- What happens to unsaved changes on `Esc` is not yet decided. The header shows "unsaved changes" while edits are pending.
- All sample values are placeholders.
