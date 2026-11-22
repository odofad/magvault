

### Hashing

SHA-256 is computed at ingest, while the card is first read.

- Hashes are written into the ASC MHL and the magcard.
- Every later copy (USB backup, rsync to server, rsync to zmag) is verified against those stored hashes.
- A mismatch is an integrity error and is reported. The copy is not committed.

## Copy method

All file copies in Magvault use `rsync` (whole directory or file as appropriate). This includes:
- Ingest (card → staging → server)
- Zmag manager (mag ↔ zmag)
- Cold pull (zmag → server)
- Any future export or restore path

Progress is reported from rsync. Verified copies still run the ASC MHL / hash check after rsync finishes.

# Magvault Planning Document

Consolidated design notes before any code. Builds on the README and NOTES.md.

## Goal

Archive and retrieval system for camera media mags. Not an editor. Keeps raw mags organised and findable, then hands out exactly the clips requested so they can be cut in a real NLE.

Terminal-first, file-based, Ubuntu 26.04+ reference, open source. Optional thin desktop GUI later. Designed so the archive remains understandable and fixable by hand if the database disappears. Fully portable: point a new install at the config directory and it continues.

## Directory layout (locked)

All roots are configurable. The program is pointed at a config directory (via argument, environment variable, or setup). Every other path is read from the files inside it. Defaults below can be overridden so the entire set lives on durable ZFS datasets.

```text
config/                            # location supplied at startup
  path.conf                        # all roots only: mags, proxies, bins, db, logs, telemetry_live, telemetry_disk, cameras, ...
  magvault.conf                    # non-path settings
  encoding.conf                    # ffmpeg presets and proxy rules (or replaced by encoder profile files)
  ...                              # additional files as needed

db/
  magvault.sqlite                  # single database file

mags/                              # flat, no subfolders
  <YYYYMMDD>_<mag-uuid>/           # shoot day (operator-confirmed) + UUID
    <exact contents of the card>   # original filenames + sidecars only
    magcard.txt                    # complete manifest + confirmed metadata
    ascmhl/                        # ASC MHL inventory (standard checksum manifest)
      manifest.mhl
    .magvault/                     # hidden, Magvault-internal only
      magcard.*.backup

proxies/
  db/                              # 320p library proxies
    <YYYYMMDD>_<mag-uuid>/
      <proxy files, original basenames>
  hq/                              # optional full-res lower-bitrate HEVC
    <YYYYMMDD>_<mag-uuid>/
      <proxy files>

custom_encode/                     # ad-hoc / manual encoder output only
  <YYYYMMDD>_<mag-uuid>/
    <profile-name>/
      <output files>

bins/                             # configured path, outside magvault tree by default
  <operator-assigned-name>/        # export folder
    exportcard.txt
    exportcard.txt.backup          # single previous version
    <YYYYMMDD>_<mag-uuid>/         # original mag folder name
      <copied original clips + sidecars>

cameras/                           # one .cam file per camera profile
  sony-a7siii.cam
  red-komodo.cam
  ...

logs/
```

No project folders on disk. All project relationships live in the database.

## Mag rules (locked)

- One physical card = one mag, unless explicitly split later.
- One camera only. No mixing sources, cameras, or formats inside a mag.
- Folder name: `<YYYYMMDD>_<mag-uuid>`. Date is the **shoot day** (guessed from camera date-modified or earliest media date, confirmed by operator). Never renamed after creation.
- Contents: exact verified copy of the card (original filenames + matching sidecars). If it was not on the SD card, it is not in the mag folder — except `magcard.txt`, the ASC MHL inventory, and the hidden `.magvault/` backups of the card.
- Mag card is plain text (`magcard.txt`), complete manifest + confirmed metadata + project link. Stays with the folder forever.
- ASC MHL inventory is written by default alongside the mag card. It contains only the standard file list, sizes, mtimes and checksums (no Magvault-specific fields).
- Merge of mags is forbidden (including cards from the same camera, because clip names are reused).
- Split (later feature): creates new UUID folders. New shoot-day date is taken from the first file after the split point and confirmed by the operator. Original folder is updated or retired accordingly.
- Whole folder remains self-describing if the database disappears.

## SQLite schema

Single file `db/magvault.sqlite`. Files on disk remain the source of truth for manifests; the database is the index.

### projects
- id (UUID, primary key)
- name (label only)
- season_or_year (optional)
- notes
- status (open / closed) — open projects appear in ingest; closed projects are hidden from ingest and only visible in the library
- created_at, updated_at

### mags
- id (UUID, primary key)
- project_id (UUID, foreign key, required)
- dump_path
- shoot_date (YYYYMMDD used in the folder name)
- camera_profile_id (from .cam, or free text if no profile)
- file_format (confirmed)
- gamma (confirmed)
- gamut (confirmed)
- status (ingesting, online, cold, archived, …)
- logging_complete (boolean)
- ingested_at
- notes

