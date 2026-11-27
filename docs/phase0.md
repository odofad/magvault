# Phase 0 — implementation notes

Phase 0 from PLAN.md's roadmap: config directory + `path.conf` + pointer file, SQLite
schema, manual-path ingest (probe, confirm, hash at ingest, verified copy, `magcard.txt` +
ASC MHL, database records, queued 320p job), and conflict handling (database wins for
metadata, magcard backed up). Command line only.

## Build

```sh
sudo apt install build-essential libsqlite3-dev rsync ffmpeg   # ffmpeg provides ffprobe
cargo build --release          # binary: target/release/magvault
cargo test                      # unit tests
tests/e2e.sh                    # end-to-end: init → ingest → checks → conflicts → failures
```

No crates are used: SQLite is called through its C API (`libsqlite3`), SHA-256 and
SHA-512 are implemented in `src/hash.rs` and checked against `sha256sum`/`sha512sum`.

## Use

```sh
magvault --config /srv/magvault/config init --root /srv/magvault --write-pointer
magvault ingest /media/odofad/A7SIII_128 --project ACME_Campaign_2025
magvault reconcile 20250520_3f6c2a1e-9b0d-4c55-8e21-a3f9c2d41b77
magvault reconcile --all --verify-hashes
```

`ingest` prints what it found and guessed and asks before copying. Options: `--camera`,
`--format`, `--gamma`, `--gamut` (only values listed in the camera profile),
`--shoot-date YYYY-MM-DD`, `--notes`, `--project-id`, `--yes`.

The config directory comes from `--config`, else `MAGVAULT_CONFIG`, else
`~/.config/magvault/config-dir`, else `/etc/magvault/config-dir`.

## What ingest does

1. Walks the source. Every file is copied: there is no junk filter (PLAN.md "Mag rules";
   AppleDouble `._*` files and macOS folders are copied as "other" files, never as clips).
   Refuses symlinks, devices, non-UTF-8 names and names containing `|` or control
   characters. Refuses a source that already holds `magcard.txt`/`ascmhl/` (importing mag
   folders from field drives is Phase 1) and a source inside any Magvault root.
2. Classifies files: media by extension (one media file = one clip); sidecars share folder
   and stem with a media file; everything else is `other`. All are copied and listed.
3. Runs `ffprobe` on media (duration, frame rate, size, audio streams/channels,
   creation time). Failures are warnings.
4. Guesses the camera from the profiles' `card_layout` and `filename_pattern`, and the
   shoot day from the earliest media creation time (else earliest modification time), as
   a local calendar date. Format/gamma/gamut default to the profile's defaults.
5. After confirmation, reads every file once to compute SHA-256 (and C4 for the MHL).
6. Checks free space, creates and locks `<incoming>/.<folder>.lock`, then copies with
   `rsync -t --files-from=<list> --from0 --no-links --no-devices --no-specials` into
   `<mags>/.incoming/<YYYYMMDD>_<uuid>/` (the list is `<incoming>/.<folder>.files`).
7. Verifies every copied file against the stored SHA-256 and that no file is missing or
   extra. Any mismatch: integrity error, the copy is deleted, nothing is committed.
8. Writes `magcard.txt` and `ascmhl/`.
9. In one SQLite transaction inserts the project (if new), mag (`online`), clips, files
   and a `queued` 320p encoder job; renames the folder into `mags/`; commits. If the
   commit fails the folder is moved back out.

Every result and every failure after the confirmation (hashing read errors, a file that
changes size while being read, not enough space, copy and verify errors, interrupts) is
written to `logs/YYYY-MM-DD.log` (UTC day).

Control characters in anything Magvault prints or logs (card file names, volume labels,
ffprobe messages) are escaped (`\u{1b}`, `\n`), so a crafted file name cannot inject
log lines or terminal escape sequences (PLAN.md Threat model, "Log injection").

### Interrupts and leftovers

- **Ctrl+C or SIGTERM** after the confirmation stops the ingest: hashing and verifying stop
  at the next read, rsync is stopped (Ctrl+C reaches it directly; on SIGTERM Magvault sends
  it SIGTERM), the staging folder and its `.files` and `.lock` are removed, and ERROR
  `interrupted by operator (SIGINT|SIGTERM)` is logged. Before the confirmation, Ctrl+C
  exits at once (nothing has been written). A signal that arrives during the rename into
  `mags/` and the database commit is held off until the commit is done.
- **SIGKILL, a crash or power loss** cannot be caught and leaves the staging folder with its
  `.files` and `.lock` in `incoming/`. The next ingest prints a warning and logs WARN naming
  each such folder and its files. A folder whose `.lock` is still held by a running ingest
  is not reported. Cleanup is manual: remove the folder and its two dot files.

## Decisions taken while implementing

These follow PLAN.md and the tech review; where the documents left a gap the smallest
consistent choice was made. Each is easy to change.

- **ASC MHL: C4 only.** ASC MHL (specification 1.0; files declare hashlist version 2.0,
  `urn:ASC:MHL:v2.0`); C4 only. The specification allows md5, sha1, c4, xxh64, xxh3 and
  xxh128, not SHA-256, and its chain file must use C4. Magvault computes SHA-512 alongside
  SHA-256 in the same read and writes C4 IDs into the MHL. SHA-256 stays the integrity hash
  everywhere else (database, magcard, verification), as PLAN.md "Hashing" now states.
- **`incoming` root.** `path.conf` key `incoming`; `magvault init` writes
  `<data root>/mags/.incoming`, and a missing key defaults to `<mags>/.incoming` (PLAN.md
  "Incoming staging"). It must be on the same filesystem as `mags` (checked; a separate
  staging dataset is refused) and is refused under `/tmp`, `/var/tmp`, `/dev/shm`. The
  hidden `.incoming` folder is not a mag and is skipped by `reconcile --all`.
- **rsync with `--files-from`.** Copies exactly the probed file list instead of the whole
  card directory, so the copy matches the hashed set.
- **Magcard backups.** Named `.magvault/magcard.YYYYMMDD-HHMMSS.backup` (UTC, PLAN.md
  "Conflict handling"); only the newest is kept (PLAN.md "Logging on the mag card":
  "single"). A `magcard.lock` file in `.magvault/` serialises rewrites.
- **Conflicts.** `reconcile` compares header fields and the `# logging` section with the
  database. If they differ the card is backed up and rewritten (unknown header keys are
  kept). Manifest differences, missing/extra/changed files are integrity errors: reported,
  logged, exit code 1, nothing rewritten. `--verify-hashes` re-hashes files.
  `reconcile --all` also lists the folders in `mags/` (hidden ones such as `.incoming`
  skipped): a folder with no database record is reported as INTEGRITY (exit 1) and logged
  as ERROR. It is never adopted, moved or deleted.
- **Camera in the database** is the profile id only; `camera_model` on the card is the
  profile's `name` at write time.
- **Project by name.** `--project` reuses the single open project with that name, creates
  a new one if no project has that name, and refuses if the name only matches a closed
  project or matches several open ones (`--project-id` then).
- **Interrupted ingest**: see "Interrupts and leftovers" above. Cleanup of SIGKILL
  leftovers is manual in Phase 0.

## Tests

`tests/e2e.sh` uses the real `rsync` when it is installed and `tests/fake-bin/rsync` (a
test double for the options above) otherwise. It builds the release binary. The interrupt
tests copy one large random file so the signal lands mid-copy;
`MAGVAULT_E2E_BIG_MB` (default 1536) sets its size. They need `setsid` and `pgrep`.
