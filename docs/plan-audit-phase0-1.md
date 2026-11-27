# Magvault — plan audit, Phase 0 and Phase 1

Date: 2026-11-28. Audited: PLAN.md at `d98e59b` (full), README.md, docs/phase0.md,
docs/phase0-report.md, docs/phase1.md, docs/camera-identification-findings.md, and the code
on `main` at `d98e59b` (`src/`, `tests/`, `share/cameras/`). docs/test-report-camera-profiles.md
is not in the repository and was not audited. No code was changed; PLAN.md was not edited.

> **Update 2026-11-28 (must-fix batch):** §8 items 1–6 are implemented and PLAN.md
> corrected for C1, C3, C4, C6, C7, C8 and C5 (server dashboard is `h`; Clean is `c`).
> Also built: card summary (Block 2), System time, dump colours, staging bar (M2, M3, M4,
> M5, M6, M8 `x` / `l`, M18). Item 7 (real `sudo -S` on rafiki) is still a human test.
> See docs/phase1.md. The rest of this audit is left as written.

## 1. Verdict

**The plan needs fixes before the next round of human testing.** The Phase 0 and Phase 1
code is in good shape against the parts of the plan it implements. Phase 0 and Phase 1
e2e and unit tests pass. But PLAN.md contradicts itself in eight places that the code
has to pick a side on (§5). Three of those affect data safety or camera labelling:

- staging autoclean: "after the server confirms" vs "after two backups";
- conflict handling: "database wins for the manifest too" vs "manifest never
  auto-resolved";
- what `model:` in a `.cam` file means: marketing name vs the string the camera writes.

Several items PLAN.md now places in Phase 1 are not built (§4). The most important is
project requirements; the others are the staging bar, dump colours, F1, Left/Right and the
autoclean policy. The code also does a fair amount that the plan never mentions (§2.2).
Most of it is sensible, but it is undocumented in the plan.

Phase 0 is still closed (§3). Phase 1 changed Phase 0 ingest behaviour in ways the plan
asked for (no junk filter, camera identification); docs/phase0.md is now stale on junk.

## 2. Mismatches between PLAN.md and the code

### 2.1 Locked rules the code does not match

