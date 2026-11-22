# Magvault Planning Document

Consolidated design notes before any code. Builds on the README and NOTES.md.

## Goal

Archive and retrieval system for camera media mags. Not an editor. Keeps raw mags organised and findable, then hands out exactly the clips requested so they can be cut in a real NLE.

Terminal-first, file-based, Ubuntu 26.04+ reference, open source. Optional thin desktop GUI later. Designed so the archive remains understandable and fixable by hand if the database disappears. Fully portable: point a new install at the config directory and it continues.

## Directory layout (locked)

All roots are configurable. The program is pointed at a config directory (via argument, environment variable, or setup). Every other path is read from the files inside it. Defaults below can be overridden so the entire set lives on durable ZFS datasets.

```text
config/                            # location supplied at startup
  path.conf                        # all roots only: mags, proxies, bins, db, logs, telemetry_live, telemetry_disk, ...
  magvault.conf                    # non-path settings
  encoding.conf                    # ffmpeg presets and proxy rules
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

bins/                             # configured path, outside magvault tree by default
  <operator-assigned-name>/        # export folder
    exportcard.txt
    exportcard.txt.backup          # single previous version
    <YYYYMMDD>_<mag-uuid>/         # original mag folder name
      <copied original clips + sidecars>

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

Applies to all fields: camera, format, gamma, gamut, project assignment, manifest, notes, shoot date.

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
   - Confirms or corrects camera model, file format, gamma, gamut, and shoot day.
   - Assigns to an existing project (select from list) or creates a new project (UUID assigned immediately).
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

All targets and states are visible and choosable on the ingest screen.

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

**Local operation**
- Plug in drives → zmag manager auto-imports and mounts only recognised zmags.
- Multiple zmags may be mounted simultaneously.
- Two-pane (or multi-pane) view: left = online server mags (filterable by project), right = contents of the selected zmag.
- Copy is whole-mag only.
- Auto-backup: insert first zmag → copy everything that needs a first copy; insert second → copy everything that needs a second copy.

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

Readable without root. Client dashboard (or any SSH session) simply reads the file. Persistent history, if wanted, is a separate less-frequent log.

The dashboard must show:
- Encoder queue depth and per-job progress.
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
- Server confirmation + autoclean.

### Phase 2 – Proxy subsystem & telemetry
- Server-side encode queue (320p).
- Proxy rules enforced.
- Live telemetry file (encoder queue) + periodic disk snapshot.
- Minimal dashboard that reads the telemetry file.

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
- Full dashboard client.
- Split function.
- HQ proxy toggle.
- Optional egui/Vulkan layer.
- Packaging and docs.

## Borrowed ideas

See NOTES.md. Main ones in use:
- Verified SHA-256 full-card copy from BitMatch.
- Camera detection seed list from BitMatch.
- Keep original names + sidecars.

## Open points

- Exact terminal UI for confirmation (plain prompts vs ratatui).
- How project list and “create new project” work in the terminal.
- Precise SQLite tables for placements, encode jobs, pull requests, exports.
- Encode queue implementation details.
- How the server notices a newly committed mag and enqueues the proxy job.
- Card-layout detection completeness and fallback.
- Exact on-zmag manifest format and location.
- Export discovery: pure filesystem walk vs light index.
- Telemetry JSON schema.
- Key module restrictions.
- Failure and retry semantics for every copy path.
- Logging strategy.
- Details of the later split UI.
