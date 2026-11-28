# Magvault — Phase 2 plan review (PLAN.md at `79d8b60`)

Date: 2026-11-28. Second review. Checked:
- PLAN.md at `79d8b60` ("Lock producer access: SSH to server for Project manager"), whole
  file, against the seven locked Phase 2 rules and the producer SSH path;
- docs/phase1.md and docs/test-checklist.md for leftovers.

The first review (at `4888830`) listed blockers B1–B6. Since then, `1110f8a` and `79d8b60`
edited PLAN.md and added the Phase 2 note to docs/phase1.md. Line numbers are PLAN.md at
`79d8b60`. No code or PLAN changes were made.

## Verdict: FAIL (three wording blockers)

- B2, B3 and B6 are fixed. B4 is fixed by the new producer SSH rule.
- B1 and B5 are mostly fixed. One question is left in each, and both change code:
  - X3: the CLI with a new project name;
  - X2: how the server picks the GPU or the CPU profile.
- The new producer path adds one question: X1, which Unix account the producer uses.

All three are short PLAN edits; none needs a new design.

## Locked rules

| # | Rule | Result | Notes |
|---|---|---|---|
| 1 | `n` always creates a temp project. FPS is required. | **Pass** | Stated in four places: L260, L274, L803 and L1324. The schema line L122 still says "optional" (N9). |
| 2 | No encode until the mag is assigned to a real project | **Pass** | Covered at ingest step 8 (L581), the Encoder module (L874), Proxies (L900), the Phase 0 roadmap (L1251), the decisions (L1322) and the roadmap (L1330). L900 says that ingest into a real project counts as assignment, and Phase 0/1 queued jobs stay queued. |
| 3 | No auto-merge. Client refuses a reused name. Producer merges or renames on the server. | **Pass** | Temp project names (L1279–1285) and the decisions (L1324). The scope of "exists on the client" is still open (N1). |
| 4 | Real projects are created only in the Project manager on the server | **Fail** | X3: PLAN is silent on `ingest --project <new name>`, and Phase 0 server ingest creates a real project today. L505 is stale ("Creating a project is logged with that label"). |
| 5 | Key manager is a TUI | **Pass** | Decisions L1326 and roadmap L1335. Naming leftovers: "Key module" at L501, L1064, L1356 (N4). |
| 6 | GPU with CPU fallback | **Fail** | X2: profile selection is not consistent, and `encoder:` is put in `path.conf`. |
| 7 | Clean: field does not manage copies. Server ZFS is the defence. | **Pass** | Clean and copies (L690–693) and the decisions (L1327). Staging autoclean and green wording still count copies on the field side (N2). |

## Also confirmed

**Producer SSH path: stated clearly, no conflict with the gate.**
- **L1304:** "normal SSH login to the server (separate from the client gate key), then runs
  `magvault ui`".
- **L244 and L503:** the producer key cannot create from the client; real project work is
  by SSH to the server.
- **L1325:** the same, plus "the client stays thin".

The gate is untouched: the producer's server login is a different key and account from
the forced-command key. The client producer key only edits the Job card.

**Two follow-ups:**
- **X1 (blocker):** which account the producer logs in as (below).
- **Job card edits (L244, L496, L503):** they need a write path from the client. The gate
  runs as `magvaultclient` and cannot write the database, so an edit must be a request that
  `serve` applies after checking the key's producer flag. Say so when the Job card is
  specified; not a blocker now.

**docs/phase1.md Phase 2 note: present.** Line 1 says Phase 2 replaces the name merge,
`--project NEW` creating a real project, and the 320p job on every commit.

## Remaining blockers

**X1. Which Unix account the producer uses on the server.**
L1304 says "a normal SSH login", and L244 says "the server console (or an admin session)".
The only server account in the User model (L496) is `magvault`, the admin: Profile Editor,
all admin modules, the sudo helper rule. If producers log in as `magvault`, the Project
manager cannot tell a producer from the admin.

The database belongs to `magvault`, so a separate producer account also needs a way to
write it. Choose one:
- producers use the admin account (then say producer = admin on the server);
- producers have their own accounts in a `magvault-producer` group, with the Project
  manager limited to project actions, and database access through that group or through
  `serve`.

The security review at the phase gate needs this answer.

**X2. GPU / CPU profile selection.**
1. **L362 vs L1317:**
   - L362 keeps one profile (`default-320p`) with `codec: h264_nvenc` and the comment
     "libx264 when encoder: cpu".
   - L1317 says "The profile file stores one codec; the server picks the matching
     profile".

   These two describe different designs: one profile with a swapped codec, or two
   profiles.
