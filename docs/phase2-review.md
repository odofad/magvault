# Magvault — Phase 2 plan review (PLAN.md at `4888830`)

Date: 2026-11-28. Checked:
- PLAN.md at `4888830` ("Align Phase 2 plan with locked decisions"), whole file;
- docs/phase1.md.

Seven locked rules were checked; each item below is a contradiction or a gap. Line numbers
are PLAN.md at `4888830` unless docs/phase1.md is named. No code was changed.

## Verdict: FAIL (close)

The new Phase 2 sections are consistent with each other:
- Phase 2 decisions;
- Phase 2 roadmap;
- Temp project names;
- Offline temp project;
- Project manager;
- Clean and copies.

Older sections still say the opposite on three locked rules: real projects only in the
Project manager, no encode until assignment, and GPU with CPU fallback. Two of these also
leave an open question that changes code (B2, B4). Six blocking edits (B1–B6) make it a
pass; none needs a new design.

## Locked rules

| # | Rule | Result | Why |
|---|---|---|---|
| 1 | `n` always creates a temp project, FPS required | **Fail** | B1, B6 |
| 2 | No encode until assignment | **Fail** | B2, B3 |
| 3 | No auto-merge; the client refuses a reused name; the producer merges or renames | **Pass** in PLAN.md | docs/phase1.md still describes the merge (D1). One wording gap (N1). |
| 4 | Real projects only in the Project manager | **Fail** | B1, B4 |
| 5 | Key manager is a TUI | **Pass** | Phase 5 still lists a Key module (N4) |
| 6 | GPU with CPU fallback | **Fail** | B5 |
| 7 | Clean: field does not manage copies; server ZFS is the defence | **Pass** | Staging autoclean and dump colours still count copies on the field side (N2) |

## Blocking contradictions (fix before Phase 2 code)

**B1. Project creation outside the Project manager.**
Four places still let other paths create real projects:
- **L244 (Job card):** "A producer key can create a project and set requirements from the
  client. The server admin can also create projects."
- **L503 (Producer key):** "It only allows production actions from the client: create a
  project, set its requirements".
- **L274 (Project requirements, Phase 1 implementation):** requirements are "Set when the
  project is created: `--require-*` on `ingest` (server or client), or the New project popup
  in the TUI."
- **Server `ingest --project NEW` (Phase 0 CLI):** this still creates a real project.
  PLAN never says what it does now.

Rule 4 says real projects are created only in the Project manager. Fix all four, and say
what `ingest --project <new name>` does:
- on the client: a temp project, with `--require-fps` mandatory;
- on the server: temp, or refused.

**B2. 320p still queued at ingest.**
- **L874 (Encoder module):** "successful ingest queues a 320p job on the server."
- **L900 (Proxies):** "320p generation is queued on the server after ingest commit".

Both contradict L1315: "A 320p job is created only when a mag is assigned to a real
project." Rewrite both lines.

**B3. When a mag ingested into a real project gets its job.**
Rule 2 and L1315 say "only when a mag is assigned". Temp project names (L1285) still allows
a mag to go straight into an open real project when the operator selects it. Say that
ingest into a real project counts as assignment, so the job is created at commit.

Also say what happens to the 320p jobs already queued by Phase 0/1. All of them belong to
real projects. The old roadmap line "picked up" was removed, and nothing replaces it.

**B4. "Server-only. Admin or producer key." (L1304) cannot work as written.**
A producer key is a client SSH key. It is locked to the forced-command gate as
`magvaultclient`, so it cannot open a server TUI. Choose one:
- the Project manager is reachable from the client for producer keys, through the gate and
  checked by `serve`;
- producer actions are a separate client screen (the Job card) that sends requests;
- producers use the server machine as admin.

L244 and L503 must then match the choice.

**B5. The GPU / CPU encoder rule is not expressible in the profile.**
- **L362:** `codec: h264_nvenc   # or libx264 if encoder: cpu` is a comment, not a rule. One
  `.enc` file has one codec.
- **L363–365:** `rate_control: crf`, `crf: 23` and `preset: veryfast` are libx264 options.
  NVENC uses `cq` / `p1`–`p7`, so the example is invalid on the GPU path.

L1314 puts `encoder: gpu|cpu` "in config" but does not name the file (server
`magvault.conf`). It also does not say what `gpu` does when no GPU is found.

Choose one of these, and give an example that ffmpeg accepts on both paths:
- separate GPU and CPU fields in one profile (`codec_gpu` / `codec_cpu` with their own
  quality keys);
- two profiles, chosen by the setting.

Also name the GPU family on courage (NVENC, VAAPI or AMF). The test checklist only says
"GPU encoder/decoder".

**B6. "FPS required" vs "All are optional".**
L260 says "A project can set any combination. All are optional. Empty means no check."
Two other places say FPS is required:
- L1317: temp projects;
- L1306: Project manager real projects.