### clips
- id (UUID, primary key)
- mag_id (UUID, foreign key)
- original_filename
- relative_path
- size
- checksum (SHA-256)
- proxy_path (nullable)
- proxy_status (none / pending / ready / failed)
- description
- rating (1–5, nullable)
- slow_motion (boolean)
- technical_note
- shot_type
- camera_operator
- director_producer
- created_at, updated_at

### clip_tags
- clip_id (UUID)
- tag (text)
- primary key (clip_id, tag)

### placements
- id (UUID, primary key)
- mag_id (UUID)
- zmag_id (text, the zmag name)
- copy_number (1, 2, …)
- status (copying / complete / failed)
- copied_at
- notes

### exports
- id (UUID, primary key)
- name (operator-assigned folder name)
- project_id (UUID, nullable)
- path (path to the export folder)
- created_by
- created_at, updated_at
- notes

### export_clips
- export_id (UUID)
- clip_id (UUID)
- in_point (nullable)
- out_point (nullable)
- primary key (export_id, clip_id)

### cold_pull_requests
- id (UUID, primary key)
- mag_id (UUID)
- zmag_id (text)
- requested_by
- status (pending / pulling / complete / failed)
- created_at, completed_at
- notes

### encoder_jobs
- id (UUID, primary key)
- mag_id (UUID, nullable)
- clip_id (UUID, nullable)
- profile_id (text)
- status (queued / running / done / failed)
- progress (0–100)
- output_path
- created_at, started_at, finished_at
- error

Projects, mags and clips are identified by UUID. Names are labels. The database can be rebuilt from the files on disk if it is lost.

## Project open / closed rule

- **Open** projects can receive new mags and are shown in the ingest project list.
- **Closed** projects are finished (typically moved toward cold storage). They are hidden from ingest and only appear in the library.
- Closing or re-opening a project is a deliberate action. It only changes the flag; files on disk never move.
- Default view everywhere except the library shows only open projects.

## Camera profiles

One plain-text `.cam` file per camera, all living in the `cameras/` folder (path in path.conf).

- Server (or standalone) scans the folder on startup and on a manual reload.
- Valid files are loaded into the database / served to clients.
- Clients never read the files directly. They request the current profile list from the server over SSH (or use a local folder when running standalone).
- Ordinary operators on the ingest screen can only select from existing profiles. They cannot invent new camera, format, gamma or gamut names.
- A starter set of common cameras will be shipped.

### `.cam` file schema (locked)

Plain text. One file per profile. Filename is `<id>.cam`.

```text
# Magvault camera profile
name: Sony A7S III
id: sony-a7siii

# Detection
vendor: Sony
model: A7S III
card_layout: DCIM, PRIVATE
filename_pattern: A001_C*.mov

# Media
file_formats: ProRes RAW, XAVC S, XAVC HS
default_format: ProRes RAW

gamma: S-Log3, S-Log2, S-Cinetone, Rec.709
default_gamma: S-Log3

gamut: S-Gamut3.Cine, S-Gamut3, Rec.709
default_gamut: S-Gamut3.Cine

# Optional technical hints
typical_resolutions: 3840x2160, 1920x1080
typical_fps: 24, 25, 30, 50, 60, 120
audio_channels: 2, 4

notes: Full-frame cinema camera. S-Log3 + S-Gamut3.Cine is the usual pair.
```

**Rules**
- `name` and `id` required.
- Lists are comma-separated.
- `default_*` must be one of the listed values.
- Unknown keys are ignored (forward compatible).
- Required minimum: `name`, `id`, `file_formats` + `default_format`, `gamma` + `default_gamma`, `gamut` + `default_gamut`.
- Ingest only offers the values listed in the matching profile.

## Encoder profiles

Plain-text files, one per profile. Live in a folder configured in path.conf (e.g. `encoders/`). Filename is `<id>.enc`.

Distilled from common ffmpeg preset tools, restricted by Magvault’s locked proxy rules.

```text
# Magvault encoder profile
name: default-320p
id: default-320p
type: 320p          # 320p | hq | custom

# Video
codec: libx264
height: 320         # width calculated to preserve aspect; "source" = no resize
rate_control: crf   # crf | bitrate
crf: 23
preset: veryfast

# Audio
audio_codec: aac
audio_bitrate: 128k
# audio_channels is always source — not stored, not editable

# Container
container: mp4

notes: Library proxy. Always queued after ingest.
```

**HQ example**
```text
name: hevc-hq
id: hevc-hq
type: hq
codec: libx265
height: source
rate_control: bitrate
bitrate: 40M
preset: medium
audio_codec: aac
audio_bitrate: 192k
container: mp4
```

