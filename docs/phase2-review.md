# Magvault — Phase 2 plan review (PLAN.md at `1afed03`)

Date: 2026-11-28. Fifth review. Checked PLAN.md at `1afed03` ("Phase 2: AMD encoder section,
green bullets, producer spool coverage") against the seven locked Phase 2 rules and the
confirmations from the last review.

The fourth review (at `58e2fa3`) left Z1–Z3. Line numbers are PLAN.md at `1afed03`. No code
or PLAN changes were made.

## Verdict: FAIL (one edit left)

Z1 and Z3 are fixed, and so are N14 and N15 (producer spool coverage and read-only
database). One blocker is left: Z2 is only half done.

- **Comment fixed:** the comment above the GPU example now says `rate_control: qp`.
- **Example not fixed:** the example lines under it still read `rate_control: cq` and
  `cq: 23`.
- **Rules not fixed:** "Required minimum" still allows only `crf` or `bitrate`.

That is three lines to change. Then this review passes.

## Locked rules

| # | Rule | Result |
|---|---|---|
| 1 | `n` always creates a temp project. FPS is required. | **Pass** |
| 2 | No encode until the mag is assigned to a real project | **Pass** |
| 3 | No auto-merge. Client refuses a reused name. Producer merges or renames on the server. | **Pass** |
| 4 | Real projects are created only in the Project manager on the server | **Pass** |
| 5 | Key manager is a TUI | **Pass** |
| 6 | GPU with CPU fallback | **Fail** (Z2 only) |
| 7 | Clean: field does not manage copies. Server ZFS is the defence. | **Pass** |

## Confirmations

| Check | Result | Notes |
|---|---|---|
| Producer account | **Pass** | L509 covers it: <ul><li>`magvault-producer`;</li><li>the database is group-readable, not writable;</li><li>rollback-journal mode, opened read-only;</li><li>every producer write goes through the spool, including edit requirements, close and reopen;</li><li>admin writes go through `serve` too.</li></ul> |
| Encoder backend | **Pass** | L1322–1328 now say: <ul><li>AMD VAAPI and the two profile settings in `magvault.conf`;</li><li>`vaapi_device`;</li><li>WARN when the render device is unusable at start;</li><li>a failed GPU job reruns once on CPU, then fails with the CPU error;</li><li>the `render` group and `vainfo`;</li><li>decode on CPU, encode on GPU (`format=nv12,hwupload,scale_vaapi`).</li></ul> No NVENC text is left. |
| GPU profile rate control | **Fail** (Z2) | See below. |
| `ingest --project` resolution | **Pass** | L514 |
| Green = on the server | **Pass** | L624–626 now match L628: red not dumped here; yellow dumped here, not yet on the server, or matching another volume; green on the server. |
| No project line in `magvault-backup.txt` | **Pass** | Pull-back rewrites the magcards on the set only (L1282). |

## Remaining blocker

**Z2. The GPU example and the profile rules still use the old vocabulary.**

- **L372–373** (GPU example): `# rate_control: cq` / `# cq: 23`. Change to
  `# rate_control: qp` / `# qp: 26`. The comment on L368 already says `qp`.
- **L409** (Required minimum): "`rate_control` + either `crf` or `bitrate`". Change to
  "`rate_control` (`crf`, `bitrate` or `qp`) + its matching value".

Until both change, a `default-320p-gpu.enc` written from the plan is refused by the
profile loader, and `cq` is not an `h264_vaapi` option.

## Non-blocking items still open

| # | Item | Where | State |
|---|---|---|---|
| N16 | CPU example CRF | L365 | The example sets `crf: 28`; the shipped `share/encoders/default-320p.enc` has `crf: 23`. Pick one; 23 matches what is already tested. |
| — | GPU example layout | L368–374 | It is still a commented stub inside the CPU block, without `type`, `height` or audio keys. A separate file block would show a complete, valid GPU profile. |
| N17 | User model list | L503–504 | `magvault-producer` is not listed with the server and client users. |
| N9 | Old schema section | L116–123 | Says requirements are "optional" and lists "format". The Phase 2 notes override it; align it with schema 3. |
| N6 | Dashboard letters | L772, L777, L804 | No entry for Encoder, Project manager, Key manager or Job card (suggested: `t`, `m`, `k`, `j`). |
| N4 | Key module / Key manager naming | L508, L1068, L1073, L1085, L1373 | Use one name; trim the Phase 5 entry. |
| N5 | Telemetry scope | L1111 vs L1340 | ZFS health is required in the dashboard sections, while day one is queue depth, disk free and heartbeat. The client queue depth is unnamed (suggest: extend the gate `status` verb). |
| N8 | "manifest" under "database wins" | L567 | Drop it; pull-back already changes project keys only. |
| N10 | Mag merge vs project merge | L108 | Add "(a project merge moves whole mags; folders unchanged)". |
| N11 | Audit leftovers | L130, L134, L426, L632, L1251, L1376 | <ul><li>camera "free text";</li><li>"archived";</li><li>no DEBUG level;</li><li>F1 "In this batch";</li><li>master/slave "or drop the distinction";</li><li>Phase 5 "Sudo helper".</li></ul> |
| — | docs/test-checklist.md | L14, L38, L45 | Still lists a `magvaultclient` user on the client. |

## To pass

Fix L372–373 (`qp`) and L409 (allow `qp` in Required minimum). The Phase 2 plan is then
consistent with all seven rules.

The Phase 1 gate is still open, and Phase 2 code should not start until it closes:
- hardware retest of `2d8f940`;
- the real TUI sudo prompt;
- the Phase 1 security review.
