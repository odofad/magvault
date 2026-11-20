# Magvault

Archive and retrieval system for camera media mags.

Strictly an archive tool, not an editor. It keeps raw camera mags organised, backed up, and findable, then hands people exactly the clips they ask for so they can cut in a real NLE.

Target: Ubuntu 26.04 and newer. Open source — if someone wants to run it on Red Hat or anything else, they are welcome to sort out the packaging.

## Design principles

- **Everything is a file.** SQLite database (one file), directories of proxies and bins, plain manifests/EDLs, status files. You can `ls`, `rsync`, `cat`, or query with `sqlite3` when the interface is down.
- **Terminal first.** Core operations are driven from the shell or small Rust binaries that call `ffmpeg` and `rsync`. No APIs, no HTML, no web stack.
- **Optional desktop layer.** A thin egui + wgpu/Vulkan front-end can sit on top for mouse-driven browsing and preview. It only reads and writes the same files; it owns no unique logic.
- **No CUDA.** Vulkan (via wgpu) for any custom rendering, VAAPI or software paths in ffmpeg for encode/decode. AMD-friendly, proprietary-free.
- **Leverage existing tools.** ffmpeg for probe/proxy/subclip work, rsync for movement and backups. The system orchestrates them and logs what they did.
- **Low resource.** 320p proxies for browsing. Originals stay cold until explicitly requested. Runs on modest hardware; can boot straight into the tool like an old bank terminal.

## What it does

1. **Ingest and logger** — Point at a mag or folder. Runs ffprobe, records metadata and checksums, notes the physical mag. Turns cards into searchable records.

2. **Database + proxy subsystem** — SQLite for assets, mags, locations, jobs, proxies. Generates tiny 320p versions of everything so people can browse and scrub without touching originals.

3. **Whole-mag mover / backup** — Treats complete raw mags as units. rsync wrapper that logs source, destination, checksum, and success. No timecodes, no partial clips.

4. **Low-res library + request layer** — Search and watch 320p proxies. Mark what you want and pull a request. If online, drops a bin folder on the SMB share. If cold, creates a pull request.

5. **Cold-storage pull and reingest** — Separate workflow for the wrangler in the server room. They see open requests, grab the physical mag, reingest the needed files, verify checksums, mark online. The bin then gets populated.

6. **Planning / selection tool** — Scrub 320p proxies, mark ranges or whole clips, hit export. Does not cut or render. Only requests those clips (triggering a cold pull if needed) and, once available, writes a bin folder on the SMB share containing the clips (or links) plus a simple manifest/EDL. Open it in any NLE.

7. **Dashboard** — Terminal (or optional GUI) overview: mags online vs cold, proxy coverage, open pull requests, recent ingests, disk space, exported bins. Just queries the SQLite file and directories.

Server and client can both run the GUI if wanted. Proxies move over SSH/rsync for remote preview. The raw archive stays put until someone asks for specific clips.

## Roadmap

**Phase 0 – Skeleton (files first)**  
SQLite schema, directory layout, small Rust binaries that ingest via ffprobe and generate 320p proxies with ffmpeg. Usable from the shell.

**Phase 1 – Whole-mag backup and basic query**  
rsync wrapper for complete mags, logging into the database. Terminal views to list mags, locations, and query assets.

**Phase 2 – Library and requests**  
320p generation, terminal browser for the low-res library, mark clips and create pull requests. Write bin folders on the SMB share when media is online.

**Phase 3 – Wrangler pull/reingest**  
Terminal tools for open requests. Targeted reingest from physical mags, checksum verify, mark online, finish the bin.

**Phase 4 – Optional desktop layer**  
egui + wgpu/Vulkan front-end that performs the same operations on the same files. Minimal boot-to-Magvault image for server-room terminals.

**Phase 5 – Polish**  
Dashboard numbers, robust proxy rules, failure handling, packaging for Ubuntu 26.04+.

Each phase leaves a usable terminal-driven system. The GUI and kiosk image are additive.

## Status

Design phase. No code yet. This repo exists so the notes survive.
