# Magvault Phase 0 — Report

**Status: CLOSED**  
**Date:** 2026-11-26  
**Final commit:** `5124209` (Phase 0 must-fixes from the courage test review)  
**Hardware:** courage (HP DL385p Gen8, Ubuntu 26.04.1, file-backed ZFS test pool)

Phase 0 delivered a working manual-path ingest: config, schema, hash-at-ingest, verified rsync copy, magcard + ASC MHL, database records, conflict handling, and the security-relevant fixes found in review.

## What Phase 0 does

- Load a config directory and `path.conf`.
- Create the SQLite schema.
- Ingest a directory (manual path):
  - Probe, operator confirm (or `--yes`)
  - SHA-256 (and C4) computed on first read
  - rsync into `incoming/` on the same filesystem as `mags/`
  - Verify copy against stored SHA-256
  - Write `magcard.txt` and ASC MHL
  - Rename into `mags/<YYYYMMDD>_<uuid>/` and commit DB in one transaction
  - Queue a 320p encoder job record (no encode yet)
- Reconcile: database wins for metadata (magcard backed up); manifest mismatches and orphan folders are integrity errors and are not auto-resolved.
- Refuse bad inputs: cross-filesystem incoming, `/tmp`, source overlapping Magvault roots, existing magcard, control characters in names, etc.

## Test history

### First hardware run (commit `3a0caf9`)
- Clean release build, 22/22 unit tests
- `tests/e2e.sh` passed with real rsync
- Real ingest, reconcile, and failure paths passed
- Report: `docs/test-report-phase0.md` (on courage; findings led to the must-fix list)

### Review findings (must-fix before close)
1. `init` defaulted `incoming` outside the mags dataset
2. Interrupted ingest leftovers not logged; concurrent ingest could false-warn
3. Ctrl+C / SIGTERM did not clean incoming
4. Orphan folders in `mags/` not detected
5. Control characters could reach the terminal and logs
6. Some post-confirm failures not logged as ERROR
7. ASC MHL version wording (spec 1.0 vs hashlist v2.0)

### Fixes (commit `5124209`)
All seven implemented. No schema change. 23 unit tests. e2e extended with interrupt, orphan, escape, and concurrency checks.

### Retest on courage (commit `5124209`)
- Clean build, 23/23 unit tests
- `tests/e2e.sh` with real rsync: ALL PASSED (including new checks)
- Manual checks all passed:
  - Ctrl+C mid-copy → `.incoming` empty, ERROR logged
  - `kill -9` mid-copy → next ingest WARN (screen + log) naming folder and `.files`
  - Three concurrent ingests → no false warning, all committed
  - Orphan folder → INTEGRITY, exit 1, not adopted
  - Escape-sequence filename → escaped as `\u{1b}` on screen and in log
- Report: `docs/test-report-phase0-retest.md`

## Code audit (post-retest)

Phase 0 code is fit to close.

Solid:
- Ordered ingest with verify-before-commit
- Incoming rules enforced (same FS, not `/tmp`, default `<mags>/.incoming`)
- Parameterized SQL
- Rename-then-commit with rename-back on failure
- Interrupt lock + cleanup + stale WARN
- Control-character escaping
- Reconcile reports integrity problems and does not auto-fix

Non-blocking (Phase 1 or later):
- rsync progress still on stdout in non-tty runs
- No ffprobe timeout yet
- No duplicate-card detection (needed for soft lock later)
- `db/` not yet 0700 (required once clients exist)
- Testing was on a file-backed pool (functional only)

## Phase gate

| Gate | Result |
|------|--------|
| Functional tests on real hardware | Pass |
| Security review against threat model | Pass for Phase 0 scope |
| Must-fix items from review | Fixed and retested |
| Code audit | Accept |

**Phase 0 is closed.** Phase 1 may start.

## Environment notes (courage)

- Ubuntu 26.04.1, Rust 1.93.1 (apt), rsync 3.4.1, ffmpeg 8.0.1, ZFS 2.4.1
- File-backed 80 GB pool `magvault` at `/magvault` (test bench; not production disks)
- User `magvault`, incoming at `/magvault/mags/.incoming`
- Client (Gen8 i5) not used in Phase 0

## Documents

- `PLAN.md` — behaviour, threat model, implementation defaults
- `docs/phase0.md` — Phase 0 implementation notes
- `docs/test-checklist.md` — hardware and test checklist
- `docs/test-report-phase0.md` — first hardware run (on courage)
- `docs/test-report-phase0-retest.md` — retest after fixes (on courage)
- `docs/technical-review-and-spec.md` — full technical specification