**Rules enforced at encode time (not editable)**
- Aspect ratio preserved.
- Frame rate = source exactly.
- Audio channels = source exactly (no downmix).

**Required minimum**
- `name`, `id`, `type`
- `codec`, `height`
- `rate_control` + either `crf` or `bitrate`
- `audio_codec`


## Logs module

Logs live in files under the `logs/` path (from path.conf). They are never written to the database.

### File layout
- One file per day: `logs/YYYY-MM-DD.log`
- Plain text, one entry per line.
- Line format:
  ```
  2026-11-22T21:15:03Z  INFO   ingest    Mag 20250520_a3f9c2 ingested (84 GB)
  2026-11-22T21:15:04Z  WARN   encoder   Proxy failed for A001_C005.mov: timeout
  2026-11-22T21:15:05Z  ERROR  zmag      zmag-02 degraded: sdg faulted
  ```
- Fields: timestamp (UTC), level (INFO/WARN/ERROR), module, message.

### Viewer
- Newest first.
- Filter by module (ingest, encoder, zmag, library, logging, …) or by level.
- Detail pane shows the full selected line plus any multi-line context that follows it.
- Optional follow / tail mode (`f`). The list stays newest-first. Follow keeps the selection on the newest entry (top of the list) and scrolls as new lines arrive.

### Retention
- Setting in `magvault.conf`: `log_retention_days`.
- Default: 30.
- On startup and daily, files older than the retention period are deleted.

### Modules that write logs
- Ingest
- Encoder
- Zmag manager
- Library / Export
- Logging
- Settings (config changes)
- System (startup, shutdown, errors)



### Sudo helper

All privileged operations go through one small root-owned helper.

- The helper only allows the exact commands Magvault needs (zpool create/export/import, format, etc.).
- `timestamp_timeout=0` — every call asks for the sudo password.
- The Magvault user has no general sudo rights.
- No root login.

## Settings module

Admin-only on the server. Client shows only client-relevant settings.

**Server (admin only, running as `magvault`)**
- Edit `path.conf` roots (mags, proxies, bins, db, logs, cameras, encoders, custom_encode, telemetry paths, etc.).
- Future non-path settings live in `magvault.conf` and appear here when they exist.
- Save writes the files.
- Validate that paths exist or can be created.

**Client**
- Local proxy cache path.
- Server SSH host / user (limited edit).
- No server paths. No admin settings.

No other non-path settings are defined today. They are added when a module needs them.

## Profile Editor module

Single admin-only module that owns creation, editing, duplication and deletion of every profile type:

- Camera profiles (`.cam` files)
- Encoder / proxy profiles (ffmpeg presets)
- Any future profile type

Files on disk remain the source of truth. The module reloads the relevant folders and the server serves the updated lists. Normal operators never see the Profile Editor.

## User model (no Magvault accounts)

Magvault has no internal user database. Access is the system user plus SSH keys.

- **Server user** (`magvault`): admin. Can open the Profile Editor and any other admin-only module. Runs the server process.
- **Client user** (`magvaultclient`): shared username for all clients. Each client is identified by its own unique SSH key. Can use Ingest, Logging, Dashboard, Encoder, etc. Cannot open admin-only modules.

Admin is simply “running as the `magvault` user”. Clients never become admin.

### Sudo (no root)

The `magvault` user never runs as root. It is granted sudo for specific ZFS commands only, and the password is still required:

```
magvault ALL=(root) /usr/sbin/zpool, /usr/sbin/zfs
```

Magvault calls `sudo zpool` / `sudo zfs` when it needs to create, import, export or manage a zmag.  
The operator must enter the sudo password. Nothing is executed until the password is accepted.  
Clients (`magvaultclient`) have no sudo rights on the server.

On a client machine the `magvaultclient` user may be given limited, password-required sudo for local USB-prepare commands only (mount, write manifest, etc.).  
This sudo is local to the client machine and does not grant any rights on the server.  
All other Magvault operations run unprivileged.

## Mag card format

Plain text file `magcard.txt` inside the dump folder. Complete manifest so missing files can be identified even without the database.

Example shape:

```text
mag_uuid: ...
project_uuid: ...
project_name: ...
shoot_date: 2026-11-18
camera_model: ...
file_format: ...
gamma: ...
gamut: ...
ingested_at: ...
notes: ...

# manifest
filename | size | sha256
clip001.MOV | 123456789 | abc...
clip001.XML | 4567 | def...
...
```

ASC MHL (`ascmhl/manifest.mhl`) is written at the same time. It holds the industry-standard inventory (filenames, sizes, mtimes, SHA-256/MD5) and nothing else.

## Conflict handling

Database is authoritative.

