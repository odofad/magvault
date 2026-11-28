# Magvault — Phase 2 readiness review

Date: 2026-11-28. Reviewed:
- PLAN.md at `b961743`: Phase 1 and 2 roadmap, Phase 2 decisions, Offline temp project,
  Assignment pull-back, and the sections they touch.
- docs/phase1.md and docs/plan-audit-phase0-1.md.
- README status.
- The code on `main`.

Test reports: docs/test-report-clean-full-run.md and docs/test-report-camera-profiles.md are
not in the repository. The clean full run is taken from the summary handed over on
2026-11-28: pass at `f8c1b32`, five findings, four of them fixed in `2d8f940`. No code
was changed for this review.

## 1. Verdict

**Not ready.** Phase 1 cannot close yet, and PLAN.md has to settle several Phase 2 rules
before coding starts.

- **Phase 1 gate not met.** PLAN.md "Phase gate" needs three things before the next phase
  starts:
  - functional tests on real hardware;
  - a security review against the threat model;
  - fixes for what that review finds.

  The clean full run passed at `f8c1b32`, and that is real progress. But:
  - the `2d8f940` fixes (unmounted set, dump identity, already dumped, ffprobe path) are
    not yet run on rafiki;
  - the real TUI sudo prompt has never been run by a person;
  - no Phase 1 security review has been written.
- **Phase 2 rules contradict Phase 1 behaviour.** The new temp-project rules conflict with
  three things Phase 1 does today, and two of them affect data (§4):
  - `n` creates a real project;
  - every commit queues a 320p job;
  - the server merges an offline project into an open project of the same name.
- **The Phase 2 list contradicts itself** in a few places (§3.2), and the Project manager
  has no section in PLAN.md.

None of this is large. Expect roughly one short Phase 1 closing round (retest, security
review, small fixes) and one PLAN editing pass, then Phase 2 can start in the order in §5.

## 2. Phase 1 gate

### 2.1 Implemented and tested on hardware (rafiki → courage)

| Area | State |
|---|---|
| Card discovery, camera identification (7 cards right profile, FX6 `SALVAGE.TMP` kept, no junk filter) | Pass, clean full run at `f8c1b32` |
| Client ingest routes (staging, set, direct, mirror), gate, `serve` commit, heartbeat | Pass |
| Two-backup autoclean (staging cleaned only after server + set; kept with server only) | Pass |
| Schema 1 → 2 upgrade on restart | Pass |
| Field sets: Prepare (smaller first, ext4, slave limited), Clean, sync | Pass (TESTSET1), CLI sudo path |
| Server camera check before upload, project requirements, mixed-camera warning | Implemented and e2e-tested. The requirements were not set on a real project in the run summary. |

### 2.2 Implemented, not yet on hardware

| Item | Since | What must be seen on rafiki |
|---|---|---|
| **TUI sudo prompt (`sudo -k -S`)** | Phase 1 TUI | Wrong password, Esc ("sudo cancelled" in both logs), correct password, for Prepare and for Clean. Tests run as root and never exercise it. |
| Unmounted set never offered for Prepare | `2d8f940` | `usb status` / TUI show "set not mounted", Prepare refused. First check that `lsblk` reports the `TESTSET1_M/_S` labels; the guard depends on them. |
| Dump status by volume identity; A039 vs CX350 image | `2d8f940` | A039 yellow "files match", never green. Old ledger lines show yellow until re-ingested. |
| Already dumped: `--yes` refuses, `--force` logs `reingest_of` | `2d8f940` | Refusal with nothing copied; TUI popup; one forced re-ingest into a throwaway project. |
| ffprobe from PATH / `ffprobe:` / standard paths | `2d8f940` | Non-interactive ingest on rafiki has fps and resolution again, or a clear warning. |
| Card summary, dump colours, staging bar, F1, ←→, `h` | `f8c1b32` | Covered by the clean run only in passing; worth one look at 120x36 and 150x42. |

### 2.3 Still open in Phase 1

**Required to close Phase 1:**
1. Hardware retest of §2.2 (above all the TUI sudo prompt, and items 2 and 4 of the clean
   run).
