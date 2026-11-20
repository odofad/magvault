# Magvault

Archive and retrieval system for camera media mags.

Strictly an archive tool, not an editor. It keeps raw camera mags organised, backed up, and findable, then hands people exactly the clips they ask for so they can cut in a real NLE.

Target: Ubuntu 26.04 and newer. Open source — if someone wants to run it on Red Hat or anything else, they are welcome to sort out the packaging.

## Design principles

- **Everything is a file.** SQLite database (one file), mag-dump directories, plain-text mag cards, manifests/EDLs, status files. You can `ls`, `rsync`, `cat`, or query with `sqlite3` when the interface is down.
- **Terminal first.** Core operations are driven from the shell or small Rust binaries that call `ffmpeg` and `rsync`. No APIs, no HTML, no web stack.
- **Optional desktop layer.** A thin egui + wgpu/Vulkan front-end can sit on top for mouse-driven browsing and preview. It only reads and writes the same files; it owns no unique logic.
- **No CUDA.** Vulkan (via wgpu) for any custom rendering, VAAPI or software paths in ffmpeg for encode/decode. AMD-friendly, proprietary-free.
- **Leverage existing tools.** ffmpeg for probe/proxy work, rsync for movement and backups. The system orchestrates them and logs what they did.
- **Immutable raw media.** Original camera filenames and all sidecars are never renamed or altered. Human-readable titles and metadata live on the mag card and in the database.
- **Low resource.** 320p proxies for browsing. Originals stay cold until explicitly requested. Runs on modest hardware; can boot straight into the tool like an old bank terminal.

## Storage layout

Every mag is its own dump folder containing the original camera files + sidecars + a mag card. No project folders on disk. All project relationships live in the database.

## Projects

Projects are created as real records with a UUID (the stable identity). The name is just a label; a season or year identifier can disambiguate. Mags are assigned to a project UUID — no free-typing of names at ingest. Projects can be edited or merged later; that only updates database links. Files on disk never move.

## Mag card

A plain-text file that stays inside the mag-dump folder forever. It is a complete manifest (every original filename, ideally with size/checksum) plus the confirmed technical metadata and project link. This keeps the dump identifiable even if it is separated from the system. The database holds the same information and does the day-to-day lifting.

## Conflict handling

Database is authoritative. When a mismatch is detected (for example a cold mag reappears with a stale card), the old mag card is copied to `*.datechanged.backup` and a fresh card is written from the database. This applies to all fields: camera model, file format, gamma, gamut, project assignment, manifest, notes, everything.

## What it does

1. **Ingest** — Operator plugs in the physical mag (SD/USB) or points at the mount. System probes the card directly and auto-detects what it can (camera model, format, gamma, gamut, full file list). Operator confirms/corrects the technical fields and assigns the mag to an existing project (or creates a new one). Only then is the media copied into a new mag-dump folder (original names preserved). The mag receives its own UUID. A complete-manifest mag card is written into the folder and the same data goes into the database. Per-file shot logging is not done here; it can happen later while viewing the proxies, in a dedicated pass, or with future ML.

2. **Database + proxy subsystem** — SQLite for projects, mags (with UUIDs), assets, locations, jobs, proxies. Generates tiny 320p versions of everything so people can browse and scrub without touching originals.

3. **Whole-mag mover / backup** — Treats complete raw mags as units. rsync wrapper that logs source, destination, checksum, and success. No timecodes, no partial clips.

4. **Low-res library + request layer** — Search and watch 320p proxies. Mark what you want and pull a request. If online, drops a bin folder on the SMB share. If cold, creates a pull request.

5. **Cold-storage pull and reingest** — Separate workflow for the wrangler in the server room. They see open requests, grab the physical mag, reingest the needed files, verify checksums, mark online. The bin then gets populated. Conflict handling applies if the card on the mag is stale.

6. **Planning / selection tool** — Scrub 320p proxies, mark ranges or whole clips, hit export. Does not cut or render. Only requests those clips (triggering a cold pull if needed) and, once available, writes a bin folder on the SMB share containing the clips (or links) plus a simple manifest/EDL. Open it in any NLE.

7. **Dashboard** — Terminal (or optional GUI) overview: mags online vs cold, proxy coverage, open pull requests, recent ingests, disk space, exported bins. Just queries the SQLite file and directories.

Server and client can both run the GUI if wanted. Proxies move over SSH/rsync for remote preview. The raw archive stays put until someone asks for specific clips.

## Roadmap

**Phase 0 – Skeleton (ingest first)**  
SQLite schema (projects with UUID, mags with UUID, assets). Directory layout for mag dumps. Probe-and-confirm ingest from physical cards: auto-detect, operator confirmation of camera/format/gamma/gamut, project assignment, verified copy with original names, complete-manifest mag card, database record. Basic 320p proxy generation. Usable from the shell.

**Phase 1 – Whole-mag backup and basic query**  
rsync wrapper for complete mags, logging into the database. Terminal views to list mags, locations, projects, and query assets. Conflict detection on mount/copy.

**Phase 2 – Library and requests**  
320p generation for the library, terminal browser, mark clips and create pull requests. Write bin folders on the SMB share when media is online.

**Phase 3 – Wrangler pull/reingest**  
Terminal tools for open requests. Targeted reingest from physical mags, checksum verify, mark online, finish the bin. Apply conflict handling when cards are stale.

**Phase 4 – Optional desktop layer**  
egui + wgpu/Vulkan front-end that performs the same operations on the same files. Minimal boot-to-Magvault image for server-room terminals.

**Phase 5 – Polish**  
Dashboard numbers, robust proxy rules, fuller conflict and failure handling, packaging for Ubuntu 26.04+.

Each phase leaves a usable terminal-driven system. The GUI and kiosk image are additive.

## Status

Design phase. Builds on ideas from MediaPipeLine (mag cards, detection). No code yet. This repo exists so the notes survive.