When a mismatch is detected (cold mag reappears, card differs from database for that mag UUID):
1. Copy existing magcard.txt to `.magvault/magcard.YYYYMMDD-HHMMSS.backup`.
2. Write a fresh magcard.txt from the current database record.

Applies to all fields: camera, format, gamma, gamut, project assignment, manifest, notes, shoot day.

## Ingest flow

1. Card discovery (both automatic and manual):
   - Automatic: watch for newly mounted volumes (Ubuntu: /media, /run/media or udisks; macOS later: /Volumes). Recognise known camera layouts (Sony, Canon, ARRI, RED, Blackmagic, Panasonic, Fujifilm, GoPro, DJI, Insta360, generic DCIM). Offer each detected card with its label, size and guessed camera.
   - Manual: operator can always type or browse to a path.
   - Rescan action available if a card is plugged in after the screen is open.
2. System probes the chosen card:
   - Walk all files and sidecars.
   - Auto-detect guesses for camera model, file format, gamma, gamut, and shoot day.
   - Build full file list with sizes.
3. Present guesses and file list to operator.
4. Operator:
   - Confirms or corrects camera model, file format, gamma, gamut, and shoot day (selected only from existing profiles).
   - Assigns to an existing **open** project (select from list) or creates a new project (UUID assigned immediately, status open).
   - Adds any mag-level notes.
5. System performs verified copy into `mags/<shootday>_<new-mag-uuid>/`:
   - Exact card contents, original filenames and sidecars preserved.
   - SHA-256 hashed on the way in; destination verified.
   - Only proceeds to commit if verification passes.
6. Write complete-manifest magcard.txt and ASC MHL inventory into the dump folder.
7. Insert records in SQLite.
8. Queue 320p proxy generation on the server (does not block ingest).
9. Mag is now online and linked. Server encodes when it has capacity.

Per-file shot logging is not part of ingest.

## Client ingest and field backups

Ingest can run on a client terminal (or standalone machine). After a successful commit the server owns the rest of the pipeline (proxy encode, etc.).

**Copy options (selectable on the main ingest screen)**
- Default: card → local staging → rsync to server → server confirms → autoclean staging.
- Backup USB drive can act as the staging location itself.
- Direct card → server (optional).
- Extra mirror to one or more backup drives at any time.
- Warning shown when no backup exists and server sync is not yet confirmed.

**Field backup drives**
- Whole drive can be dedicated.
- Gets `magvault-backup.txt` at the root:
  - `role: master` or `slave`
  - label, created time, host
  - list of mag folders and whether each is pending server sync or already synced
- Same mag-folder layout as the server + original files + `magcard.txt` (with SHA-256).
- When mounted, client lists projects/mags/free space the same way the server view does.
- Operator can reconnect the drive later and finish “sync pending to server.”
- Checksums live in the mag cards, so verification works on any filesystem.
- Encryption is handled at the OS level if desired (not by Magvault).

**USB drive diagnostics and prepare (ingest screen)**
- Only a Magvault-prepared drive (has valid `magvault-backup.txt` at root) can be selected as a backup target.
- Any other drive is shown as “not valid”.
- Not-valid drives offer a **Prepare** action.
- Prepare requires the system sudo password as final confirmation. Nothing is written until the password is accepted.
- Once prepared, Magvault will not offer Prepare again while the drive still contains mags marked “pending server sync”.
- Prepare is only re-enabled after every mag on the drive has been successfully transferred to the server.
- Valid drives with pending mags remain selectable as targets; they simply cannot be re-prepared until synced.

All targets and states are visible and choosable on the ingest screen.


### Logging on the mag card

Logging fields are written into `magcard.txt` under a `# logging` section so a lost database can be rebuilt.

- Rewritten when the operator leaves the clip or the Logging module (same moment as autosave).
- Previous version kept as the single `.magvault/magcard.*.backup`.
- Fields: description, rating, slow_motion, technical_note, shot_type, camera_operator, director_producer, tags.

## Logging module

Separate from ingest. Per-clip (or per-asset) logging. Original filenames and sidecars stay untouched. Log data lives in the database (and can optionally be written into a sidecar or the mag card).

- Available on both **client** and **server**.
- Client logging works against local staging, USB backups, or recently synced mags. Changes sync to the server database when connected. Offline logging is supported on local media.
- Server logging has full access to all online mags.
- Playback uses proxies (see Media playback). Camera raws are not opened for casual viewing.

### Logging fields (locked)

