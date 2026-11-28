# Magvault — Phase 2 plan review (PLAN.md at `58e2fa3`)

Date: 2026-11-28. Fourth review. Checked:
- PLAN.md at `58e2fa3` ("Phase 2: AMD GPU profiles, producer spool, temp name resolution,
  green = on server"), whole file, against the seven locked Phase 2 rules and the four
  confirmations asked for;
- the encoder profile parser (`src/profiles.rs`), for what it accepts today.

The third review (at `e524900`) left blockers Y1–Y3. Line numbers are PLAN.md at
`58e2fa3`. No code or PLAN changes were made.

## Verdict: FAIL (three edits left in two sections)

Y2 and Y3 are fixed. The producer spool is stated well.

What remains is text the new edits did not reach:
- **Encoder backend section (L1322–1326):** still describes NVIDIA, not AMD VAAPI.
- **GPU example:** uses `rate_control: cq`, which the profile rules do not allow and the
  parser refuses.
- **Card dump status:** the colour bullets (L624–626) still define green as "two copies",
  directly above the new sentence that says green means "on the server".

All three are text edits; no design is open.

## Locked rules

| # | Rule | Result | Notes |
|---|---|---|---|
| 1 | `n` always creates a temp project. FPS is required. | **Pass** | L260, L274, L812, L1339, and `--project` resolution at L514. |
| 2 | No encode until the mag is assigned to a real project | **Pass** | Covered at L590, L883, L909, L1260, L1337 and L1345. |
| 3 | No auto-merge. Client refuses a reused name. Producer merges or renames on the server. | **Pass** | L1290–1294 and L514. `n` refuses an existing name; `--project` attaches to a known temp project; duplicate temp names from two offline clients are merged by the producer; the server lists open temp projects to clients. |
| 4 | Real projects are created only in the Project manager on the server | **Pass** | L244, L509, L514 ("It never creates a real project"), L1313 and L1340. |
| 5 | Key manager is a TUI | **Pass** | L1341 and L1350. "Key module" naming is left at L508, L1068, L1073, L1085 and L1371 (N4). |
| 6 | GPU with CPU fallback | **Fail** | Z1, Z2. The decision line (L1336) is right. The Encoder backend section and the GPU example contradict it. |
| 7 | Clean: field does not manage copies. Server ZFS is the defence. | **Pass** | Clean and copies (L699) and L1342. Staging is now called a convenience buffer (L628). |

## Confirmations

| Check | Result | Notes |
|---|---|---|
| Producer account named, group-readable only, changes through a spool that serve applies | **Pass** | L509: `magvault-producer` can read the database but not write it. Create, merge, rename and approve go through a producer spool that `serve` checks and applies. `magvault ui` as that account shows only the Project manager and a read-only dashboard. L244 now names the login. One gap: N14 (edit requirements is not in the spool list). |
| Encoder profiles: `default-320p` CPU (libx264); `default-320p-gpu` AMD VAAPI; `encoder:` in `magvault.conf`; GPU failure reruns once on CPU | **Fail** (Z1, Z2) | Correct in the decisions (L1336) and the CPU example (L355–366). L1322–1326 still say NVENC. The GPU example's `rate_control: cq` breaks the profile rules (L406–410). |
| `ingest --project` resolves open real → known temp → new temp; never creates a real project | **Pass** | L514, word for word. A new temp needs `--require-fps`. |
| Green means on the server; no project line in `magvault-backup.txt` | **Fail** (Z3) on green; **Pass** on the project line | <ul><li>The project line is removed from the schema notes. Pull-back compares magcards with the cached server mapping and rewrites only those that differ, project keys only, old card backed up (L1282).</li><li>The paragraph at L628 says green means on the server, but the bullets above it (L624–626) still say "two copies".</li></ul> |

## Remaining blockers

**Z1. The Encoder backend section still describes NVIDIA.**
L1322–1326:
- "`gpu` uses NVENC when a GPU is present"
- "NVENC uses its own rate control"
- "Courage: confirm GPU model before Phase 2 coding. The profile file stores one codec;
  the server picks the matching profile."

These contradict L1336:
- GPU is AMD `h264_vaapi`;
- two named settings (`default_profile_320p`, `default_profile_320p_gpu`);
- the per-job rerun on CPU.

L1325 also gives only the "no GPU found" fallback, not "job fails on the GPU encoder → rerun
once on CPU".

**Fix:** rewrite the section to match L1336 (VAAPI, the two settings, both fallbacks), or
delete it and point to the decision line. The GPU model is now known (AMD), so the
"confirm" sentence can go.

**Z2. The GPU profile's rate control is outside the profile schema.**
- **Not allowed by the rules.** The GPU example (L368–373) uses `rate_control: cq` and
  `cq: 23`. The profile rules (L406–410) allow `rate_control` with "either `crf` or
  `bitrate`". `src/profiles.rs` accepts only those two and refuses the GPU profile as
  written ("rate_control must be crf or bitrate").
- **No such ffmpeg option.** `h264_vaapi` has no `cq` option. Its constant-quality mode is
  `-rc_mode CQP` with `-qp N`; `-rc_mode ICQ` with `-global_quality N` also exists.

**Fix:**
- add one value to the `.enc` vocabulary, e.g. `rate_control: qp` with `qp: <n>` (mapped to
  `-rc_mode CQP -qp N` for VAAPI);
- add it to "Required minimum";
- use it in the GPU example.

Optional, but it helps whoever writes the worker:
- show the GPU example as its own file block, with `type`, `height` and audio keys, which
  "Required minimum" asks for. Today it is a commented-out stub inside the CPU block.
- note that VAAPI needs a render device (`/dev/dri/renderD128`, the `magvault` user in the
  `render` group) and hardware scaling (`scale_vaapi` after `hwupload`).

**Z3. The dump colour bullets contradict green = on the server.**
L624–626 still read:
- Yellow: "dumped, but not yet two copies and uploaded to the server";
- Green: "two copies exist and the mag is uploaded to the server".

L628 says: green means on the server; yellow means dumped here but not yet on the server.

**Fix:** reword the two bullets to match L628. The code still shows green for "two
copies, one on the server" (Phase 1). That changes in Phase 2 with the new wording.

## Non-blocking items still open

| # | Item | Where | State |
|---|---|---|---|
| N14 | Producer spool actions | L509 vs L1318 | The spool covers create, merge, rename and approve. The Project manager also lists "Edit requirements" (and the open / closed rule allows close and reopen), and those writes are not in the spool list. Add them, or say they are admin-only. |
| N15 | Read-only database access | L509 | A group-readable SQLite file can be read without write rights only in rollback-journal mode, or when opened read-only / immutable. WAL mode would need write access to the `-shm` file. Say so, or keep the database in rollback-journal mode (as today). |
| N16 | CPU example CRF | L365 | The example now sets `crf: 28`; the shipped `share/encoders/default-320p.enc` has `crf: 23`. Pick one. |
| N17 | User model list | L503–504 | `magvault-producer` is described under the Producer key but not listed with the server and client users. |
| N9 | Old schema section | L116–123 | Still says requirements are "optional" and lists "format". The Phase 2 notes (L1330–1332) override it; align it when schema 3 is written. |
| N6 | Dashboard letters | L772, L777, L804 | No entry or letter for Encoder, Project manager, Key manager or the client Job card (suggested: `t`, `m`, `k`, `j`). |
| N4 | Key module / Key manager naming | L508, L1068, L1073, L1085, L1371 | Use one name; trim the Phase 5 entry. |
| N5 | Telemetry scope | L1111 vs L1338 | ZFS health is required in the dashboard sections, while day one is queue depth, disk free and serve heartbeat. The client queue-depth gate verb is unnamed. |
| N8 | Conflict handling vs threat model | L567 vs L1195 | The pull-back rule (L1282, project keys only) settles it for pull-back; L567 still lists "manifest" under "database wins". |
| N10 | Mag merge vs project merge | L108 vs L1317 | One clause would avoid a misread. |
| N2 | Staging autoclean | L615, L640 | Consistent now that staging is a "convenience buffer" (L628). No action needed unless you want the two-backup rule simplified. |
| N11 | Audit leftovers | L130, L134, L426, L632, L1251, L1374 | <ul><li>camera "free text";</li><li>"archived";</li><li>no DEBUG level;</li><li>F1 "In this batch";</li><li>master/slave "or drop the distinction";</li><li>Phase 5 "Sudo helper".</li></ul> |
| — | docs/test-checklist.md | L14, L38, L45 | Still lists a `magvaultclient` user on the client. |

## To pass

1. **Z1:** rewrite L1322–1326 for AMD VAAPI, the two profile settings and both CPU
   fallbacks (or remove the section and keep L1336).
2. **Z2:** add `rate_control: qp` + `qp: <n>` (VAAPI CQP) to the profile vocabulary and
   "Required minimum", and use it in the GPU example.
3. **Z3:** reword the yellow and green bullets at L625–626 to "not yet on the server" /
   "on the server".

**Before the code that needs them:** N14 and N15 before the Project manager; N9 with
schema 3. The rest as the code reaches them.