2. **Phase 1 security review** against the threat model. It must cover the new trust
   boundaries:
   - the forced-command gate (verbs, path checks, per-client spool, `.client-log` limits);
   - `serve` reading client uploads;
   - the root helper verbs (`usb-format`, `usb-clean`, mmcblk refusal, unmounted-set
     refusal);
   - `sudo -S` with the password in memory (the `Secret` type);
   - and the known gaps listed below.
3. Fixes for what the review finds.
4. `debug: no` on both hosts (operator; no code change, as agreed).
5. README status updated from "hardware testing next" when the gate passes.

**Known gaps to record in the security review:** these are accepted Phase 1 temporaries,
or items the review must decide on.

| Gap | PLAN | State |
|---|---|---|
| Hand-written `authorized_keys` | "AuthorizedKeysCommand reading a Magvault-managed file" | Temporary, documented. **Phase 2 Key manager must replace it** (§4, R5). |
| Full paths in logs and in client lines forwarded to the server | Threat model: "no … full paths that leave the system in logs" | Not fixed (audit M13). Decide: fix, or narrow the sentence. |
| Log retention | `log_retention_days` deleted at startup and daily | Not built (audit M12). |
| `serve` started by hand | `magvaultd` | Temporary. A systemd unit is advisable before Phase 2 adds a worker that must run all the time. |
| Client-local camera profiles | Clients never read profile files | Mitigated: the server list is checked before upload. Still temporary. |

**Deferred and agreed:** these stay out of the Phase 1 gate. Some Phase 1 locked items
were never built (audit M9, M16, M17, M19); this review moves them to later phases:
- mirror to a set after ingest (M9);
- volume watching (M16);
- ARRI layout (M17);
- the module shape (M19).

Introduce the module shape early in Phase 2 (§5, step 1), because Phase 2 adds five
screens.

## 3. Phase 2 scope

### 3.1 The locked list

From the Phase 2 roadmap, the Phase 2 decisions, and the new Offline temp project /
Assignment pull-back sections:

1. 320p encode worker and rules. Jobs queued in Phase 0/1 are picked up. **Server GPU
   only.**
2. Encoder module (queue + ad-hoc to `custom_encode/`).
3. Server dashboard (landing) and client dashboard. Library is a stub.
4. Telemetry file. Day one: queue depth, disk free, serve heartbeat.
5. Project manager:
   - create a project;
   - set and edit requirements;
   - approve a temp project and assign its mags to a real project.
6. Job card on the client: read-only for normal keys.
7. Key manager as a TUI: generate, accept, revoke, producer flag.
8. Producer key: can create a project and edit requirements of an existing project.
9. Temp projects `(T)`:
   - ingest `n` always creates one;
   - name and frame rate are set; other requirements are optional;
   - dump and upload are never blocked;
   - no encode until assigned.
10. Assignment pull-back:
    - the server rewrites the magcard;
    - the client updates staged magcards, and set magcards when the set is next mounted;
    - "assigned, drives not updated" is shown until then.

Confirmed, with the contradictions below.

### 3.2 PLAN.md lines that contradict the list (fix before coding)