| Field | Type | Behaviour |
|-------|------|-----------|
| Description | Short free text | What is happening. Drawn as a real text field (border, cursor, placeholder when empty). |
| Keywords / Tags | Type then click | Type in the field; matching tags pop up. Click (or Enter) to add. If no match, offer “[create ‘…’]” once. No permanent picker pane. |
| Rating | Stars (1–5) | |
| Slow motion | Tick | Checkbox |
| Technical note | Optional short text | Audio, focus, exposure, etc. Drawn as a real text field. |
| Shot type | Optional clickable | Wide / medium / close / aerial… |
| Camera operator | Type then click | Same popup behaviour as tags. |
| Director / Producer | Type then click | Same popup behaviour as tags. |

Free-text fields (Description, Technical note) must look like editable input boxes, not static display. Clickable / popup fields avoid typos while remaining growable.

### Logging screen behaviour

- **Layout**: Mag list on the left (same pattern as ingest Detected pane). Selecting a mag loads its clips.
- **Save**: Autosave on field exit. Explicit save on leaving the clip or the module. Clear “saved” indicator.
- **Proxy status**: Small icon in the player area shows proxy state (ready / pending / pulling / local fallback).
- **Remove**: Each tag, camera operator, or director/producer chip has a small × to remove it.
- **Mark complete**: Action to mark the whole mag “logging complete”.
- **Undo / revert**:
  - Light undo (`Ctrl+Z`) for the current field.
  - Revert loads the last backup of the mag file (`.magvault/magcard.*.backup` or equivalent log sidecar backup). The database does not store its own backup; the mag file on disk is the recoverable copy.
- **No clip-list filter in v1.** Mag list + logged/unlogged markers + next/previous unlogged is enough. Filter can be added later if needed.

## Dashboard (landing screen)

The dashboard is the first screen after launch. It shows status and is the way to enter other modules.

**Server (or standalone) dashboard**
- Full status from the local telemetry file: encoder queue depth + active job progress, ZFS health for both working datasets and zmags, simple counts (open projects, online mags, pending requests), recent events.
- ZFS section shows:
  - Working mounts (the datasets that hold mags, proxies, bins, db, etc. from path.conf): pool/dataset name, mountpoint, state (ONLINE / DEGRADED / FAULTED / UNAVAIL), used/available.
  - Mounted zmags: same health view as before (overall state + any non-ONLINE drives).
- Module menu: Ingest, Logging, Library, Export bins, Zmag manager, Profile Editor (admin), Logs, Settings.
- Reads local telemetry file + light local checks (`zfs list`, `zpool status`).

**Client dashboard**
- Thin status: server connection state (online/offline/syncing), simple queue depth number when connected, local USB/staging status.
- Module menu: Ingest, Logging (local-capable), Library (local proxies + export requests), and the ability to open the full server dashboard over SSH.
- Does **not** show full encoder progress or zmag drive health by default. Those stay on the server.
- Local USB/staging and recent local activity are first-class.

Both versions are ratatui. Esc/q returns to the dashboard from any module. Keyboard first (arrows + Enter or single-letter shortcuts). Mouse optional.

## Shared key convention (terminal)

One set of navigation keys for every screen. Defined once so modules stay consistent.

| Key | Meaning |
|-----|---------|
| `Esc` | Return to the dashboard (from any module). Blocked while a write/copy is in progress. |
| `q` | Quit the whole program. Only on the dashboard itself. |
| `Tab` / `Shift+Tab` | Next / previous pane |
| `↑` / `↓` | Move within a list or menu |
| `Enter` | Confirm, open, or select the current item |
| Mouse | Optional. Click a pane to focus it. |

Module-specific actions use single letters. They must not collide with the shared keys above.

### Dashboard
| Key | Action |
|-----|--------|
| letter | Open module (`i` Ingest, `g` Logging, `l` Library, `e` Export bins, `z` Zmag, `p` Profile Editor, `v` Logs, `s` Settings) |
| `r` | Refresh status |

### Ingest
| Key | Action |
|-----|--------|
| `n` | New project |
| `r` | Rescan for cards |
| `p` | Pull (start verified copy) |
| `o` | Prepare selected not-valid USB (sudo) |
| `x` | Abort (pulling state only) |
| `l` | Show log |

### Encoder
| Key | Action |
|-----|--------|
| `n` | New job (ad-hoc to custom_encode/) |
| `p` | Pause / resume queue |
| `r` | Retry failed job |
| `c` | Cancel job |
| `u` | Re-queue job |
| `+` / `-` | Raise / lower priority |
| `e` | Edit profiles (jump to Profile Editor) |

### Profile Editor (admin only)
| Key | Action |
|-----|--------|
| `n` | New profile |
| `c` | Duplicate selected profile |
| `x` | Delete selected profile (asks first) |
| `r` | Reload from disk |
| `Ctrl+S` | Save |
| `Esc` | Return to dashboard. If there are unsaved changes, prompt “Discard changes?” |

#
### Becoming cold

A mag becomes cold only by an explicit operator action (evict / mark cold).

