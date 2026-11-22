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
- Contents: exact verified copy of the card (original filenames + matching sidecars). If it was not on the SD card, it is not in the mag folder — except `magcard.txt` and the hidden `.magvault/` backups of that card.
- Mag card is plain text (`magcard.txt`), complete manifest + confirmed metadata + project link. Stays with the folder forever.
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

## Conflict handling

Database is authoritative.

When a mismatch is detected (cold mag reappears, card differs from database for that mag UUID):
1. Copy existing magcard.txt to `.magvault/magcard.YYYYMMDD-HHMMSS.backup`.
2. Write a fresh magcard.txt from the current database record.

Applies to all fields: camera, format, gamma, gamut, project assignment, manifest, notes, shoot date.

## Ingest flow

1. Operator plugs in physical mag (SD/USB) or points at mount point.
2. System probes the card (BitMatch-style full copy preparation):
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
6. Write complete-manifest magcard.txt into the dump folder.
7. Insert records in SQLite.
8. Generate 320p proxies into `proxies/db/<shootday>_<mag-uuid>/` (immediate or queued).
9. Mag is now online and linked.

Per-file shot logging is not part of ingest.

## Proxies

- `proxies/db/` — always-on 320p library proxies. Same folder naming as mags.
- `proxies/hq/` — optional full-resolution lower-bitrate HEVC (30-60 Mbps). Same naming. Generated only when requested.
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

## Telemetry

Live dashboard numbers live in a RAM-backed file (tmpfs or /dev/shm) so frequent updates cause no disk wear. A periodic copy (default every 30 minutes) is written to disk for reboot survival or light history.

Both paths are in path.conf:

```text
telemetry_live=/dev/shm/magvault-telemetry.json
telemetry_disk=logs/telemetry-latest.json
```

Readable without root. Client dashboard (or any SSH session) simply reads the file. Persistent history, if wanted, is a separate less-frequent log.

## Phase 0 – done when

- Schema exists.
- Probe + operator confirmation (including shoot day) + project assignment works from the terminal.
- Verified full-card copy into correctly named mag folder succeeds.
- Mag card (complete manifest) and database records are written.
- Basic 320p proxy can be generated into the matching proxies/db folder.
- Conflict detection and card rewrite (with backup in .magvault/) works.
- Usable entirely from the shell. No GUI required.

## Later phases

- Phase 1: whole-mag rsync backup + query views.
- Phase 2: 320p library browser + request layer + bin export.
- Phase 3: wrangler cold-storage pull and reingest.
- Phase 4: optional egui/Vulkan desktop layer + boot-to-app image. Split function.
- Phase 5: polish, packaging. HQ proxy toggles.

## Borrowed ideas

See NOTES.md. Main ones in use:
- Verified SHA-256 full-card copy from BitMatch.
- Camera detection seed list from BitMatch.
- Keep original names + sidecars.

## Open points

- Exact terminal UI for confirmation (plain prompts vs ratatui).
- Whether 320p generation is synchronous or queued.
- ASC MHL alongside mag card or not.
- Automatic card discovery vs manual path.
- Details of the later split UI and how the original folder is retired.
- Dashboard client that reads the telemetry file.
