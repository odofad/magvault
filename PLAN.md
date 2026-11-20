# Magvault Planning Document

Consolidated design notes before any code. Builds on the README and NOTES.md.

## Goal

Archive and retrieval system for camera media mags. Not an editor. Keeps raw mags organised and findable, then hands out exactly the clips requested so they can be cut in a real NLE.

Terminal-first, file-based, Ubuntu 26.04+ reference, open source. Optional thin desktop GUI later.

## Directory layout

```text
archive/
  mag-dumps/
    <mag-uuid>/
      <original camera files and sidecars>
      magcard.txt          # complete manifest + confirmed metadata
      magcard.*.backup     # conflict backups when rewritten
  proxies/
    320p/
      <mag-uuid>/
        <proxy files>
  bins/                    # SMB-exported bins for NLEs
    <user-or-request-id>/
```

No project folders on disk. All project relationships live in the database.

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

The card stays with the folder forever for identification if the dump is separated from the system.

## Conflict handling

Database is authoritative.

When a mismatch is detected (cold mag reappears, card differs from database for that mag UUID):
1. Copy existing magcard.txt to magcard.YYYYMMDD-HHMMSS.backup (or *.datechanged.backup).
2. Write a fresh magcard.txt from the current database record.

Applies to all fields: camera, format, gamma, gamut, project assignment, manifest, notes.

## Ingest flow

1. Operator plugs in physical mag (SD/USB) or points at mount point.
2. System probes the card:
   - Walk files and sidecars.
   - Auto-detect guesses for camera model, file format, gamma, gamut (using available tools + detection lists inspired by BitMatch etc.).
   - Build full file list with sizes.
3. Present guesses and file list to operator.
4. Operator:
   - Confirms or corrects camera model, file format, gamma, gamut.
   - Assigns to an existing project (select from list) or creates a new project (which immediately receives a UUID).
   - Adds any mag-level notes.
5. System performs verified copy into a new mag-dump folder named by the new mag UUID:
   - Original filenames and sidecars preserved.
   - SHA-256 hashed on the way in; destination verified (BitMatch-style).
   - Only proceeds to commit if verification passes.
6. Write complete-manifest magcard.txt into the dump folder.
7. Insert/update records in SQLite (mag UUID, project UUID, confirmed fields, asset rows with checksums).
8. Generate 320p proxies (can be immediate or queued).
9. Mag is now online and linked.

Per-file shot logging is not part of ingest. It can happen later while viewing proxies, in a dedicated pass, or with future ML.

## Phase 0 – done when

- Schema exists and can be created.
- Probe + operator confirmation + project assignment works from the terminal.
- Verified copy into mag-dump folder succeeds.
- Mag card (complete manifest) and database records are written.
- Basic 320p proxy can be generated.
- Conflict detection and card rewrite (with backup) works when tested.
- Usable entirely from the shell / small binaries. No GUI required.

## Later phases (unchanged)

- Phase 1: whole-mag rsync backup + query views.
- Phase 2: 320p library browser + request layer + bin export to SMB.
- Phase 3: wrangler cold-storage pull and reingest.
- Phase 4: optional egui/Vulkan desktop layer + boot-to-app image.
- Phase 5: polish, packaging.

## Borrowed ideas

See NOTES.md. Main ones for Phase 0:
- Verified SHA-256 copy and “only commit when verified” from BitMatch.
- Camera detection seed list from BitMatch.
- Keep original names + sidecars.

## Open points still to decide when coding

- Exact terminal UI for the confirmation step (plain prompts vs ratatui).
- Whether 320p generation is synchronous at end of ingest or always queued.
- Checksum algorithm details and whether we also write an ASC MHL alongside the mag card.
- How mount-point probing discovers cards automatically vs manual path.
