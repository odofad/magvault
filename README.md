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

A plain-text file that stays inside the mag-dump folder forever. It is a complete manifest (every original filename, ideally with size/checksum) plus the confirmed technical metadata and project link. This keeps the dump identifiable even if it is separated from the system. The database holds the same information and does the day-to-day lifting. An ASC MHL inventory is also written by default for interoperability.

## Conflict handling

Database is authoritative. When a mismatch is detected (for example a cold mag reappears with a stale card), the old mag card is copied to a dated backup and a fresh card is written from the database. This applies to all fields.

## What it does

1. **Ingest** — Probe card, operator confirms technical metadata and assigns project, verified full-card copy, mag card + ASC MHL, queue proxies on server.
2. **Client field backups** — Staging, USB master/slave drives, reconnect-and-sync.
3. **Proxies** — 320p library + optional HQ, strict frame-rate and audio rules, server-side queue.
4. **Export bins** — Named folders of original clips only, export card, no proxies.
5. **Zmag manager** — 4-drive RAIDZ1 cold storage sets, auto-mount, two-pane copy, remote stations, cold pull.
6. **Dashboard** — Telemetry file (encoder queue, zpool health) readable over SSH.

## Roadmap

**Phase 0 – Foundation**  
Config system, schema, manual-path verified ingest, mag card + ASC MHL, queue 320p, conflict handling. Shell only.

**Phase 1 – Client ingest & field backups**  
Card discovery, full client copy options, USB backup drives with manifests, reconnect-and-sync.

**Phase 2 – Proxy subsystem & telemetry**  
Server encode queue, proxy rules, live telemetry file, minimal dashboard.

**Phase 3 – Export bins & library requests**  
Named export folders, clip selection, cold-pull requests.

**Phase 4 – Zmag manager (local)**  
Create/mount zmags, two-pane backup terminal, auto-backup, placement tracking, zpool health.

**Phase 5 – Remote zmag + key module + cold pull**  
SSH keys, remote stations, full restore workflow.

**Phase 6 – Polish & optional desktop**  
Full dashboard, split, HQ toggles, optional egui layer, packaging.

Each phase leaves a usable terminal-driven system. See PLAN.md for full detail.

## Status

Design phase. Builds on ideas from MediaPipeLine and BitMatch. No code yet. This repo exists so the notes survive.