The Job card (L245) still has "If a project has no requirements, the page says so". That
can now happen only for projects from Phase 0/1. Reword L260: FPS required for new
projects; resolution and GOP optional; older projects may have none.

## Non-blocking contradictions and stale lines

| # | Where | Issue |
|---|---|---|
| N1 | L1283, L1317 | "If the name already exists on the client, the client refuses": does "on the client" include the server's open projects cached on the client (`projects.cache`), or only temps this client made? The difference decides whether a temp may share a real project's name, which is the case the producer merge exists for. |
| N2 | L606–610, L617, L631 | **Staging autoclean and dump colours count copies on the field side.** Staging is kept until server + field set (two backups); green needs "two copies". With rule 7 the field does not manage archive copies, so confirm one of these:<ul><li>keep as is (staging is a convenience buffer, not archive management);</li><li>simplify to "clean staging after server confirm" and "green = on the server".</li></ul>After a set is cleaned, a docked card turns yellow ("1 copy"). That is consistent with today's text but may confuse the crew. |
| N3 | L803 | Ingest key `n`: "New project (then its requirements from the fixed lists)". Should say "New temp project (name + FPS required; `(T)`)". |
| N4 | L1349, Key module section (L1059) | Phase 5 still lists "Key module (admin, generate or accept, USB export, discard on leave)", but Phase 2 has the Key manager TUI. Merge the two: the Phase 2 Key manager is the Key module. Trim Phase 5 to what is left, if anything. |
| N5 | L759, L1102 vs L1316 | Telemetry: the dashboard and Telemetry sections require ZFS health of the working datasets and zmags; Phase 2 day one is queue depth, disk free and serve heartbeat. Say ZFS health comes in Phase 4 (or Phase 2 for the working datasets), and add the gate verb for the client's queue depth (L768 "simple queue depth number"). |
| N6 | L763, L768, L795 | Dashboard menus and letters have no Encoder, Project manager, Key manager or client Job card entry. |
| N7 | L1273 | Pull-back rewrites "the project line in `magvault-backup.txt`". That file has no project line (`mag_folder \| state \| updated`). Add a column (format bump) or drop the clause. |
| N8 | L558 vs L1186, L1269 | Conflict handling: the database wins for "project assignment, manifest…", but the threat model says the manifest is never auto-resolved. The pull-back rewrite should change project keys only. Say so there. |
| N9 | L116–123 (schema) | The projects table has no temp kind or approval fields, and status is open / closed only. The Project manager also needs merge history and who approved. Add the fields before schema 3 is written. |
| N10 | L108 vs L1284, L1308 | "Merge of mags is forbidden" sits next to "merge a temp into an existing project". The second is a project merge (mags move between projects, folders unchanged). One clause avoids a misread. |
| N11 | L130, L134, L419, L623, L1242, L1352 | Leftovers from the Phase 0/1 audit:<ul><li>camera "or free text";</li><li>status "archived";</li><li>log levels without DEBUG;</li><li>F1 "In this batch";</li><li>master/slave "or drop the distinction";</li><li>Phase 5 "Sudo helper" (built in Phase 1).</li></ul> |

## docs/phase1.md (describes Phase 1 code that Phase 2 replaces)

These are correct for the Phase 1 code but contradict the Phase 2 rules. Add a one-line
note at the top: Phase 2 replaces project creation and the name merge. Or update them when
Phase 2 lands.

| # | Where | Issue |
|---|---|---|
| D1 | docs/phase1.md L402–410 ("Projects on a client"), L492 (tests: "offline projects merged by name") | The server reuses the one open project with the same name: an auto-merge, against rule 3. |
| D2 | docs/phase1.md L48, L83, L274–275 | `ingest --project NEW --require-*` and the TUI `n` create a real project with requirements. Against rules 1 and 4. |
| D3 | docs/phase1.md L334, L384–386 | Every commit queues a 320p job. Against rule 2 (fine for Phase 1, which has no worker). |

## To pass

**Edit PLAN.md:**
1. **B1, B4:** say where real projects are made, and how a producer reaches the Project
   manager. Fix L244, L274, L503. Say what `ingest --project <new>` does on the client and
   on the server.
2. **B2, B3:** fix L874 and L900. Say that ingest into a real project is an assignment.
   Say what happens to the Phase 0/1 queued jobs.
3. **B5:** choose how a profile names its GPU and CPU codecs. Fix the example and name the
   config file. Say what happens when `encoder: gpu` finds no GPU.
4. **B6:** reword L260 for FPS required.

**Then:**
5. Put the Phase 2 note on docs/phase1.md (D1–D3).
6. The N items can go in the same edit, or as the code reaches them. N1, N7 and N9 are
   needed before the temp-project and pull-back code.