| # | PLAN.md says | Code does | Where |
|---|---|---|---|
| M1 | Client staging cache: "Auto-clean removes a staged mag as soon as at least two backups exist … If staging is the only copy, it is never auto-cleaned." | Staging is deleted as soon as the server commits, even with no field set. The server copy is then the only Magvault copy (zmags do not exist yet). Staging + set (`--mirror-set`) does give two copies. | `src/client_ingest.rs:218-221` (autoclean after `sync::send`); `src/sync.rs` (staging mags removed after commit) |
| M2 | Staging space bar (used / free) on the ingest screen; autoclean can be disabled. | Not built. Dashboard shows the pending count only; no setting to disable autoclean. | `src/tui/view.rs` (no bar); `src/config.rs` `ClientConfig` (no option) |
| M3 | Card dump status colours (red / yellow / green, from hashes / mag identity). | Not built. Cards show "ready" / "selected" / a problem. | `src/tui/view.rs` Detected pane; `src/discover.rs` `Card` |
| M4 | Project requirements: fixed lists (fps 24/25/30/50/60, resolution 4K/HD/SD, GOP intra/long-GOP), shown at ingest, warning "Camera footage does not meet project requirements.", mismatch logged to the project log and the mag. | Not built. No `requirements` column in `projects`, no CLI flags, no warning, no project log. | `src/db.rs:324` (projects table); `src/ingest.rs` (no check) |
| M5 | Shared keys: "Left / Right move between panes (same as Tab). Up / Down select the item." | Only Tab / Shift+Tab move panes; Left / Right do nothing. | `src/tui/app.rs:636` `ingest_key` |
| M6 | F1: help for the current screen ("in this batch"). | Not built. | `src/tui/app.rs` (no `KeyCode::F`) |
| M7 | Dashboard keys: `s` = Settings; `v` Logs, `e` Export bins, `z` Zmag, `p` Profile Editor. | `s` opens a "server dashboard" stub; `y` (sync) and `d` (details) are extra keys; `e z p v` are absent (modules not built). | `src/tui/app.rs:225` `MENU` |
| M8 | Ingest keys: `x` Abort (pulling state only), `l` Show log. | Abort is Ctrl-C in the progress popup; no `x`. No `l`: the log pane is always shown at ≥ 42 rows, hidden below. `c` Clean is used but not in the PLAN ingest table. | `src/tui/app.rs` `on_key`, `ingest_key`; `src/tui/view.rs` `ingest` |
| M9 | Copy options: "Extra mirror to one or more backup drives at any time." | Mirror only at ingest (`--mirror-set`), to one set; no later mirror. | `src/client_ingest.rs` `ClientArgs.mirror_set` |
| M10 | Camera profiles: "Clients never read the files directly. They request the current profile list from the server over SSH." Server profiles are "loaded into the database". | Client reads its own local copies (`<config>/cameras`, copied by `init --mode client`); the server re-reads files on each use, no DB load. Documented as temporary in docs/phase1.md. | `src/config.rs` `load_client` (`cameras`); `src/serve.rs:352` |
| M11 | Client dashboard: "simple queue depth number when connected"; "open the full server dashboard over SSH". | Queue depth shown as "not reported yet"; server dashboard is a stub. | `src/tui/view.rs` dashboard; `src/gate.rs` (no queue verb) |
| M12 | Logs retention: `log_retention_days` (default 30); "On startup and daily, files older than the retention period are deleted." | Setting is written by `init` but nothing reads it; no log is ever deleted. | `src/init.rs:55`; `src/log.rs` (no pruning) |
| M13 | Threat model: "No keys, passwords, or full paths that leave the system in logs." | Logs contain full paths (source card path, staging, rsync destinations in DEBUG, helper mount paths). Client issue lines and client DEBUG lines carry them to the server log. No secrets are logged. | `src/ingest.rs` `log_failure`, `rsync_to`; `src/client.rs` `queue_issue`; `src/fieldset.rs:708` |
| M14 | Security default: client keys served via `AuthorizedKeysCommand` from a Magvault-managed file, not a hand-edited `authorized_keys`. | Hand-written `authorized_keys` forced-command line (documented temporary). | docs/phase1.md "Setup on courage" |
| M15 | Process: `magvaultd` daemon. | `magvault serve` started by hand; no service unit. Documented as "first piece of magvaultd". | `src/serve.rs` |
| M16 | Ingest flow: automatic discovery "watch for newly mounted volumes". | Scans on demand (TUI refresh / `r`, CLI `cards`); no watching. | `src/discover.rs` `volumes` |
| M17 | Ingest flow: recognise ARRI and Fujifilm layouts. | No ARRI layout; Fujifilm only as generic DCIM. | `src/discover.rs:202` `MEDIA_TREES`, `layout_of` |
| M18 | Mag rules: "One camera only. No mixing sources, cameras, or formats inside a mag." | Not enforced. A card whose live media names two models gets the hint "… (more than one camera on this card …)" and no camera guess (so `--yes` stops). Interactive ingest and the TUI let the operator pick one camera for the whole card without a warning or extra confirmation. | `src/camera_meta.rs:119` `describe`; `src/ingest.rs` `choose_details` |
| M19 | Internal modules: each module in `src/<module>/` with a fixed shape (name, keys, draw, on_key, cli command); dashboard lists enabled modules. | No module shape: the TUI is one `App` with screens; the menu is a fixed array. | `src/tui/app.rs` |
| M20 | Mag card example: `shoot_date: 2026-11-18`, manifest `filename \| size \| sha256`. | Header has more keys (`format`, `mag_folder`, `project_status`, `camera_profile_id`, `source_label`, `ingested_by`, `updated_at`; optional `camera_model_hint`); manifest is `relative_path \| size \| sha256 \| mtime`; `# logging` section always present. PLAN calls its example a "shape", so this is drift, not a breach. | `src/magcard.rs:18,37` |
| M21 | Schema: `mags.dump_path`; `projects.requirements`; soft-lock fields. | `mags.folder` instead of `dump_path`; no `requirements`; no soft-lock columns (soft lock is a later feature). | `src/db.rs:324,335` |

### 2.2 Code behaviour PLAN.md does not mention

**Client ↔ server.**
- The heartbeat file `mags/.incoming/.results/serve.alive`, written every 2 s; 10 s
  freshness.
- The gate verb set: rsync upload into a per-client spool, `commit`, `status`,
  `projects`, `ping` (with a committer line), and `log` with lines on stdin. PLAN names
  only `log`.
- Results directory readable but not writable by clients.
- Per-client `.client-log` queue.
- `committer_wait` (client `magvault.conf`, default 120 s).
- Client `issues.pending` queue: same line dropped within 60 s, at most 200 lines.
  serve writes at most 30 operational and 300 DEBUG lines per client per minute.
