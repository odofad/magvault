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

1. Walks the source. Skips junk (`.DS_Store`, AppleDouble `._*`, macOS volume folders).
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
6. Checks free space, then copies with
   `rsync -t --files-from=<list> --from0 --no-links --no-devices --no-specials` into
   `incoming/<YYYYMMDD>_<uuid>/`.
7. Verifies every copied file against the stored SHA-256 and that no file is missing or
   extra. Any mismatch: integrity error, the copy is deleted, nothing is committed.
8. Writes `magcard.txt` and `ascmhl/`.
9. In one SQLite transaction inserts the project (if new), mag (`online`), clips, files
   and a `queued` 320p encoder job; renames the folder into `mags/`; commits. If the
   commit fails the folder is moved back out.

Every result and failure is written to `logs/YYYY-MM-DD.log` (UTC day).

## Decisions taken while implementing

These follow PLAN.md and the tech review; where the documents left a gap the smallest
consistent choice was made. Each is easy to change.

- **ASC MHL hash format is C4, not SHA-256.** PLAN.md says the SHA-256 hashes go into the
  ASC MHL, but ASC MHL 1.0 only allows md5, sha1, c4, xxh64, xxh3 and xxh128, and its
  chain file must use C4. Magvault therefore computes SHA-512 alongside SHA-256 in the
  same read and writes C4 IDs into the MHL. SHA-256 stays the integrity hash everywhere
  else (database, magcard, verification). Needs an owner decision; the review document's
  claim that ASC MHL supports SHA-256 was wrong.
- **`incoming` root.** New optional `path.conf` key (tech spec §8.1). Default: a sibling
  `incoming/` next to `mags/`. Must be on the same filesystem as `mags` (checked) and is
  refused under `/tmp`, `/var/tmp`, `/dev/shm`.
- **rsync with `--files-from`.** Copies exactly the probed file list instead of the whole
  card directory, so junk is excluded by construction and the copy matches the hashed set.
- **Magcard backups.** Named `.magvault/magcard.YYYYMMDD-HHMMSS.backup` (UTC, PLAN.md
  "Conflict handling"); only the newest is kept (PLAN.md "Logging on the mag card":
  "single"). A `magcard.lock` file in `.magvault/` serialises rewrites.
- **Conflicts.** `reconcile` compares header fields and the `# logging` section with the
  database. If they differ the card is backed up and rewritten (unknown header keys are
  kept). Manifest differences, missing/extra/changed files are integrity errors: reported,
  logged, exit code 1, nothing rewritten. `--verify-hashes` re-hashes files.
- **Camera in the database** is the profile id only; `camera_model` on the card is the
  profile's `name` at write time.
- **Project by name.** `--project` reuses the single open project with that name, creates
  a new one if no project has that name, and refuses if the name only matches a closed
  project or matches several open ones (`--project-id` then).
- **Interrupted ingest** (Ctrl+C, crash) leaves a folder in `incoming/` that is neither in
  `mags/` nor in the database; the next ingest warns about it. Cleanup is manual in
  Phase 0.

## Not tested here

The development machine had no `rsync`; the end-to-end test ran with
`tests/fake-bin/rsync`, a test double that implements only the options above. Run
`tests/e2e.sh` on Ubuntu with rsync installed (it uses the real one automatically) before
relying on it.
