

### Hashing

SHA-256 is computed at ingest, while the card is first read.

- SHA-256 is stored in the magcard manifest and in the database. It is the integrity hash for all later verification.
- ASC MHL (specification 1.0; files declare hashlist version 2.0, `urn:ASC:MHL:v2.0`); C4 only. ASC MHL does not support SHA-256, so the MHL stores C4 hashes (computed in the same read). SHA-256 remains the hash used for verification.
- Every later copy (USB backup, rsync to server, rsync to zmag) is verified against the stored SHA-256.
- A mismatch is an integrity error and is reported. The copy is not committed.


### Incoming staging

`incoming` lives inside the mags dataset so the final move is an atomic rename.

- Default: `incoming=/magvault/mags/.incoming` (or equivalent under the configured mags root).
- Magvault refuses to start an ingest if `incoming` is on a different filesystem from `mags/`.
- The mags root may contain the hidden `.incoming` directory. It is not a mag and is ignored by mag listing.
- A separate staging dataset is not used for ingest. It would require a second full copy.

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
- One camera only. No mixing sources, cameras, or formats inside a mag. A card whose live media names more than one camera is never ingested silently: ingest warns and asks; `--yes` refuses unless `--mixed-ok` is given, and the mag card then records every model (`mixed_cameras`).
- Folder name: `<YYYYMMDD>_<mag-uuid>`. Date is the **shoot day** (guessed from camera date-modified or earliest media date, confirmed by operator). Never renamed after creation.
- Contents: exact verified copy of the card (original filenames + matching sidecars). If it was not on the SD card, it is not in the mag folder — except `magcard.txt`, the ASC MHL inventory, and the hidden `.magvault/` backups of the card.
- No junk filter. Files that were on the card are copied, including `SALVAGE.TMP`, sidecars, and other non-media. Magvault does not drop them.

**Card path**
- Cards go from the camera to ingest. Do not mount them on a Mac before ingest.
- Magvault does not filter Mac junk (`.DS_Store` and similar). If a Mac build is made later, that build handles Mac junk there.
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
- requirements (optional) — fixed values chosen at project creation from lists (resolution, frame rate, format). No free text. Shown at ingest. Not a hard limit.
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


## Soft lock (used on a production)

A mag can be soft-locked once it has been used on a production. Soft lock does not delete or hide the mag. It warns and requires an explicit override before the mag can be reused.

### When a mag is soft-locked
- Operator marks it used (or it is marked when added to an export that is marked final / delivered).
- The lock records:
  - locked (boolean)
  - locked_at
  - production_name (or project name)
  - locked_by
  - note (optional)

### Behaviour
- In Library, Logging, and Export the mag shows a lock indicator and the production it was assigned to.
- Attempting to add clips from a soft-locked mag to a new export, or to start a new ingest that would reuse it, shows a warning:
  “This mag was used on <production>. Continue anyway?”
- Override is allowed (operator confirms). The override is logged.
- Soft lock can be cleared by an admin (or the same operator) with a reason. Clearing is logged.
- Cold / evicted mags can still be soft-locked. The lock travels with the mag record.

### Not a hard lock
- Files are never made immutable by the soft lock.
- ZFS or filesystem permissions are unchanged.
- It is a workflow warning, not a security boundary.



### Job card (client) — Phase 2

The client dashboard has a Job card page. The field operator loads it to see what the producer required before setting up gear.

- Operator selects an open project.
- The page shows the fixed values: frame rate, resolution, GOP.
- Read-only for normal keys. Real projects are created only in the Project manager on the server. A producer key is a client SSH key; it cannot open a server screen. The producer uses the server console (or an admin session) for the Project manager. The client Job card only sets values on an existing open project.
- If a project has no requirements, the page says so.

### Accountability

The ingest warning is the record. If the operator loads footage that does not meet the job card, Magvault logs the mismatch to the project. The producer can open that log and confront the operator. The warning does not block the ingest. It makes the miss visible.

## Project requirements warning

Requirements are set at project creation. Fixed values only, chosen from lists. No free text, so no typos.

Lists (locked for now), in priority order:
1. Frame rate: 24, 25, 30, 50, 60
2. Resolution: 4K, HD, SD
3. GOP: intra, long-GOP