2. **Invalid example:** the example still sets `rate_control: crf`, `crf: 23` and
   `preset: veryfast` with `h264_nvenc`. L1316 itself says CRF and preset apply only to
   libx264, so the example is invalid as written.
3. **How the server picks:** the server's `magvault.conf` has one
   `default_profile_320p: default-320p`, and PLAN does not say how a GPU and a CPU profile
   are named or chosen. Example: `default_profile_320p_gpu` / `_cpu`, or a `backend: gpu|cpu`
   key in each `.enc`.
4. **Wrong file:** L1314 puts `encoder:` in `path.conf`. The directory layout (L48–49,
   L471) says `path.conf` holds roots only and non-path settings live in `magvault.conf`.

Fix:
- give two example profiles (one NVENC with its own rate control, one libx264);
- state the selection rule;
- move `encoder:` to `magvault.conf`.

Confirming courage's GPU model (L1317) is a pre-coding task, not a plan contradiction.

**X3. CLI ingest with a new project name (B1 remainder).**
PLAN now says ingest `n` creates a temp project. It does not say what the CLI does with a
name that is not an open real project:
- client `magvault ingest --project <new>`;
- server `magvault ingest --project <new>`, the Phase 0 path, which today creates a real
  project.

"CLI and scripting" requires every TUI action to have a CLI form. Under rule 4 the answer
should be one of:
- both create a temp project, with `--require-fps` mandatory;
- the server refuses a new name.

Also fix L505 "Creating a project is logged with that label". On the client a producer key
only edits the Job card, so the log line should cover that edit.

## Non-blocking items still open

| # | Item | Where | State |
|---|---|---|---|
| N1 | Name-exists scope | L1283 "already exists on the client", L1324 "in use locally" | **Open.** Does it include the server's open projects cached on the client (`projects.cache`)? That decides whether a temp may share a real project's name, which is the case the producer merge is for. Needed before the temp-project code. |
| N7 | `magvault-backup.txt` project line | L1273 | **Open.** The file has no project line (`mag_folder \| state \| updated`). Add a column (format bump) or drop the clause. Needed before the pull-back code. |
| N9 | Schema fields | L116–123 | **Open.** No temp kind, approval or merge history, and status is still open / closed only. L122 still says requirements are "optional" and lists "resolution, frame rate, format" (should be FPS required; GOP, not format). Needed before schema 3. |
| N2 | Staging autoclean and green wording | L606–610, L617, L631 | **Open.** Staging is still kept until "two backups"; green still means "two copies". With rule 7, confirm one of these:<ul><li>keep as is (staging is a buffer, not archive management);</li><li>simplify to "clean after server confirm" and "green = on the server".</li></ul> |
| N6 | Dashboard letters | L763, L768, L795 | **Open.** No letter or menu entry for Encoder, Project manager, Key manager, or the client Job card. |
| N4 | Key module naming | L501, L1064, L1356 | **Open.** "Key module" (and Phase 5 "Key module (admin…)") vs the Phase 2 "Key manager" TUI. Use one name; trim Phase 5. |
| N5 | Telemetry scope | L759, L1102 vs L1323 | **Open.** ZFS health is required in the dashboard sections, while day one is queue depth, disk free and serve heartbeat. The client queue-depth gate verb is unnamed. |
| N8 | Conflict handling vs threat model | L558 vs L1186 | **Open.** The pull-back rewrite should change project keys only, never the manifest. |
| N10 | "Merge of mags is forbidden" next to a project merge | L108 vs L1284, L1308 | **Open.** One clause would avoid a misread. |
| N11 | Audit leftovers | L130, L134, L419, L623, L1242, L1359 | **Open:**<ul><li>camera "free text";</li><li>"archived";</li><li>no DEBUG level;</li><li>F1 "In this batch";</li><li>master/slave "or drop the distinction";</li><li>Phase 5 "Sudo helper".</li></ul> |
| — | docs/test-checklist.md | L14, L38, L45 | **Open.** Still lists a `magvaultclient` user on the client; the client runs as the operator's own account. |

## To pass

1. **X1:** name the producer's server account and how it differs from the admin `magvault`
   account, including database access.
2. **X2:**
   - two example encoder profiles (NVENC and libx264, each with valid options);
   - the rule that picks one;
   - `encoder:` moved to `magvault.conf`.
3. **X3:**
   - what `ingest --project <new name>` does on the client and on the server;
   - L505 reworded for Job card edits.

**Before the code that needs them:** N1 and N9 before the temp-project code and schema 3;
N7 before the pull-back code. The other N items can follow as the code reaches them.