| # | Contradiction | Where |
|---|---|---|
| P1 | **Key manager form.** Roadmap: "Thin CLI is enough. Full Key module screen can follow." Decision: "Key manager in Phase 2 is a TUI, not CLI-only." Phase 5 still lists the "Key module (admin, generate or accept, USB export, discard on leave)". | Phase 2 roadmap, Phase 2 decisions, Phase 5 |
| P2 | **Offline job card still "needs a design pass".** That design is now written as "Offline temp project". The decision sentence is stale. | Phase 2 decisions |
| P3 | **Ingest `n` and project creation.** Offline temp project: "`n` always creates a temp project. There is no real-project create on that screen." But other sections say it creates a real project with requirements: the Ingest key table ("New project (then its requirements…)"), Project requirements "Phase 1 implementation" (created by `ingest --require-*` or the New project popup), and docs/phase1.md. Who may create a real project from the client is also split: Job card says a producer key; the producer key list says create + set requirements; the decision says create + edit. The CLI path (`--project NEW` on the client) is not decided at all. | Offline temp project, Ingest keys, Project requirements, Job card, User model |
| P4 | **Requirements of a temp project.** Temp project: "name and FPS (other requirements optional)". Project requirements: "All are optional." Is FPS required for a temp project? | Offline temp project vs Project requirements |
| P5 | **Editing requirements.** PLAN allows editing later and a producer key edits them. Phase 1 code refuses different flags for an existing project. PLAN must say who edits (admin in the Project manager; producer from the client) and whether old mags are re-checked or only new ones. | Project requirements, Phase 2 decisions |
| P6 | **320p always queued vs temp projects.** Three places say every ingest queues a 320p job: the encoder profile ("Always queued after ingest."), Proxies ("320p generation is queued on the server after ingest commit"), and the Encoder module. Temp project: "It does not enter the encode pipeline… until the Project manager assigns it." Phase 0/1 code queues a job on every commit. | Encoder profiles, Proxies, Encoder module, Offline temp project |
| P7 | **GPU only vs the default profile.** "Encode runs on the server GPU only", but `default-320p` is `codec: libx264` (CPU). PLAN does not say which GPU encoder (NVENC, VAAPI, AMF), how the worker detects it, or what happens without a GPU: refuse, or fall back to CPU. | Phase 2 decisions vs Encoder profiles |
| P8 | **No Project manager section.** The Project manager is named in the roadmap, Job card, Offline temp project and Assignment pull-back. It has no screen, keys, CLI or rules. Missing: close / reopen, temp approval, assign, the project log view, and who may do what. | — |
| P9 | **Dashboard keys for the new modules.** The dashboard letters cover `i g l e z p v s h`. There is no letter for Encoder, Project manager, Key manager or the client Job card. The server dashboard menu lists none of them either. | Dashboard, Shared key convention |
| P10 | **Telemetry scope.** The Telemetry section requires ZFS health of the working datasets and zmags. The decision says day one is queue depth, disk free and serve heartbeat. Say what is Phase 2 (zmags are Phase 4) and how a client gets queue depth (a gate verb; none exists). | Telemetry vs Phase 2 decisions |
| P11 | **Name merge vs temp projects.** Phase 1 (docs/phase1.md, `serve.rs` `project_for_upload`): a project created offline on a client is merged into the one open server project with the same name. Temp projects must stay temp until a producer assigns them. PLAN must replace the merge rule for temp projects, and say whether it stays for anything. | docs/phase1.md vs Offline temp project |
| P12 | **Pull-back on the field set.** "client rewrites … the project line in `magvault-backup.txt`", but that file has no project line (`mag_folder \| state \| updated`). Either add a column (format bump) or drop the sentence. | Assignment pull-back vs field set format |
| P13 | **Magcard rewrite of an approved mag.** The server rewrites `magcard.txt` (with a backup). But Conflict handling says the database wins for "project assignment" and for "manifest", while the threat model says the manifest is never auto-resolved (audit C2). The rewrite must change project fields only. Say so. | Assignment pull-back, Conflict handling, threat model |
| P14 | Stale or leftover lines: "F1 … In this batch"; Phase 5 still lists "Sudo helper" (built in Phase 1); `camera_profile_id … or free text`; `status … archived`; "master/slave … or drop the distinction"; log levels without DEBUG; docs/test-checklist.md still has a `magvaultclient` user on the client. | various (audit C9–C12) |

## 4. Dependencies and risks

### 4.1 What Phase 2 needs from Phase 1