A project can set any combination. FPS is required. Other requirements are optional. Empty of those means no check for them.
Frame rate is checked first. Then resolution. Then GOP.

At ingest, after the card is probed and the project is selected:

- Magvault compares the detected resolution, frame rate and format against the project's fixed values.
- If any set value does not match, it shows: "Camera footage does not meet project requirements."
- The warning is not a hard stop. The operator can still confirm and ingest.
- The mismatch is logged to the project log (and the mag) so the producer sees it later.
- If the project has no requirements set, no check is done.

Project creation must offer these lists. Editing later is allowed, but the values stay from the same lists.

Phase 1 implementation:
- Stored as `projects.requirements` (`fps=25 res=HD gop=intra`). Set when the project is created in the Project manager. Ingest `n` creates a temp project only (FPS required). An existing real project keeps its values; ingest cannot change them.
- The card is compared with one value per card (the clips' frame rate, resolution class 4K / HD / SD, intra or long-GOP from the codec). 23.976 counts as 24, 29.97 as 30, 59.94 as 60. A mixed card fails when any clip differs. A value the card does not report is not a mismatch ("not checked").
- The project log is the `project_log` table; `magvault project-log <project>` shows it. The mag card gets `requirements_warning: …`. The server re-checks every client upload against its own requirements and logs the mismatch.

## Project open / closed rule

- **Open** projects can receive new mags and are shown in the ingest project list.
- **Closed** projects are finished (typically moved toward cold storage). They are hidden from ingest and only appear in the library.
- Closing or re-opening a project is a deliberate action. It only changes the flag; files on disk never move.
- Default view everywhere except the library shows only open projects.


## Profile updates

Camera and encoder profiles are updated without a full Magvault upgrade.

- Shipped inside the `.deb` (the `share/cameras` and `share/encoders` set).
- Or pulled from a git repository (or another profile repo) by a profile update command.
- Update copies new or changed `.cam` / `.enc` files into the configured cameras and encoders folders.
- Existing local edits are not overwritten unless the operator confirms.
- `init` still installs the starter set into an empty folder.

## Camera profiles

One plain-text `.cam` file per camera, all living in the `cameras/` folder (path in path.conf).

- Server (or standalone) scans the folder on startup and on a manual reload.
- Valid files are loaded into the database / served to clients.
- Clients never read the files directly. They request the current profile list from the server over SSH (or use a local folder when running standalone).
- Phase 1: clients still pick from a local copy (temporary), but before copying a card they check the chosen camera, format, gamma and gamut against the list the server publishes (gate verb `cameras`). A mismatch is refused before anything is copied; offline, the cached list gives a warning and `sync` never uploads a mag the server would refuse.
- `model:` is the string the camera writes in its clip metadata (Sony body codes such as `ILCE-7SM3`, `ILME-FX6V`), not the marketing name. A comma list allows several spellings (`AG-CX350, CX350`).
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
model: ILCE-7SM3
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
codec: h264_nvenc   # server uses this when encoder: gpu; libx264 when encoder: cpu
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

notes: Library proxy. Queued only after assignment to a real project.
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


### Debug logging

Extra operational detail for test benches and fault finding. Off by default.

- Config: `debug: yes` in `magvault.conf` (or `debug: no`).
- CLI override: `--verbose` / `--debug` for one run.
- Writes to the same daily log files under `logs/` that the Logs viewer reads. Level `DEBUG`.
- Filterable in the Logs viewer like INFO / WARN / ERROR.
- Covers: gate verbs, serve accept/reject, rsync start/end, hash verify, incoming rename, helper calls (never the sudo password).
- No secrets. No key material.
- Client debug lines are also sent to the server log when the client-issues path is up (same rate limit).

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
- **Client user** (`magvaultclient`): shared username for all clients. Each client is identified by its own unique SSH key. Can use Ingest, Logging, Dashboard, Encoder, etc. Cannot open admin-only modules. A producer key may edit the Job card on an open project. It cannot create real projects from the client.

**Producer key** (optional) — Phase 2.
- Most clients use a normal key. They cannot create projects.
- A key can be marked `producer: yes` in the Key module.
- This is not sudo. It does not grant ZFS, Prepare, or any root helper rights.
- It only allows production actions from the client: edit the Job card on an open project (requirements from the fixed lists). It cannot create a real project.
- The Job card stays read-only for normal keys.
- The gate checks the key label. Creating a project is logged with that label.
- Server admin sets or clears the flag. Normal keys stay the default.

### Sudo (no root)

The `magvault` user never runs as root. Its only sudo right is the one root-owned helper (see "Sudo helper"); the helper runs the exact commands needed (`zpool` / `zfs` for zmags from Phase 4; ext4 format and clean of field USB drives from Phase 1). The password is still required:

```
magvault ALL=(root) /usr/lib/magvault/magvault-helper
```

No direct sudo on `zpool` or `zfs`. The operator must enter the sudo password. Nothing is executed until the password is accepted.  
Clients (`magvaultclient`) have no sudo rights on the server.

On a client machine the operator's own account (not `magvaultclient`, which exists only on the server) may run the same helper with password-required sudo, for USB Prepare / Clean only.  
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

ASC MHL (`ascmhl/manifest.mhl`) is written at the same time. It holds the industry-standard inventory (filenames, sizes, mtimes, C4 hashes; see "Hashing") and nothing else.

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
8. If the mag is assigned to a real project, queue a 320p job. Temp mags are not encoded.
9. Mag is now online and linked. Server encodes when it has capacity.

Per-file shot logging is not part of ingest.



## Time display

- Always 24-hour clock.
- Date format DD/MM/YYYY.
- Both can be set in config (`date_format: DD/MM/YYYY | YYYY-MM-DD | MM/DD/YYYY`, `time_format: 24h | 12h` in magvault.conf). Defaults are 24-hour and DD/MM/YYYY. Stored values (magcard, database, logs) stay ISO / UTC.
- Ingest has a system-time block (wall clock).
- Block 2 is a card summary, not a file list:
  - Shoot date and clip time range (e.g. 11:30 – 15:43)
  - Number of clips
  - Total size
  - Frame rate for the card (dominant value; “mixed” if clips differ)
  - Resolution for the card (same rule)

## Client staging cache

Local staging on the client is a temporary holding area between the card and confirmed backups.

- The ingest screen shows a space bar for the staging cache (used / free).
- Auto-clean removes a staged mag as soon as at least two backups exist (for example the field set and the server, or two other confirmed copies).
- Auto-clean can be disabled (`staging_autoclean: no` in the client's magvault.conf). When disabled, staged mags stay until the operator clears them (`magvault staging clean`).
- If staging is the only copy, it is never auto-cleaned.
- The server confirming a mag is one backup. Staging + server alone keeps the staged copy; staging + server + field set removes it. `magvault staging` lists every staged mag with its state; `magvault staging clean <mag>` removes one server-confirmed mag by hand (asks first when the server is its only backup). A pending mag is never removed.

## Card dump status

On the card detect screen, each card shows whether it has already been dumped:

- Red: not dumped. No Magvault copy exists.
- Yellow: dumped, but not yet two copies and uploaded to the server.
- Green: two copies exist and the mag is uploaded to the server.

Status comes from hashes / mag identity, not from the volume label. Phase 1: the client records each mag it ingests with a fingerprint of its card (relative path, size and mtime of every file) in `<config>/dumps.txt`; a detected card with the same fingerprint is dumped. Copies counted: server (confirmed), staging, field set. A card counts as dumped only when its volume identity matches too (this client ingested this card); files that match a mag ingested from another volume (a card image, a copy) are yellow with a note, never green. Ingesting a dumped card again warns "already dumped"; `--yes` refuses unless `--force`, and the re-ingest is logged.

## Help and log keys

- F1: help for the current screen. In this batch.
- F2: deferred. Not in this batch. Revisit when the Logs module is real.

## Client ingest and field backups

Ingest can run on a client terminal (or standalone machine). After a successful commit the server owns the rest of the pipeline (proxy encode, etc.).

**Copy options (selectable on the main ingest screen)**
- Default: card → local staging → rsync to server → server confirms. Staging is auto-cleaned only once two backups exist (see "Client staging cache").
- Backup USB drive can act as the staging location itself.
- Direct card → server (optional).
- Extra mirror to one or more backup drives at any time.
- Warning shown when no backup exists and server sync is not yet confirmed.

**Field backup sets**

Field backups are matched pairs. High-pace, opinionated. The operator does not choose layouts.

- A set is exactly one **master** and one **slave**.
- Both drives hold the exact same set of mags. Always.
- Usable size is the size of the smaller drive.
- Prepare order:
  - Warning shown when creating a set: “Mount the smaller drive first and prepare it as master.”
  - Master is prepared first. Its size becomes the set size.
  - Slave must be the same size or larger. If smaller, refuse.
  - If the slave is larger, create a partition (or limit) equal to the master’s usable size. The rest of the slave is unused.
- A slave cannot be prepared before its master exists.
- A mag cannot be added to one drive without the other.
- If the larger drive was prepared first, a smaller slave is refused. The operator must start a new set with the smaller drive as master.

**magvault-backup.txt** (at the root of each drive in the set)
- `role: master` or `slave`
- `set_id`
- `set_size` (usable bytes)
- label, created time, host
- list of mag folders and whether each is pending server sync or already synced

- Same mag-folder layout as the server + original files + `magcard.txt` (with SHA-256).
- When mounted, client lists the set, projects/mags, and free space.
- Operator can reconnect the set later and finish “sync pending to server.”
- Checksums live in the mag cards, so verification works on any filesystem.
- Encryption is handled at the OS level if desired (not by Magvault).

This is field USB only. It is not related to zmag primary/backup roles.

**USB drive diagnostics and prepare**

- Only a Magvault-prepared drive (has valid `magvault-backup.txt` at root) can be selected as a backup target.
- Any other drive is shown as “not valid”, except a drive of a prepared set that is simply not mounted (ext4 labelled `<SET>_M` / `<SET>_S`): it is shown as “set not mounted” and is never offered for Prepare.
- Not-valid drives offer a **Prepare** action. Prepare opens a popup. It is not inline on the main ingest screen.
- USB hard disks are valid Prepare targets even when the kernel reports them non-removable (`RM=0`). Detect them by USB transport (`TRAN=usb`).
- Camera cards and card-reader devices (for example `mmcblk`) are not Prepare targets. `usb status` must not offer Prepare on them.
- A disk that still has a ZFS, LVM, RAID, or LUKS signature is refused. The message must name the signature and say to wipe it first (`wipefs`). Magvault does not auto-wipe. It must not say only “not a removable drive.”

**Prepare popup**
1. Warning: “Mount the smaller drive first and prepare it as master.”
2. List mounted drives with sizes.
3. Operator picks the master. Confirm size. Sudo password. Format ext4. Write `magvault-backup.txt`.
4. Prompt for the slave. Must be the same size or larger. If smaller, refuse with a clear reason.
5. If the slave is larger: “Will use X GB to match master. Rest unused.” Confirm. Sudo. Partition or limit to the master size. Write `magvault-backup.txt`.
6. Esc cancels. Nothing is written until each sudo password is accepted.

- Once a set is prepared, Magvault will not offer Prepare again on those drives while any mag is marked “pending server sync”.
- Prepare is only re-enabled after every mag on the set has been successfully transferred to the server.
- A valid set with pending mags remains selectable as a target; it simply cannot be re-prepared until synced.


**Clean and copies**
- Field crew do not manage archive copies. Once a mag is synced, Clean may wipe it from the USB set. No warning about "only copy".
- The server is the archive. ZFS on the server is the primary defence (pool redundancy). Zmags and further copies are a server-side concern, not a field concern.
- The camera card is outside Magvault. Clean does not touch it.

**Clean**
- Available only when every mag on the set is synced (no pending).
- Opens a popup: “All mags on <set> are on the server. Wipe master and slave?”
- Sudo required. Nothing is deleted until the password is accepted.
- Deletes the mag folders on both drives.
- Keeps `magvault-backup.txt`, the set id, the size limit, and the master/slave roles.
- After clean the set is empty, still valid, and ready for new mags. No re-prepare.
- Logged.
- Re-prepare is only for a new set (different drives, different size, or a drive that was never prepared).

The main ingest screen shows the finished set (master + slave, valid, pending/synced) or “not valid · Prepare”.

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

Both versions are ratatui. Esc returns to the dashboard from any module (blocked while a write/copy runs); `q` quits, only on the dashboard (see "Shared key convention"). Keyboard first (arrows + Enter or single-letter shortcuts). Mouse optional.

## Shared key convention (terminal)

Left / Right move between panes (same as Tab). Up / Down select the item in the current pane.

One set of navigation keys for every screen. Defined once so modules stay consistent.

| Key | Meaning |
|-----|---------|
| `Esc` | Return to the dashboard (from any module). Blocked while a write/copy is in progress. |
| `q` | Quit the whole program. Only on the dashboard itself. |
| `←` / `→`, `Tab` / `Shift+Tab` | Previous / next pane |
| `↑` / `↓` | Move within a list or menu |
| `F1` | Help for the current screen |
| `Enter` | Confirm, open, or select the current item |
| Mouse | Optional. Click a pane to focus it. |

Module-specific actions use single letters. They must not collide with the shared keys above.

### Dashboard
| Key | Action |
|-----|--------|
| letter | Open module (`i` Ingest, `g` Logging, `l` Library, `e` Export bins, `z` Zmag, `p` Profile Editor, `v` Logs, `s` Settings, `h` Server dashboard over SSH (client)) |
| `r` | Refresh status |
| `y` | Sync pending mags (client) |
| `d` | Show / hide details (paths, UUIDs, device names) |

### Ingest
| Key | Action |
|-----|--------|
| `n` | New temp project (name + FPS required) |
| `r` | Rescan for cards |
| `p` | Pull (start verified copy) |
| `o` | Prepare selected not-valid USB (sudo) |
| `c` | Clean a fully synced field set (sudo) |
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

Mostly automatic: assignment to a real project queues a 320p job on the server. The module is the place to watch, manage, and override.

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
- A 320p job is created only when a mag is assigned to a real project (ingest into a real project, or Project manager assignment). Temp mags are not encoded. Jobs already queued in Phase 0/1 for real projects stay queued.
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





## Client ↔ server status

SSH reachable is not the same as the server committing.

- The client treats the server as ready only if `serve` answers a status/ping that means it is alive and accepting commits.
- If SSH works but `serve` does not answer, status is **server up, committer down**. Do not show “reachable” alone.
- Ingest and sync wait with that warning. They time out with a clear message. They do not spin forever.
- The dashboard uses the same check.

### Client issues on the server log

Operational client problems are also written to the server daily log, for debug when nobody is on the field machine.

- Sent over the existing SSH gate (a `log` verb, or with status). No new API.
- Server line includes the client label, for example: `WARN client rafiki  server committer down; ingest waiting`
- Only operational events: connect failure, serve down, timeout, ingest/sync error, sudo cancelled.
- Not keypresses. Not secrets (passwords, key material).
- Rate-limited so a stuck client cannot flood the log.
- The client still writes its own local log.


## Internal modules

Magvault is extended by modules we build. There are no third-party plugins.

- Each module is Rust in this repo (`src/<module>/` or a crate) with a fixed shape: name, keys, draw, on_key, cli command.
- Modules are compiled into the same binary. No dynamic `.so` loading. No WASM. No external plugin directory.
- A module may use only the shared services: config, database, log, gate, rsync, sudo helper.
- It must not write outside its configured roots.
- The dashboard menu lists enabled modules. Core (dashboard, ingest) is always present.
- Later phases (encoder, library, logging, zmag, settings, keys) are modules in this sense. They are not plugins.

## CLI and scripting

Magvault is terminal-first. It is not an API. There is no HTTP service and no client library.

### Help

- `magvault --help` lists commands only, grouped (server / client / both). One line each. No flag dumps.
- `magvault <command> --help` documents that command: purpose, flags, one example.
- `magvault help <command>` is an alias.
- Top-level help stays short as modules are added. Flags live on the command that uses them.

Every action the TUI can perform must also be performable from the CLI with flags, so a human, a shell script, or a bot can run it without a TUI.

- Non-interactive mode must work when the inputs are unambiguous (`--yes`, one discovered card, explicit paths, explicit set).
- If the input is ambiguous (several cards, missing project, smaller slave, pending sync), refuse with a clear message. Do not guess.
- `--yes` accepts guessed values that are already determined (camera profile, single card). It does not skip integrity or sudo checks.
- Bots and tests use the same binary and the same flags. No separate automation interface.
- Privileged actions (Prepare, Clean) still require sudo. Scripts must be able to supply that through the normal sudo mechanism.

## Threat model

Magvault handles original camera media and production metadata. The main risks are data loss, silent corruption, unauthorized access, and privilege misuse.

### Assets
- Original camera files and sidecars (irreplaceable)
- Mag cards, export cards, ASC MHL
- Database (index and metadata)
- SSH client keys and the sudo helper
- ZFS pools and zmags

### Actors
- Operator on the server (magvault user)
- Field client (magvaultclient via SSH key)
- Admin (same as server user for now)
- Remote zmag station (later)

### Threats and mitigations

| Threat | Mitigation |
|--------|------------|
| Accidental or malicious overwrite of originals | Originals are never renamed or edited. Copies are verified. Soft lock warns before reuse. |
| Silent corruption | SHA-256 at ingest. Every later copy verified against stored hash. Mismatch is an integrity error; nothing is committed. |
| Path traversal or write outside roots | All paths resolved against configured roots. Incoming refused if cross-filesystem or under /tmp. Clients never write directly to mags/. |
| Unauthorized client access | SSH keys only. Forced command + restrict. AuthorizedKeysCommand. No passwords. Admin verbs refused on the client gate. |
| Privilege escalation | Single root-owned helper. timestamp_timeout=0. Magvault user has no general sudo. No root login. |
| Database / file divergence | Database authoritative for metadata. Manifest fixed at ingest. Mismatches reported, never auto-resolved. Magcard backed up before rewrite. |
| Key material leakage | Private keys shown once, written to file or USB, then discarded. Not stored on the server after handover. |
| Log injection or secret leakage | Structured log lines. No keys, passwords, or full paths that leave the system in logs. |
| Denial of service via large ingest | Free-space checks before copy. Incoming cleaned on failure. |

### Phase gate
Each phase must pass:
1. Functional tests on real hardware
2. Security review against this threat model (new code paths, new privileges, new trust boundaries)
3. Fixes for any findings

Only then does the next phase start.

### Out of scope for v1
- Multi-tenant isolation beyond Unix users and SSH keys
- Encryption at rest beyond what ZFS provides
- Formal penetration testing
- Protection against a compromised root or physical access to drives

## Implementation defaults

These are the remaining recommendations from `docs/technical-review-and-spec.md` that are adopted as defaults. They were not owner decisions; they are the smallest consistent choices an implementer should follow unless a later decision overrides them.

### Process and transport
- `magvaultd` daemon + TUI client. Client talks to the server over SSH with a forced-command gate.
- Per-client `incoming/` spool (via `rrsync -wo`). Server commits into `mags/` after verification. Clients never write directly to `mags/`.
- Staging is never `/tmp`. Use a path on the mags dataset or a dedicated staging dataset.

### Naming and paths
- Mag folder names use the full UUID. Short forms are display-only.
- Relative paths everywhere in cards and manifests. No absolute paths stored in magcard or exportcard.
- Export layout recreates the original relative path under the mag folder inside the bin.

### Cards and manifests
- `magcard.txt` and `exportcard.txt` use a simple `key: value` header plus a `# manifest` or `# clips` section.
- Pipe `|` is the field separator in list sections. Reject lines that contain extra pipes or control characters.
- ASC MHL is written at ingest. Later copies (zmag, cold pull) can write additional MHL generations.
- Logging fields live in a `# logging` section (already locked).

### Schema notes
- Camera is always a profile id (or empty). No free-text camera model in the database.
- One media file = one clip in v1. Spanned/chaptered clips are a later feature.
- `archived` status is not used. Cold is the terminal state for a mag that has been evicted.
- Cold-pull requests link to the export that triggered them.

### Security
- SSH keys for clients are served via `AuthorizedKeysCommand` reading a Magvault-managed file. Not a hand-edited `authorized_keys`.
- Forced command + `restrict` on client keys. Admin verbs are refused over the client gate.
- Sudo helper is the only privileged path (already locked). `timestamp_timeout=0`.
- Input validation on every path and filename that crosses the SSH boundary.

### Other
- Snapshot creation is external (cron or ZFS auto). Magvault only lists, holds, and destroys.
- Offline logging conflicts: per-field last-writer-wins, with a WARN log line.
- RAW formats without an ffmpeg decoder: proxy job fails with `unsupported codec` and is logged. No vendor SDK in v1.
- Log day boundary is UTC.
- USB master/slave: master = staging + sync source; slave = mirror only. Or drop the distinction if it adds no value.

Full rationale and the complete specification are in `docs/technical-review-and-spec.md`.

## Roadmap

### Phase 0 – Foundation
- Config directory + path.conf + system pointer file.
- SQLite schema.
- Manual-path ingest: probe, confirm, hash at ingest, verified copy, magcard.txt + ASC MHL, DB records. 320p job only if the project is real.
- Conflict handling (database wins for metadata, magcard backed up).
- Basic shell only.

### Phase 1 – Client ingest & field backups
- Card discovery.
- Full client ingest (staging, USB, direct, mirror).
- Prepare formats USB as ext4.
- Reconnect-and-sync, pending-sync safety.
- Server confirmation + autoclean (after two backups).




### Assignment pull-back

When a producer assigns a temp mag to a real project:

- Server updates `project_id` and rewrites `magcard.txt` (previous card backed up).
- Encode is allowed only after assignment.
- On the next sync or status, the server sends mag UUID → real project.
- Client updates its local record and any staged `magcard.txt`.
- When the field set is next mounted, client rewrites `magcard.txt` on master and slave and the project line in `magvault-backup.txt`.
- Folder name does not change. Media is not copied again.
- Until the set is seen, drives may still say `(T)`. Server is the authority.
- Client shows “assigned, drives not updated” if the set is absent.


### Temp project names

- No auto-merge. A temp project is never attached to an existing project because the name matches.
- Dump and sync are not blocked. The mag is pulled to the server and stays temp.
- If the name already exists on the client, the client refuses to reuse it for a new temp project.
- In the Project manager, a producer either merges the temp into the existing project or assigns a new name.
- Real projects may still attach a mag when the operator selects that open project. Closed projects never receive new mags.

### Offline temp project

Never refuse a dump because the server is down.

- Ingest `n` (new project) always creates a temp project. There is no real-project create on that screen.
- Temp projects show a `(T)` suffix.
- Operator sets a project name and FPS (other requirements optional).
- Dumping and uploading are unhindered. Approval only gates the encode pipeline, not the copy.
- Dump proceeds as normal and gets a UUID. Copy can go to staging and/or the field set.
- When the server is reachable, the mag is pulled up.
- It does not enter the encode pipeline.
- It sits in waiting until the Project manager assigns it to a real project.
- Anyone can create a temp project. It is not in the final archive until a producer approves it in the Project manager and assigns it to a real project.


### Project manager — Phase 2

Server-only. Admin or producer key.

- Create a real project (name, requirements from the fixed lists, FPS required).
- List open, closed, and temp projects.
- Merge a temp into an existing real project, or assign a new name.
- Edit requirements. A change only affects the incompatibility warning on later ingests. Old mags are not rewritten.
- Job card is the client view of an open project's requirements.


**Encoder backend**
- `path.conf` key `encoder: gpu` or `encoder: cpu`. Default `gpu`.
- `gpu` uses NVENC when a GPU is present. If no GPU is found, fall back to CPU and log WARN.
- `cpu` uses libx264. CRF and preset apply only to libx264. NVENC uses its own rate control.
- Courage: confirm GPU model before Phase 2 coding. The profile file stores one codec; the server picks the matching profile.

### Phase 2 decisions

- Encode runs on the server. Clients do not encode. GPU if present, CPU fallback. Setting in config (`encoder: gpu` or `cpu`).
- A 320p job is created only when a mag is assigned to a real project. Temp mags are not encoded.
- Telemetry on day one: queue depth, disk free, serve heartbeat.
- Temp projects: ingest `n` always creates a temp project. FPS is required. No auto-merge by name. Client refuses a name already in use locally. Producer merges or renames in the Project manager.
- Real projects are created only in the Project manager (admin or producer key).
- Key manager in Phase 2 is a TUI.
- Clean: field does not manage archive copies. Server ZFS is the primary defence. USB may be wiped once synced.

### Phase 2 – Proxies, encoder, dashboards, projects, keys
- 320p encode worker. Jobs start only on assignment to a real project.
- Encoder module (queue + ad-hoc to custom_encode/).
- Server dashboard (landing) and client dashboard. Library is a stub only. Full Library is Phase 3.
- Telemetry file.
- Project manager: create real projects, set requirements, merge or rename temps, Job card on the client.
- Key manager TUI: generate, accept, revoke, producer flag.
- Producer key and Job card are Phase 2, not deferred.

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