- Client lines on the server carry the client's own time in brackets.

Code: `src/gate.rs`, `src/serve.rs:61,106,121`, `src/client.rs:45,132,154`.

**Projects.**
- Client cache `projects.cache` and `projects.local`.
- The server merges an offline client's new project into the one open project of the
  same name and rewrites the uploaded magcard.

Code: `src/client.rs:464`, `src/serve.rs:423`.

**Field sets.**
- Set label `A-Z0-9-` (≤ 12) and filesystem labels `<SET>_M` / `<SET>_S`, mounted at
  `/media/<user>/<label>` with `nosuid,nodev,noexec`.
- The slave is "limited" with an ext4 filesystem of the set size; no partition is
  created.
- A slave can only be added to an empty master.
- Sync and Clean need both drives mounted.
- The helper has two verbs, `usb-format` and `usb-clean`, plus
  `/etc/magvault/allow-loop-devices` for tests.

Code: `src/fieldset.rs:452,464,492`, `src/usb.rs:187,202`, `src/bin/magvault-helper.rs`.

**Camera identification.**
- The model hint comes only from metadata next to live media and is stored on the mag
  card as `camera_model_hint`; there is no DB column.
- `--model <name>`.
- `model:` matching: exact, the same body without its maker prefix, or a letter suffix.
- A specific profile whose `model:` names none of the hints is ruled out.
- `card_layout` is a requirement; nested entries must hold media.
- A profile with `filename_pattern` must match a media file.
- `vendor: Generic` profiles are only a fallback.
- `#` in a filename pattern means one digit.
- Layout labels join every tree that holds media.
- Sony `M01.XML` / `R01.BIM` sidecar pairing.
- AppleDouble `._X.mov` is copied but classed as "other", not a clip.
- `.mpg` is media.

Code: `src/camera_meta.rs`, `src/probe.rs:198,336,390`, `src/discover.rs:202,218`,
`src/profiles.rs` `glob_match`.

**Profiles shipped.**
- `build.rs` lists `share/cameras/*.cam` at build time.
- `init` copies them only into an empty folder; existing installs need a manual copy.

**TUI.**
- Plain wording by default, with paths, UUIDs and devices behind `d`.
- `y` sync.
- Progress popup with Ctrl-C cancel.
- The sudo password is asked in the TUI and passed once to `sudo -k -S`.
- `e` edits the set label in the Prepare popup.
- `magvault` with no command opens the TUI on a terminal with a client config;
  otherwise it prints the help (exit 2).
- `magvault ui --scan-root`.

Code: `src/tui/*`, `src/out.rs`, `src/main.rs:230,511`.

**Discovery.**
- `--scan-root` and `MAGVAULT_SCAN_ROOTS`.
- A device mounted twice counts as one card, preferring `/media`.
- `--yes` picks the card only when exactly one is found.

Code: `src/discover.rs`.

**Schema extras.**
- Tables `files`, `encoder_job_clips` (now filled), `copy_jobs`, `zmags`, `tags`,
  `people`, `clip_people`, `clip_field_times`, `meta`.
- `seq` columns.
- `mags.size_bytes`, `file_count`, `source_label`, `ingested_by`.
- Technical clip columns.
- `mags.status` has no `archived` (follows Implementation defaults).

Code: `src/db.rs:319-506`.

**Test transport.**
- `server_transport: local` runs the gate as a local process. It is used only by tests.

## 3. Phase 0 status

**Still closed.** docs/phase0-report.md (`5124209`) lists the delivered behaviour, and
`tests/e2e.sh` still passes on `main`. Phase 1 changes that altered Phase 0 ingest
behaviour:

| Change | Phase 0 before | Now | Plan basis |
|---|---|---|---|
| Junk filter | `.DS_Store`, `._*`, macOS volume folders skipped and listed | Every file copied; `._X.mov` copied as "other"; a card name with a control character is now refused (it used to be skipped as junk) | PLAN "Mag rules": no junk filter (`6405940`) |
| Camera guess | Layout + filename score; any tie = no guess | Model hint from live media first; `card_layout` required; generic fallback; contradicted profiles ruled out; no layout-vendor guess when the card names an unknown model | camera-identification findings |
| Sidecars / media | Same-stem sidecars only; `.mpg` was "other" | Sony `M01.XML` / `R01.BIM` paired; `.mpg` media | findings |
| Mag card | Fixed header keys | Optional extra key `camera_model_hint` (kept by reconcile) | findings ("optional model hint") |
| 320p job | `encoder_job_clips` empty | Filled, one row per clip | Phase 1 fix |
| `init` | 1 camera profile | Every `share/cameras/*.cam` (10 today); `debug: no` in `magvault.conf` | PLAN "Profile updates", "Debug logging" |
| Server `ingest` | Path required | Path optional (single discovered card with `--yes`) | PLAN "CLI and scripting" |
| Help | One usage dump | Grouped `--help`, per-command help | PLAN "Help" |