- Allowed only if the mag has at least two complete placements on different zmags.
- Once cold, the working copy may be removed from the server.
- The placements remain in the database so the mag can be pulled back later.

## Zmag manager
| Key | Action |
|-----|--------|
| `c` | Copy whole mag to selected zmag |
| `f` | Cycle project filter |
| `←` / `→` | Switch zmag |
| `a` | Toggle / trigger auto-backup |
| `s` | Snapshots screen |
| `p` | Pool / drive health screen (local only) |
| `h` | Hold / release snapshot (on snapshots screen) |
| `d` | Destroy snapshot (asks, needs sudo) |
| `b` | Scrub pool |
| `r` | Replace drive |
| `t` | SMART self-test |
| `m` | Refresh SMART |
| `Esc` | From secondary screens → main two-pane; from main → dashboard |

The dashboard is the landing screen. Every other module returns to it with `Esc`.

## Media playback

- **Player**: mpv (orchestrated by Magvault via IPC). Handles 320p and HQ proxies, exact frame rate, original audio channels.
- **Audio**: system audio server. PipeWire on a normal Ubuntu 26.04 desktop; ALSA fallback for barebone/minimal systems. Magvault does not implement its own audio path.
- **Video**: kitty graphics protocol or sixel when the terminal supports it. Over pure SSH without graphics support, audio + status line only.
- **Proxy preference**: logging prefers server-generated 320p proxies. Client can pull them when ready. Explicit fallback to quick temporary local proxies for a single mag when offline or server proxies are not yet available.
- Camera raws are never used for casual playback.

**Desktop client** (later, separate)
- Standalone mouse-first application (egui/Vulkan or similar).
- Not part of the current terminal work. Terminal version remains primary.

## Encoder module

Mostly automatic: successful ingest queues a 320p job on the server. The module is the place to watch, manage, and override.

**Queue view**
- Depth, active jobs (progress, mag, type 320p/HQ/custom, profile, elapsed), waiting jobs, failed jobs.
- Per failed job: clear error reason (ffmpeg exit, disk full, missing source, bad profile, etc.).
- Actions: pause/resume queue, retry failed, cancel, re-queue, bump priority.

**New job screen (ad-hoc)**
1. Select mag (online mags, filterable by project).
2. Select files or whole mag.
3. Select output profile (from existing encoder profiles).
4. Destination is fixed: the `custom_encode/` folder (path in path.conf).
   Output lands under `custom_encode/<YYYYMMDD>_<mag-uuid>/<profile-name>/`.
   These files are **not** written into `proxies/db/` or `proxies/hq/`, are not linked as official proxies, and are not logged as Magvault assets. Pure encoder output.

**Rules / profiles**
- Show currently active default profile(s).
- One-key jump to the Profile Editor for encoder profiles.
- Strict proxy rules still apply (aspect preserved, exact frame rate, no audio downmix).

Dashboard stays light (queue depth + active progress). Full management lives in this module.

## Proxies

- `proxies/db/` — always-on 320p library proxies. Same folder naming as mags.
- `proxies/hq/` — optional full-resolution lower-bitrate HEVC (30-60 Mbps). Same naming. Generated only when requested.
- 320p generation is queued on the server after ingest commit so it never delays I/O. Faster servers simply clear the queue sooner.
- Per-asset (or whole-mag) toggle in the encoding stage decides what also goes to `hq/`. Noted for later.
- NLE-specific proxies (Resolve etc.) are created with bin exports and live with the bins, not here.

### Proxy encoding rules (locked, all proxy types)

- Resolution may change, but aspect ratio must be preserved.
- Frame rate must match the source exactly. No conversion, no pulldown, no approximation.
- Audio channels must never be downmixed. Channel count and layout stay identical to source. Codec or container may change; channels may not be collapsed.

## Library module

Available on both server and client.

**Server**
- Project → Mag → Clips → Player + Export list.
- “Add to list” updates `exportcard.txt` immediately.
- Export action starts the media copy. If any clip is cold, a cold-pull request is created for the Zmag manager.
- Status strip shows warm / cold / pulling.

**Client**
- Same four-pane layout.
- Keeps a local cache of 320p proxies (path in client path.conf, e.g. `client-proxies/`).
- Pull proxies action (or automatic on connect) fetches missing 320p proxies from the server for the mags being browsed.
- Clip list shows proxy status: local / missing / pulling.
- Export creates a request on the server. When the server finishes (including any cold pull), the client can pull the finished files.
- Offline: can browse recently synced mags and build a list; Export is queued until connected.

The export list the operator sees is the export card. The program uses the card to know which mags and clips to pull.

## Export bins (locked)

Path configured in path.conf. Default suggestion outside the magvault tree (e.g. separate ZFS dataset).

Each export is a folder whose name is assigned by the operator at creation time.