| Need | Phase 1 today | Gap |
|---|---|---|
| **Temp project in the schema** | `projects.status` is `open`/`closed` (CHECK); no kind, approval or approver; `mags.project_id` required | Schema 3: a project kind (`real` / `temp`), approved_by / approved_at, and an assignment history. A temp mag then points at its temp project row. |
| **Held encode for temp mags** | Every commit inserts a `queued` 320p job (`ingest.rs` `commit_db`); `encoder_jobs.status` has no hold state | Either no job until assignment, or a `held` status released on assignment. Existing queued jobs from Phase 0/1 all belong to real projects and can be picked up as they are. |
| **Magcard project fields** | `project_uuid`, `project_name`, `project_status` in the header, rewritten with a backup by serve (name merge) | Usable for pull-back. Needs a temp marker (`project_kind: temp`, or `(T)` in the name; decide which). The rewrite must touch only project keys (P13). |
| **Gate verbs** | upload, `commit`, `status`, `projects`, `requirements`, `cameras`, `ping`, `log` | Missing: **assignments** (mag → real project, for pull-back); **queue depth / status summary** (client dashboard); **producer requests** (create project, edit requirements). The gate runs as `magvaultclient` and cannot write the database, so producer requests must go through the spool to `serve`, like `commit`, and `serve` checks the producer flag. |
| **Key identity** | One forced-command line per key by hand; the client label comes from `--client` in that line | The Key manager must own the key file: labels, producer flag, revoke. `AuthorizedKeysCommand` then turns hand edits into managed state. Without it, the producer flag has nowhere safe to live. |
| **Camera and encoder profiles** | Camera list published by serve; encoder profiles loaded on the server; `default-320p` is libx264 | A GPU encoder profile and a worker that can detect the hardware (P7). |
| **Telemetry paths** | `telemetry_live`, `telemetry_disk` in path.conf (validated); nothing writes them | Phase 2 writes them. Fine. |
| **Field set records** | `magvault-backup.txt` lists `mag_folder \| state \| updated` | Pull-back needs a project column or a decision not to record it there (P12). |
| **Dump ledger and staging state** | Per client, in `<config>/dumps.txt` and `<staging>/.magvault/synced` | Pull-back should update staged magcards; the ledger does not need a project. No gap. |

### 4.2 Risks that would block a Phase 2 start

| # | Risk | Why it matters | Mitigation before Phase 2 code |
|---|---|---|---|
| R1 | **Temp mags encoded or merged.** With Phase 1 behaviour, a temp mag would be queued for 320p, and merged into a same-name open project, the moment it is uploaded. | Breaks the core temp rule (not in the archive until approved). It is hard to undo once jobs and project rows are written. | Decide P6 and P11 in PLAN; schema 3 before the worker runs. |
| R2 | **One copy after autoclean, and after Clean.** Staging is cleaned only after server + set. But Clean of a synced set is still allowed when the server holds the only other copy, and there are no zmags until Phase 4. | Phase 2 adds no copy; the server stays the single archive copy for mags whose set was cleaned. | Accept and state it, or block Clean unless staging (or a later zmag) also holds the mags. Owner decision (audit §8.1, still open). |
| R3 | **Wrong camera on unverified profiles.** The A7S III profile is a stand-in. The MPEG `MVI_DATA` and Panasonic MOV readers have not been run on real bytes. | Wrong labels flow into Phase 2 encode output names and the Library. | Not a blocker for the encoder. Keep `--yes` refusing when there is no guess. Collect A7S III / VG20 / CX350 reference cards. |
| R4 | **The TUI sudo path is untested.** Phase 2 adds no new sudo, but the Phase 1 gate requires it. | A field operator's first real Prepare from the TUI is the first time it runs. | Hardware retest (§2.3). |
| R5 | **Key model.** Producer rights hang on a key label in a hand-edited `authorized_keys` line, and anyone with shell access as `magvault` can change it. | A wrong producer flag lets a field key create and edit projects. | Key manager first in Phase 2, with the managed file and `AuthorizedKeysCommand`. Producer actions are checked by `serve`, never only by the client. |
| R6 | **Version skew.** New gate verbs: an old client against a new gate is fine, but a new client against an old gate treats unknown verbs as refusals. | Field clients updated before courage would lose the Job card and pull-back silently. | Keep the rule "update the server first". Clients treat a refused verb as "server too old", stated on screen. |
| R7 | **Missing ffprobe on the server.** Requirements and the 320p worker both depend on it. | Silent empty proxies, or no requirement check. | Phase 2 worker refuses to start without ffmpeg/ffprobe; telemetry shows it. |