**Stale Phase 0 docs.** docs/phase0.md lines 38, 62 and 98 still describe junk skipping.
docs/phase0-report.md is a historical record and can stay as it is.

## 4. Phase 1: implemented vs locked-but-not-built

| Item | State |
|---|---|
| Card discovery (manual path, scan of `/media`, `/run/media`, `/mnt`, camera layouts, operator pick) | **Built.** No volume watching (M16). |
| Field backup sets: Prepare (ext4, smaller first, slave refused / limited, blocked while pending), Clean (only when all synced, keeps the set) | **Built** (CLI + TUI popups). Real USB tested on rafiki (TESTSET1). |
| Prepare refusals (ZFS / LVM / RAID / LUKS message; `TRAN=usb` accepted; camera cards never targets) | **Built.** |
| Copy options: staging, set as staging, direct, mirror; no-backup warning | **Built.** Mirror only at ingest, one set (M9). |
| Reconnect-and-sync, verify against stored SHA-256, synced only after server confirms | **Built.** |
| Client → server via gate (rrsync spool, serve commits) | **Built.** Temporary: hand-written keys (M14), manual serve (M15), local camera profiles (M10). |
| Client ↔ server status (ready / committer down / offline), bounded waits | **Built.** |
| Client issues on the server log; debug logging | **Built.** Full paths in lines (M13). |
| TUI: client dashboard, Ingest, Prepare / Clean popups, in-TUI progress and sudo prompt, plain wording | **Built.** Real `sudo -S` not yet run by a human. |
| Camera identification (model hint, per-body profiles, nested layouts, no junk filter) | **Built.** Model readers for MPEG `MVI_DATA` and Panasonic MOV XML not yet run on real bytes. |
| **Project requirements** (fixed lists, ingest warning, project log) | **Not built** (M4). PLAN keeps it in Phase 1. |
| **Staging space bar** | **Not built** (M2). |
| **Auto-clean after two backups; disable option** | **Not built** (M1, M2). The code autocleans after one server copy. |
| **Card dump colours** | **Not built** (M3). |
| **F1 help** | **Not built** (M6). |
| **Left / Right panes, Up / Down items** | Up / Down built; **Left / Right not built** (M5). |
| Job card, producer key | **Deferred** (agreed) to the producer console. |
| F2 log key | **Deferred** (agreed) until the Logs module. |

## 5. Contradictions inside PLAN.md