```text
bins/
  <operator-assigned-name>/
    exportcard.txt
    exportcard.txt.backup          # single previous version only
    <YYYYMMDD>_<mag-uuid>/         # original mag folder name, recreated
      <original clip names + sidecars>   # copied, never linked
```

- Only original media and matching sidecars. No proxies of any kind.
- Copies, not symlinks or hardlinks. Space saving left to ZFS dedup or the operator.
- Folder names and clip names are immutable and match the originals.
- Multiple source mags produce multiple original-named subfolders.
- Magvault discovers exports by walking the bins path and reading the cards. Folders are the record; a full export table in the database is optional.

### exportcard.txt

```text
name: Bamba-Cutaways
project_uuid: ...
project_name: ...
created: 2026-11-22T14:30:00
created_by: odofad
updated: 2026-11-22T16:15:00
notes: ...

# clips
# mag_folder | original_filename | source_path | optional_in | optional_out
20261118_abc123… | clip001.MOV | /mags/.../clip001.MOV | 00:01:12 | 00:01:45
...
```

When clips are added:
1. Copy current exportcard.txt to exportcard.txt.backup (overwrite any previous backup).
2. Write updated card with new clip list and refreshed updated timestamp.

### Export bins browser

Read-only viewer for existing exports.

- List of export folders under `bins/` (name, project, date, status).
- Open one → show the export card (clips, optional in/out) and the on-disk files.
- Status per clip or per export: warm / cold / copying / done.
- No editing and no delete in this screen.
- Shortcut (`e` or `l`) jumps to the Library module with this export open for editing.

**Status rules**
- Export status is `cold` if any clip is cold.
- Export status is `done` when all clips have been copied (and verified if verification is on).
- Date column shows the export card’s `created` date.
- On-disk listing includes original clips and their matching sidecars (names come from the source mag).


### Becoming cold

A mag becomes cold only by an explicit operator action (evict / mark cold).

- Allowed only if the mag has at least two complete placements on different zmags.
- Once cold, the working copy may be removed from the server.
- The placements remain in the database so the mag can be pulled back later.

## Zmag manager

A **zmag** is a set of 4 drives in ZFS RAIDZ1 used for server-side cold storage.

**Creation**
- Operator selects four raw drives.
- Command creates the pool with:
  - `recordsize=1M`
  - `compression=lz4`
  - `atime=off`
  - `magvault:type=zmag`
  - human name `zmag-01`, `zmag-02`, …
- Only pools with that property (or matching name pattern) are ever imported or mounted. All other zpools are ignored.

**Create flow** (opened with `n` from the Zmag manager)
1. List available raw drives.
2. Operator marks exactly four.
3. Confirm auto-generated name (`zmag-NN`) and the fixed properties above.
4. Sudo password required.
5. Create the pool and mount it.
6. New zmag appears in the right pane.

**Local operation**
- Plug in drives → zmag manager auto-imports and mounts only recognised zmags.
- Multiple zmags may be mounted simultaneously.
- Two-pane (or multi-pane) view: left = online server mags (filterable by project), right = contents of the selected zmag.
- Copy is whole-mag only.
- Copies use `rsync` behind the scenes (whole mag directory). Progress is shown in the operations strip.
- Role is chosen at creation (primary or backup).
- Auto-backup fills the chosen role:
  - A primary zmag receives mags that do not yet have a primary copy.
  - A backup zmag receives mags that do not yet have a backup copy.
- A mag has its first copy when it is on at least one primary zmag, and its second copy when it is on at least one backup zmag.

**Eject**
- `x` or an eject action runs `sudo zpool export` on the selected zmag (password required).
- After export the drives can be safely removed.
- The zmag disappears from the mounted list.

**Secondary screens** (opened by key, Esc returns to the main two-pane view)
- `s` — Snapshots: ZFS snapshot list for the selected mag or zmag. List, hold, destroy (with confirm). Inspired by ZFS Commander’s snapshot pane.
- `p` — Pool / drive health: status, scrub, replace drive, SMART. Inspired by zfleet’s pool inspector. Local pools only. Remote zmag stations are not managed this way.

**On-zmag manifest**
```text
id: zmag-01
role: primary   # or backup
created: ...
# list of mag UUIDs
```
Identity lives in the manifest and database, not in a UUID name.

**Remote zmag station**
- A second server can run the same zmag manager.
- Connects over SSH (keys only).
- Reports its zmags, receives copy/restore jobs, performs transfers locally, updates the central database.
- Appears to the operator as additional zmags.

**Cold pull**
- Driven by requests from the library/export side.
- Wrangler docks the correct zmag (local or remote); manager copies requested mags back to the server.

**Key module** (admin only)

Two ways to add a client:

