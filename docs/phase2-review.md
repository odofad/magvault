# Magvault — Phase 2 plan review (PLAN.md at `e524900`)

Date: 2026-11-28. Third review. Checked:
- PLAN.md at `e524900` ("Close Phase 2 review X1-X3"), whole file, against the seven
  locked Phase 2 rules and the four confirmations asked for;
- the shipped encoder profiles (`share/encoders/`).

The second review (at `79d8b60`) left blockers X1–X3. Line numbers are PLAN.md at
`e524900`. No code or PLAN changes were made.

## Verdict: FAIL (three small blockers)

X1 and X3 are fixed. X2 is half fixed:
- `encoder:` is now in `magvault.conf` and "two profiles" is stated;
- but the example profile breaks the rule it sits next to, and nothing says how the
  server tells the GPU profile from the CPU one.

Two new gaps:
- the `ingest --project` wording has no case for a temp project that already exists;
- the new `project=` line in `magvault-backup.txt` cannot describe a set that holds mags
  from several projects.

Each is a short PLAN edit.

## Locked rules

| # | Rule | Result | Notes |
|---|---|---|---|
| 1 | `n` always creates a temp project. FPS is required. | **Pass** | L260, L274, L805, L1323, L1333, and `ingest --project` at L507. |
| 2 | No encode until the mag is assigned to a real project | **Pass** | Covered at L583, L876, L902, L1253, L1331 and L1339. Ingest into a real project counts as assignment (L902). |
| 3 | No auto-merge. Client refuses a reused name. Producer merges or renames on the server. | **Pass** | L1283–1287 and L1333. The CLI case of an existing temp is a gap, not a contradiction (Y2). |
| 4 | Real projects are created only in the Project manager on the server | **Pass** | L244, L274, L497, L503, L507 ("It never creates a real project"), L1306 and L1334. Real-project creation is logged with the server account (L506). |
| 5 | Key manager is a TUI | **Pass** | L1335 and L1344. "Key module" naming is left at L501, L1066, L1078 and L1365 (N4). |
| 6 | GPU with CPU fallback | **Fail** | Y1. The backend setting and fallback are stated correctly (L1316–1318, L1330). The example profile and the selection rule are not. |
| 7 | Clean: field does not manage copies. Server ZFS is the defence. | **Pass** | L692–695 and L1336. The staging and green wording still counts copies on the field side (N2). |

## Confirmations

| Check | Result | Notes |
|---|---|---|
| Producer SSH account named, distinct from admin and gate | **Pass** | L502: user `magvault-producer` (or equivalent). It runs `magvault ui` and writes the database. It is not the admin `magvault` user and not a gate key; the security review treats it as a limited server login. Two follow-ups are in the non-blocking table: N12 (L244 still says "or an admin session") and N13 (what the account sees and how it writes the database). |
| `encoder:` in `magvault.conf`; two profiles; server picks by setting | **Fail** (Y1) | `magvault.conf`: yes (L1316, "Not in path.conf"). Two profiles: stated (L1330). Picks by setting: stated, but there is no rule for which profile is the GPU one, and the example contradicts L1330. |
| `ingest --project` never creates a real project | **Pass**, with a gap (Y2) | L507 says it in so many words. The case of a name that is an existing temp project is missing. |
| Phase 2 schema notes present (kind, approval, requirements, backup project line) | **Fail** (Y3) | `projects.kind` (L1323), `approved_at` / `approved_by` (L1324) and requirements "FPS required … GOP not format" (L1325) are present and fine. The backup project line (L1326) is present but has the wrong shape. |

## Remaining blockers

**Y1. The GPU example breaks the two-profile rule, and the selection rule is missing.**
- **The example contradicts L1330.** The example profile (L355–377) is now the GPU
  profile: `codec: h264_nvenc` (L362). But it still sets `rate_control: crf`, `crf: 23`
  and `preset: veryfast` (L364–366). L1330 says the NVENC profile has "no CRF/preset", and
  L1318 says CRF and preset apply only to libx264.
- **No way to tell the profiles apart.** "The server picks the profile that matches the
  setting" (L1319, L1330), but nothing marks a profile as GPU or CPU. The server's
  `magvault.conf` has a single `default_profile_320p` (used by Phase 0/1 code).
- **Shipped file conflict.** The shipped `share/encoders/default-320p.enc` is the libx264
  profile with the same id `default-320p` as the PLAN's NVENC example.