| # | Topic | Conflict | Lines |
|---|---|---|---|
| C1 | Copy rules: autoclean | "Default: card → local staging → rsync to server → server confirms → autoclean staging" vs "Auto-clean removes a staged mag as soon as at least two backups exist". The code follows the first. | 613 vs 588 |
| C2 | Copy rules: conflict handling | "Database is authoritative … Applies to all fields: … manifest" vs threat model "Manifest fixed at ingest. Mismatches reported, never auto-resolved". The code follows the threat model (reconcile reports manifest differences as INTEGRITY). | 548-554 vs 1158 |
| C3 | Hashes in ASC MHL | "C4 only. ASC MHL does not support SHA-256" vs Mag card format "ASC MHL … holds … SHA-256/MD5". The code writes C4 only (`src/mhl.rs`). The second sentence is a factual error. | 8 vs 544 |
| C4 | Keys: Esc / q | "Esc/q returns to the dashboard from any module" vs shared key table "`q` Quit … Only on the dashboard". The code follows the table. | 748 vs 759 |
| C5 | Keys: dashboard `s` and server dashboard | `s` = Settings on the dashboard; the client dashboard must "open the full server dashboard over SSH" but no key is assigned. The code took `s`. Ingest's Clean key is not assigned either (the code uses `c`). | 770, 744, 671-681 |
| C6 | Roles: sudo | "All privileged operations go through one small root-owned helper" vs "`magvault ALL=(root) /usr/sbin/zpool, /usr/sbin/zfs`" (direct sudo on zpool / zfs). Implementation defaults say the helper is the only privileged path. | 453-456, 1206 vs 505-511 |
| C7 | Roles: client user | User model: `magvaultclient` is the shared SSH account *on the server* for all clients. Sudo section: "On a client machine the `magvaultclient` user may be given limited … sudo". The implementation (and docs/phase1.md) runs the client as the operator's own account, with sudo for the helper only. | 492 vs 515 |
| C8 | Profiles: what `model:` means | The `.cam` example uses a marketing name (`model: A7S III`). The camera-identification work matches `model:` against the string the camera writes (`ILCE-7SM3`, `ILME-FX6V`, …), and the shipped per-body profiles use firmware strings. PLAN does not say which. | 314 vs docs/camera-identification-findings.md |
| C9 | Schema: camera | `camera_profile_id (from .cam, or free text if no profile)` vs "Camera is always a profile id (or empty). No free-text camera model in the database." | 130 vs 1198 |
| C10 | Schema: status | `mags.status (ingesting, online, cold, archived, …)` vs "`archived` status is not used". | 134 vs 1200 |
| C11 | Roles: field master / slave | Locked: exactly one master and one slave, same mags. Implementation defaults: "Or drop the distinction if it adds no value." | 623-633 vs 1214 |
| C12 | Logs: levels | Line format lists INFO / WARN / ERROR; Debug logging adds DEBUG. | 414 vs 429 |

**Editorial clutter, not contradictions.**
- "Hashing", "Incoming staging" and "Copy method" sit above the document title
  (lines 1-30).
- "Becoming cold" appears twice (805, 961).
- "## Zmag manager" appears twice (813 is a key table under a bare `#`; 969 is the
  section). The Zmag key table omits `n` (create) and `x` (eject), which the prose uses.
- F1 is listed twice (604, 606).
- A second "Prepare" section sits at the end (1288-1296).
- "Job card (client)" sits under "Soft lock" (238).

## 6. Risks if left as-is

**Data loss**
1. **One copy after autoclean (M1 / C1).** With the default route and no field set, staging
   is deleted when the server commits. Until zmags exist (Phase 4), the server's `mags/`
   dataset is then the only copy apart from the camera card. Field crews reformat cards.
   Clean of a field set has the same exposure: it is allowed once the server has the mags.
2. **Profile mismatch after a full upload (M10).** The server checks camera values
   against *its* profiles only after the whole upload has arrived (`src/serve.rs:352`).
   It deletes the spool on refusal. The local copy stays pending, so nothing is lost. But
   every retry repeats the full transfer until the profiles agree. rafiki's client config
   still has only `sony-a7siii`, because `init` copies profiles only into an empty folder.

**Mislabelled camera**
3. **`sony-a7siii` has `model: A7S III` (C8).** It can never match the string an A7S III
   writes. Under the "contradicted profile" rule, any A7S III card with clip metadata
   rules this profile out. The result is no guess, which blocks `--yes`; in interactive
   use the operator may pick a wrong profile.
4. **Mixed-camera cards (M18).** A reused card holding two cameras' clips is ingested as
   one mag with one camera if the operator picks one. The hint mentions both models, but
   nothing asks for a deliberate decision.
5. **Layout-only guesses without metadata.** A card with no live-media model string is
   still guessed from folders and file names (e.g. an XAVC S card without sidecars).
   `--yes` accepts such a guess. This is allowed by "CLI and scripting", but it is weaker
   evidence than the findings recommend.
6. **Unverified metadata readers.** MPEG `MVI_DATA` (VG20) and Panasonic XML inside MOV
   were tested only on synthetic bytes. Only the first and last MiB of up to 8 clips are
   read.

**Blocks field use**
7. **Committer is manual (M15).** If nobody starts `magvault serve`, every client
   mag stays pending. This is now clearly shown ("server up, committer down") but not
   prevented.
8. **Version skew.** A new client treats an old gate (no committer line) as "committer
   down" and times out on every sync. The server must be updated first.
9. **Sync and Clean need both drives.** A crew with only the master on hand cannot sync
   from it.
10. **Real `sudo -S` in the TUI is untested** (tests run as root). Scripts need a sudo rule
    that does not prompt, or an askpass helper.
11. **Missing keys (M5, M6, C5).** Operators expect Left / Right and F1 from the plan. `s`
    will collide with Settings when that module lands.

