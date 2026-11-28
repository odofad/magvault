# Magvault — Technical Review and Implementation Specification

| | |
|---|---|
| Repository | `odofad/magvault`, branch `main`, PLAN.md at `8749c01` ("Phase 2: GPU profile block, qp rate control, CRF 23") |
| Read | PLAN.md (1420 lines, every section), README.md, NOTES.md, docs/phase0.md, docs/phase0-report.md, docs/phase1.md, docs/plan-audit-phase0-1.md, docs/phase2-readiness.md, docs/phase2-review.md, docs/mockups/*.md (spot checks), and the code on `main` (`src/`, `tests/`, `share/`) |
| Date | 2026-11-28 |
| State of the project | Phase 0 closed (2026-11-26). Phase 1 implemented; clean full hardware run passed at `f8c1b32`; Phase 1 gate still open. Phase 2 planned; plan review passed for the seven locked rules. |
| Replaces | The pre-code review of 2026-11-22 (PLAN at `ca0cf18`). Most of its findings were settled by owner decisions or by the Phase 0/1 code. The ones still open are carried forward here under new IDs. |

**How to read this document**

- **Part A — Plan audit.** Every phase is read against the rest of PLAN.md and the
  mockups.
- **Part B — Technical specification.** The architecture as built, and the target for
  Phases 2–6.
- **Part C — Code vs plan.** What the code does, where it differs from PLAN.md, and the
  changes needed.
- **Part D — Priorities and open questions.**

**Finding IDs.** Each finding has an ID by kind:

| Kind | Meaning |
|---|---|
| `C` | Contradiction (PLAN vs itself, or PLAN vs mockups) |
| `M` | Missing behaviour |
| `S` | Security |
| `D` | Data integrity |
| `T` | Hard to implement or test |
| `E` | Editorial (structure, duplicates, stale text) |

**Blocks** names the earliest point the finding must be settled:
- "P2 start": before Phase 2 code;
- "in P2": while Phase 2 is built;
- "P3", "P4" and so on: before that phase;
- "—": it can wait.

`PLAN:591` means PLAN.md line 591 at `8749c01`.

---

## Summary

1. **The plan is in good shape for Phase 2.**
   - The seven locked Phase 2 rules are consistent:
     - temp projects with FPS required;
     - no encode until assignment;
     - no auto-merge;
     - real projects only in the Project manager;
     - Key manager TUI;
     - AMD VAAPI with CPU fallback;
     - the field does not manage archive copies.
   - The producer model is sound: a separate `magvault-producer` login, a read-only
     database, and every write through `serve`.
2. **Older sections were not updated for Phase 2.** Five places still describe Phase 1
   behaviour or cannot work under the Phase 2 model:
   - Ingest flow step 4 still creates real projects (C1).
   - The client dashboard still "opens the full server dashboard over SSH" through a
     gate-locked key (C15).
   - The Key module writes `authorized_keys` directly, against the AuthorizedKeysCommand
     default (S2).
   - Proxies use original basenames, which collide inside one mag (D1).
   - The schema section is two phases out of date (C7).
3. **Phases 3–6 have six design gaps.** They are not urgent, but each must be settled
   before its phase:
   - clients have no read path for proxies and exports; the gate is upload-only (S3);
   - auto-import of zmags conflicts with password-every-time sudo (C23);
   - evict-to-cold rules do not match the primary/backup roles (C22);
   - the export card stores absolute paths (C21);
   - soft lock has no phase (M12);
   - the remote zmag station would write the database outside the single write path (M20).
4. **The code matches the Phase 0/1 plan well.** About 16 000 lines; 75 unit tests, the
   Phase 0 e2e, and the Phase 1 e2e with 76 checks including a pseudo-terminal TUI bot.
   Phase 2 rules change existing behaviour in six places (Part C §11):
   - project creation;
   - the name merge;
   - the 320p job at commit;
   - green = on the server;
   - the `qp` rate control;
   - schema 3.
5. **The real gate is Phase 1, not the plan.** Three things are outstanding:
   - a hardware retest of `2d8f940`;
   - the real TUI sudo prompt;
   - a written Phase 1 security review.

   Phase 2 code should start after those, with the groundwork in Part D.

---

# Part A — Plan audit

## 1. Phase map

| Phase | Scope (PLAN roadmap) | State |
|---|---|---|
| 0 Foundation | config, schema, manual-path ingest, magcard + ASC MHL, conflict handling | **Closed** 2026-11-26 (`docs/phase0-report.md`) |
| 1 Client ingest & field backups | card discovery, client ingest routes, Prepare ext4, sync, autoclean after two backups | **Implemented.** Clean full run passed at `f8c1b32`. Gate open: retest `2d8f940`, TUI sudo, security review. |
| 2 Proxies, encoder, dashboards, projects, keys | 320p worker (VAAPI + CPU), Encoder module, dashboards, telemetry, Project manager, Job card, Key manager TUI, producer key, temp projects, assignment pull-back | Planned; plan review passed (`docs/phase2-review.md`) |
| 3 Logging & Library | Logging module + magcard write-back, Library (server + client proxy cache), export card, cold-pull requests | Planned; design gaps S3, M11, C21, M12, T2 |
| 4 Zmag manager | create (role), two-pane copy, auto-backup, eject, evict to cold, placements, pool health | Planned; design gaps C22, C23, C24, M15, M16 |
| 5 Remaining modules | Export bins browser, Profile Editor, Logs viewer, Settings (Key module and Sudo helper are listed but already done or moved) | Planned; C25 |
| 6 Polish & desktop | remote zmag stations, split, HQ toggle, desktop client, packaging | Planned; M20 |

## 2. Cross-cutting and editorial

| ID | Where | Finding | Fix | Blocks |
|---|---|---|---|---|
| E1 | PLAN:3–30 | "Hashing", "Incoming staging" and "Copy method" sit above the document title. | Move them under the title, next to "Mag rules". | — |
| E2 | PLAN:850, 851/1007, 859/1015 | A stray `#` line, "Becoming cold" twice, and "## Zmag manager" twice. The first Zmag manager heading is a key table without `n` (create, PLAN:1029) or `x` (eject, PLAN:1050). | Remove the duplicates; put the key table in the Zmag section with `n` and `x`. | — |
| E3 | PLAN:1283–1353 | Assignment pull-back, Temp project names, Offline temp project, Project manager, Encoder backend, Phase 2 schema and Phase 2 decisions all sit between the Phase 1 and Phase 2 headings, so they read as Phase 1. | Move them under "Phase 2". | — |
| E4 | PLAN:1412–1420 | A second "Prepare" section at the very end. | Merge it into "Prepare popup" (PLAN:695). | — |
| E5 | PLAN:1103 | The placement-tracking paragraph sits under "Key module". | Move it to the Zmag manager. | — |
| E6 | PLAN:1401–1410 | Open points are partly stale. The offline logging rule exists (PLAN:1257), and "how the server notices a new mag" is now "at assignment" (PLAN:918, 1348). | Keep only: telemetry schema, on-zmag manifest, retry semantics, split UI. | — |
| E7 | README Roadmap, README tick-tock | README Phase 2 lists only "320p queue, encoder module, dashboards, telemetry", without projects, keys or producer, and Phase 5 still has the "Key module". The tick-tock version track (1.0 / 1.5 / 2.0) is in README only. PLAN does not say which phase completes 1.0. | Update the README phase lines. Add the version track to PLAN with a phase mapping (Q7). | — |
| E8 | PLAN:517, 1077–1101, 1382; mockups/key-module.md | "Key module" and "Key manager" name the same thing. | Use "Key manager" everywhere. | in P2 |

## 3. Phase 0 and Phase 1 sections

These sections have code behind them; Part C says where code and text differ.

| ID | Where | Finding | Fix | Blocks |
|---|---|---|---|---|
| C1 | PLAN:591 | Ingest flow step 4: "Assigns to an existing open project … or creates a new project (UUID assigned immediately, status open)". This breaks Phase 2 rules 1 and 4: `n` creates a temp project; real projects only in the Project manager. | "Assigns to an existing open project (real or temp), or creates a temp project (`n`, name + FPS)." | P2 start |
| C2 | PLAN:588 | Step 3: "Present guesses and file list to operator". The ingest screen shows a card summary, not a file list (PLAN:612). | "Present guesses and the card summary". | — |
| C3 | PLAN:545–564 | The magcard example differs from the real format. Real manifest: `relative_path \| size \| sha256 \| mtime`. Header keys: `format`, `mag_folder`, `project_status`, `camera_profile_id`, `source_label`, `ingested_by`, `updated_at`. Optional keys: `camera_model_hint`, `mixed_cameras`, `requirements_warning`, `reingest_of`, `project_requirements`. The example also uses `filename`, against "relative paths everywhere" (PLAN:1234). | Replace the example with the real one and list the optional keys. In Phase 2 add `project_kind` and drop `project_requirements` (C1). | in P2 |
| C4 | PLAN:576 vs PLAN:1204 | Conflict handling: "database wins" covers "manifest". The threat model says "Manifest fixed at ingest. Mismatches reported, never auto-resolved." The code follows the threat model: reconcile reports manifest differences as INTEGRITY. | Remove "manifest" from PLAN:576. | — |
| C5 | PLAN:20 vs PLAN:1230 | "A separate staging dataset is not used for ingest" vs "use a path on the mags dataset or a dedicated staging dataset". The two lines are about different things: the server's `incoming` and the client's local staging. docs/test-checklist.md also creates a `tank/staging` dataset. | Server `incoming` is on the mags filesystem (enforced). Client staging is any path that is not a tmpfs. | — |
| C6 | PLAN:608 vs 610 | "Always 24-hour clock" vs "can be set in config". | "24-hour by default". | — |
| C7 | PLAN:112–207 | The schema section is two phases out of date: <br>• projects: requirements "optional" and lists "format"; no `kind`, approval or `requirements` rules. <br>• mags: `dump_path` (code `folder`); camera "free text" (against PLAN:1244); status "archived" (against PLAN:1246). <br>• encoder_jobs: `clip_id` (the code uses `encoder_job_clips`). <br>• Missing tables: `files`, `people` / `clip_people`, `project_log`, `encoder_job_clips`, `copy_jobs`, `zmags`. <br>• No soft-lock fields. | Replace it with an entity list and rules: what each table means, which fields are locked, and the Phase 2 additions (PLAN:1340–1343). Point to `src/db.rs` for the columns. | P2 start (with schema 3) |
| C8 | PLAN:265 | "compares … resolution, frame rate and format". The third check is GOP (PLAN:258). | Change "format" to "GOP". | — |
| C9 | PLAN:435 | Log levels "INFO/WARN/ERROR", but DEBUG exists (PLAN:450). | Add DEBUG. | — |
| C10 | PLAN:294 | "`init` still installs the starter set into an empty folder." The code (since `ab3a1f6`) installs every missing shipped profile, never overwrites, and reports local files that differ. | Make PLAN match the code. | — |
| C11 | PLAN:496 | "No other non-path settings are defined today." Today's settings: <br>• server: `debug`, `log_retention_days`, `default_profile_320p`, `encoder`, `default_profile_320p_gpu`, `vaapi_device`, `ffprobe`; <br>• client: `debug`, `committer_wait`, `staging_autoclean`, `date_format`, `time_format`, `ffprobe`. | List them (Part B §6). | — |
| C12 | PLAN:641 | F1 "In this batch". | Drop "In this batch". | — |
| C13 | PLAN:1260 | USB master/slave "… Or drop the distinction if it adds no value". The distinction is locked and built. | Delete the second sentence. | — |
| C14 | PLAN:301 | Camera profiles "are loaded into the database". The code reads the files on every use and publishes a list to clients (gate `cameras`). | Drop "into the database", or schedule it with the Profile Editor (Phase 5). | — |
| M1 | PLAN:581 | Automatic card discovery ("watch for newly mounted volumes") is not built. Discovery runs on rescan. | Say rescan is enough (the TUI rescans on `r` and on entry), or schedule watching. | — |
| M2 | PLAN:581 | ARRI and Fujifilm card layouts are listed but not recognised (Fujifilm only as generic DCIM). | Schedule with a reference card; until then they are manual-path only. | — |
| M3 | PLAN:652 | "Extra mirror to one or more backup drives at any time." Only a mirror at ingest (`--mirror-set`, one set) is built. | Schedule it (Phase 6 polish), or reduce to "at ingest". | — |
| M4 | PLAN:457–459 | Log retention ("on startup and daily") is locked, but nothing deletes logs. | Build it with the `serve` service in Phase 2 (Part C §11.7). | in P2 |
| S1 | PLAN:1206 | "No … full paths that leave the system in logs." The client issue lines and forwarded DEBUG lines carry full paths to the server log. | Decide in the Phase 1 security review: strip the paths, or narrow the sentence to secrets and key material. | Phase 1 gate |
| S2 | PLAN:1083, 1095 vs PLAN:1202, 1250 | The Key module adds keys to `~magvaultclient/.ssh/authorized_keys`, while the Security defaults and the threat model say keys come from AuthorizedKeysCommand reading a Magvault-managed file. mockups/key-module.md follows the first. The Phase 2 Key manager needs one of them; the producer flag and labels need the managed file. | AuthorizedKeysCommand + `/etc/magvault/client_keys`, owned by `magvault`, read by a dedicated `AuthorizedKeysCommandUser`. Each line holds: label, producer flag, public key, created and revoked. The Key manager writes that file; it never touches `authorized_keys`. | P2 start |
| — | mockups/dashboard.md | The client dashboard mockup shows `[s] Server dashboard` (PLAN: `h`), and its staging is `/tmp/magvault-staging`, which the code refuses. | Update the mockup. | — |

## 4. Phase 2

| ID | Where | Finding | Fix | Blocks |
|---|---|---|---|---|
| C15 | PLAN:786, 813 (`h`) | The client "can open the full server dashboard over SSH". The client's key is locked to the forced-command gate, so it cannot open a server screen. This is the same reason the producer got a separate login (PLAN:518). | The client dashboard shows a server status summary from a gate verb: queue depth, active job, free space, heartbeat. `h` opens that summary. Anyone who needs the full server dashboard logs in to the server. | P2 start |
| C16 | PLAN:513 | "Client user (`magvaultclient`) … Can use Ingest, Logging, Dashboard, Encoder". `magvaultclient` is only the SSH identity behind the gate; the client TUI runs as the operator. "Encoder" also contradicts "Clients do not encode" (PLAN:1347) and "client does not show full encoder progress" (PLAN:787). | `magvaultclient` is the gate identity. The client TUI (operator account) has Dashboard, Ingest, Job card, and later Logging and Library. Encoder is server-only; the client sees the queue depth. | in P2 |
| C17 | PLAN:781, 786, 813 | The dashboard menus and letters have no Encoder, Project manager, Key manager or Job card entry. The `magvault-producer` dashboard (read-only + Project manager) has no defined menu. | `t` Encoder, `m` Project manager, `k` Key manager (server, admin), `j` Job card (client). The producer sees `m` and the read-only dashboard only. | in P2 |
| C18 | PLAN:1118–1121 vs PLAN:1349 | The telemetry "must show" list includes ZFS health of the working datasets and of zmags. Day one is queue depth, disk free and serve heartbeat. The JSON schema is an open point (PLAN:1407). | Phase 2 telemetry: <br>• required: queue depth; active job and progress; encoder backend in use and fallback count; free space per root; serve heartbeat. <br>• optional: working-dataset ZFS health. <br>• Phase 4: zmags. <br>Define the JSON (Part B §7). | in P2 |
| C19 | PLAN:1326 | "Merge a temp into an existing real project, or assign a new name." It is unclear whether renaming promotes the temp to a real project, and who checks requirements when a temp with 25 fps is merged into a project that requires 50. | Two producer actions: <br>• **promote:** the temp becomes real, optionally renamed, and its mags get 320p jobs; <br>• **merge:** its mags move to an existing real project, the temp is closed, and the mags get 320p jobs. <br>Both re-check the requirements of the target and write mismatches to the project log. | in P2 |
| C20 | PLAN:1189–1207 | The threat model actors have no `magvault-producer`. The new Phase 2 trust boundaries are not in the table: the producer spool, the client Job card write through the gate, and the GPU device (`render` group). | Add the actor and three rows. | Phase 2 gate |
| M5 | PLAN:244, 522 | A producer key "edits the Job card on an open project" from the client. The gate cannot write the database, and the request path is not described. It is also not said whether a temp project's FPS can be edited there. | Job card edits are spool requests (`jobcard` file in the client's spool). `serve` applies them only if the key label has `producer: yes` in the managed key file, and logs them with the label. Temp projects: the producer may edit FPS and the other fields too. | in P2 |
| M6 | PLAN:1289 | Pull-back: "On the next sync or status, the server sends mag UUID → real project." No gate verb or client cache is named. | Gate verb `assignments`: `mag_uuid \| project_uuid \| project_name` for mags this client uploaded, cached in `<config>/assignments.cache`. The client applies it to staging, and to sets when they are mounted. | in P2 |
| M7 | PLAN:518 | The producer spool has no location, format, permissions or result path. | `mags/.incoming/.producer/` (group `magvault-producer`, 2730), one request file per action (`key: value`), results in `.results/producer/`. `serve` validates, applies, logs with the Unix user, and writes a result file. | in P2 |
| M8 | PLAN:785, 1116 | "A client may request a short status summary over SSH (connection state, queue depth)". No verb is named. | Extend the existing `ping` / `status` output (backward compatible: new lines after the current ones). | in P2 |
| M9 | PLAN:1340–1343 | Schema 3 notes have `kind`, `approved_at`, `approved_by` and requirements. They do not cover: <br>• a record of merges (which temp went where); <br>• the migration of existing projects; <br>• whether existing projects without FPS stay valid. | Add: `merged_into` (project id, null) and `merged_at` on projects. Migration 2 → 3: existing projects become `kind = real`, requirements unchanged; FPS is required only for new projects. | P2 start |
| M10 | PLAN:413, 926 | "Audio channels = source exactly". Many cameras record several mono audio streams (FX6, CX350: 4 or 8 streams), not one multichannel stream. VFR sources (phones, drones) need "frame rate = source" to mean passthrough. | Map every audio stream and keep each stream's channel count and layout. For frame rate, use passthrough timing (no resample) and record the source fps. | in P2 |
| D1 | PLAN:68, 916 | Proxies go to `proxies/db/<mag>/` with "original basenames". One card can hold the same basename in two folders (Canon `DCIM/100CANON/MVI_0001.MOV` and `DCIM/101CANON/MVI_0001.MOV`), so proxies would overwrite each other. Bins already recreate relative paths (PLAN:1235). | Proxies mirror the clip's relative path: `proxies/db/<mag>/<relative_path>.mp4`. | P2 start |
| T1 | PLAN:1331–1337 | The VAAPI path is specified well, with three gaps for the implementer: <br>• whether 4:2:2 10-bit and ProRes decode on the CPU (the plan says decode on CPU, so yes); <br>• the width rounding to even; <br>• one GPU job at a time. | Note in the worker spec: GPU concurrency 1; CPU fallback reuses the same job row (`attempts`); the backend used is stored on the job. | in P2 |

## 5. Phase 3 (Logging and Library)

| ID | Where | Finding | Fix | Blocks |
|---|---|---|---|---|
| S3 | PLAN:941, 943 | The client "pulls proxies" and "can pull the finished files", but the gate is upload-only (`rrsync -wo` into the client's spool). A download path is a new trust boundary. | Add read-only gate verbs scoped to `proxies/db/<mag>/` for mags in open projects, and to export results requested by that key (`rrsync -ro` with a per-request path check). Add them to the threat model. | P3 |
| M11 | PLAN:741, 1257 | Client logging "syncs to the server database when connected". The write path is not specified. The conflict rule exists (per-field last-writer-wins + WARN). | Logging edits are spool requests (`logging` file: mag, clip, field, value, client time). `serve` applies them with the per-field rule and rewrites the magcard `# logging` section. | P3 |
| C26 | PLAN:738 vs PLAN:730 | "Log data … can optionally be written into a sidecar or the mag card" vs "Logging fields are written into magcard.txt". | Drop "optionally … sidecar". | P3 |
| C21 | PLAN:981 vs PLAN:1234 | The export card's clip line has `source_path` with an absolute path (`/mags/.../clip001.MOV`). The implementation defaults forbid absolute paths in export cards. | `# mag_folder \| relative_path \| optional_in \| optional_out`. | P3 |
| C27 | PLAN:1002 vs PLAN:9 | Export `done` is reached when copied "(and verified if verification is on)", while "Every later copy … is verified against the stored SHA-256". | Always verified; drop "if verification is on". | P3 |
| M12 | PLAN:210–234, 215 | Soft lock is specified but has no phase. It also triggers on an export "marked final / delivered", and exports have no such state (PLAN:1000–1004: warm, cold, copying, done). | Put it in Phase 3 with the Library. Add an export flag `delivered` (operator action) separate from the copy status. Add the lock fields to schema. | P3 |
| M13 | PLAN:933–946 | Export in/out points exist in the card and the schema, but the Library section does not say how the operator sets them. | Specify the action (keys and fields) in the Library section and mockup. | P3 |
| T2 | PLAN:880–884 | Playback: mpv runs where the TUI runs. A server TUI used over SSH would play audio on the server's speakers. | Playback only where the TUI runs locally (client with local proxies, or the server console). Over SSH: status only, no audio. | P3 |

## 6. Phase 4 (Zmag manager)

| ID | Where | Finding | Fix | Blocks |
|---|---|---|---|---|
| C22 | PLAN:855/1011 vs PLAN:1043–1047 | Evict to cold needs "at least two complete placements on different zmags", but the copy model is roles: first copy on a primary zmag, second on a backup zmag. Two primaries would allow eviction without a backup copy. | Evict needs a complete placement on at least one primary zmag and at least one backup zmag. | P4 |
| C23 | PLAN:1038 vs PLAN:477, 1203 | "Plug in drives → zmag manager auto-imports and mounts" vs "every call asks for the sudo password" (`zpool import` runs through the helper). | Docking shows the recognised pool and asks for the password once to import (no silent auto-import). Or decide that import is the one helper verb allowed without a password (sudoers `NOPASSWD` for `magvault-helper zpool-import <name>` only). Owner decision (Q4). | P4 |
| C24 | PLAN:864, 874 vs PLAN:794, 800 | Zmag keys `←` / `→` = switch zmag, but the shared keys say `←` / `→` = previous / next pane. Zmag `Esc` goes "from secondary screens → main" while the shared rule is "Esc returns to the dashboard". | Switch zmags with `[` / `]`. Keep the Esc exception, and say in the shared key convention that secondary screens step back one level. | P4 |
| M15 | PLAN:868–873, 1055–1056 | The helper verbs for Phase 4 are not listed: <br>• zpool create, import, export, scrub, replace; <br>• `zfs hold` / `zfs release` / destroy snapshot; <br>• `smartctl`. <br>The key table says only destroy needs sudo, but hold, release, scrub, replace and SMART need root too. | List the verbs and mark each "needs password". Keep one helper. | P4 |
| M16 | PLAN:1058–1065, 1406 | The on-zmag manifest format and location are open. It is also not said whether a zmag copy writes a new ASC MHL generation (PLAN:1240 says it "can"). | `/<pool>/magvault-zmag.txt` (`key: value` + `# mags` lines `mag_uuid \| mag_folder \| copied_at \| verified_at`). Write an MHL generation on every verified zmag copy. | P4 |

## 7. Phase 5, Phase 6 and unassigned

| ID | Where | Finding | Fix | Blocks |
|---|---|---|---|---|
| C25 | PLAN:1382, 1385 | Phase 5 still lists the "Key module (admin, generate or accept, USB export, discard on leave)", which is the Phase 2 Key manager, and the "Sudo helper", which was built in Phase 1. | Remove both, or reduce the first to "Key manager: USB export polish". | — |
| C28 | PLAN:838 | Encoder `e` jumps to the Profile Editor (Phase 5) while the Encoder is Phase 2. | Until Phase 5, `e` shows "Profile Editor comes in Phase 5; edit `encoders/*.enc`". | in P2 |
| M17 | PLAN:485–494 | Settings edits `path.conf` roots on a live system without saying what happens to `serve` and running jobs. The client settings list is incomplete (see C11). | Settings writes the file and asks for a `serve` restart; roots cannot change while jobs run. List the client settings. | P5 |
| M18 | PLAN:291 | "Profile update command" (git or repo) has no phase. | Phase 6 packaging. | — |
| M19 | various | These items are locked or deferred but in no phase: <br>• volume watching (M1); <br>• ARRI layouts (M2); <br>• mirror at any time (M3); <br>• mouse (PLAN:806); <br>• the F2 key (PLAN:642). <br>The module shape (PLAN:1152) should come early in Phase 2. | Assign each, or mark "not planned". | — |
| M20 | PLAN:1070 | The remote zmag station "updates the central database", which breaks the single write path through `serve` (PLAN:518). | The station sends results through a spool, the same pattern as clients and producers. | P6 |

## 8. Mockups

Mockups exist for 11 screens. The Phase 1 ingest mockup is current. Gaps:
- **No mockup:** Project manager, Job card, Key manager (Phase 2 shape), server status
  summary on the client.
- **Dashboard:** missing letters (C17), client `[s]` vs `h`, a `/tmp` staging path.
- **Key module:** writes `authorized_keys` (S2).

Draw the four new screens before their code, at both sizes (150x42 and 120x36), as for the
existing ones.

---

# Part B — Technical specification

This part describes the system as built (Phases 0–1) and the target for Phases 2–6. Where
the target needs a mechanism that PLAN does not name, it is marked **proposal** and listed
in Part D.

## 1. Programs

| Program | Runs as | Role | Phase |
|---|---|---|---|
| `magvault <command>` (server config) | `magvault` | init, ingest (manual path or discovered card), reconcile, projects, project-log, cards | 0–1 |
| `magvault serve` | `magvault` | single writer: commits client uploads, publishes projects / requirements / cameras, heartbeat, drains client log lines. Phase 2: becomes `magvaultd` (systemd), adds the encode worker, telemetry, producer spool, log retention. | 1 → 2 |
| `magvault gate` | `magvaultclient` (SSH forced command) | client verbs only; never writes the database; reads `.results/` | 1 |
| `magvault` / `magvault ui` (client config) | operator's own account | client CLI and TUI: ingest, sync, usb, staging, projects, cards | 1 |
| `magvault ui` (server config) | `magvault` (admin) or `magvault-producer` | Phase 2: server dashboard; Project manager (producer and admin); Encoder, Key manager (admin) | 2 |
| `magvault-helper` | root via `sudo` (password every time) | the only privileged path. Today: `usb-format`, `usb-clean`. Phase 4: zpool / zfs / smartctl verbs. | 1 → 4 |

## 2. Principals and trust boundaries

| Principal | Where | Can | Cannot |
|---|---|---|---|
| `magvault` (admin) | server | everything in Magvault; sudo for the helper with a password | general sudo |
| `magvault-producer` | server, own SSH login | read the database (read-only, rollback-journal mode); write producer requests to the producer spool; `magvault ui` with the Project manager and a read-only dashboard | write the database or files under `mags/`; admin modules; the helper |
| `magvaultclient` | server, SSH forced command `magvault gate --client <label>` | upload into its own spool (`rrsync -wo`); commit / status / ping / projects / requirements / cameras / log. Phase 2: `assignments`, `jobcard` requests. Phase 3: read-only proxy and export downloads (S3). | shell, `mags/`, other clients' spools, any database write |
| operator | client machine, own account | client CLI / TUI; sudo for the local helper (USB Prepare / Clean) | — |
| AuthorizedKeysCommandUser (**proposal**, S2) | server | read `/etc/magvault/client_keys` | anything else |

Write path: **every database write happens in `serve`** (or in a server-side `magvault`
command run by the admin). Clients and producers write request files; `serve` validates
and applies them, then writes a result file. Phase 6 remote stations follow the same
pattern (M20).

## 3. Where state lives

| State | Location | Authority |
|---|---|---|
| Media | `mags/<YYYYMMDD>_<uuid>/` | the files; manifest fixed at ingest |
| Mag metadata | database + `magcard.txt` (header, manifest, `# logging`) | database for metadata; manifest never auto-resolved |
| ASC MHL | `mags/<mag>/ascmhl/` | C4 hashes; `magcard.txt` and `.magvault/` ignored |
| Projects, requirements, project log | database | `serve` |
| Client uploads | `mags/.incoming/.clients/<label>/` (spool), `.results/` (written by `serve`) | `serve` commits after verification |
| Client staging | client path (never tmpfs) + `.magvault/synced` | client; kept until server + field set (convenience buffer) |
| Dump ledger | `<client config>/dumps.txt` (fingerprint, folder, server, set, volume identity) | client; drives the card colour and "already dumped" |
| Field sets | `magvault-backup.txt` on each drive (`magvault-backup/1`) | the drives |
| Client caches | `projects.cache`, `projects.local`, `requirements.cache`, `requirements.local`, `cameras.cache`, `issues.pending`; Phase 2 `assignments.cache` | server lists win when online |
| Logs | `logs/YYYY-MM-DD.log` (UTC day), both sides | files only |
| Telemetry (Phase 2) | `telemetry_live` (tmpfs), copied to `telemetry_disk` every 30 min | `serve` |

## 4. Gate verbs

| Verb | Phase | Purpose |
|---|---|---|
| `rsync --server …` | 1 | upload into this client's spool only (`rrsync -wo`) |
| `commit <folder>` | 1 | ask `serve` to verify and commit |
| `status <folder>` | 1 | commit result |
| `ping` | 1 | `magvault-gate ok <label>` + `serve alive` / `serve down: …`. Phase 2: add queue depth, active job, free space (M8, C15). |
| `projects` | 1 | open projects `uuid \| name`. Phase 2: include open temp projects, marked as temp (PLAN:523). |
| `requirements` | 1 | `uuid \| fps=25 res=HD gop=intra` |
| `cameras` | 1 | the server's camera profiles, for the pre-upload check |
| `log` | 1 | client issue / DEBUG lines on stdin (rate-limited) |
| `assignments` | 2 (**proposal**) | `mag_uuid \| project_uuid \| project_name` for pull-back (M6) |
| Job card request | 2 (**proposal**) | uploaded request file, applied by `serve` if the key is a producer key (M5) |
| proxy / export download | 3 (**proposal**) | `rrsync -ro` scoped per request (S3) |
| logging request | 3 (**proposal**) | per-field edits (M11) |

## 5. Lifecycles

**Mag:** `ingesting` → `online` → (Phase 4) `cold` ↔ `pulling` → `online`; `failed` on a
commit error. A mag in a temp project is `online` but gets no encoder job. Folder names
never change.

**Project (Phase 2):**

- **temp:**
  - created by `n` or `--project <new>`, with name + FPS;
  - open;
  - its mags are copied and committed but not encoded.
- **promote (C19):** the temp becomes real, optionally renamed; it is approved; its mags
  get 320p jobs.
- **merge (C19):** its mags move to an existing real project; the temp is closed with
  `merged_into`; the mags get 320p jobs.
- **real:**
  - created only in the Project manager (FPS required);
  - open or closed;
  - closed projects receive no mags.
- After promote or merge, pull-back rewrites the magcards on the server at once, in
  staging at the next sync, and on field sets when they are next mounted (project keys
  only, previous card backed up).

**Encoder job (Phase 2):**

- **Created:** when a mag is in a real project, at commit or at promote / merge.
- **States:** `queued` → `running` → `done` / `failed` / `cancelled`.
- **GPU path:** with `encoder: gpu` the worker uses `default_profile_320p_gpu` (VAAPI). A
  failure reruns once on `default_profile_320p` (CPU) with a WARN.
- **Concurrency:** one GPU job at a time.
- **Output:** `proxies/db/<mag>/<relative_path>.mp4` (D1).
- **Phase 0/1 jobs:** already queued for real projects, so they stay queued.

## 6. Configuration

`path.conf` (roots only): `mags`, `proxies`, `bins`, `db`, `logs`, `cameras`, `encoders`,
`custom_encode`, `telemetry_live`, `telemetry_disk`, optional `incoming`. Client
`path.conf`: `staging`, `logs`, `cameras`, `server_host`, `server_user`,
`server_transport`, and others.

`magvault.conf`:

| Key | Side | Default | Phase |
|---|---|---|---|
| `debug` | both | no | 1 |
| `log_retention_days` | both | 30 | written in 0; enforced in 2 (M4) |
| `ffprobe` | both | (PATH, then standard paths) | 1 |
| `default_profile_320p` | server | default-320p | 0 |
| `encoder` | server | gpu | 2 |
| `default_profile_320p_gpu` | server | default-320p-gpu | 2 |
| `vaapi_device` | server | /dev/dri/renderD128 | 2 |
| `committer_wait` | client | 120 | 1 |
| `staging_autoclean` | client | yes | 1 |
| `date_format`, `time_format` | client | DD/MM/YYYY, 24h | 1 |

## 7. Telemetry JSON (Phase 2, **proposal**)

```json
{
  "written_at": "2026-11-28T10:00:00Z",
  "serve": {"pid": 1234, "state": "running", "heartbeat": "2026-11-28T09:59:58Z"},
  "encoder": {"backend": "gpu", "fallbacks_today": 0, "queue_depth": 3,
              "active": {"job": "…", "mag": "20261127_…", "progress": 62}},
  "roots": [{"name": "mags", "path": "/magvault/mags", "free_bytes": 0, "total_bytes": 0}],
  "zfs": []
}
```

`zfs` stays empty until working-dataset health is added (optional in Phase 2); zmags come
in Phase 4. The client summary from `ping` carries `queue_depth`, the active job's
progress and heartbeat age.

## 8. Security model (target)

- **sudoers:** `magvault ALL=(root) /usr/lib/magvault/magvault-helper` with
  `timestamp_timeout=0`. On clients, the operator has the same rule for the local helper.
  No direct `zpool` / `zfs`.
- **Helper verbs:**
  - Phase 1: `usb-format`, `usb-clean`. They refuse mmcblk devices, unmounted set drives,
    and drives with pending mags.
  - Phase 4: `zpool-create`, `zpool-import`, `zpool-export`, `zpool-scrub`,
    `zpool-replace`, `zfs-hold`, `zfs-release`, `zfs-destroy-snapshot`, `smart` (M15).
- **SSH:** `restrict,command="magvault gate --client <label>"`. Keys come from the managed
  file via AuthorizedKeysCommand (S2, Phase 2).
- **Filesystem:**
  - Spools: per-client `2770` with group `magvault-spool`; `.results` readable by
    clients, writable by `magvault` only.
  - Producer spool: group `magvault-producer`, `2730`.
  - Database: `0640`, group `magvault-producer`.
  - `render` group for `magvault` (VAAPI).
- **Input validation:** every path and name that crosses the gate or a spool: mag folder
  pattern, labels `A-Z0-9-`, no `|` or control characters in card fields.
- **Secrets:** the sudo password is passed once to `sudo -k -S`, held in a `Secret` type,
  and never logged. Private keys are shown once and never stored after handover.

## 9. Error handling principles (built in Phase 0–1; keep)

- **Copies:**
  - Nothing is committed without SHA-256 verification.
  - A failure leaves the source untouched and the incoming copy is removed.
  - Every failure after confirmation is logged.
- **Pending work:**
  - Client mags stay pending until `serve` confirms.
  - A committer that is down is shown as "server up, committer down", with bounded waits.
- **Re-ingest:** "already dumped" is refused under `--yes` unless `--force`.
- **Interrupts:** Ctrl-C (or `x` in the TUI) aborts with nothing committed.
- **Retry semantics** for Phase 2–4 copy paths (zmag, cold pull, export) are still an open
  point (PLAN:1408). Use the same rule: verify, then rename or record; never mark complete
  on a partial copy.

---

# Part C — Code vs plan

## 10. What the code is

| Area | Modules | Notes |
|---|---|---|
| Config, init | `config.rs`, `init.rs` | roots, same-filesystem `incoming`, tmpfs refused; starter profiles installed when missing |
| Ingest core | `ingest.rs`, `probe.rs`, `hash.rs`, `mhl.rs`, `magcard.rs`, `catalog.rs`, `reconcile.rs` | hash once (SHA-256 + C4), rsync `--files-from`, verify, magcard + MHL, DB commit, 320p job |
| Camera identification | `discover.rs`, `camera_meta.rs`, `profiles.rs` | model hints from live media, layout gate, generic fallback, volume identity |
| Card summary, requirements | `cardinfo.rs` | one value per card (fps, resolution, GOP); requirement check |
| Client | `client.rs`, `client_ingest.rs`, `sync.rs`, `staging.rs` | transport, routes, pre-upload camera check, two-backup autoclean, dump ledger |
| Server commit | `gate.rs`, `serve.rs` | forced-command gate, spool commit, heartbeat, publishing |
| Field sets | `fieldset.rs`, `usb.rs`, `bin/magvault-helper.rs` | Prepare / Clean, lsblk classification, unmounted-set guard |
| TUI | `tui/*` | client dashboard, Ingest (mockup layout), Prepare / Clean / requirements / mixed / redump popups, F1 |
| Database | `db.rs` | SQLite via C FFI, schema 2 (`project_log`, `projects.requirements`) |

**Tests:**
- 75 unit tests;
- `tests/e2e.sh` (Phase 0);
- `tests/e2e-phase1.sh`: 76 checks, including root loop-device sets and a pseudo-terminal
  TUI bot (`tests/tui_pty.py`).

**Dependencies:** `ratatui` only (Cargo.lock committed).

## 11. Changes needed in the existing code for the current PLAN

These are Phase 2 rules that change behaviour already built.

| # | Change | Files | Size |
|---|---|---|---|
| 11.1 | **Project creation → temp only.** <br>• `n` and `--project <unknown>` create a temp project with `--require-fps` mandatory, on client and server. <br>• `--project` resolves open real, then known temp, then new temp. <br>• `n` refuses an existing name (real or temp). <br>• Server `ingest --project <new>` no longer creates a real project. <br>• `--require-resolution` / `--require-gop` stay optional on a new temp. <br>• Requirements popup: FPS row required. | `ingest.rs` (`choose_project`, `pick_project`, `with_requirements`), `client.rs` (`choose_project`, `remember_local_project`, `parse_projects`), `tui/app.rs` (Requirements popup), `main.rs`, `help.rs` | M |
| 11.2 | **No name merge.** `serve` must never attach an upload to another project because the name matches. An unknown project UUID from a client becomes that temp project (kind temp). Clients drop the "server's project wins" rewrite. | `serve.rs` (`project_for_upload`), `client.rs` (local project drop), docs/phase1.md | S |
| 11.3 | **320p job only for real projects.** No `encoder_jobs` row when the mag's project is temp; create it at promote / merge (11.1 + Project manager). Existing queued jobs stay. | `ingest.rs` (`commit_db`), `serve.rs` (`commit_upload`) | S |
| 11.4 | **Green = on the server.** `status_of` returns Safe when the server confirmed (no copy count); Dumped when there is a local copy only; texts and tests follow. | `staging.rs` (`status_of`, `DumpStatus::text`), `tui/view.rs`, `tui/tests.rs`, `tests/e2e-phase1.sh` | S |
| 11.5 | **Encoder profiles.** <br>• Accept `rate_control: qp` + `qp`. <br>• Refuse `crf` / `preset` on `*_vaapi` codecs. <br>• Ship `share/encoders/default-320p-gpu.enc`. <br>• Read and validate `encoder`, `default_profile_320p_gpu`, `vaapi_device`; add them to the server `magvault.conf` template. | `profiles.rs` (rate_control check), `ingest.rs` (default 320p selection), `init.rs`, `share/encoders/` | S |
| 11.6 | **Schema 3.** <br>• `projects.kind` (`temp` / `real`), `approved_at`, `approved_by`, `merged_into`, `merged_at` (M9). <br>• Migration 2 → 3: existing projects become `real`. <br>• Magcard: add `project_kind`; stop writing `project_requirements` (a temp carries its own requirements on the server). | `db.rs`, `ingest.rs`, `catalog.rs`, `magcard.rs` | M |
| 11.7 | **Log retention** (locked since Phase 0, M4): delete daily logs older than `log_retention_days` at start and daily; on both sides. | `log.rs`, `serve.rs` | S |
| 11.8 | **Module shape** (PLAN:1152) before the Phase 2 screens: name, keys, draw, on_key, cli; the dashboard menu comes from enabled modules. | `tui/*` (refactor) | M |
| 11.9 | **Server `ui`:** today it refuses ("comes in Phase 2"). Phase 2 opens the server dashboard; as `magvault-producer`, only the Project manager and a read-only dashboard. | `main.rs`, new `tui` modules | — (new work) |
| 11.10 | **Full paths in client log lines** (S1): strip or shorten if the security review keeps the rule. | `client.rs` (`queue_issue`), `log.rs` | S |

These items need **no code change**; PLAN should follow the code:
- C4: the manifest is never auto-resolved;
- C10: `init` installs missing profiles;
- C11: the settings list;
- C14: profiles are not loaded into the database.

## 12. Phase 1 temporaries still in the code

| Temporary | Replaced by | Phase |
|---|---|---|
| Hand-written forced-command lines in `authorized_keys` | Key manager + AuthorizedKeysCommand (S2) | 2 |
| `magvault serve` started by hand | `magvaultd` systemd unit (Part D, first Phase 2 step) | 2 |
| Client-local camera profiles (checked against the server list) | profiles served by the server | 2–5 |
| `server_transport: local` | tests only; keep | — |
| Uploaded files keep the `magvaultclient` owner (group-readable) | acceptable; document in the security review | Phase 1 gate |

## 13. New code per phase (from PLAN, with the fixes above)

| Phase | New code | Depends on |
|---|---|---|
| 2 | Schema 3; module shape; `magvaultd` unit and log retention; Key manager (TUI + CLI; managed key file; AuthorizedKeysCommand); producer spool and Project manager (create, promote, merge, rename, edit requirements, close / reopen, project log view); temp projects on the client (11.1, 11.2); `assignments` verb and pull-back (staging, field sets); Job card (client, read-only; producer edits via a spool request); 320p worker (VAAPI + CPU fallback, proxy rules, relative-path output); Encoder module; telemetry writer; server dashboard; client status summary via `ping`; Library stub | 11.1–11.8, S2, C15, C19, D1, M5–M10 |
| 3 | Logging module (TUI, magcard `# logging`, client spool requests, per-field LWW); Library server + client (proxy cache, read-only download verb); export card (relative paths) and copy jobs; cold-pull request generation; soft lock; mpv playback (local only) | S3, M11, C21, C26, C27, M12, M13, T2 |
| 4 | Zmag manager: helper verbs, create with role, import on dock (C23), two-pane rsync copy with SHA-256 verify and MHL generation, auto-backup by role, eject, evict (primary + backup, C22), placements, on-zmag manifest, snapshots, pool health, SMART | C22, C23, C24, M15, M16 |
| 5 | Export bins browser, Profile Editor (and profile reload in `serve`), Logs viewer (filters, follow), Settings (server, client) | C25, M17 |
| 6 | Remote zmag stations (spool to the central `serve`), split, HQ toggle, desktop client, packaging (`.deb`, profile update command) | M18, M20 |

---

# Part D — Priorities and open questions

## 14. Before Phase 1 closes (the gate)

1. Hardware retest of `2d8f940` on rafiki:
   - unmounted set ("set not mounted", no Prepare);
   - already dumped (`--yes` refuses, TUI popup, one `--force`);
   - ffprobe found in a non-interactive session;
   - A039 vs the CX350 image shows yellow.
2. The real TUI sudo prompt: wrong password, Esc ("sudo cancelled" in both logs), the
   right password, for Prepare and for Clean.
3. Phase 1 security review against the threat model:
   - gate verbs and path checks, spool permissions, `.client-log` limits;
   - `serve` reading uploads;
   - helper verbs and refusals;
   - `sudo -S` handling;
   - S1 (paths in logs), log retention (M4), `serve` by hand, hand-written keys.

   Each gets a decision to fix or to accept until a named phase.
4. Fixes from that review. Then: `debug: no` on both hosts, README status, Phase 1
   closed.

## 15. PLAN fixes before Phase 2 code

Each of these changes what the first Phase 2 code does:
- **C1:** ingest flow step 4.
- **C7 + M9:** schema section and schema 3 notes (merge record, migration).
- **S2:** Key manager writes a managed file served by AuthorizedKeysCommand.
- **C15:** client `h` = status summary via the gate.
- **D1:** proxies mirror relative paths.

Settle these during Phase 2, before the matching code:
- C16, C17, C18, C19, C20;
- M5, M6, M7, M8, M10;
- T1;
- C28, E8.

## 16. Phase 2 start order

1. **Groundwork:**
   - module shape (11.8);
   - schema 3 and migration (11.6);
   - `magvaultd` unit and log retention (11.7);
   - encoder profile loader (11.5).
2. **Key manager** (TUI + CLI) with the managed key file and AuthorizedKeysCommand. This
   is first because producer rights depend on it.
3. **Temp projects** on client and server (11.1–11.3) and the **producer spool + Project
   manager** (create, promote, merge, rename, requirements, close / reopen, project log).
4. **Assignment pull-back** (`assignments` verb; staging and set magcards).
5. **Job card** (client) with producer edits via spool requests.
6. **320p worker** (VAAPI, CPU fallback, proxy rules, relative-path output) and the
   **Encoder module**.
7. **Telemetry**, server dashboard, client status summary, Library stub, green = on the
   server (11.4).

Steps 6 and 7 can run in parallel with 3–5 once step 1 is in.

## 17. Open questions (owner decisions)

| # | Question | Recommendation |
|---|---|---|
| Q1 | C19: is "assign a new name" a promotion to a real project? | Two actions: promote (temp becomes real, renamable) and merge (into an existing real project). |
| Q2 | S1: keep "no full paths in logs" literally? | Narrow it to secrets and key material. Keep paths in the server's own log; shorten paths in forwarded client lines. |
| Q3 | S3: what may a client download in Phase 3? | Proxies of mags in open projects, and only the export results its key requested. |
| Q4 | C23: zmag import on dock: password every time, or a single password-free verb? | Password once per dock. Keep "every privileged call asks". |
| Q5 | C22: evict rule | At least one complete primary placement and one complete backup placement. |
| Q6 | M12: soft lock phase and the "delivered" export state | Phase 3, with an explicit `delivered` flag on exports. |
| Q7 | E7: which phase completes version 1.0 in the tick-tock track? | 1.0 = end of Phase 5 (all terminal modules). 1.5 = the rewrite. Phase 6 items go to 1.x or 2.0. |
| Q8 | M1–M3: volume watching, ARRI layout, mirror at any time | Rescan is enough. ARRI when a reference card exists. Mirror at any time in Phase 6. |
