# Magvault

Archive and retrieval system for camera media mags.

Strictly an archive tool, not an editor. It keeps raw camera mags organised, backed up, and findable, then hands people exactly the clips they ask for so they can cut in a real NLE.

Target: Ubuntu 26.04 and newer. Terminal-first (ratatui). Open source.

## Design principles

- **Everything is a file.** SQLite database (one file), mag-dump directories, plain-text mag cards, export cards, profiles, logs. You can `ls`, `rsync`, `cat`, or query with `sqlite3` when the interface is down.
- **Terminal first.** All core modules are ratatui screens. Keyboard driven. A separate mouse-first desktop client is a later option.
- **Files are the long-term record.** The database is the fast index and is authoritative for metadata. The file manifest (names, sizes, SHA-256) is fixed at ingest. Logging fields are written back to the mag card so a lost database can be rebuilt.
- **Immutable raw media.** Original camera filenames and all sidecars are never renamed or altered.
- **Hash at ingest.** SHA-256 is computed when the card is first read. Every later copy (USB backup, server, zmag) is verified against those stored hashes.
- **rsync for all copies.** Progress comes from rsync. Verification happens after the copy.
- **Low resource.** 320p proxies for browsing. Originals stay cold until explicitly requested. Can boot straight into the tool.

## User model

No Magvault accounts. Access is the Unix user:

- **Server** runs as `magvault` (admin). Can open Profile Editor, Settings, Key module, Zmag manager.
- **Clients** share the username `magvaultclient`. Each client is identified by its own unique SSH key.
- Privileged operations go through one root-owned helper. Password required every time. No root login.

## What it does

- **Ingest** — Probe card, confirm metadata, verified copy, mag card + ASC MHL, queue 320p proxy. Prepare formats a USB as ext4 for field staging.
- **Logging** — Per-clip notes, tags, rating, slow-motion, shot type, names. Written to the database and to the mag card.
- **Library** — Browse project → mag → clips. Build an export list (the export card). Works on server and client (client uses a local 320p proxy cache).
- **Export bins** — Named folders of original clips only. Read-only browser; edit jumps to Library.
- **Encoder** — Mostly automatic 320p queue. Manual jobs go to a fixed `custom_encode/` path.
- **Zmag manager** — 4-drive RAIDZ1 cold storage. Role (primary/backup) chosen at creation. Whole-mag rsync copies. Eject, snapshots, local pool health.
- **Profile Editor** — Admin-only. Camera (`.cam`) and encoder profiles.
- **Key module** — Admin-only. Generate or accept client SSH keys. Private key can be written to a file or USB and is not kept on the server.
- **Logs** — File-based, daily files, 30-day retention by default.
- **Settings** — Admin on server (path roots). Client sees only client-relevant paths.
- **Dashboard** — Landing screen. Server shows full status; client is thin and includes Library.

## Roadmap

**Phase 0 – Foundation**  
Config, schema, manual-path verified ingest (hash at ingest), mag card + ASC MHL, conflict handling, basic shell.

**Phase 1 – Client ingest & field backups**  
Card discovery, full client ingest, USB Prepare (ext4), reconnect-and-sync.

**Phase 2 – Proxies, encoder, dashboards**  
320p queue, encoder module, server and client dashboards, telemetry.

**Phase 3 – Logging & Library**  
Logging module (with mag-card write-back), Library (server + client), export card, cold-pull requests.

**Phase 4 – Zmag manager**  
Create (with role), two-pane copy, auto-backup, eject, placements, local pool health.

**Phase 5 – Export bins, Profiles, Keys, Logs, Settings**  
Export bins browser, Profile Editor, Key module, Logs viewer, Settings.

**Phase 6 – Polish & optional desktop**  
Remote zmag stations, split, HQ toggle, separate mouse-first desktop client, packaging.

Each phase leaves a usable terminal system. Full detail is in PLAN.md.

## Status

Phase 0 implemented: config directory, SQLite schema, manual-path ingest (`magvault ingest <card> --project <name>`) and magcard conflict handling (`magvault reconcile`). Build and usage: `docs/phase0.md`. All other modules have locked behaviour in PLAN.md and mockups in `docs/mockups/`.