## 7. Deferred items (agreed)

- **Producer console:** Job card page; producer key (`producer: yes`, project creation from
  the client).
- **Keys:** F2 log key, until the Logs module is real.
- **Phase 2:** proxy encoding and the 320p queue worker; Encoder module; server dashboard;
  telemetry file; client queue depth (needs a gate verb).
- **Phase 3:** Logging module and magcard `# logging` writes; Library (client proxy cache);
  export cards; cold-pull generation.
- **Phase 4:** Zmag manager, placements, cold eviction, pool health.
- **Phase 5:** Export bins browser, Profile Editor, Key module (`AuthorizedKeysCommand`),
  Logs viewer, Settings.
- **Phase 6:** remote zmag stations, split, HQ toggle, desktop client, packaging (`.deb`
  with profiles; profile update command).
- **Documented temporary Phase 1 items** (docs/phase1.md "Temporary in Phase 1"):
  - hand-written forced-command keys;
  - `serve` started by hand;
  - client-local camera profiles;
  - `magvaultclient` ownership of committed files;
  - the `local` test transport.

## 8. Must-fix before more human testing

1. **Decide C1 and make the code match.**
   - **PLAN "two backups" rule:** keep staging after the server commit unless a field set
     also holds the mag; add the disable option; keep "never clean the only copy".
   - **Or keep "autoclean after server confirms":** delete line 588's rule.
   - **Either way, Clean:** state in PLAN whether Clean of a set may proceed when the
     server holds the only other copy.
   - **Code:** `src/client_ingest.rs:218`, `src/sync.rs`.
2. **Decide C8 and fix `sony-a7siii.cam`.** State in PLAN that `model:` is the string the
   camera writes, comma list allowed. Then give the A7S III profile its firmware string
   (expected `ILCE-7SM3`; confirm from a real card) alongside or instead of `A7S III`.
   Check the other shipped profiles the same way.
3. **Camera profiles on clients.**
   - Copy the current `share/cameras/*.cam` set to rafiki's client config.
   - Before copying a card, check that the chosen profile exists on the server (or fetch
     the server's list through the gate), so a mismatch is caught before the upload, not
     after it.
4. **Mixed-camera warning (M18).** When the live media names more than one model, show an
   explicit warning and require a deliberate confirmation (TUI and interactive CLI). `--yes`
   should refuse. This enforces "one camera only" without blocking a deliberate choice.
5. **Keys from the plan (M5, M6, C5).** Add Left / Right = previous / next pane, and F1
   help per screen. Assign a PLAN key for "server dashboard over SSH" and move it off `s`.
   Also assign the Clean key in PLAN (the code uses `c`).
6. **Project requirements (M4)** are locked in Phase 1:
   - `projects.requirements` (fixed lists);
   - CLI flags at project creation;
   - the ingest comparison against probed fps / resolution / GOP;
   - the warning;
   - the mismatch logged to the project log and the mag. Define "project log" first:
     PLAN logs are daily files, not per project.
7. **Run the TUI sudo path for real** on rafiki: wrong password, Esc, and a real Clean of a
   test mag on TESTSET1. Then remove "untested" from docs/phase1.md.

## 9. Nice-to-fix

- **Log retention (M12):** delete daily logs older than `log_retention_days` at startup
  and daily.
- **Paths in logs (M13):** keep full paths out of lines that leave the machine (client
  issues, forwarded DEBUG), or narrow the threat-model sentence to what is intended.
- **systemd unit for `magvault serve`** (and document it) until `magvaultd` exists.
- **Later mirror (M9):** mirror an existing mag to a set later, as PLAN's "at any time"
  says.
- **Staging space bar and card dump colours (M2, M3):** Phase 1 locked UI, not blocking
  tests.
- **Volume watching (M16):** add it or say in PLAN that rescan is enough. Add ARRI
  layouts (M17).
- **Model hint in the DB:** `camera_model_hint` lives only on the mag card. Add a column
  with the next schema change so the library can show it.
- **Clean up PLAN.md:** fix the MHL hash sentence (C3, line 544); settle C2, C4, C6, C7,
  C9, C10, C11, C12; remove the duplicated sections.
- **Clean up docs:** update docs/phase0.md lines 38, 62 and 98 (no junk filter).
- **Module shape (M19):** introduce the PLAN module shape before Phase 2 adds more
  screens, so the dashboard menu comes from enabled modules.