**Fix:**
- Give two examples: `default-320p` (libx264, `crf`, `preset`) and `default-320p-gpu`
  (`h264_nvenc` with its own rate control, for example `rate_control: cq` and `cq: 23`).
- Add one selection rule, for example `default_profile_320p` (cpu) and
  `default_profile_320p_gpu` in `magvault.conf`, or a `backend: gpu|cpu` key in each
  `.enc`.

**Y2. `ingest --project <existing temp name>` has no rule.**
L507 has two branches:
- an open real project: assign;
- an unknown name: create a temp project, and refuse if the name is in use on the client.

A name that is an existing temp project falls in neither branch. That is the usual offline
case: the second and third card of the same temp shoot. Read as written, the CLI would
refuse those cards. The TUI side is not stated either; can the operator pick an existing
temp project in 3 Project?

**Fix:** add one line. A name that matches a temp project this client knows (one it
created, or one the server lists) attaches the mag to that temp project; only creating a
new temp with a name already in use is refused. This also settles N1 (what "in use on the
client" covers).

**Y3. One `project=` line cannot describe a field set.**
L1326: "`magvault-backup.txt` gains a `project=` line so pull-back can rewrite it."
- **Several projects per set.** A field set holds many mags, often from different
  projects: a temp project and a real one, several shoot days. Pull-back assigns one mag
  at a time (L1269–1276), so one line per file cannot record it.
- **Wrong format.** The file uses a `key: value` header and `# mags` lines
  `mag_folder | state | updated`; a `project=` line does not match either.

**Fix:** add a project column to each `# mags` line (`mag_folder | state | updated |
project_uuid`), with a format bump (`magvault-backup/2`); readers accept the old
three-column lines. Or drop the clause and rewrite only the magcards on the set.

## Non-blocking items still open

| # | Item | Where | State |
|---|---|---|---|
| N12 | "admin session" | L244 | The Job card says the producer uses "the server console (or an admin session)", but L502 says the producer account is not the admin. Change it to "the `magvault-producer` login". |
| N13 | Producer account scope | L502 | The account "can run `magvault ui` and write the database". Not stated:<ul><li>which modules it sees (Project manager only? not Key manager, Profile Editor, Zmag, Encoder controls);</li><li>how it writes a database owned by `magvault` (group-writable db file and folder, which SQLite needs for its journal).</li></ul>Needed before the Project manager code and for the security review. |
| N1 | Name-exists scope | L1285, L1333 | Settled once Y2 is written. |
| N9 | Old schema section | L116–123 | Still says requirements are "optional" and lists "format". The Phase 2 notes (L1322–1326) override it; align it when schema 3 is written. |
| N2 | Staging and green wording | L608, L619, L631 | Staging is kept until "two backups"; green means "two copies". Confirm (buffer convenience) or simplify, given rule 7. |
| N6 | Dashboard letters | L765, L770, L797 | No entry or letter for Encoder, Project manager, Key manager or the client Job card. |
| N4 | Key module / Key manager naming | L501, L1066, L1078, L1365 | Use one name; trim the Phase 5 entry. |
| N5 | Telemetry scope | L1104 vs L1332 | ZFS health is required in the dashboard sections, while day one is queue depth, disk free and serve heartbeat. The client queue-depth gate verb is unnamed. |
| N8 | Conflict handling vs threat model | L560 vs L1188 | The pull-back rewrite changes project keys only, never the manifest. |
| N10 | Mag merge vs project merge | L108 vs L1286 | One clause would avoid a misread. |
| N11 | Audit leftovers | L130, L134, L419, L625, L1244, L1368 | <ul><li>camera "free text";</li><li>"archived";</li><li>no DEBUG level;</li><li>F1 "In this batch";</li><li>master/slave "or drop the distinction";</li><li>Phase 5 "Sudo helper".</li></ul> |
| — | docs/test-checklist.md | L14, L38, L45 | Still lists a `magvaultclient` user on the client. |

## To pass

1. **Y1:**
   - two valid example profiles (libx264 with CRF and preset; NVENC with its own rate
     control);
   - one rule for how the server picks between them.
2. **Y2:** `--project <existing temp>` (and the TUI pick) attaches to that temp project;
   only a new temp with a used name is refused.
3. **Y3:** a per-mag project column in `magvault-backup.txt` (format bump), or no project
   in that file.

**Before the code that needs them:** N13 before the Project manager and the security
review; N9 when schema 3 is written. The rest as the code reaches them.