1. **Generate on server**
   - Admin generates a key pair in the Key module.
   - Public key is added to `~magvaultclient/.ssh/authorized_keys`.
   - Private key is shown once.
   - Admin can:
     - `w` write it to a chosen file, or
     - `u` write it directly to a mounted USB drive (lists removable drives, admin picks one, writes e.g. `magvault-client.key`).
   - After either action the private key is no longer stored on the server.
   - Leaving the generate screen without writing (Esc or otherwise) discards the private key. It is never stored.
   - Client never connects until it already has the private key.

2. **Accept a client public key**
   - Client generates its own key pair.
   - Admin copies the public key (USB, paste) and adds it via the Key module.
   - Public key is added to `~magvaultclient/.ssh/authorized_keys`.

Both keep the admin as the only person who can grant access. No self-service enrollment.

The module lists existing keys, shows which client they belong to, and can revoke a key (remove from authorized_keys).

Private keys generated on the server are not stored long-term after they have been handed over. Only the public keys and a label (client name) are kept.

Database tracks every placement (mag UUID, zmag id, copy number, date, status). Backup view shows which mags have zero, one, or two-plus copies.

## Telemetry

Live dashboard numbers live in a RAM-backed file (tmpfs or /dev/shm) so frequent updates cause no disk wear. A periodic copy (default every 30 minutes) is written to disk for reboot survival or light history.

Both paths are in path.conf:

```text
telemetry_live=/dev/shm/magvault-telemetry.json
telemetry_disk=logs/telemetry-latest.json
```

Readable without root. The **server** dashboard reads the local file. A client may request a short status summary over SSH (connection state, queue depth) but does not pull the full telemetry file by default.

The server dashboard must show:
- Encoder queue depth and per-job progress.
- ZFS health for the working datasets (mags, proxies, bins, db and any other configured roots) — pool/dataset, mount state, used/available.
- Zpool health for every mounted zmag (overall state + any non-ONLINE drives) so failing disks are visible.

## Roadmap

### Phase 0 – Foundation
- Config directory + path.conf + system pointer file.
- SQLite schema.
- Manual-path ingest: probe, confirm, hash at ingest, verified copy, magcard.txt + ASC MHL, DB records, queue 320p.
- Conflict handling (database wins for metadata, magcard backed up).
- Basic shell only.

### Phase 1 – Client ingest & field backups
- Card discovery.
- Full client ingest (staging, USB, direct, mirror).
- Prepare formats USB as ext4.
- Reconnect-and-sync, pending-sync safety.
- Server confirmation + autoclean.

### Phase 2 – Proxies, encoder, dashboards
- 320p encode queue and rules.
- Encoder module (queue + ad-hoc to custom_encode/).
- Server dashboard (landing) and client dashboard (includes Library).
- Telemetry file.

### Phase 3 – Logging & Library
- Logging module (fields written to magcard # logging section).
- Library on server and client (local proxy cache, pull proxies).
- Export card updated on add.
- Cold-pull request generation.

### Phase 4 – Zmag manager
- Create zmag (role primary/backup chosen at creation).
- Two-pane whole-mag rsync copy.
- Auto-backup fills the chosen role.
- Eject (zpool export).
- Explicit evict to cold (requires ≥ 2 placements).
- Placement tracking + on-zmag manifest.
- Local pool health.

### Phase 5 – Remaining modules
- Export bins browser (read-only, shortcut to Library).
- Profile Editor (admin).
- Key module (admin, generate or accept, USB export, discard on leave).
- Logs viewer (file-based, 30-day default).
- Settings (admin on server, client-relevant on client).
- Sudo helper (single root-owned binary, password every time).

### Phase 6 – Polish & optional desktop
- Remote zmag stations.
- Split function.
- HQ proxy toggle.
- Separate mouse-first desktop client.
- Packaging and docs.

## Borrowed ideas

See NOTES.md. Main ones in use:
- Verified SHA-256 full-card copy from BitMatch.
- Camera detection seed list from BitMatch.
- Keep original names + sidecars.

## Open points

- Exact terminal UI details still open in some mockups (truncation at 120×36, a few key letters).
- Encode queue implementation details (how the server notices a new mag and enqueues the job).
- Card-layout detection completeness and fallback.
- Exact on-zmag manifest format and location.
- Telemetry JSON schema.
- Failure and retry semantics for every copy path.
- Details of the later split UI.
- Offline logging conflict rule (needed when client logging ships).

### Prepare

Prepare formats the selected USB drive.

- Filesystem: ext4 (default).
- Clear warning that all data on the drive will be destroyed.
- Sudo password required.
- After format, Magvault writes its manifest and the drive is ready for ingest staging.
- Mac compatibility is not a goal for field USBs. A Mac that needs the data uses a network pull or a separate export.