## 5. Phase 2 start order

When the plan fixes in §6 are done and Phase 1 has closed:

1. **Groundwork.**
   - Introduce the PLAN module shape (name, keys, draw, on_key, cli) and move Ingest into
     it.
   - Write schema 3:
     - project kind and approval fields;
     - the assignment history;
     - the job hold state (or "no job until assigned");
     - a keys table (label, fingerprint, producer, revoked, created).

     The migration from 2 is tested like 1 → 2.
   - Install a systemd unit for `serve`.
2. **Key manager (TUI + CLI).**
   - Generate, accept, revoke, producer flag.
   - A Magvault-managed key file served by `AuthorizedKeysCommand`, replacing hand-written
     lines.
   - The gate gets the label from it.

   This first, because producer rights and the Job card depend on it (R5).
3. **Temp projects and the Project manager.**
   - Client `n` → `(T)` project.
   - `serve` keeps temp projects separate (no name merge) and holds their encode.
   - Project manager (server TUI + CLI): create a real project, edit requirements, close /
     reopen, approve a temp project, assign its mags, view the project log.
4. **Assignment pull-back.**
   - Gate verb `assignments`.
   - The client rewrites staged magcards, and set magcards (and the list line, per P12)
     when the set is mounted.
   - "assigned, drives not updated" until then.
   - Project fields only; previous card backed up.
5. **Job card and producer requests.**
   - Client Job card (read-only).
   - Producer create / edit requirements via spool requests that `serve` checks against
     the key's producer flag.
   - Logged with the key label.
6. **320p worker and Encoder module.**
   - GPU encoder per P7.
   - Picks up Phase 0/1 queued jobs; never runs held / temp jobs.
   - Proxy rules enforced: exact frame rate, aspect kept, no downmix.
   - Encoder queue screen and ad-hoc jobs to `custom_encode/`.
7. **Telemetry and dashboards.**
   - Telemetry writer (queue depth, disk free, serve heartbeat, active job).
   - Server dashboard.
   - Client queue depth via a gate verb.
   - Library stub.

Steps 6 and 7 could start in parallel with 3–5 once schema 3 is in. The worker only needs
the hold rule from step 1.

## 6. Plan fixes required before coding

**Decisions the owner must take:**
1. **P3:** who may create a real project, and from where:
   - the Project manager (admin), plus a producer key from the client (Job card)?
   - client CLI `--project NEW`: temp or refused?
   - remove `--require-*` on client ingest, or keep it for temp projects?
2. **P4:** whether a temp project requires FPS.
3. **P5:** who edits requirements, and whether a change re-checks old mags or only new
   ingests.
4. **P6:** no 320p job for a temp mag until assignment (or a `held` job). Fix the three
   "always queued" lines.
5. **P7:** which GPU encoder, how the worker detects it, and what happens with no GPU.
6. **P11:** temp projects are never merged by name. Decide whether the merge stays for
   anything else.
7. **R2:** whether Clean of a set may leave the server as the only copy until zmags exist.

**PLAN.md edits:**
8. **P1:** Key manager is a TUI in Phase 2 (with a CLI). Remove "Thin CLI is enough" and
   trim the Phase 5 Key module to what is left.
9. **P2:** remove "Offline custom-project behaviour still needs a design pass" (the design
   is the Offline temp project section).
10. **P8:** add a Project manager section: screen, keys, CLI, roles, temp approval and
    assign, project log view.
11. **P9:** dashboard letters for Encoder, Project manager, Key manager, and the client
    Job card.
12. **P10:** what telemetry contains in Phase 2, and the gate verb for client queue depth.
13. **P12:** add a project column to `magvault-backup.txt` (format bump), or drop the
    sentence.
14. **P13:** the pull-back rewrite changes project keys only, never the manifest; settle
    the Conflict handling vs threat-model wording (audit C2).
15. **P14:** remove the stale lines, and fix docs/test-checklist.md (client user).

When these are in PLAN.md, the Phase 2 work order in §5 can be written as a task list
without further owner questions.
