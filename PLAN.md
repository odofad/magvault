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

## SQLite schema (sketch)

### projects
- id (UUID, primary key)
- name (label only)
- season_or_year (optional disambiguator)
- notes
- status (open / closed) — open projects appear in ingest; closed projects are hidden from ingest and only visible in the library
- created_at, updated_at

### mags
- id (UUID, primary key)
- project_id (UUID, foreign key, required)
- dump_path
- shoot_date (the YYYYMMDD used in the folder name)
- camera_model (confirmed)
- file_format (confirmed)
- gamma (confirmed)
- gamut (confirmed)
- status (ingesting, online, cold, etc.)
- ingested_at
- notes

### assets (files inside a mag)
- id
- mag_id (UUID)
- original_filename
- relative_path
- size
- checksum (SHA-256)
- proxy_path (nullable)
- later logging fields (description, etc.) can be added without schema pain

Projects and mags are identified by UUID. Names are labels.

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
- Module menu: Ingest, Logging (local-capable), and the ability to open the full server dashboard over SSH.
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

### Zmag manager
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
- Auto-backup: insert first zmag → copy everything that needs a first copy; insert second → copy everything that needs a second copy.

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

**Key module**
- Manages Magvault SSH key pairs.
- Installs public keys to clients or remote stations.
- No passwords. Security is SSH keys + normal Unix permissions on the paths.

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

## Roadmap (revised)

### Phase 0 – Foundation
- Config directory + path.conf + system pointer file.
- SQLite schema created.
- Manual-path ingest: probe, operator confirm, verified copy, magcard.txt + ASC MHL, DB records, queue 320p job.
- Conflict rewrite of mag card.
- Basic shell only.

### Phase 1 – Client ingest & field backups
- Automatic + manual card discovery.
- Full client ingest screen (staging, USB-as-staging, direct, mirror, warnings).
- Field backup drives with magvault-backup.txt, reconnect-and-sync.
- USB diagnostics, Prepare (sudo), and pending-sync safety rule.
- Server confirmation + autoclean.

### Phase 2 – Proxy subsystem & telemetry
- Server-side encode queue (320p).
- Proxy rules enforced.
- Live telemetry file (encoder queue) + periodic disk snapshot.
- Server dashboard (landing screen) that reads the telemetry file.
- Client dashboard (thin status + module menu).
- Encoder module: queue management, new ad-hoc job to custom_encode/, profile shortcut.

### Phase 3 – Export bins & library requests
- Named export folders + exportcard.txt.
- Copy originals only.
- Discover exports by walking bins path.
- Select clips → create/add to export.
- Cold-pull request generation.

### Phase 4 – Zmag manager (local)
- Create zmag from raw drives.
- Auto-import/mount recognised zmags only.
- Two-pane backup terminal.
- Auto-backup pass.
- Placement tracking + on-zmag manifest.
- Zpool health in telemetry.

### Phase 5 – Remote zmag + key module + cold pull
- SSH key module.
- Remote zmag station.
- Full cold-pull workflow.

### Phase 6 – Polish & optional desktop
- Logging module (client + server) with mpv proxy playback.
- Full dashboard client refinements.
- Split function.
- HQ proxy toggle.
- Separate standalone mouse-first desktop client (egui/Vulkan or similar).
- Packaging and docs.
- Profile Editor module (camera + encoder profiles).

## Borrowed ideas

See NOTES.md. Main ones in use:
- Verified SHA-256 full-card copy from BitMatch.
- Camera detection seed list from BitMatch.
- Keep original names + sidecars.

## Open points

- Exact terminal UI for confirmation (plain prompts vs ratatui).
- How project list and “create new project” work in the terminal.
- Precise SQLite tables for placements, encode jobs, pull requests, exports, logging fields.
- Encode queue implementation details.
- How the server notices a newly committed mag and enqueues the proxy job.
- Card-layout detection completeness and fallback.
- Exact on-zmag manifest format and location.
- Export discovery: pure filesystem walk vs light index.
- Telemetry JSON schema.
- Key module restrictions.
- Failure and retry semantics for every copy path.
- Logging field set and whether logs are also written to a sidecar.
- Details of the later split UI.
- Exact fields inside a camera `.cam` profile.
