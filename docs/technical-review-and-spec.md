# Magvault — Technical Review and Implementation Specification

| | |
|---|---|
| Repository | `odofad/magvault`, branch `main`, commit `ca0cf18` ("Export bins browser: lock status rules, date column, sidecars") |
| Read | `PLAN.md` (913 lines), `README.md`, `NOTES.md`, `docs/mockups/*.md` (11 files, both 150x42 and 120x36 drawings) |
| Date | 2026-11-22 |
| Status of project | Design only. No code. |

**How to read this document.** Part A is the review: module map, end-to-end flows, and findings. Part B is the specification an implementer can start from. Findings have IDs so Part B and the priority list can point at them:

- `C` contradiction (PLAN vs mockups, or PLAN vs itself)
- `M` missing behaviour
- `S` security
- `D` data integrity
- `P` performance / operations
- `T` hard to implement or test

Line numbers such as `PLAN:440` refer to `PLAN.md` at commit `ca0cf18`. Mockup references use the file name, e.g. `ingest.md:108`. Where something is ambiguous, the finding proposes the smallest change that removes the ambiguity. Nothing here adds features beyond what PLAN.md and the mockups already describe; where a mechanism is needed to make a described feature work (for example a staging directory or an SSH forced command), it is labelled **proposal** and listed again under open questions.

---

## Summary

The plan is unusually well thought through for a pre-code project: it has locked directory layouts, plain-text formats, a clear "no internal accounts" model, and mockups at two terminal sizes for every module. The weak spots are where the pieces meet:

1. **There is no defined process architecture.** PLAN never says what runs where: whether there is a server daemon, how a client "creates a request on the server", how the server "notices a newly committed mag" (PLAN:903), or how a remote zmag station "updates the central database" (PLAN:784). Every client flow depends on this. (Part B §5 proposes one.)
2. **Authority between the database and the files is stated both ways** (`C2`, `C3`, `C4`). This decides crash recovery, rebuild, revert and conflict handling, so it has to be settled before the first table is created.
3. **The SSH and sudo model as written is not safe or not workable.** `sudo zpool`/`sudo zfs` with unrestricted arguments is effectively root (`S1`); a client key with no forced command is a shell on the server with write access to `mags/` (`S3`, `S4`); the `magvault` user cannot edit `~magvaultclient/.ssh/authorized_keys` under sshd's default `StrictModes` without root (`S5`); and auto-import contradicts password-only sudo (`C12`).
4. **The mag lifecycle has no exit.** Nothing in PLAN or the mockups ever removes a mag from the server after it is on zmags, so no mag can become `cold`, yet cold pull, the Library warm/cold status and the export `cold` state all depend on it (`M1`). There is also no way to cleanly undock a zmag (`M2`).
5. **Several UI flows have no entry point**: Encoder and Key module are missing from the dashboard menu, Settings is unreachable on the client, exports cannot be created, named or have in/out points set, projects cannot be closed (`C8`, `M3`, `M4`).

None of these are expensive to fix on paper. The priority list at the end separates what must be decided before coding from what can wait.

---

# Part A — Review

## 1. Module map

Conventions in the tables: **R** = reads, **W** = writes. "DB" names tables from PLAN:87-179. External commands are those PLAN or the mockups name, plus the ones the described behaviour implies (marked *implied*).

### 1.1 Dashboard

| Aspect | Detail |
|---|---|
| Purpose | Landing screen. Status at a glance and the menu into every module (PLAN:519-537). |
| Inputs | Server: `telemetry_live` (fallback `telemetry_disk`), light checks `zfs list`, `zpool status` (PLAN:529), counts from DB. Client: short status summary over SSH (PLAN:830), local staging and USB drive scan. |
| Outputs | None (navigation only). |
| Files | R `telemetry_live`, `telemetry_disk`; client R staging dir, USB `magvault-backup.txt`. "Recent events" (dashboard.md:41) and "Recent local activity" (dashboard.md:121) have **no defined source** (telemetry or `logs/`?). |
| DB | `projects` (count `status=open`), `mags` (count `status=online`), `cold_pull_requests` (count pending). "Pending requests" is not defined (cold pulls only, or client export requests too?). |
| External | `zfs list`, `zpool status` (no sudo needed for read-only status), `ssh` (client). |
| Keys | `i` Ingest, `g` Logging, `l` Library, `e` Export bins, `z` Zmag, `p` Profile Editor, `v` Logs, `s` Settings, `r` refresh, `q` quit (PLAN:554-558). Client: `i`, `g`, `l`, `s` = **server dashboard** (dashboard.md:220). |
| Server vs client | Server: encoder queue, zmag health, working-dataset health, counts, events. Client: connection state, queue depth only, local media, local activity, "open server dashboard over SSH". |
| Admin only | Profile Editor entry hidden for non-admin (dashboard.md:228). No menu entry for Encoder or Key module at all (`C8`). |

### 1.2 Ingest

| Aspect | Detail |
|---|---|
| Purpose | Probe a card, confirm metadata and project, verified copy into a new mag folder, write `magcard.txt` + ASC MHL, insert DB rows, queue the 320p proxy (PLAN:423-447). Client variant adds staging, USB backup targets, server sync (PLAN:449-481). |
| Inputs | Mounted card (auto-detected or manual path), camera profiles, open projects, operator choices. |
| Outputs | `mags/<YYYYMMDD>_<mag-uuid>/` with card contents, `magcard.txt`, `ascmhl/`; DB rows; 320p encoder job; on client: staging copy, USB copies, `magvault-backup.txt` updates. |
| Files | R card tree; W staging (`/tmp/magvault-staging` in ingest.md:53), USB drive root and mag folders, server `mags/`; W `logs/`. |
| DB | R `projects` (open only); W `projects` (new), `mags`, `clips`, `encoder_jobs`. Camera profiles: PLAN:195 says profiles are "loaded into the database" but there is no table for them (`M13`). |
| External | `rsync` (PLAN:4-8), SHA-256 (in-process), ASC MHL writer, mount discovery via `/media`, `/run/media` or udisks (PLAN:426), `sudo` on the client for Prepare (PLAN:382, 476), `ssh`/`rsync` to the server. Guessing camera, format, gamma, gamut and shoot day implies `ffprobe` or file metadata (*implied*, not stated). |
| Keys | `n` new project, `r` rescan, `p` pull, `o` prepare USB, `x` abort, `l` log (PLAN:560-568). `Esc` blocked while copying (PLAN:545, ingest.md:198). |
| Server vs client | Server/standalone: card → `mags/` directly. Client: card → staging (or USB-as-staging, or direct to server) → server, plus optional mirrors; "Warning shown when no backup exists and server sync is not yet confirmed" (PLAN:458). |
| Admin only | None. Missing camera profile is "Admin only" to add (ingest.md:42). |

### 1.3 Logging

| Aspect | Detail |
|---|---|
| Purpose | Per-clip metadata: description, tags, rating, slow motion, technical note, shot type, camera operator, director/producer (PLAN:492-503). Proxy playback via mpv. |
| Inputs | Mags and clips, proxies (`proxies/db`, client cache, or temporary local proxy), tag/person vocabularies. |
| Outputs | DB updates; optionally the mag card or a sidecar (PLAN:485); `logging_complete` flag. |
| Files | R `proxies/db/…`, client proxy cache, `.magvault/magcard.*.backup` (revert, PLAN:516); optionally W `magcard.txt` or a sidecar. |
| DB | R/W `clips`, `clip_tags`, `mags.logging_complete`. Schema cannot hold multiple camera operators or directors (`C6`). |
| External | `mpv` via IPC (PLAN:612), `ffmpeg` for "quick temporary local proxies" (PLAN:615), `ssh`/`rsync` for proxy pulls and change sync. |
| Keys | `1`-`5` rating, `m` slow-mo, `t` tags, `]`/`[` next/prev unlogged, `c` complete, `v` revert, `Ctrl+Z` undo, `Space` play, `←/→` seek (logging.md:212-228). |
| Server vs client | Server: all online mags. Client: local staging, USB backups, recently synced mags, offline supported; header shows "3 changes pending sync" (logging.md:103). |
| Admin only | None. |

### 1.4 Encoder

| Aspect | Detail |
|---|---|
| Purpose | Watch and manage the proxy queue; create ad-hoc jobs into `custom_encode/` (PLAN:622-644). |
| Inputs | `encoder_jobs`, encoder profiles (`.enc`), online mags. |
| Outputs | `proxies/db/<mag>/…` (320p), `proxies/hq/<mag>/…` (HQ), `custom_encode/<mag>/<profile>/…` (ad-hoc); job state. |
| Files | R `encoders/*.enc`, source media in `mags/`; W the three output trees; W `logs/` (the mockup says "full ffmpeg log in Logs", encoder.md:50, but multi-line ffmpeg output must fit the log line format, `D11`). |
| DB | R/W `encoder_jobs`; R `mags`, `clips`, `projects`. Schema lacks priority, job type, multi-file subset, cancelled/paused state (Part B §7). |
| External | `ffmpeg`, `ffprobe` (*implied* for source fps/channels/aspect). |
| Keys | `n` new job, `p` pause/resume, `r` retry, `c` cancel, `u` re-queue, `+`/`-` priority, `e` Profile Editor (PLAN:570-579). New-job screen: `Space` toggle, `a` all, `n` none, `Enter` start (encoder.md:113, 217-218). |
| Server vs client | Runs on the server. PLAN:366 says clients "can use … Encoder" but no client menu entry exists (`C8`) and it is undefined whether a client may cancel or re-prioritise server jobs (`S10`). |
| Admin only | `e` jumps to an admin-only module; must be hidden or refused for non-admin. |

### 1.5 Profile Editor

| Aspect | Detail |
|---|---|
| Purpose | Create, edit, duplicate, delete `.cam` and `.enc` files (PLAN:351-359). |
| Inputs | `cameras/`, `encoders/` folders. |
| Outputs | Profile files; reload of served profile lists. |
| Files | R/W `cameras/*.cam`, `encoders/*.enc`. |
| DB | R `encoder_jobs` (in-use warning, profile-editor.md:139), `mags.camera_profile_id` (in-use); the "loaded into the database" step has no table (`M13`). |
| External | None. |
| Keys | `n` new, `c` duplicate, `x` delete (asks), `r` reload, `Ctrl+S` save, `Esc` with "Discard changes?" (PLAN:581-589). |
| Server vs client | Server only. |
| Admin only | Yes (PLAN:359, profile-editor.md:6). |

### 1.6 Zmag manager

| Aspect | Detail |
|---|---|
| Purpose | Create 4-drive RAIDZ1 cold-storage pools, import/mount recognised zmags, copy whole mags onto them, auto-backup, snapshots, pool and drive health, cold pull (PLAN:738-790). |
| Inputs | Raw drives, importable pools, online mags, placements, cold-pull requests. |
| Outputs | Pools, mag copies on zmags, on-zmag manifest, placements, restored mags on the server. |
| Files | R `mags/`; W `<zmag mount>/<mag folder>/`, on-zmag manifest (format and location open, PLAN:905); W restored `mags/<mag>/` on cold pull. |
| DB | R/W `placements`, `cold_pull_requests`, `mags.status`. No `zmags` table exists, although role, location (local/remote), capacity and state are displayed (zmag-manager.md:16-17). |
| External | `sudo zpool create/import/export/status/scrub/replace`, `sudo zfs list/get/set/snapshot/hold/release/destroy`, `rsync`, SMART (`smartctl`, *implied* by zmag-manager.md:204-214, needs root and is not in sudoers), `lsblk` (*implied* for the raw drive list), `ssh` for remote stations. |
| Keys | `c` copy, `f` project filter, `←/→` zmag, `a` auto-backup, `n` new zmag, `s` snapshots, `p` health; snapshots: `h`, `d`, `m`, `z`, `y/n`; health: `b`, `r`, `t`, `m` (PLAN:591-606, zmag-manager.md:383-400). |
| Server vs client | Server only. Remote stations appear as extra zmags, read-only health. |
| Admin only | Not marked admin-only anywhere, but every pool action needs the `magvault` user's sudo, so it is admin-only in practice. Should be stated (`S10`). |

### 1.7 Library

| Aspect | Detail |
|---|---|
| Purpose | Browse Project → Mag → Clips → Player; build an export list (the export card); start the export copy or, on a client, send an export request (PLAN:660-678). |
| Inputs | Projects (open and closed), mags, clips, proxies, export cards. |
| Outputs | `bins/<name>/exportcard.txt` (+ `.backup`), export media copy, cold-pull requests; client: export request, pulled proxies, pulled finished files. |
| Files | R/W `bins/<name>/exportcard.txt`, `exportcard.txt.backup`; W `bins/<name>/<mag folder>/…`; R `proxies/db`; client R/W proxy cache. |
| DB | R `projects`, `mags`, `clips`; R/W `exports`, `export_clips` (optional per PLAN:699), W `cold_pull_requests`. |
| External | `mpv`, `rsync` (export copy, proxy pull, finished-file pull), `ssh`. |
| Keys | `a` add to list, `e` export, `p` pull proxies (client), `Enter`, `Tab` (library.md:212-222). No key to create/name/open an export, remove a clip, or set in/out (`M3`). |
| Server vs client | Server writes the card and copies. Client keeps a proxy cache, shows proxy status, builds a list offline and queues the export (PLAN:671-676). |
| Admin only | None. |

### 1.8 Export bins browser

| Aspect | Detail |
|---|---|
| Purpose | Read-only view of exports under `bins/` with status per clip and per export (PLAN:722-736). |
| Inputs | `bins/*/exportcard.txt`, files on disk, mag warm/cold status, copy progress. |
| Outputs | None. `e`/`l` jumps to Library with the export open. |
| Files | R `bins/` tree. |
| DB | R `mags.status`, copy job state (no table holds export copy progress today, see Part B §7). |
| External | None. |
| Keys | `Tab`, `↑/↓`, `Enter`, `e`/`l`, `Esc` (export-bins.md:14-20). |
| Server vs client | Mockup header says "server" (export-bins.md:27). Not in the client menu. Not stated whether clients may use it. |
| Admin only | Not stated. |

### 1.9 Logs

| Aspect | Detail |
|---|---|
| Purpose | Read-only viewer of `logs/YYYY-MM-DD.log`, filter by module and level, follow mode (PLAN:298-331). |
| Inputs | Daily log files. |
| Outputs | None. Retention deletes old files (done by the program, not this screen). |
| Files | R `logs/*.log`; retention W (delete). |
| DB | None ("never written to the database", PLAN:300). |
| External | None (*implied* inotify for follow). |
| Keys | `f` follow, `m` module filter, `l` level filter (logs.md:12-20). |
| Server vs client | Server module; not in client menu (logs.md:22). |
| Admin only | Not stated. |

### 1.10 Settings

| Aspect | Detail |
|---|---|
| Purpose | Edit `path.conf` roots (server) or client settings (PLAN:334-349). |
| Inputs | `path.conf`, `magvault.conf`, client `path.conf`. |
| Outputs | Rewritten config files. Effect on a running server is undefined (does changing `db` or `mags` need a restart? `M8`). |
| Files | R/W config directory. |
| DB | None. |
| External | None (filesystem checks only). |
| Keys | `Enter` edit, `Ctrl+S` save, `r` reload, `Esc` with discard prompt (settings.md:203-212). |
| Server vs client | Server: all roots. Client: proxy cache path, server host/user "limited edit". Client has no menu entry (`C8`). |
| Admin only | Server settings admin only (PLAN:336). |

### 1.11 Key module

| Aspect | Detail |
|---|---|
| Purpose | Grant and revoke client SSH access: generate on server and hand over the private key once, or accept a client's public key (PLAN:791-815). |
| Inputs | Admin input (label, pasted public key), removable drives. |
| Outputs | Lines in `~magvaultclient/.ssh/authorized_keys`; private key file on USB or chosen path. |
| Files | R/W `~magvaultclient/.ssh/authorized_keys`; W `<usb mount>/magvault-client.key` or a chosen file (mockup default `/home/magvault/magvault-client.key`, `C13`). |
| DB | None. Only public keys and labels are kept (PLAN:815). |
| External | `ssh-keygen` or an in-process key generator (*implied*), drive listing and (un)mount (`lsblk`, `udisksctl`, *implied*). |
| Keys | `n`, `g`, `p`, `x`, `w`, `u`, `Enter`, `Esc` back one step (key-module.md:19-30). No dashboard letter (key-module.md:652). |
| Server vs client | Server only. |
| Admin only | Yes (PLAN:791). |

## 2. Main flows, end to end

Each flow lists the steps as PLAN and the mockups describe them, then the gaps found while tracing. The process names (`magvaultd`, `magvault-gate`) are the ones proposed in Part B §5; PLAN itself names no processes.

### 2.1 Ingest a card → mag on server → 320p proxy queued

Server or standalone (card attached to the server):

1. **Discover.** Watch `/media`, `/run/media` or udisks for new mounts; recognise layouts; list label, size, guessed camera (PLAN:426). Manual path entry and `r` rescan always available.
2. **Probe.** Walk files and sidecars; skip junk (`.DS_Store`, AppleDouble, NOTES:10); guess camera, format, gamma, gamut, shoot day; build file list with sizes (PLAN:429-432).
3. **Confirm.** Operator picks values from the matching `.cam` only, picks an open project or creates one (`n`, UUID on create), adds notes. `p` is gated on the checklist (ingest.md:202).
4. **Copy.** `rsync` card → mag folder. Inputs lock, `Esc` blocked, `x` aborts (ingest.md:96-112).
5. **Verify.** SHA-256 of source and destination. Mockup: "hashing SHA-256 after rsync finishes" (ingest.md:108). PLAN:440: "SHA-256 hashed on the way in" (`C1`).
6. **Write** `magcard.txt` and `ascmhl/` (PLAN:442).
7. **Commit to DB**: `mags` (status `online`), `clips`, `encoder_jobs` (320p, `queued`) (PLAN:443-444).
8. **Encode** later: `magvaultd` picks the job, runs `ffmpeg` with the active `type: 320p` profile, writes `proxies/db/<mag folder>/…`, sets `clips.proxy_status=ready` (PLAN:650).
9. **Log** `INFO ingest Mag … ingested (84 GB)`.

Gaps found on this path:

- Where the copy lands before verification passes. PLAN:441 says "Only proceeds to commit if verification passes", but rsync writes straight into `mags/<folder>/`, so a half-copied folder sits in the archive root with a real-looking name (`D1`).
- Which UUID form names the folder: full UUID (PLAN:79) or the 6-hex short form every example uses (`20250520_a3f9c2`, PLAN:307 and all mockups) (`C15`).
- What counts as a "clip" vs a sidecar, and how spanned or chaptered clips are handled (`M10`).
- Free-space preflight on the destination (`M9`). Abort, card pulled mid-copy, disk full, crash between steps 6 and 7 (`M8`, `D6`).
- How the job reaches the encoder: PLAN lists this as open (PLAN:903). Proposal: the `encoder_jobs` row is inserted **in the same SQLite transaction** as the `mags` row; `magvaultd` polls or is notified (Part B §5.5).
- Which 320p profile is "active" if more than one `type: 320p` file exists (`M14`).
- RAW sources that stock `ffmpeg` cannot decode (`P10`).

Client variant adds:

1. Targets selected (staging default, USB-as-staging, direct, extra mirrors) (PLAN:453-458).
2. Card → staging, verified. Card may be removed once staging is verified ("Keep the card inserted until hashes match", ingest.md:104).
3. Staging → server: "Server queued · waits for staging" (ingest.md:102), then "server confirms → autoclean staging" (PLAN:454).
4. Staging → USB mirrors; `magvault-backup.txt` records pending/synced.

Additional gaps: how the client writes to the server and how the server "confirms" (no protocol, `S3`, `S4`); who writes the DB rows for a client-ingested mag (the client has no access to the server DB); where the client stores projects/profiles offline (`M11`); `/tmp` as the staging default (`P2`); and there is no screen or key for "sync pending to server" later (PLAN:468, `M5`).

### 2.2 Log a clip

1. Open Logging. Pick a mag in pane 1 (logging.md:192). **No project selector is drawn**; the header shows one project name (logging.md:15) (`M15`).
2. Pick a clip; player loads the 320p proxy (or local fallback) via mpv IPC.
3. Edit fields. Autosave on field exit; explicit save when leaving the clip or module (PLAN:511).
4. Tags/operators/directors: type, popup, Enter adds; `[create "…"]` if no match (logging.md:10).
5. `c` marks the mag `logging_complete`.
6. `v` reverts from the last mag file backup (PLAN:516).

Gaps:

- Where the data is durable outside SQLite. PLAN:485 says "can optionally be written into a sidecar or the mag card"; PLAN:516 says "the mag file on disk is the recoverable copy". These cannot both be optional (`C3`, `C4`).
- `camera_operator` and `director_producer` are single columns (PLAN:127-128) but the UI shows removable chips (`C6`).
- Typing into Description while `1`-`5`, `m`, `t`, `c`, `v`, `[`, `]`, `q` are single-letter shortcuts requires an explicit edit mode (`C7`).
- Client offline edits: "3 changes pending sync" implies a local change journal and a conflict rule (`M11`, `D7`).
- What makes a clip "logged" (`○`/`✓` in logging.md:30-40). No column holds it. Proposal: logged = any logging field non-empty.
- The shot type list ("Wide / medium / close / aerial…", PLAN:501) has no defined source (`M15`).
- Over SSH, mpv plays audio on the machine running mpv, not on the operator's machine (`T4`).

### 2.3 Build an export in Library → export card written → media copy (incl. cold pull)

1. Library: Project → Mag → Clip. `a` adds the clip to the export list (library.md:219).
2. "Add to list updates `exportcard.txt` immediately" (PLAN:666): copy card to `exportcard.txt.backup`, write new card (PLAN:718-720).
3. `e` starts the media copy into `bins/<name>/<mag folder>/` (PLAN:667).
4. If any clip's mag is cold, create a `cold_pull_requests` row (PLAN:667).
5. Zmag manager shows the request; wrangler docks the right zmag; manager copies the **whole mag** back to `mags/` (PLAN:787-789).
6. Export copy resumes for the restored clips; status becomes `done` when all clips are copied "and verified if verification is on" (PLAN:734).

Gaps:

- **No way to create or name an export** in Library, although the folder "name is assigned by the operator at creation time" (PLAN:684). The first `a` needs an export to write into (`M3`).
- No key to remove a clip from the list, or to enter in/out points, although the card and both mockups display them (`M3`).
- In/out meaning: the export copies whole original files ("Only original media", PLAN:695), so in/out can only be notes for the editor. PLAN never says so (`M3`).
- `source_path` in the card is absolute (`/mags/.../clip001.MOV`, PLAN:714). It breaks portability (PLAN:20) and, if trusted when copying, lets anyone who can edit a card make Magvault copy arbitrary server files into a bin that clients can pull (`D4`, `S9`).
- `cold_pull_requests` has no link to the export that needs it; the dashboard shows "cold pull · 20250412_55d0aa → Bamba-Cutaways" (dashboard.md:46), which needs that link (`M17`).
- No table or field holds export copy progress, so the browser's `copying` state has no source (Part B §7).
- No rule picks the source zmag when a mag has several placements. The sample asks for "zmag-03 remote, dock needed" for `20250412_55d0aa` (zmag-manager.md:49) while the same mag shows a copy on zmag-01, which is mounted locally (zmag-manager.md:21) (`M17`).
- **Nothing ever makes a mag cold** (`M1`).
- "verification is on" refers to a setting that does not exist (`C9`).
- Whole-mag restore for one clip: space and time cost (`P3`).

### 2.4 Create a zmag and copy a mag onto it

1. `n` in Zmag manager. List raw drives ("no pool and no partitions", zmag-manager.md:289). Mark exactly four.
2. Confirm `zmag-NN` and fixed properties `recordsize=1M`, `compression=lz4`, `atime=off`, `magvault:type=zmag` (PLAN:744-749).
3. Sudo password → `zpool create` → mount → appears in pane 2 (PLAN:752-758).
4. Select a mag in pane 1, `c` → `rsync` whole mag directory to the zmag; progress in the operations strip; hash check after rsync (zmag-manager.md:368).
5. `placements` row (`copying` → `complete`), on-zmag manifest updated (PLAN:772-778, 817).
6. Auto-backup (`a`): "insert first zmag → copy everything that needs a first copy; insert second → copy everything that needs a second copy" (PLAN:766).

Gaps:

- **Role (`primary`/`backup`) is never chosen.** The on-zmag manifest has `role:` (PLAN:775), the mockups show it, but the create flow asks only for name and properties (`M16`).
- "First" and "second" copy are not defined against roles or `copy_number` (`M16`).
- Pool creation should use stable device paths; the mockups use `sdX` names, and the samples themselves show why that breaks (`P6`).
- Recognising a zmag before import (`C12`, `T1`), no eject/`zpool export` action (`M2`), zmag full mid-copy (`M9`), drive replace flow (`M16`).
- Per-mag snapshots are not possible when mags are plain directories (`C20`).
- Copy verification compares against what: the magcard manifest (proposal) or a fresh source hash?

### 2.5 Client connects, pulls proxies, builds an export request

1. Client TUI starts; dashboard shows connection state online/offline/syncing (dashboard.md:208).
2. Library on the client: browse projects/mags/clips; clip list shows proxy `local / missing / pulling` (PLAN:674).
3. `p` (or automatic on connect) pulls missing 320p proxies for the mags being browsed into the client cache (PLAN:673).
4. `a` builds the list; `e` "creates a request on the server" (PLAN:675). Offline: list kept, export queued until connected (PLAN:676).
5. Server copies (with cold pull if needed). "When the server finishes … the client can pull the finished files" (PLAN:675).

Gaps:

- Transport: what command the client runs over SSH, under what restrictions (`S3`).
- The client cannot write `bins/<name>/exportcard.txt` directly, and offline it has no card at all, yet "The export list the operator sees is the export card" (PLAN:678). Proposal: the client keeps a local draft card in the same format and sends it with the request (Part B §6.7).
- Where the client stores browsable project/mag/clip lists offline (`M11`).
- Where the client puts the finished files it pulls; the client `path.conf` defines only the proxy cache (`M20`).
- How the client learns the export is finished (poll on connect; Part B §5.6).
- Proxy naming on disk ("original basenames", PLAN:48) collides when two files in different card folders share a basename (`D12`).

### 2.6 Generate a client SSH key and hand it over

1. Key module (admin, running as `magvault`). `g`, enter label.
2. Generate key pair; add public key to `~magvaultclient/.ssh/authorized_keys`; show private key once (PLAN:795-798; key-module.md:92 shows "✓ public key added" before handover).
3. `w` write to a chosen file, or `u` write to a mounted USB drive as `magvault-client.key` (PLAN:799-801).
4. "After either action the private key is no longer stored on the server" (PLAN:802). `Esc` without writing discards it.
5. Client installs the key, sets server host/user in client Settings, connects.

Gaps:

- `w` writes to a file **on the server** (the mockup default is `/home/magvault/magvault-client.key`), so the key *is* stored on the server (`C13`).
- The `magvault` user cannot write another user's `authorized_keys` without root, and sshd `StrictModes` refuses a file owned by someone other than the target user or root (`S5`).
- The private key is displayed in the terminal and is unencrypted (the sample key body decodes to cipher `none`, key-module.md:96) (`S7`).
- If the admin leaves without writing, the public key is already installed with no matching private key anywhere (`S7`).
- A FAT/exFAT USB stick cannot carry `0600` permissions; OpenSSH refuses private keys with loose permissions, so the client must copy and `chmod 600` (`S7`).
- No host key verification step for the client's first connection (`S11`).
- No forced command or restrictions on the installed key (`S3`).
- Revocation does not end sessions already connected (`S8`).
- The client Settings screen has host and user but no private key path (`M20`).

## 3. Findings

Format for each finding: **what the documents say** (quoted), **why it matters**, **smallest fix**.

### 3.1 Contradictions

**C1. When the SHA-256 is computed.**
- PLAN:440: "SHA-256 hashed on the way in; destination verified." NOTES:8: "hash while copying (SHA-256), read the destination back".
- PLAN:10: "Verified copies still run the ASC MHL / hash check after rsync finishes." ingest.md:108: "hashing SHA-256 after rsync finishes".
- rsync cannot report a SHA-256 of what it read, so "on the way in" and "all copies use rsync" cannot both hold. Post-copy hashing reads the card a second time (`P1`).
- **Fix:** keep rsync (it is locked in PLAN:4-8). Change PLAN:440 to "After rsync, SHA-256 of every source file and every destination file; commit only if all pairs match." Hash source and destination in parallel (different devices). Remove "hash while copying" from NOTES.

**C2. Which side is authoritative: database or files.**
- PLAN:89: "Files on disk remain the source of truth for manifests; the database is the index."
- PLAN:415: "Database is authoritative." PLAN:421 applies that to "manifest" too.
- NOTES:35 lists "Treating the card as authoritative over the database" as explicitly rejected. ingest.md:15: "files are the source of truth".
- This decides what crash recovery, rebuild, revert and the conflict rewrite do.
- **Fix (one sentence in PLAN):** "The database is authoritative for metadata (project link, camera, format, gamma, gamut, shoot day, notes, logging). The file manifest (names, sizes, SHA-256) is fixed at ingest and never rewritten; a manifest mismatch between card, database and disk is an integrity error that is reported, never auto-resolved." The conflict rewrite (PLAN:417-421) then covers metadata only.

**C3. "Rebuildable from files" vs data that only lives in SQLite.**
- PLAN:181: "The database can be rebuilt from the files on disk if it is lost." PLAN:20: "understandable and fixable by hand if the database disappears."
- Only in the DB: all logging fields (PLAN:485 "Log data lives in the database"), tag vocabulary, project `status`/`season_or_year`/`notes`, closed/open state, placements of zmags that are not mounted, encoder jobs, cold-pull requests.
- **Fix:** state what a rebuild recovers. Smallest viable: (a) write logging fields into `magcard.txt` under a `# logging` section (PLAN:485 already allows this), (b) add `project_status` to the magcard header, (c) placements rebuild from on-zmag manifests when each zmag is mounted, (d) jobs and requests are not recovered. See Part B §8.3.

**C4. Revert has nothing to revert to.**
- PLAN:516: "Revert loads the last backup of the mag file (`.magvault/magcard.*.backup` or equivalent log sidecar backup)."
- Backups are only created by the conflict rewrite (PLAN:418). Logging data is not in the mag card unless the optional write is chosen.
- **Fix:** follows from C3(a): before each rewrite of `magcard.txt` for logging, copy the current card to `.magvault/magcard.<UTC timestamp>.backup`. Rewrite on "leave clip / leave module" (PLAN:511's explicit save), not on every field exit, to avoid hundreds of backups. Revert = load the newest backup's `# logging` section for the current clip into the DB.

**C5. Free-text camera.**
- PLAN:104: "camera_profile_id (from .cam, or free text if no profile)".
- PLAN:197: "They cannot invent new camera, format, gamma or gamut names." ingest.md:180: 'No "add new" here'.
- **Fix:** delete "or free text if no profile". Unknown cameras use a shipped generic profile (`generic-dcim.cam` already appears in profile-editor.md:24).

**C6. People fields: single value in the schema, multiple chips in the UI.**
- PLAN:127-128: `camera_operator`, `director_producer` as single columns.
- PLAN:502-503: "Type then click | Same popup behaviour as tags." PLAN:513: "Each tag, camera operator, or director/producer chip has a small × to remove it."
- **Fix:** model like tags: `clip_people(clip_id, role, name)` with `role ∈ {camera_operator, director_producer}` (Part B §7). If only one value per role is intended, say so and draw a single chip.

**C7. What `Esc` and `q` do.**
- PLAN:537 and dashboard.md:15: "Esc/q returns to the dashboard from any module."
- PLAN:545-546: `Esc` returns to the dashboard; `q` "Quit the whole program. Only on the dashboard itself."
- encoder.md:221: "`Esc`/`q` means back everywhere else."
- `Esc` is used as "back one step" in: Key module (key-module.md:30), Encoder new job (encoder.md:219), Zmag secondary screens (zmag-manager.md:378), every popup, and leaving a text field.
- ingest.md:112 (pulling state footer) still offers "Esc dashboard" while PLAN:545 says `Esc` is "Blocked while a write/copy is in progress."
- Letters typed into text fields (Logging description, Settings paths, Key labels) collide with every single-letter shortcut, including `q`.
- **Fix:** (1) `Esc` = back one level: close popup → leave edit mode → leave secondary screen → dashboard. (2) `q` = quit, only on the dashboard; nowhere else. (3) Text fields have an explicit edit mode entered with `Enter`; in edit mode all printable keys are text, `Esc`/`Enter`/`Tab` leave it. (4) Fix the pulling-state footer.

**C8. Modules without a way in.**
- PLAN:528 dashboard menu: "Ingest, Logging, Library, Export bins, Zmag manager, Profile Editor (admin), Logs, Settings". No **Encoder**, no **Key module**. PLAN:557 has no letters for them. key-module.md:652 notes the same.
- PLAN:366: client "Can use Ingest, Logging, Dashboard, Encoder, etc." Client menu (PLAN:533, dashboard.md:207): Ingest, Logging, Library, Server dashboard. No Encoder, no **Settings**, although PLAN:344-347 defines client settings. On the client `s` opens the server dashboard (dashboard.md:220), so `s` cannot also be Settings.
- **Fix:** server menu adds Encoder and Key module (admin). Free letters: `c` (encoder) and `k` (keys) are unused by the dashboard. Client menu adds Settings on a free letter (e.g. `t`) and moves "server dashboard" off `s`, or keeps `s` = Settings on both and gives the server dashboard another letter (e.g. `d`). Decide whether the client gets Encoder at all (`S10`).

**C9. "No non-path settings" vs settings that exist.**
- PLAN:349: "No other non-path settings are defined today." settings.md:29: "none defined".
- PLAN:320: "`magvault.conf`: `log_retention_days`. Default: 30." PLAN:734: "verified if verification is on". PLAN:821: telemetry disk copy "default every 30 minutes". PLAN:766/`a`: auto-backup on/off is a persistent toggle.
- **Fix:** list them in `magvault.conf` (Part B §8.2) and show them in Settings. For "verification is on": either drop the phrase (always verify) or add `export_verify: true`. Recommendation: always verify, drop the phrase.

**C10. Telemetry paths and `path.conf` syntax.**
- PLAN:826-827: `telemetry_live=/dev/shm/magvault-telemetry.json`, `telemetry_disk=logs/telemetry-latest.json` (files; one relative).
- settings.md:23-24: `/dev/shm/magvault`, `/srv/magvault/telemetry` (directories).
- PLAN:28 calls `path.conf` "all roots only"; `.cam`, `.enc`, mag cards and export cards all use `key: value`, `path.conf` uses `key=value`. Base for relative paths is undefined.
- **Fix:** telemetry keys are files. Relative paths in `path.conf` resolve against the config directory. One syntax for every Magvault text file: `key: value` (Part B §8.1). `telemetry_disk` should not default into `logs/` (log retention walks that folder; `D11`).

**C11. Proxies in export bins.**
- PLAN:652: "NLE-specific proxies (Resolve etc.) are created with bin exports and live with the bins, not here."
- PLAN:695: "Only original media and matching sidecars. No proxies of any kind."
- **Fix:** delete PLAN:652 (bins hold no proxies), or move NLE proxies to the "later" list explicitly. The export bins layout is locked, so deleting is the smaller change.

**C12. Auto-import vs password-only sudo.**
- PLAN:761: "Plug in drives → zmag manager auto-imports and mounts only recognised zmags."
- PLAN:379: "The operator must enter the sudo password. Nothing is executed until the password is accepted."
- Also PLAN:750: "Only pools with that property (or matching name pattern)". A dataset user property (`magvault:type`) is not readable before the pool is imported; only the pool name (and GUID) is visible in `zpool import` listings.
- **Fix:** "auto" = auto-*detect*. When importable pools named `zmag-NN` appear, the Zmag manager shows them and asks for the sudo password to import. Import with `-N` (no mount), read `magvault:type`; if it is not `zmag`, export again and ignore. Drop "or matching name pattern" as an acceptance rule (the name is only a pre-filter).

**C13. Key `w` stores the private key on the server.**
- PLAN:800-802: "`w` write it to a chosen file … After either action the private key is no longer stored on the server."
- key-module.md (write to file screen): default path `/home/magvault/magvault-client.key`. The TUI runs on the server, so every "chosen file" is on the server unless it is removable media.
- **Fix:** restrict `w` to paths under a removable-media mount (then `w` and `u` differ only in how the target is picked), or drop `w`. The paste-a-public-key path (PLAN:806-809) is the safer default and should be presented first.

**C14. Stale statements inside the mockups.**
- profile-editor.md:228: "What happens to unsaved changes on `Esc` is not yet decided" vs PLAN:589 and profile-editor.md:8 (decided: prompt).
- logs.md:202 says PLAN wants follow to keep "the view at the bottom"; PLAN:317 now says newest-first with selection pinned to the top. Resolved; remove the open point.
- export-bins.md:112-117 lists four open points that PLAN:732-736 has since locked; its On disk pane (export-bins.md:55-62) still lists no sidecars although PLAN:736 now says it does.
- PLAN:591-606 Zmag key table has no `n`, although PLAN:752 says the create flow is "opened with `n`".
- PLAN:897-912 "Open points" still lists "Exact fields inside a camera `.cam` profile" and "Logging field set", both locked above (PLAN:200, PLAN:492).
- **Fix:** housekeeping commit; no design change.

**C15. Folder names and dates.**
- PLAN:79: folder is `<YYYYMMDD>_<mag-uuid>`. Every example (PLAN:307, all mockups) uses a 6-hex suffix, e.g. `20250520_a3f9c2`.
- PLAN:103: `shoot_date (YYYYMMDD used in the folder name)`; PLAN:396 magcard: `shoot_date: 2026-11-18`; ingest.md:34: `2025-05-20`.
- **Fix:** folder suffix = full canonical lowercase UUID (36 chars). Display may truncate to the first 6 hex chars where unambiguous. `shoot_date` is stored and written as ISO `YYYY-MM-DD`; the folder uses `YYYYMMDD`.

**C16. Key letters that collide or disagree.**
- encoder.md:50 "R retry C cancel" (uppercase) vs footer and PLAN:574-576 `r`, `c`.
- encoder.md:113 new-job screen `n` = "none" vs `n` = "new job" on the queue screen; acceptable only because they are different screens. Say so, or use `0` for none.
- profile-editor.md:52 "Tab next field" vs PLAN:547 `Tab` = next pane.
- **Fix:** one key table per screen in the spec (Part B §6), lowercase everywhere, `↑/↓` moves between fields inside a form pane.

**C17. Displayed values differ from profile values.**
- ingest.md:32-33 shows "A7SIII S-Log3", "Sony S-Gamut3.Cine"; the `.cam` example (PLAN:219-223) lists `S-Log3`, `S-Gamut3.Cine`.
- **Fix:** the picker shows exactly the strings in the profile; those strings are what is stored.

**C18. `encoding.conf` vs `.enc` files.**
- PLAN:30: "`encoding.conf` # ffmpeg presets and proxy rules (or replaced by encoder profile files)". PLAN:241-296 defines `.enc` files.
- **Fix:** drop `encoding.conf`. Add `encoders` to the path list in PLAN:28 (Settings already shows it).

**C19. Export bins layout is flat; cards are not.**
- PLAN:62-63 and PLAN:691-692: `<YYYYMMDD>_<mag-uuid>/ <original clip names + sidecars>`.
- Cards have folder structure (`PRIVATE/M4ROOT/CLIP/…`, `DCIM/100MSDCF/…`); the same file name can occur in two folders of one card; a RED clip is an `.RDC` folder of `.R3D` segments. A flat layout collides and separates sidecars from what reads them.
- **Fix:** inside the export, recreate the clip's **relative path** under the mag folder (`bins/<name>/<mag folder>/<relative path>`). Names stay original and immutable, satisfying PLAN:697.

**C20. Per-mag snapshots.**
- PLAN:769: "ZFS snapshot list for the selected mag or zmag." zmag-manager.md:109 `m mag snapshots`.
- A ZFS snapshot belongs to a dataset. Mags are directories (PLAN:36 "flat, no subfolders") in one dataset, so a mag has no snapshots of its own. Creating one dataset per mag would need `sudo zfs create` (password) on every ingest, including unattended client syncs.
- **Fix:** "mag snapshots" = snapshots of the dataset holding the mag, filtered to those in which `<mountpoint>/.zfs/snapshot/<snap>/<mag folder>` exists. Destroying a snapshot still affects the whole dataset; the confirm dialog must say so.

### 3.2 Missing behaviour

**M1. How a mag becomes cold.** `mags.status` lists "ingesting, online, cold, archived, …" (PLAN:108). Library shows "cold = on a zmag only" (library.md:49). Nothing in PLAN or any mockup removes a mag from `mags/` after it is on zmags, so `cold` is unreachable, and with it cold pull and the export `cold` state. `archived` is never defined. README:17 ("Originals stay cold until explicitly requested") implies eviction exists. **Fix:** define the lifecycle (Part B §7.2) and one action that evicts a mag from the server only when it has ≥ 2 `complete` placements on different zmags (or a configured minimum), with sudo-free `rm` as the `magvault` user, a confirm, and a log line. Decide where the action lives (Zmag manager is the natural home since it shows copies).

**M2. No undock.** PLAN:378 lists "export" among the zpool operations, but no screen or key exports a zmag. Pulling drives of an imported pool leaves it `SUSPENDED`/`UNAVAIL` and can hang I/O. **Fix:** an "Eject zmag" action in the Zmag manager (sudo `zpool export`), blocked while a copy or pull uses that zmag.

**M3. Export lifecycle in Library.** Missing: create/name an export, choose which export is open, remove a clip, enter in/out, re-run the copy after adding clips, what in/out mean. **Smallest fix:** on the first `a` with no export open, prompt for a name (validated, unique); `e`/`l` from Export bins opens one; a "remove from list" key on the export list pane; in/out are notes copied into the card, the media copy is always the whole file; `e` copies whatever is on the card and not yet in the bin (idempotent).

**M4. Project management.** PLAN:187: "Closing or re-opening a project is a deliberate action." README:25: "Projects can be edited or merged later." No screen does any of it. **Fix:** put close/reopen and rename on the Library Projects pane (the only place closed projects appear, library.md:190). Merge stays out of v1 and should be removed from README or listed as later.

**M5. Client sync after the fact.** PLAN:468: "Operator can reconnect the drive later and finish 'sync pending to server.'" The client dashboard shows "1 mag pending sync" (dashboard.md:112) but no key starts a sync. **Fix:** a "sync pending" action on the Ingest Targets pane (and optionally automatic on connect), plus a defined staging root in the client `path.conf`.

**M6. What "Prepare" does to a USB drive.** PLAN:475-479 and ingest.md:207 describe gating, not effect. The "will not offer Prepare again while … pending server sync" rule (PLAN:477) only makes sense if Prepare erases the drive. PLAN:382 allows client sudo for "mount, write manifest, etc." **Fix:** state whether Prepare formats (and with which filesystem; FAT32 cannot hold files > 4 GiB, `D9`) or only writes `magvault-backup.txt`. List the exact client sudo commands; "etc." cannot go into a sudoers file.

**M7. USB `role: master` / `slave`.** PLAN:463 defines the field; nothing says how master and slave behave differently. **Fix:** define it (e.g. master = may act as staging and is the sync source; slave = mirror only) or drop the field.

**M8. Failure and retry semantics** (PLAN:911 still open). Needed for every copy path: abort, source removed, destination full, network loss, crash/reboot mid-step, partial outputs, retry/resume, and what the UI shows afterwards. Also whether a saved `path.conf` applies live or needs a restart. **Fix:** Part B §9.

**M9. Free-space checks.** Only the Encoder new-job screen shows free space (encoder.md:137). Ingest (each target), zmag copy, cold pull and export copy have none. **Fix:** preflight `required = sum(source sizes) + margin` per target; refuse to start, log WARN.

**M10. What a "clip" is.** The schema's `clips` table carries both media and sidecar files (manifest example PLAN:406-407) and logging fields. Undefined: which files are clips vs sidecars; how sidecars are matched ("matching sidecars", PLAN:695); chaptered/spanned recordings (GoPro chapters, Sony/Canon 4 GiB splits, RED `.RDC` segments). **Fix for v1:** every file is a manifest entry (`files` table); a clip is one media file selected by extension; sidecars match by same directory + same stem (case-insensitive) or by a per-camera rule added to `.cam` later; spanned recordings are separate clips in v1. Part B §7.

**M11. Client-side state and sync.** The client browses projects/mags offline, logs offline with "changes pending sync", queues exports offline, and lists profiles from the server. PLAN does not say where the client stores this or how it syncs. **Fix:** a client cache database plus an outbox (Part B §5.7).

**M12. Who did it.** `created_by: odofad` (PLAN:708), `requested_by` (PLAN:165), and the `odofad` shown in every mockup header. With only `magvault` and a shared `magvaultclient`, there is no person identity. **Fix:** identity = SSH key label for clients (passed by the forced command, `S3`), `magvault@<host>` for the server, optionally suffixed with `$SUDO_USER`/`$LOGNAME` of the login that switched to `magvault`. Headers show that string.

**M13. Camera profiles "loaded into the database"** (PLAN:195) but no table exists. **Fix:** profiles stay files; the daemon parses them into memory and serves them; no table. Mags store the confirmed text values, so a later profile edit never changes history.

**M14. Default profile per type.** encoder.md:35-37 shows "Active defaults 320p default-320p / HQ hevc-hq". Nothing says how a default is chosen when two `type: 320p` files exist. **Fix:** `default_profile_320p: <id>` and `default_profile_hq: <id>` in `magvault.conf`.

**M15. Logging context.** No project selector (logging.md:15); no way to choose local source on the client (staging vs USB vs synced, logging.md:103 shows "USB BACKUP1" in the header); shot type list source (PLAN:501). **Fix:** project picker popup in pane 1's title; client source picker in the same popup; shot types as a list in `magvault.conf` (`shot_types: wide, medium, close, aerial`).

**M16. Zmag details.** Role at creation; meaning of "first"/"second" copy; replace-drive flow (which new drive, confirm, `zpool replace`, resilver progress); location and format of the on-zmag manifest (PLAN:905); next free `zmag-NN` when some zmags are not mounted (must come from the DB plus manifests). **Fix:** Part B §6.6.

**M17. Cold pull details.** Link to the export(s) waiting for it; source selection (prefer a locally mounted zmag with a `complete` placement, then a remote station, else "dock needed: zmag-NN"); what happens to the restored mag afterwards (stays `online` until evicted again, `M1`). **Fix:** Part B §6.6, schema §7.

**M18. Snapshot creation.** The mockup shows `auto-20251122-0200`, `pre-scrub-…`, `first-copy-complete` snapshots (zmag-manager.md:103-107). PLAN only lists, holds and destroys. **Decide:** snapshots are created by an external tool (e.g. a system snapshot scheduler) and Magvault only manages them, or Magvault takes one named snapshot after each completed auto-backup pass. Either way, say it.

**M19. Detecting a mismatch.** PLAN:417: "When a mismatch is detected (cold mag reappears, card differs …)". Nothing defines when Magvault looks. **Fix:** on cold-pull completion, on zmag import, and on an explicit "verify mag" action: compare `magcard.txt` header with DB (metadata rewrite per PLAN:418-419) and the file list/sizes with the manifest (integrity report). Full re-hash only on explicit verify.

**M20. Client paths.** Client `path.conf` has only the proxy cache (PLAN:345). Missing: staging root, where pulled export files go, private key path, server host key (known_hosts).

**M21. Roadmap ordering.** Logs and Settings modules are in no phase. Phase 1 (client ingest) needs SSH transport and installed keys; the Key module is Phase 5 (fine if keys are installed by hand meanwhile, but say so). Phase 3 Library shows a player; mpv integration is Phase 6. Profile Editor is Phase 6 but profiles are needed from Phase 0 (fine if hand-edited; say so). Logging is Phase 6 but the schema and magcard `# logging` section (C3) should exist from Phase 0.

**M22. Encoder gaps.** Parallel job limit (the mockup runs two at once, encoder.md:14-17); how HQ jobs are requested ("Generated only when requested", PLAN:649) when the per-asset toggle is "Noted for later" (PLAN:651).

**M23. Dashboard sources.** "Recent events" and "Pending requests" need definitions (Part B §6.1).

### 3.3 Security gaps

**S1. `sudo zfs`/`sudo zpool` with any arguments is root.** PLAN:375: `magvault ALL=(root) /usr/sbin/zpool, /usr/sbin/zfs`. With arbitrary arguments the `magvault` user can `zfs set mountpoint=/etc` on a dataset, destroy the root or working pools, change `zfs allow` delegations, or import any pool. The password requirement protects against a stolen session only while sudo's credential cache is cold (`S2`). Also `smartctl` (SMART panel and self-tests) needs root and is not listed. **Fix:** a root-owned helper (e.g. `/usr/lib/magvault/zmag-helper`, mode 0755, owned by root) that accepts a fixed set of verbs (`create`, `import`, `export`, `scrub`, `replace`, `snapshot-hold`, `snapshot-release`, `snapshot-destroy`, `smart-read`, `smart-test`), validates every argument (pool name matches `^zmag-[0-9]{2,}$`, devices are under `/dev/disk/by-id/` and not part of any non-zmag pool, snapshot names belong to a zmag pool), and refuses everything else. sudoers lists only the helper. Read-only `zpool status`/`zfs list` stay unprivileged.

**S2. Sudo password handling.** sudo caches credentials (default 15 minutes per terminal), so after one prompt later commands do not ask again, contradicting "Nothing is executed until the password is accepted" for each action. Magvault must also pass the password to `sudo -S` without logging it. **Fix:** run every privileged call as `sudo -k -S <helper> …` (or set `Defaults:magvault timestamp_timeout=0`), hold the password only for the call, zero the buffer, never include it in logs or error text.

**S3. Client keys are shells.** PLAN:796 and PLAN:808 add keys to `authorized_keys` with no options. Every client key can open a shell as `magvaultclient` on the server, read whatever that user can read and run anything. The plan's client/admin split is then UI-only. **Fix:** every client line is installed as
`restrict,command="/usr/lib/magvault/magvault-gate --client <label>" <type> <key> <label>`
The gate dispatches on `SSH_ORIGINAL_COMMAND` to a fixed set of operations (Part B §5.6). `restrict` disables forwarding and PTY; add `pty` only for the read-only server dashboard operation if that is kept.

**S4. Client write access to the archive.** PLAN:454: "card → local staging → rsync to server". If the client rsyncs into `mags/`, `magvaultclient` needs write access to `mags/`, and any client can delete or alter any mag. **Fix:** clients write only to a per-client spool, e.g. `incoming/<label>/<mag folder>/`, via `rrsync -wo` (the restricted rsync wrapper shipped with rsync). `magvaultd` (as `magvault`) verifies the files against the uploaded `magcard.txt` manifest, then moves the folder into `mags/` with a same-filesystem `rename(2)` and inserts the DB rows. `mags/`, `proxies/`, `bins/`, `db/` are writable only by `magvault`.

**S5. Editing `~magvaultclient/.ssh/authorized_keys` as `magvault`.** sshd's default `StrictModes yes` rejects an `authorized_keys` file that is writable by anyone other than its owner (the target user) or root, so the `magvault` user cannot be given write access to it, and PLAN allows no other sudo. **Fix:** keep the key list in a file owned by `magvault` (e.g. `/etc/magvault/client_keys`, mode 0640) and have sshd read it through `AuthorizedKeysCommand /usr/lib/magvault/magvault-keys %u` with `AuthorizedKeysCommandUser magvault`, matched only for `User magvaultclient`. The Key module edits that file. Installer sets this up once (root, at install time). The mockups' "~magvaultclient/.ssh/authorized_keys" wording changes to that path.

**S6. Pasted public keys.** The paste flow (key-module.md "Add key") accepts free text. A pasted line can carry its own options (`command=…`, `from=…`, `environment=…`), contain a newline that adds a second key, duplicate an existing key, or reuse a label. **Fix:** parse strictly: exactly one line, `<type> <base64>[ <comment>]`, type in an allow-list (`ssh-ed25519`, `sk-ssh-ed25519@openssh.com`, `ecdsa-sha2-nistp256`, RSA ≥ 3072 bits), base64 decodes to a key of that type; strip the comment and use the operator's label; reject duplicates by fingerprint; labels unique and `^[a-z0-9][a-z0-9-]{0,31}$`.

**S7. Private key generation and hand-over.**
- Shown on screen: terminal scrollback, `tmux`/`screen` buffers and SSH session recording keep it. Showing it adds nothing once `w`/`u` exist. **Fix:** do not render the key body; show fingerprint only.
- Unencrypted: the sample decodes to cipher `none`. **Fix:** generate Ed25519 in-process (no temp file from `ssh-keygen` on disk), write it with an operator-entered passphrase, or document that the client must add one (`ssh-keygen -p`).
- Order: PLAN:796-797 installs the public key before hand-over. **Fix:** install the public key only after the private key was written successfully; on `Esc`, install nothing.
- USB filesystem permissions: tell the operator the client must copy the file and `chmod 600` it; write a `known_hosts` line for the server next to it (`S11`).
- Mounting/unmounting the USB stick as `magvault` on a headless server needs udisks policy or root. **Fix:** require the stick to be mounted already, or add `mount`/`umount` of removable devices to the helper (S1).

**S8. Revocation leaves live sessions.** Removing a key stops new logins only. **Fix:** on revoke, also terminate gate processes for that label (the gate records its PID under `/run/magvault/gate/<label>.<pid>`).

**S9. Path and input validation.**
- Export name → folder name: reject `/`, `\0`, leading `.`, `..`, control characters, names longer than 100 bytes; must be unique under `bins/`.
- Export card `source_path`: never used to locate media. Media is located as `mags_root / mag_folder / relative_path` from the DB, and the resolved path must stay inside `mags_root` (after `realpath`).
- Card contents: refuse or skip symlinks, device files and FIFOs during probe; rsync with `--no-links --no-devices --no-specials`. Manual ingest paths must be directories and must not be inside Magvault's own roots.
- File names containing `|`, newline or other control characters break the text card formats (Part B §8); reject at probe with a clear message (FAT/exFAT cards cannot contain `|` anyway).
- RPC inputs from clients: UUIDs parsed as UUIDs, labels/names matched against the patterns above, never concatenated into shell commands (use `Command::new` with argument vectors only).
- `rrsync` roots: proxies read-only, bins read-only, incoming write-only per client.

**S10. Client vs admin enforced server-side.** Hiding menu items is not enforcement. **Fix:** the server checks: admin operations only when the process uid is `magvault` and it is not running under the gate; the gate's verb list contains no admin operations. Decide: may clients pause/cancel/re-prioritise server encoder jobs? Recommendation: no; clients get a read-only queue view. Decide whether the "server dashboard over SSH" (PLAN:533) is a read-only view (recommended) or a full server TUI as `magvaultclient` (then every module needs gate-level checks).

**S11. Server host key.** A client's first connection will trust-on-first-use. **Fix:** the Key module writes the server's host key line into `magvault-server.known_hosts` next to the private key; client Settings imports it.

**S12. Audit.** Key add/revoke, sudo actions, evictions and settings saves are security-relevant and should always produce a log line. PLAN:324-331 lists writers; add `keys` and log zmag pool operations with the operator identity (M12). Never log secrets.

**S13. Remote zmag stations.** PLAN:782-785: "Connects over SSH (keys only) … updates the central database." No principal, key type or rights are defined. A station needs more than a client (report placements, accept jobs) and less than an admin. **Fix (Phase 5 decision):** stations get their own key class with a separate verb list in the gate (`station.report`, `station.job.next`, `station.job.update`); they never write SQLite directly.

### 3.4 Data integrity

**D1. Half-written mags look real.** rsync into the final `mags/<folder>/` exposes partial folders under their final names; after a crash, a rebuild from files would index them. **Fix:** copy into `<mags_root>/../incoming/…` or a dedicated `incoming` root **on the same dataset**, verify, write card and MHL, then `rename(2)` into `mags/`. Same pattern for cold pulls (`restore/…`) and export copies (`<file>.partial` renamed on success).

**D2. Card writes.** `magcard.txt` and `exportcard.txt` rewrites must be atomic (write `<name>.tmp`, `fsync`, `rename`, `fsync` dir) and serialised (`flock` on the card while doing backup-then-write). Two Library sessions adding to the same export at once otherwise lose updates, and `exportcard.txt.backup` keeps only one previous version (PLAN:690), so a lost update cannot be recovered.

**D3. ASC MHL.**
- PLAN:41 writes `ascmhl/manifest.mhl`. The ASC MHL standard defines its own naming for the files inside `ascmhl/` (numbered generation files plus `ascmhl_chain.xml`). A fixed `manifest.mhl` will not be recognised as a history by ASC MHL tools.
- `magcard.txt` and `.magvault/` sit inside the hashed folder and are rewritten later; if they are in the MHL, every later verification reports them as modified. **Fix:** list them in the MHL ignore patterns.
- Decide whether zmag copies and cold pulls add MHL generations (that is what ASC MHL is for: chain of custody) or leave `ascmhl/` untouched. Recommendation: add a generation on each verified zmag copy and cold pull.
- ASC MHL supports SHA-256, so the plan's choice works ([ascmhl on PyPI](https://pypi.org/project/ascmhl/0.7.1/)).

**D4. Absolute paths.** `mags.dump_path`, `exports.path`, `clips.proxy_path` and `exportcard` `source_path` hold paths. PLAN:20 wants "point a new install at the config directory and it continues". **Fix:** store paths relative to their root (`mag_folder`, `relative_path`, export `name`); compute absolute paths from `path.conf` at runtime. Keep `source_path` in the export card only as an informational relative path (`<mag_folder>/<relative_path>`).

**D5. Stale copies.** After logging (C3) or a metadata edit, the `magcard.txt` on every zmag copy is stale. This is by design (DB authoritative, rewrite on reappearance), but it means a zmag copy's card must never be read as current. Rebuild-from-zmag must say "metadata as of placement date".

**D6. Crash recovery.** Rows left in `copying`, `pulling`, `running`, `ingesting` after a crash or reboot. **Fix:** on `magvaultd` start, every such row moves to `failed` with error `interrupted` (or back to `queued` for encoder jobs), partial directories are removed, and a WARN line is logged. All jobs must be idempotent (rsync re-run + full verify).

**D7. Offline logging conflicts.** Two clients (or a client and the server) edit the same clip offline. "Database is authoritative" does not say which edit wins. **Fix:** per-field last-writer-wins by the edit's own timestamp, with a WARN log line naming both versions when a newer server value is overwritten or a client value is discarded. Clock skew is then a known limitation; alternatively server-wins (simpler, loses client edits). Decide.

**D8. Placement trust.** `placements.status = complete` is never re-checked. A zmag can lose data (scrub errors, a pool imported read-write elsewhere). **Fix:** after a scrub with errors, or on explicit verify, mark affected placements `suspect`; eviction (M1) refuses mags whose placements are not `complete`.

**D9. Field-drive filesystems.** FAT32 cannot store files over 4 GiB (common for ProRes RAW and long takes); FAT/exFAT have coarse timestamps, so rsync needs `--modify-window=1` to avoid recopying, and ASC MHL mtimes will differ by up to 2 s across filesystems. **Fix:** Prepare refuses FAT32; mtime comparison tolerance of 2 s on FAT/exFAT targets; SHA-256 remains the integrity check.

**D10. Deleting things still referenced.** Profiles referenced by queued jobs (the mockup warns, profile-editor.md:139); zmags referenced by placements; exports referenced by pending cold pulls. **Fix:** soft references by text id; deletion warns and blocks only for queued/running jobs.

**D11. Logs.** Multiple processes (daemon, TUIs, gate) append to one daily file. Multi-line "context that follows" (PLAN:316) from two writers can interleave and attach to the wrong entry. Day files are named by date; with UTC timestamps (PLAN:311) and an operator in UTC+2, entries between 00:00 and 02:00 local go into the previous day's file. Retention (PLAN:322) would delete `telemetry-latest.json` if it simply removes "files older than" N days in `logs/` (PLAN:827 puts it there). **Fix:** one `write(2)` per entry including its context lines; context lines start with two spaces; day files use the UTC date (state it); retention only deletes names matching `^\d{4}-\d{2}-\d{2}\.log$`.

**D12. Proxy file names.** "proxy files, original basenames" (PLAN:48). Two files `A/C0001.MP4` and `B/C0001.MP4`, or `C0001.MOV` and `C0001.MP4` in one folder, map to the same proxy name. **Fix:** proxies mirror the relative path and append the container extension: `proxies/db/<mag folder>/<relative_path>.mp4`.

### 3.5 Performance and operations

**P1. Every card is read twice.** rsync reads it once, the SHA-256 pass reads it again (C1). At an assumed ~90 MB/s sustained card read, a 256 GB card takes about 47 minutes per pass, so roughly 95 minutes before the card may be removed. Hash source and destination concurrently so the destination pass costs nothing extra. Display both stages separately (ingest.md:98 already has `rsync copy` and `hash + verify`).

**P2. `/tmp` as staging** (ingest.md:53, dashboard.md:111). Ubuntu cleans `/tmp` at boot, so a mag "pending sync" disappears with a reboot, possibly after the card was reformatted. If `/tmp` is a tmpfs, a 256 GB card also exhausts RAM. **Fix:** staging root in the client `path.conf`, default under `/var/lib/magvault/staging` or the user's data directory, never `/tmp`.

**P3. Cold pull granularity.** One cold clip restores the whole mag (PLAN:789, whole-mag copies). For a 200 GB mag that is hours from a USB-docked RAIDZ1 and needs 200 GB free on the mags dataset. Acceptable, but the preflight (M9) and the Library status line must show the size of what will be restored.

**P4. Large mags.** Photo/timelapse cards can hold tens of thousands of files. Needs: virtualised lists in ratatui (render only visible rows), batched DB inserts in one transaction, rsync progress from `--info=progress2` (one total line) rather than per-file output, and hashing with a bounded worker pool.

**P5. Many zmags.** The copies column draws one sub-column per zmag (`01 02 03`, zmag-manager.md:16). This does not fit past roughly 6-8 zmags at 120 columns. Show the count plus a compact list of zmag numbers, or only mounted zmags as columns.

**P6. Device names.** Pools must be created with `/dev/disk/by-id/…` paths, not `sdX`, because `sdX` changes between boots and USB docks. The mockups illustrate the problem: `sdh` is a zmag-02 member (dashboard.md:27) and also the `KEYSTICK` USB stick (key-module.md, USB screen); `sdi` is a new zmag drive (zmag-manager.md:279) and also `BACKUP-USB`. Create with `ashift=12` explicitly (most 4 TB+ drives are 4K-sector).

**P7. Drive choice.** The raw drive list samples include `WD40EFAX` (zmag-manager.md:283), a drive-managed SMR model; SMR drives are known for very slow resilvers in ZFS. Show the model clearly; no new feature needed.

**P8. Video in the terminal.** kitty/sixel frames over SSH are large (uncompressed or PNG frames per video frame). 320p at 24 fps is feasible on a LAN, poor over a field link. mpv and ratatui both writing to the same TTY need coordination (mpv draws only inside the player rectangle; ratatui must not redraw that region while playing).

**P9. Telemetry writes.** Write to `<file>.tmp` then `rename` so readers never parse a half-written JSON. Fine on tmpfs.

**P10. Decoding RAW and audio rules.**
- Stock FFmpeg added a ProRes RAW decoder in 8.0 ([ubuntuhandbook](https://ubuntuhandbook.org/index.php/2025/08/ffmpeg-8-0-released-apv-prores-raw-decoder/)); confirm the Ubuntu 26.04 build includes it. RED R3D and Blackmagic RAW are vendor-SDK formats that stock FFmpeg does not decode, yet `red-komodo.cam` and `bmpcc-6k.cam` are in the sample profile list (profile-editor.md:17-19). Proxies for those cameras will fail as "bad source" unless an SDK-based decoder is added. Say this in PLAN.
- "Audio channels must never be downmixed" (PLAN:658) with `audio_codec: aac`: FFmpeg's AAC encoder handles common layouts up to 7.1; cameras that record many discrete mono tracks need one output stream per input stream (`-map 0:a`) or PCM in a MOV container. Define: preserve stream count and per-stream channel layout.
- Variable-frame-rate sources (phones, action cameras): "Frame rate = source exactly" means passthrough timestamps (`-fps_mode passthrough`), not a fixed output rate.
- `height: 320` with odd aspect ratios needs an even width (`scale=-2:320`); anamorphic sources need display aspect (SAR) handled.

**P11. SQLite with several processes.** Daemon, server TUIs and remote operations all touch one file. Use WAL mode, `busy_timeout`, short transactions, and a single process (the daemon) that executes jobs. Do not put the DB on NFS.

**P12. ZFS dedup** (PLAN:697 "Space saving left to ZFS dedup") has a large RAM cost per TB of deduplicated data. Mention it as an operator choice with that caveat; Magvault should not enable it.

### 3.6 Hard to implement or test

**T1. ZFS and sudo.** Needs root, a kernel module and real or file-backed vdevs. Put all privileged operations behind one helper (S1) and one Rust trait (`PoolOps`) with a fake implementation for unit tests; run the real helper against file-backed pools (`truncate -s 1G` files) in a VM in CI.

**T2. Card discovery.** udisks events, mount points and camera layouts. Test with directory fixtures that mimic each layout (Sony, Canon, ARRI, RED, BMD, Panasonic, Fujifilm, GoPro, DJI, Insta360, generic DCIM, NOTES:9) and a fake mount watcher.

**T3. SSH transport.** Run sshd in a container with the real `AuthorizedKeysCommand`, gate and `rrsync`; contract tests per RPC verb, including refusal tests (admin verbs over the gate, path traversal, oversize input).

**T4. Playback.** mpv IPC, kitty/sixel and audio routing vary by terminal and by local vs SSH. Manual test matrix: kitty, WezTerm, foot (sixel), xterm (sixel), plain SSH. Note: over SSH, mpv on the server plays audio on the server's sound device, not the operator's; PLAN:614 ("audio + status line only") needs "on the machine running Magvault".

**T5. rsync output parsing.** Progress format differs by version and locale. Run with `LC_ALL=C`, `--info=progress2 --no-inc-recursive --outbuf=L`, parse defensively, and treat exit codes (23, 24, 30, 35 …) explicitly.

**T6. Text formats.** Golden-file tests and round-trip property tests for every card parser and writer (unknown keys preserved, `|` and control characters rejected, CRLF tolerated on read).

**T7. Layout at 120x36.** Several mockups overflow their panes at minimum size: the tag popup hint line runs past the Log clip pane in both sizes (logging.md:32, 79), Library's "+ already on the export list" overwrites the border at 120x36 (library.md:89), the Encoder footer is cut off ("e pr", encoder.md:93). Define truncation (ellipsis) and footer priority rules; use ratatui's `TestBackend` for snapshot tests at 120x36 and 150x42.

---

# Part B — Technical specification

This specification resolves the findings with the smallest changes listed above. Where a choice is still the owner's to make it is marked **[decision]** and repeated in §11 with the proposed default. Everything else is written as the default an implementer can start from.

## 5. Architecture

### 5.1 Programs

| Program | Runs as | Where | Role |
|---|---|---|---|
| `magvaultd` | `magvault` | server (systemd service) | The only process that executes jobs: commits incoming mags, encodes, zmag copies, cold pulls, export copies. Writes telemetry. Runs log retention. Serves the client gate over a Unix socket. |
| `magvault` (TUI) | `magvault` on the server, any user on a client | server and clients | ratatui front-end. On the server it reads SQLite directly and makes small interactive writes (logging fields, projects, export cards) in short transactions; it never runs long jobs itself except where PLAN requires the operator to wait (server-side card ingest, see §6.2). On a client it talks to the server only through SSH + the gate. |
| `magvault-gate` | `magvaultclient` | server (SSH forced command) | Thin dispatcher. Reads `SSH_ORIGINAL_COMMAND`, refuses anything not on its list, then either pipes JSON lines to `magvaultd`'s socket with the client label attached, or `exec`s `rrsync` with a fixed root and mode. Holds no state. |
| `magvault-keys` | `magvault` (via sshd `AuthorizedKeysCommandUser`) | server | Prints the client key lines for `magvaultclient` from `/etc/magvault/client_keys` (S5). |
| `zmag-helper` | root (via sudo) | server | Validated privileged verbs for zpool/zfs/smartctl/removable mount (S1). |
| Client local helper | root (via client sudo) | client | Only the exact Prepare commands decided in M6. |

A **standalone** install is a server with `magvaultd` running and no clients.

"No APIs, no HTML, no web stack" (README:12) holds: there is no network listener. The gate is reachable only as an SSH forced command, and `magvaultd`'s socket is a local Unix socket.

### 5.2 Where state lives

| State | Location | Authority |
|---|---|---|
| Roots and settings | config dir: `path.conf`, `magvault.conf` | file |
| Camera / encoder profiles | `cameras/*.cam`, `encoders/*.enc` | file (parsed into memory, M13) |
| Original media + manifest | `mags/<folder>/`, `magcard.txt` `# manifest`, `ascmhl/` | file, fixed at ingest (C2) |
| Mag/project/clip metadata, logging | SQLite | **database** (written through to `magcard.txt`, C3) |
| Placements | SQLite + on-zmag manifest | database (manifest rebuilds it) |
| Exports | `bins/<name>/exportcard.txt` | **file** (DB rows are an index rebuilt from cards; PLAN:699) |
| Jobs, requests | SQLite | database (not recoverable from files) |
| Telemetry | `telemetry_live` (tmpfs) + `telemetry_disk` copy | derived |
| Logs | `logs/YYYY-MM-DD.log` | file, never in DB |
| Client keys | `/etc/magvault/client_keys` | file |
| Client cache | client `client.sqlite`, proxy cache, staging | derived; outbox is the client's pending changes |

### 5.3 Config resolution

Order: `--config <dir>` → `MAGVAULT_CONFIG` → pointer file `/etc/magvault/config-dir` (server) or `~/.config/magvault/config-dir` (client) → first-run setup prompt. The pointer file contains one absolute path. ("system pointer file", PLAN:840.)

Relative paths in `path.conf` resolve against the config directory. On start, every root is checked (exists, writable by the running user); missing roots that "can be created" are created; any error stops `magvaultd` with an ERROR log line and a non-zero exit.

Changing `db`, `mags`, `proxies`, `bins`, `logs` or `telemetry_*` in Settings takes effect after `magvaultd` restarts; Settings says so after save. Profile folders reload live.

### 5.4 Concurrency

- SQLite in WAL mode, `busy_timeout = 5000`, `foreign_keys = ON`. Transactions short; no transaction spans an external command.
- Only `magvaultd` changes job state from `queued` onwards. TUIs insert jobs (`queued`) and may set `cancel_requested = 1`; the daemon acts on it.
- File writes to cards use `flock` on the card file plus atomic rename (D2).
- Job classes and default concurrency: encode 1 (configurable, M22), zmag copy 1 per zmag, cold pull 1 per zmag, export copy 1, incoming commit 1.

### 5.5 Job model

All long operations are rows in job tables (§7). States: `queued → running → done | failed | cancelled`; encoder jobs also `paused` (queue-level). On daemon start, `running` rows become `failed` with `error = 'interrupted'` (encoder jobs: back to `queued`), partial outputs are deleted, a WARN line is logged (D6). Retry = new attempt on the same row (`attempts += 1`). Every job is idempotent: re-run rsync, re-verify everything.

The daemon wakes on a socket notification from a TUI or gate request, and polls every 2 s as a fallback. This answers PLAN:903: the 320p job is inserted in the same transaction that marks the mag `online`.

### 5.6 Client ↔ server

Transport: `ssh -i <client key> magvaultclient@<host> <operation>` with `StrictHostKeyChecking yes` and the known_hosts line from hand-over (S11).

`authorized_keys` line produced by `magvault-keys`:

```text
restrict,command="/usr/lib/magvault/magvault-gate --client dit-cart-a" ssh-ed25519 AAAAC3Nz... dit-cart-a
```

sshd configuration (installed once, by root):

```text
Match User magvaultclient
    AuthorizedKeysFile none
    AuthorizedKeysCommand /usr/lib/magvault/magvault-keys %u
    AuthorizedKeysCommandUser magvault
    PasswordAuthentication no
```

Gate operations (`SSH_ORIGINAL_COMMAND`, first word):

| Operation | Action |
|---|---|
| `rpc` | Pipe newline-delimited JSON between client and `/run/magvault/gate.sock`. Daemon adds `client = <label>` to each request. |
| `put-incoming` | `exec rrsync -wo <incoming>/<label>` (client uploads a verified mag folder). |
| `get-proxies` | `exec rrsync -ro <proxies>/db` |
| `get-bins` | `exec rrsync -ro <bins>` |
| `dashboard` | **[decision]** read-only server dashboard with a PTY (needs `pty` added to the key options), or dropped in favour of the client dashboard pulling `status.summary`. |

RPC verbs (client-allowed). Requests: `{"id":n,"verb":"...","args":{...}}`. Replies: `{"id":n,"ok":true,"result":...}` or `{"id":n,"ok":false,"error":{"code":"...","message":"..."}}`.

| Verb | Purpose |
|---|---|
| `status.summary` | connection state, queue depth (PLAN:830) |
| `profiles.list` | camera and encoder profiles (PLAN:196) |
| `projects.list` / `projects.create` | open projects for ingest; create with client-generated UUID |
| `catalog.changes(since)` | projects, mags, clips, tags, people changed since a server sequence number (client cache sync) |
| `incoming.commit(mag_folder)` | after `put-incoming`: daemon verifies against the uploaded `magcard.txt`, moves into `mags/`, inserts rows, queues 320p; replies `committed` or an error. This is "server confirms" (PLAN:454). |
| `logging.apply(changes[])` | outbox batch of field edits with client timestamps (D7) |
| `export.request(card)` | client draft card → server writes `bins/<name>/exportcard.txt`, queues copy and cold pulls |
| `export.status(name)` | per-clip status for the client's pending requests |
| `proxies.list(mag_ids[])` | which proxies are ready, with sizes, for `get-proxies` |

No admin verb is reachable through the gate (S10).

### 5.7 Client cache and outbox

`client.sqlite` holds a copy of `projects`, `mags`, `clips`, `clip_tags`, `clip_people`, tag and people vocabularies (from `catalog.changes`), the client's own local mags (staging/USB) and an `outbox` table:

```sql
CREATE TABLE outbox (
  seq        INTEGER PRIMARY KEY AUTOINCREMENT,
  kind       TEXT NOT NULL,   -- logging.field | export.request | incoming.commit | projects.create
  payload    TEXT NOT NULL,   -- JSON
  created_at TEXT NOT NULL,   -- UTC ISO-8601
  sent_at    TEXT,
  result     TEXT
);
```

The header's "N changes pending sync" is `COUNT(*) WHERE sent_at IS NULL`. On connect the client sends the outbox in order, then pulls `catalog.changes`.

---

## 6. Module specifications

Shared rules (resolving C7, C16, T7):

- `Esc` = back one level (popup → edit mode → secondary screen → dashboard). `q` quits on the dashboard only.
- Text fields: `Enter` enters edit mode; in edit mode every printable key is text; `Enter` or `Tab` commits, `Esc` cancels the edit. Module shortcuts are active only outside edit mode.
- `Esc` is refused (status line message) while the current screen owns a running write; jobs run by `magvaultd` never block `Esc`.
- Minimum 120x36; below that a "resize terminal" message. Text that does not fit is cut with `…`; footers drop low-priority hints first.
- Every action that changes something writes one log line `<UTC>  <LEVEL>  <module>  <message>`.
- Admin-only modules are hidden unless uid = `magvault` and not under the gate; they also refuse to start otherwise.

Dashboard letters (server): `i` Ingest, `g` Logging, `l` Library, `e` Export bins, `c` Encoder, `z` Zmag manager, `p` Profile Editor\*, `k` Key module\*, `v` Logs, `s` Settings\*, `r` refresh, `q` quit. (\* admin.) Client: `i`, `g`, `l`, `s` Settings, `d` server dashboard **[decision]**, `r`, `q`.

### 6.1 Dashboard

- **State:** telemetry snapshot, its age ("telemetry updated 12s ago"), counts, recent events.
- **Recent events (M23):** the newest 50 INFO/WARN/ERROR lines from today's and yesterday's log files, filtered to modules ingest, encoder, zmag, library, system. No new storage.
- **Pending requests:** `cold_pull_requests` with status `pending` or `pulling` plus export copy jobs `queued` or `running`.
- **Client:** `status.summary` every 30 s when connected; offline shows "offline", hides queue depth (dashboard.md:230).
- **Errors:** telemetry missing or older than 5 minutes → header shows "telemetry stale" in warning colour; `zpool status` failure → zmag pane shows the error text.

### 6.2 Ingest

- **Screens:** ready, pulling (inputs locked), done/failed summary. Panes as ingest.md:175-183.
- **State machine (per ingest):** `probing → ready → copying → verifying → writing_card → committing → done | failed | aborted`.
- **Pull (`p`), preconditions:** camera, format, gamma, gamut, shoot day, project, ≥1 target (ingest.md:202); free space on each selected target ≥ total size + 1 %; no symlinks/devices on the card; no file name with `|` or control characters (S9).
- **Side effects, server/standalone:**
  1. New mag UUID (v4). Folder `YYYYMMDD_<uuid>`.
  2. `rsync -rt --no-links --no-devices --no-specials --info=progress2 <card>/ <incoming>/<folder>/` (junk excluded by pattern list).
  3. SHA-256 every source and destination file concurrently; compare sizes and hashes.
  4. Write `magcard.txt` and ASC MHL into the incoming folder.
  5. `rename(incoming/<folder>, mags/<folder>)`.
  6. One transaction: `mags` (status `online`), `files`, `clips`, `encoder_jobs` (320p default profile).
  7. Log INFO.
- **Side effects, client:** steps 1-4 into the staging root (or USB-as-staging). Then for each extra target: rsync from staging, verify by hashing the copy against the magcard manifest, update that drive's `magvault-backup.txt`. Server target: `put-incoming` rsync, then RPC `incoming.commit`; on `committed`, mark the mag `synced` in every `magvault-backup.txt` that lists it and autoclean staging (PLAN:454). Offline: server step stays pending; warning shown if staging is the only copy (PLAN:458).
- **Abort (`x`):** stop rsync (SIGTERM), delete the incoming/staging folder for this mag, no DB rows, log WARN "aborted".
- **Card removed / I/O error:** state `failed`, partial folder deleted, log ERROR with the first rsync error line. Retry starts over.
- **Sync pending (M5):** a key on the Targets pane (e.g. `y`) **[decision on letter]**, enabled when the selected target lists mags pending server sync.
- **Prepare (`o`):** per M6 **[decision]**.

### 6.3 Logging

- **Panes:** Mags (with project picker in the pane title, M15), Clips, Player, Log clip. Client: source picker (Server cache / staging / USB `<label>`).
- **State:** selected project, mag, clip; field edit buffers; per-field undo stack for `Ctrl+Z`; dirty flags.
- **Save:** on field exit, write the field to SQLite (server) or to `client.sqlite` + outbox (client). On leaving the clip or the module, rewrite `magcard.txt` `# logging` section for that mag (server only), preceded by a backup to `.magvault/magcard.<YYYYMMDDTHHMMSSZ>.backup` (C3, C4). Keep the newest 20 backups per mag.
- **Logged flag:** a clip is logged if any logging field is non-empty.
- **`c` complete:** sets `mags.logging_complete = 1`; second press asks to clear.
- **`v` revert:** loads the newest backup's `# logging` row for the current clip into the form as unsaved edits; operator confirms with save.
- **Tags/people:** vocabulary is global. `[create "…"]` inserts into `tags` / `people` (§7).
- **Shot type:** single choice from `shot_types` in `magvault.conf`.
- **Player:** mpv with `--input-ipc-server=<runtime dir>/mpv-<pid>.sock`, `--vo=kitty` or `--vo=sixel` sized to the player rectangle when supported, otherwise `--vo=null` and status line. Proxy icon: ready / pending / pulling / local fallback.
- **Local fallback proxy (client):** `ffmpeg` with the default 320p profile into `<proxy cache>/tmp/<mag folder>/…`, deleted when the server proxy arrives.

### 6.4 Encoder

- **Queue screen:** panes per encoder.md:184-192. Header: queue RUNNING/PAUSED, depth.
- **Keys:** `n` new job, `p` pause/resume queue (stops starting new jobs; running jobs finish), `r` retry failed, `c` cancel (queued: immediately; running: SIGTERM to ffmpeg, partial output deleted), `u` re-queue done/cancelled job, `+`/`-` priority (`low`/`normal`/`high`), `e` Profile Editor (admin only).
- **Ordering:** priority desc, then `created_at`.
- **Per-clip execution:** one ffmpeg process per clip. Job progress = clips done / clips total, plus current clip percentage from `-progress pipe:1` `out_time_us` against the source duration.
- **Command shape (320p):** `ffmpeg -hide_banner -nostdin -i <src> -map 0:v:0 -map 0:a? -c:v <codec> -vf scale=-2:<height> -fps_mode passthrough <rate control> -c:a <audio_codec> -b:a <audio_bitrate> -progress pipe:1 <tmp out>`; rename on success to `proxies/db/<mag folder>/<relative_path>.<container>` (D12). Audio: every input audio stream mapped, channel layout kept (P10).
- **Failure reasons** (one line, encoder.md:190): `source missing`, `bad profile: <id>`, `no space left`, `ffmpeg exit <n>: <last stderr line>`, `unsupported codec` (RAW, P10). Full stderr tail (last 40 lines) goes into the log entry as context lines.
- **New job:** online mags only, open projects only; destination fixed `custom_encode/<mag folder>/<profile id>/` (PLAN:636; use `id`, not `name`, for the folder).
- **Client:** read-only queue view via `status.summary` **[decision S10]**.

### 6.5 Profile Editor (admin)

- **Screens:** Camera, Encoder (Type pane), list, edit form, file preview + checks (profile-editor.md:188-195).
- **Validation:** required keys (PLAN:233-238, 291-295); every `default_*` in its list; `id` = filename stem and matches `^[a-z0-9][a-z0-9-]{0,63}$`; `type` ∈ {`320p`,`hq`,`custom`}; `rate_control` ∈ {`crf`,`bitrate`} with the matching value; `height` integer or `source`. Save blocked until valid.
- **Unknown keys** in existing files are kept verbatim on save (forward compatibility, PLAN:237).
- **Delete:** confirm; if the id is referenced by a queued/running job or is a configured default, refuse with the reason.
- **Renaming `id`:** treated as duplicate-then-delete.
- **Reload:** re-parse folder; invalid files are listed with their error and not served.

### 6.6 Zmag manager

- **Main screen:** Server mags (copies column per P5), Zmag contents (tabs), Operations.
- **Recognition (C12):** a udev/poll watcher lists importable pools named `zmag-NN`. The manager shows "zmag-NN available: import?" and asks for the sudo password. Import: `zmag-helper import <name>` → `zpool import -N`, check `magvault:type=zmag` on the root dataset, mount, read on-zmag manifest. Mismatch → export again, log WARN, ignore.
- **Create (`n`):** list drives from `lsblk -J -o NAME,MODEL,SIZE,TYPE,FSTYPE,PTTYPE,SERIAL,WWN` with no partitions and no filesystem signature; show `/dev/disk/by-id` name and model. Exactly four. Name = next free `zmag-NN` across DB `zmags` and visible pools. **Role** picker `primary`/`backup` (M16). Confirm → `zmag-helper create <name> <role> <id1> <id2> <id3> <id4>` → `zpool create -o ashift=12 -O recordsize=1M -O compression=lz4 -O atime=off -O magvault:type=zmag -O magvault:role=<role> <name> raidz1 <ids>`; write on-zmag manifest; insert `zmags` row.
- **Copy (`c`):** preflight free space; `placements` row `copying`; `rsync -rt --info=progress2 mags/<folder>/ <zmag>/.incoming/<folder>/`; hash every file against the magcard manifest; rename into `<zmag>/<folder>/`; placement `complete`, `copy_number` = number of complete placements for that mag after this one; update on-zmag manifest; optionally add an ASC MHL generation (D3 **[decision]**).
- **Auto-backup (`a`), defined against roles (M16):** when ON and a zmag is imported, queue copies of every `online` mag that has no `complete` placement on any zmag of the **same role**, oldest `ingested_at` first, until the zmag is full (skip, log WARN). So inserting a primary gives first copies; inserting a backup gives second copies.
- **Eject (M2) [decision on letter]:** refuse while a job uses the zmag; `zmag-helper export <name>`; zmag shown as offline.
- **Evict (M1) [decision on where]:** for a mag with ≥ 2 complete placements on zmags of different roles and no pending export copy: confirm, `rm -rf mags/<folder>` as `magvault`, proxies stay, `mags.status = cold`, log INFO.
- **Cold pull:** from `cold_pull_requests`. Source = a locally mounted zmag with a complete placement, else a remote station holding one, else status `dock needed: zmag-NN, zmag-MM`. Copy into `restore/<folder>/`, verify against manifest, run mismatch checks (M19), rename into `mags/`, `mags.status = online`, request `complete`, wake waiting export jobs.
- **Snapshots (`s`):** list `zfs list -t snapshot -o name,creation,used,userrefs` for the zmag; "mag snapshots" filtered as in C20. Hold/release/destroy via helper with sudo. Creation policy **[decision M18]**.
- **Health (`p`):** `zpool status -P <name>` (unprivileged), SMART via `zmag-helper smart-read <by-id>`; scrub, self-test via helper. Replace: pick a raw drive (same list rules as create), confirm, `zmag-helper replace <pool> <old> <new>`; resilver progress from `zpool status`.
- **Remote stations:** read-only tab with what the station reports; jobs sent to it via gate station verbs (S13, Phase 5).

### 6.7 Library

- **Panes:** Projects (open + closed), Mags (warm/cold/pulling), Clips (`+` on list; client proxy status), Player + export list.
- **Current export:** none until the operator opens one (from Export bins `e`/`l`) or presses `a` with none open, which prompts for a name (validated per S9) and creates `bins/<name>/exportcard.txt` (server) or a local draft card (client) (M3).
- **Add (`a`):** backup card → write card with the clip appended (D2). Server: `export_clips` index updated in the same step.
- **Remove [decision on letter]:** same write path, clip removed. Files already copied into the bin are not deleted (bins are never deleted by Magvault).
- **In/out [decision on letter]:** text entry `HH:MM:SS` or `HH:MM:SS:FF`, relative to clip start, stored on the card only as notes for the editor; the copy is always the whole file and its sidecars.
- **Export (`e`), server:** preflight space; for each card line not yet present and verified in the bin: warm → queue export copy; cold → create/attach `cold_pull_requests` with `export_id`. Copy = `rsync` file and sidecars to `bins/<name>/<mag folder>/<relative_path>.partial` then rename; verify SHA-256 against `files.sha256`.
- **Export (`e`), client:** outbox `export.request` with the draft card; on success the server's card replaces the draft; status via `export.status`. Pull finished files with `get-bins` into the client export root (M20).
- **Project actions (M4) [decision on letters]:** close/reopen, rename (label only).

### 6.8 Export bins browser

- **Discovery:** walk `bins/*/exportcard.txt` on open and on `r`; parse cards; unknown folders without a card are listed as "no card".
- **Per-clip status** (PLAN:732-736): `done` = file and sidecars present in the bin and verified; `copying` = an export copy job for it is running; `cold` = source mag `cold` or `pulling`; `warm` = source mag `online` and not yet copied.
- **Per-export status:** `cold` if any clip is cold; else `copying` if any is copying; else `warm` if any is warm; else `done`.
- **Date column:** card `created`. On-disk pane shows clips and sidecars with relative paths.
- **Server only** unless decided otherwise; clients see their requests through Library.

### 6.9 Logs

- **Source:** `logs/*.log`, newest file first, newest line first (PLAN:314, 317).
- **Parsing:** a line starting with a timestamp is an entry; following lines starting with two spaces are its context (D11). Unparseable lines are shown as `?` level entries.
- **Filters:** module (`m` cycles), level (`l` cycles). Follow (`f`): inotify on today's file and on the logs directory (day rollover); selection pinned to the newest entry.
- **Retention:** `magvaultd` at start and daily at 00:05 UTC deletes `^\d{4}-\d{2}-\d{2}\.log$` files whose date is older than `log_retention_days`.

### 6.10 Settings

- **Server (admin):** every `path.conf` key with validation (exists / will be created / error); `magvault.conf` keys from §8.2 with type checks. `Ctrl+S` writes both files atomically (backup `*.bak` first), logs INFO with the changed keys (not values of anything secret; none are secret today). Banner: "restart magvaultd to apply root changes" when a root changed.
- **Client:** proxy cache root, staging root, export download root, server host, server user (fixed `magvaultclient`, shown read-only), private key path, "import server host key" from the hand-over file (M20, S11).

### 6.11 Key module (admin)

- **List:** label, fingerprint, key type from `/etc/magvault/client_keys`.
- **Generate (`g`):** label (unique, S6 pattern) → Ed25519 generated in memory → optional passphrase → choose target: `u` removable drive (must already be mounted, or mounted via helper) or `w` file path that must resolve under a removable mount (C13). Write `magvault-client.key` (0600 where the filesystem supports it) and `magvault-server.known_hosts`. Only after a successful write and `fsync`: append the public key line to `client_keys`, log INFO `keys  Client key added: <label> <fingerprint>`. Key body is never rendered (S7). Memory zeroed on leaving the screen.
- **Paste (`p`):** strict parse (S6) → fingerprint shown → `Enter` appends.
- **Revoke (`x`):** confirm → remove line (atomic rewrite) → terminate live gate processes for the label (S8) → log INFO.

---

## 7. Data model

### 7.1 Schema (SQLite, refined)

Changes from PLAN:87-179, with the finding that motivates each: relative paths only (D4); `files` separated from `clips` (M10); people as a table (C6); tags with a vocabulary; `zmags` table (M16); job tables carry type, priority, subsets and progress (M22, §6.4); cold pulls link to exports (M17); export copy jobs (export-bins status); `meta` for schema version. Timestamps are UTC ISO-8601 text (`2026-11-22T19:21:03Z`). UUIDs are lowercase canonical text.

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
-- ('schema_version','1'), ('catalog_seq','0')

CREATE TABLE projects (
  id              TEXT PRIMARY KEY,                 -- UUID
  name            TEXT NOT NULL,                    -- label only
  season_or_year  TEXT,
  notes           TEXT,
  status          TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','closed')),
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  seq             INTEGER NOT NULL                  -- catalog change sequence (client sync)
);

CREATE TABLE mags (
  id               TEXT PRIMARY KEY,                -- UUID
  project_id       TEXT NOT NULL REFERENCES projects(id),
  folder           TEXT NOT NULL UNIQUE,            -- YYYYMMDD_<uuid>, relative to mags root
  shoot_date       TEXT NOT NULL,                   -- YYYY-MM-DD
  camera_profile_id TEXT NOT NULL,                  -- .cam id at ingest (C5)
  camera_name      TEXT NOT NULL,                   -- profile name at ingest
  file_format      TEXT NOT NULL,
  gamma            TEXT NOT NULL,
  gamut            TEXT NOT NULL,
  status           TEXT NOT NULL CHECK (status IN ('ingesting','online','cold','pulling','failed')),
  logging_complete INTEGER NOT NULL DEFAULT 0,
  size_bytes       INTEGER NOT NULL,
  file_count       INTEGER NOT NULL,
  source_label     TEXT,                            -- card volume label
  ingested_by      TEXT NOT NULL,                   -- identity (M12)
  ingested_at      TEXT NOT NULL,
  notes            TEXT,
  updated_at       TEXT NOT NULL,
  seq              INTEGER NOT NULL
);

-- Every file in the mag folder that came from the card (the manifest).
CREATE TABLE files (
  mag_id         TEXT NOT NULL REFERENCES mags(id),
  relative_path  TEXT NOT NULL,                     -- POSIX, no leading '/', no '..'
  size           INTEGER NOT NULL,
  mtime          TEXT NOT NULL,
  sha256         TEXT NOT NULL,
  role           TEXT NOT NULL CHECK (role IN ('media','sidecar','other')),
  clip_id        TEXT REFERENCES clips(id),         -- media: its clip; sidecar: the clip it belongs to (nullable)
  PRIMARY KEY (mag_id, relative_path)
);

-- One row per media file (v1: a clip is one media file, M10).
CREATE TABLE clips (
  id                TEXT PRIMARY KEY,
  mag_id            TEXT NOT NULL REFERENCES mags(id),
  relative_path     TEXT NOT NULL,                  -- the media file
  original_filename TEXT NOT NULL,                  -- basename, for display
  duration_ms       INTEGER,                        -- from ffprobe at ingest (player, in/out)
  fps               TEXT,                           -- rational, e.g. '24000/1001'
  width             INTEGER,
  height            INTEGER,
  audio_streams     INTEGER,
  audio_channels    TEXT,                           -- e.g. '2,1,1' per stream
  proxy_status      TEXT NOT NULL DEFAULT 'none' CHECK (proxy_status IN ('none','pending','ready','failed')),
  hq_status         TEXT NOT NULL DEFAULT 'none' CHECK (hq_status IN ('none','pending','ready','failed')),
  description       TEXT,
  rating            INTEGER CHECK (rating BETWEEN 1 AND 5),
  slow_motion       INTEGER NOT NULL DEFAULT 0,
  technical_note    TEXT,
  shot_type         TEXT,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  seq               INTEGER NOT NULL,
  UNIQUE (mag_id, relative_path)
);
-- proxy path is derived: proxies/db/<mags.folder>/<relative_path>.<container> (D12)

CREATE TABLE tags (name TEXT PRIMARY KEY COLLATE NOCASE, created_at TEXT NOT NULL);
CREATE TABLE clip_tags (
  clip_id TEXT NOT NULL REFERENCES clips(id),
  tag     TEXT NOT NULL REFERENCES tags(name),
  PRIMARY KEY (clip_id, tag)
);

CREATE TABLE people (name TEXT PRIMARY KEY COLLATE NOCASE, created_at TEXT NOT NULL);
CREATE TABLE clip_people (
  clip_id TEXT NOT NULL REFERENCES clips(id),
  role    TEXT NOT NULL CHECK (role IN ('camera_operator','director_producer')),
  name    TEXT NOT NULL REFERENCES people(name),
  PRIMARY KEY (clip_id, role, name)
);

-- Per-field edit times for last-writer-wins sync (D7).
CREATE TABLE clip_field_times (
  clip_id    TEXT NOT NULL REFERENCES clips(id),
  field      TEXT NOT NULL,       -- description | rating | slow_motion | technical_note | shot_type | tags | camera_operator | director_producer
  edited_at  TEXT NOT NULL,
  edited_by  TEXT NOT NULL,
  PRIMARY KEY (clip_id, field)
);

CREATE TABLE zmags (
  id          TEXT PRIMARY KEY,   -- 'zmag-01'
  role        TEXT NOT NULL CHECK (role IN ('primary','backup')),
  pool_guid   TEXT NOT NULL UNIQUE,
  station     TEXT NOT NULL DEFAULT 'local',  -- 'local' or remote station label
  capacity_bytes INTEGER,
  created_at  TEXT NOT NULL,
  last_seen_at TEXT,
  notes       TEXT
);

CREATE TABLE placements (
  id          TEXT PRIMARY KEY,
  mag_id      TEXT NOT NULL REFERENCES mags(id),
  zmag_id     TEXT NOT NULL REFERENCES zmags(id),
  copy_number INTEGER,
  status      TEXT NOT NULL CHECK (status IN ('copying','complete','failed','suspect')),
  copied_at   TEXT,
  verified_at TEXT,
  error       TEXT,
  notes       TEXT,
  UNIQUE (mag_id, zmag_id)
);

CREATE TABLE exports (                      -- index of bins/*/exportcard.txt (cards are authoritative)
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,          -- folder name under bins/
  project_id  TEXT REFERENCES projects(id),
  created_by  TEXT NOT NULL,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  notes       TEXT
);

CREATE TABLE export_clips (
  export_id  TEXT NOT NULL REFERENCES exports(id),
  clip_id    TEXT NOT NULL REFERENCES clips(id),
  in_point   TEXT,
  out_point  TEXT,
  copy_status TEXT NOT NULL DEFAULT 'pending' CHECK (copy_status IN ('pending','copying','done','failed')),
  PRIMARY KEY (export_id, clip_id)
);

CREATE TABLE cold_pull_requests (
  id           TEXT PRIMARY KEY,
  mag_id       TEXT NOT NULL REFERENCES mags(id),
  zmag_id      TEXT REFERENCES zmags(id),     -- chosen source, null until chosen
  export_id    TEXT REFERENCES exports(id),   -- M17
  requested_by TEXT NOT NULL,
  status       TEXT NOT NULL CHECK (status IN ('pending','dock_needed','pulling','complete','failed','cancelled')),
  error        TEXT,
  created_at   TEXT NOT NULL,
  completed_at TEXT,
  notes        TEXT
);

CREATE TABLE encoder_jobs (
  id          TEXT PRIMARY KEY,
  type        TEXT NOT NULL CHECK (type IN ('320p','hq','custom')),
  mag_id      TEXT NOT NULL REFERENCES mags(id),
  profile_id  TEXT NOT NULL,
  priority    INTEGER NOT NULL DEFAULT 0,     -- -1 low, 0 normal, 1 high
  status      TEXT NOT NULL CHECK (status IN ('queued','running','done','failed','cancelled')),
  cancel_requested INTEGER NOT NULL DEFAULT 0,
  clips_total INTEGER NOT NULL,
  clips_done  INTEGER NOT NULL DEFAULT 0,
  progress    INTEGER NOT NULL DEFAULT 0,     -- 0-100
  output_root TEXT NOT NULL,                  -- relative: 'proxies/db/<folder>' etc.
  attempts    INTEGER NOT NULL DEFAULT 0,
  requested_by TEXT NOT NULL,
  created_at  TEXT NOT NULL,
  started_at  TEXT,
  finished_at TEXT,
  error       TEXT
);
CREATE TABLE encoder_job_clips (                -- subset; absent rows = whole mag
  job_id  TEXT NOT NULL REFERENCES encoder_jobs(id),
  clip_id TEXT NOT NULL REFERENCES clips(id),
  status  TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued','running','done','failed')),
  error   TEXT,
  PRIMARY KEY (job_id, clip_id)
);

CREATE TABLE copy_jobs (                        -- zmag copy, export copy, incoming commit
  id          TEXT PRIMARY KEY,
  kind        TEXT NOT NULL CHECK (kind IN ('zmag_copy','export_copy','incoming_commit')),
  mag_id      TEXT REFERENCES mags(id),
  zmag_id     TEXT REFERENCES zmags(id),
  export_id   TEXT REFERENCES exports(id),
  status      TEXT NOT NULL CHECK (status IN ('queued','running','done','failed','cancelled')),
  cancel_requested INTEGER NOT NULL DEFAULT 0,
  bytes_total INTEGER,
  bytes_done  INTEGER NOT NULL DEFAULT 0,
  attempts    INTEGER NOT NULL DEFAULT 0,
  requested_by TEXT NOT NULL,
  created_at  TEXT NOT NULL,
  started_at  TEXT,
  finished_at TEXT,
  error       TEXT
);

CREATE INDEX clips_by_mag       ON clips(mag_id);
CREATE INDEX mags_by_project    ON mags(project_id);
CREATE INDEX placements_by_mag  ON placements(mag_id);
CREATE INDEX jobs_queue         ON encoder_jobs(status, priority DESC, created_at);
```

Notes:

- `mags.dump_path` (PLAN:102) is replaced by `folder`; absolute paths are computed (D4).
- `clips.proxy_path` (PLAN:120) is dropped; the path is derived (D12).
- `exports.path` (PLAN:149) is dropped; it is `bins/<name>`.
- `camera_operator`, `director_producer` columns move to `clip_people` (C6).
- `seq` columns: every write increments `meta.catalog_seq` and stamps the row; `catalog.changes(since)` returns rows with `seq > since` (§5.6). Deletions are rare (none in v1 except tag/person unlink, which bumps the clip's `seq`).

### 7.2 Mag lifecycle

```text
ingesting ──verify ok──▶ online ──evict (≥2 complete placements, M1)──▶ cold
    │                      ▲                                             │
    └─fail─▶ failed        └──────────── cold pull complete ◀── pulling ◀┘
```

`archived` (PLAN:108) is dropped unless it is given a meaning **[decision]**.

### 7.3 Flow → table check

| Flow | Tables written |
|---|---|
| Ingest (server) | `projects` (if new), `mags`, `files`, `clips`, `encoder_jobs` (+ `copy_jobs incoming_commit` for clients) |
| Encode | `encoder_jobs`, `encoder_job_clips`, `clips.proxy_status`/`hq_status` |
| Logging | `clips`, `tags`, `clip_tags`, `people`, `clip_people`, `clip_field_times`, `mags.logging_complete` |
| Export | `exports`, `export_clips`, `copy_jobs export_copy`, `cold_pull_requests` |
| Zmag create/copy | `zmags`, `placements`, `copy_jobs zmag_copy` |
| Cold pull | `cold_pull_requests`, `mags.status`, `placements.verified_at` |
| Evict | `mags.status` |
| Keys, settings, profiles | none (files) |

---

## 8. File formats

All Magvault text files: UTF-8, LF line endings (CRLF accepted on read), `#` starts a comment line, header lines are `key: value` (first `:` splits; value trimmed). Unknown keys are preserved on rewrite. Every file starts with a `format:` line so later versions can migrate it (nice to have, cheap now).

### 8.1 `path.conf`

```text
format: magvault-path/1
mags: /srv/magvault/mags
incoming: /srv/magvault/incoming        # same dataset as mags (rename must be atomic)
restore: /srv/magvault/restore          # same dataset as mags
proxies: /srv/magvault/proxies
custom_encode: /srv/magvault/custom_encode
bins: /srv/bins
db: /srv/magvault/db                    # directory; file is magvault.sqlite
logs: /srv/magvault/logs
cameras: cameras                        # relative = relative to the config dir
encoders: encoders
telemetry_live: /dev/shm/magvault-telemetry.json
telemetry_disk: /srv/magvault/state/telemetry-latest.json
```

`incoming` and `restore` are new keys required by D1. **[decision]** Alternative: hidden `.incoming/` inside `mags/`, which bends "flat, no subfolders" (PLAN:36).

Client `path.conf`:

```text
format: magvault-client-path/1
proxy_cache: ~/.local/share/magvault/proxies
staging: ~/.local/share/magvault/staging
export_downloads: ~/magvault-exports
server_host: server.example
server_key: ~/.ssh/magvault-client.key
server_known_hosts: ~/.config/magvault/known_hosts
```

### 8.2 `magvault.conf`

```text
format: magvault/1
log_retention_days: 30                  # PLAN:320
telemetry_disk_interval_minutes: 30     # PLAN:821
auto_backup: on                         # Zmag manager 'a' state
default_profile_320p: default-320p      # M14
default_profile_hq: hevc-hq             # M14
encoder_parallel_jobs: 1                # M22
shot_types: wide, medium, close, aerial # M15
evict_min_placements: 2                 # M1
```

### 8.3 `magcard.txt`

```text
format: magvault-magcard/1
mag_uuid: 3f6c2a1e-9b0d-4c55-8e21-a3f9c2d41b77
mag_folder: 20250520_3f6c2a1e-9b0d-4c55-8e21-a3f9c2d41b77
project_uuid: 6d1e…
project_name: ACME_Campaign_2025
project_status: open
shoot_date: 2025-05-20
camera_profile_id: sony-a7siii
camera_model: Sony A7S III
file_format: XAVC S
gamma: S-Log3
gamut: S-Gamut3.Cine
source_label: A7SIII_128
ingested_by: dit-cart-a
ingested_at: 2025-05-20T19:20:11Z
updated_at: 2025-05-21T08:02:40Z
notes: Second unit, stage left

# manifest
# relative_path | size | sha256 | mtime
PRIVATE/M4ROOT/CLIP/C0001.MP4 | 4123456789 | 9f2c… | 2025-05-20T14:02:11Z
PRIVATE/M4ROOT/CLIP/C0001M01.XML | 4567 | 1ab0… | 2025-05-20T14:02:11Z

# logging
# relative_path | rating | slow_motion | shot_type | tags | camera_operator | director_producer | description | technical_note
PRIVATE/M4ROOT/CLIP/C0001.MP4 | 4 | 0 | wide | sunset; crowd | Thabo | Naledi | Sun setting behind the stage | slight focus hunt at start
```

Rules: fields separated by ` | ` (space-pipe-space); a field may not contain `|` or a line break (rejected at entry; file names with `|` are rejected at probe, S9). Lists inside a field use `; `. The manifest section is written once at ingest and never changed (C2). The header and the logging section are rewritten from the database (PLAN:417-421; C3). Before any rewrite, copy to `.magvault/magcard.<YYYYMMDDTHHMMSSZ>.backup`.

### 8.4 ASC MHL

Written by an ASC MHL implementation into `ascmhl/` using the standard's own file naming and chain file (D3), hash format SHA-256, ignore patterns `magcard.txt` and `.magvault`. A new generation is added after each verified zmag copy and cold pull **[decision]**.

### 8.5 `exportcard.txt`

```text
format: magvault-exportcard/1
name: Bamba-Cutaways
project_uuid: …
project_name: Bamba_Cutaways
created: 2026-11-22T14:30:00Z
created_by: magvault@vault01
updated: 2026-11-22T16:15:00Z
notes: rough cut cutaways

# clips
# mag_folder | relative_path | optional_in | optional_out
20250517_0cd8e1…-… | PRIVATE/M4ROOT/CLIP/C0002.MP4 |  | 
20250517_0cd8e1…-… | PRIVATE/M4ROOT/CLIP/C0004.MP4 | 00:01:12 | 00:01:45
```

Changes from PLAN:712-715: `original_filename` + `source_path` replaced by `relative_path` (the original file name is its last component; D4, S9, C19). Timestamps UTC with `Z`. Backup rule unchanged (PLAN:718-720), under `flock` (D2).

### 8.6 `magvault-backup.txt` (field drive root)

```text
format: magvault-backup/1
role: master            # [decision M7]
label: BACKUP1
created: 2026-11-20T06:12:00Z
host: dit-cart-a

# mags
# mag_folder | state | updated
20250520_3f6c… | pending | 2026-11-22T19:21:00Z
20250518_7be4… | synced  | 2026-11-22T18:52:00Z
```

### 8.7 On-zmag manifest

At `<pool mountpoint>/magvault-zmag.txt` (PLAN:905):

```text
format: magvault-zmag/1
id: zmag-01
role: primary
pool_guid: 1234567890123456789
created: 2026-03-02T10:00:00Z

# mags
# mag_uuid | mag_folder | copied_at | verified_at
3f6c2a1e-… | 20250520_3f6c… | 2026-11-22T20:10:00Z | 2026-11-22T20:41:00Z
```

### 8.8 Log lines

```text
2026-11-22T21:15:05Z  ERROR  zmag      zmag-02 degraded: sdg faulted
  pool: zmag-02
  state: DEGRADED
```

Timestamp UTC, two spaces, level padded to 5, two spaces, module padded to 8, two spaces, message. Context lines start with two spaces and are written in the same `write(2)` as their entry. Modules: `ingest`, `encoder`, `zmag`, `library`, `logging`, `settings`, `keys`, `profiles`, `system`, `gate`. File name uses the UTC date.

### 8.9 Telemetry JSON (`telemetry_live`)

```json
{
  "format": "magvault-telemetry/1",
  "written_at": "2026-11-22T19:21:30Z",
  "encoder": {
    "state": "running",
    "depth": 3,
    "active": [{"job_id": "…", "type": "320p", "mag_folder": "20250520_…", "project": "ACME_Campaign_2025",
                "clip_index": 3, "clip_total": 6, "percent": 62, "elapsed_s": 252}],
    "next": [{"type": "320p", "mag_folder": "…", "project": "…"}]
  },
  "working_datasets": [{"root": "mags", "dataset": "tank/magvault", "mountpoint": "/srv/magvault",
                        "state": "ONLINE", "used_bytes": 0, "avail_bytes": 0}],
  "zmags": [{"id": "zmag-02", "role": "backup", "station": "local", "state": "DEGRADED",
             "drives": [{"id": "ata-ST4000VN006-…", "short": "sdg", "state": "FAULTED"}]}],
  "counts": {"open_projects": 4, "online_mags": 37, "pending_requests": 2},
  "copies": [{"kind": "zmag_copy", "mag_folder": "…", "zmag": "zmag-02", "percent": 41,
              "bytes_done": 0, "bytes_total": 0}]
}
```

Written every 2 s while anything is running, every 30 s when idle, via temp file + rename (P9). Copied to `telemetry_disk` every `telemetry_disk_interval_minutes`.

### 8.10 `.cam` and `.enc`

As PLAN:200-296, with: `format:` line added; `id` pattern per §6.5; `height` = integer or `source`; `audio_channels` key is ignored on `.enc` if present (always source); `.cam` stays as is (sidecar rules can be added later without breaking files, since unknown keys are kept).

### 8.11 `/etc/magvault/client_keys`

One public key per line, `<type> <base64> <label>`. `magvault-keys` turns each into the restricted line in §5.6. Owned `magvault:magvault`, mode 0640.

---

## 9. Error handling and recovery

### 9.1 General rules

1. **Nothing lands under its final name until verified.** Copies go to `incoming/`, `restore/`, `<zmag>/.incoming/`, or `<file>.partial`, then `rename(2)` on the same filesystem (D1).
2. **Cards and config are written atomically**: temp file, `fsync`, rename, `fsync` the directory; card rewrites under `flock`; backup before rewrite (D2).
3. **Jobs are idempotent and restartable** (§5.5). Restart: `running` → `failed: interrupted` (encoder → `queued`), partial outputs removed, WARN logged (D6).
4. **Preflight free space** on every destination: `sum(sizes) × 1.01 + 1 GiB` (M9).
5. **One clear reason per failure** in the UI, full detail as log context lines.
6. **Nothing auto-deletes originals** except the explicit evict action, which re-checks placements in the same transaction.

### 9.2 Per path

| Path | Cancel | Source gone / I/O error | Destination full | Offline / network loss | Crash or reboot | Retry |
|---|---|---|---|---|---|---|
| Card → staging / mags (ingest) | `x`: kill rsync, delete partial folder, no DB rows | `failed`, partial deleted, ERROR with first rsync error | blocked by preflight; if it still happens: `failed` | n/a (local) | partial folder deleted on next start; card must be re-ingested | `p` again (new UUID) |
| Staging → USB mirror | per-target cancel | `failed` for that target only; others continue | preflight; else `failed` for that target | n/a | target left `pending` in `magvault-backup.txt` | "sync pending" |
| Staging → server (`put-incoming` + `incoming.commit`) | stop rsync; server deletes `incoming/<label>/<folder>` after 24 h idle | n/a | server replies `no_space`; staging kept | stays `pending`; outbox retries on connect | server: incoming folder kept for resume (rsync is resumable); client: still `pending` | automatic on connect, or "sync pending" |
| Commit on server (verify + move + DB) | not cancellable (seconds) | hash mismatch → `failed: verify`, folder kept in `incoming/` for inspection, client told | n/a (same dataset) | n/a | before rename: re-run verify; after rename but before DB: startup scan finds `mags/<folder>` without DB row → insert from `magcard.txt` | automatic |
| Encode | `c`: SIGTERM ffmpeg, delete temp output, `cancelled` | `failed: source missing` | `failed: no space left` | n/a | running → `queued` | `r` |
| Zmag copy | cancel: stop rsync, delete `<zmag>/.incoming/<folder>`, placement `failed` | `failed` | preflight; auto-backup skips mags that do not fit and logs WARN | zmag removed mid-copy: pool I/O error → `failed`, pool health shows state | `failed: interrupted`; `.incoming` cleaned on next import | `c` again |
| Cold pull | cancel request → `cancelled`; dependent export clips stay `cold` | `failed`; try another placement if one exists | preflight with whole-mag size (P3) | remote station unreachable → `dock_needed` | `failed: interrupted`; `restore/<folder>` deleted | automatic when a source zmag appears |
| Export copy | cancel job; copied files stay | clip `failed` | preflight | client sees `pending` until `export.status` succeeds | `.partial` files deleted | `e` again (copies only what is missing) |
| Proxy pull (client) | stop rsync | `missing` | warn, keep partial out of cache | stays `missing`; retry on connect | partial files have `.partial` suffix, removed on start | `p` |
| Profile / settings save | `Esc` → "Discard changes?" | n/a | write fails → error, old file intact (atomic) | n/a | old file intact | save again |
| Key generate | `Esc` → nothing installed, key zeroed | target unmounted → error, nothing installed | write fails → nothing installed | n/a | nothing installed | `g` again |

### 9.3 Database vs disk disagreements

| Detected | Action |
|---|---|
| `mags/<folder>` exists, no DB row | Read `magcard.txt`, verify sizes against manifest, insert as `online`, log WARN "adopted". |
| DB row `online`, folder missing | Set `status = failed`, log ERROR; if complete placements exist, offer cold pull. |
| Card header differs from DB | Back up card, rewrite header and logging from DB (PLAN:417-421). |
| File missing / size differs from manifest | Integrity ERROR; mag flagged; no automatic action (C2). |
| SHA-256 differs (explicit verify) | Integrity ERROR; placement or mag marked `suspect`/`failed`. |
| On-zmag manifest lists a mag with no placement row | Insert placement `complete` with `verified_at = NULL`, log WARN. |
| Placement row, mag absent on mounted zmag | Placement `failed`, log ERROR. |

### 9.4 Rebuild from files (DB lost)

Recovers: projects (UUID, name, status from magcards), mags, files, clips, logging (from `# logging`), tags/people vocabularies (from logging sections), exports (from cards), placements (as each zmag is mounted). Lost: jobs, cold-pull requests, edits made after a mag's last card rewrite, logging on mags whose cards are only on unmounted zmags. Implement as `magvault rebuild --from-files` (CLI, admin).

---

## 10. Security model

### 10.1 Principals

| Principal | What it is | Can |
|---|---|---|
| `magvault` (server) | system user running `magvaultd` and the admin TUI | everything in Magvault; privileged pool operations only through `zmag-helper` with the sudo password |
| client `<label>` | one SSH key, logged in as `magvaultclient`, forced into `magvault-gate --client <label>` | gate verbs in §5.6; write only `incoming/<label>/`; read `proxies/db/`, `bins/` |
| remote station `<label>` | separate key class (Phase 5) | station verbs only (S13) |
| root | installer and `zmag-helper` | sshd config, sudoers, helper |

### 10.2 Filesystem permissions

| Path | Owner | Mode | Notes |
|---|---|---|---|
| config dir, `path.conf`, `magvault.conf` | magvault | 0750 / 0640 | |
| `cameras/`, `encoders/` | magvault | 0750 | served via RPC, never read by clients directly (PLAN:196) |
| `db/` | magvault | 0700 | |
| `mags/`, `restore/`, `proxies/`, `custom_encode/`, `bins/` | magvault | 0750 | `magvaultclient` gets read on `proxies/db` and `bins` through group `magvault-read` |
| `incoming/<label>/` | magvault, group `magvault-in` (`magvaultclient` member) | 2770 | per-client dirs created on key add |
| `logs/` | magvault | 0750 | |
| `telemetry_live` | magvault | 0644 | "Readable without root" (PLAN:830) |
| `/etc/magvault/client_keys` | magvault | 0640 | S5 |
| `/run/magvault/gate.sock` | magvault, group `magvault-gate` | 0660 | `magvaultclient` member |
| `/usr/lib/magvault/*` | root | 0755 | helpers not writable by `magvault` |

### 10.3 sudoers (server)

```text
Defaults:magvault timestamp_timeout=0
magvault ALL=(root) /usr/lib/magvault/zmag-helper
```

Replaces PLAN:375. Client sudoers: exact commands from M6, same `timestamp_timeout=0`.

### 10.4 Input validation

As S6 and S9. In addition: all external commands are spawned with argument vectors (no shell); every path derived from user or card input is joined to a root, normalised, and checked to stay inside that root after `realpath`.

### 10.5 Secrets

Sudo passwords and generated private keys live only in memory for the operation, are zeroed afterwards, and never appear in logs, telemetry, error messages or the screen (S2, S7).

---

## 11. Open questions (decision needed)

Each has a proposed default; adopting all defaults gives a consistent system.

| # | Question | Proposed default | From |
|---|---|---|---|
| Q1 | DB vs files authority | DB authoritative for metadata; manifest fixed at ingest; mismatches reported, not resolved | C2 |
| Q2 | Is logging written to the mag card? | Yes, `# logging` section, rewritten on leave-clip/module with backup | C3, C4 |
| Q3 | When is SHA-256 computed? | After rsync, source and destination in parallel | C1 |
| Q4 | Folder UUID form | Full UUID in folder names; short form only for display | C15 |
| Q5 | Process architecture | `magvaultd` + TUI + SSH gate as §5 | Summary |
| Q6 | Client write path | Per-client `incoming/` via `rrsync -wo`, server commits | S4 |
| Q7 | Key storage for sshd | `AuthorizedKeysCommand` + `/etc/magvault/client_keys` | S5 |
| Q8 | Sudo scope | Root-owned `zmag-helper` only, `timestamp_timeout=0` | S1, S2 |
| Q9 | How a mag becomes cold | Explicit evict with ≥ 2 complete placements on different roles | M1 |
| Q10 | Eject zmag | Add action (sudo `zpool export`) | M2 |
| Q11 | Zmag role at create; meaning of first/second copy | Role chosen at create; auto-backup fills each role | M16 |
| Q12 | Auto-import | Auto-detect, import after sudo password | C12 |
| Q13 | Export creation, remove, in/out | Prompt for name on first add; remove key; in/out are notes only | M3 |
| Q14 | Export layout inside bin | Recreate relative path under the mag folder | C19 |
| Q15 | NLE proxies in bins | Delete PLAN:652 | C11 |
| Q16 | Verification setting for exports | Always verify; drop "if verification is on" | C9 |
| Q17 | Client menu letters; Settings on client | `s` Settings, `d` server dashboard | C8 |
| Q18 | Server dashboard over SSH | Read-only view, or drop in favour of `status.summary` | S10 |
| Q19 | Client control of server encoder queue | Read-only | S10 |
| Q20 | `Esc`/`q` semantics and edit mode | §6 shared rules | C7 |
| Q21 | Prepare: format or not, filesystem, exact sudo commands | **Needs owner input**; if it formats, exFAT or ext4, never FAT32 | M6, D9 |
| Q22 | USB master/slave meaning | Master = staging + sync source; slave = mirror only; or drop the field | M7 |
| Q23 | `w` in Key module | Removable mounts only | C13 |
| Q24 | Offline logging conflicts | Per-field last-writer-wins with WARN log | D7 |
| Q25 | Snapshot creation | External scheduler; Magvault lists/holds/destroys only | M18 |
| Q26 | MHL generations on zmag copy / cold pull | Yes | D3 |
| Q27 | `incoming`/`restore` roots | New `path.conf` keys on the mags dataset | D1 |
| Q28 | `archived` status | Drop | §7.2 |
| Q29 | Clip vs file; spanned clips | One media file = one clip in v1 | M10 |
| Q30 | RAW formats without ffmpeg decoders | Proxy job fails with `unsupported codec`; documented | P10 |
| Q31 | Identity string in headers and cards | Key label for clients; `magvault@<host>` (+ login name if known) on server | M12 |
| Q32 | Remote station principal and verbs | Separate key class, station verbs in gate | S13 |
| Q33 | Log day boundary | UTC date | D11 |

---

## 12. Priorities

### Must fix before coding

These change the schema, the on-disk formats, or the security boundary. Getting them wrong means migrations of archived data or a redesign of the client path.

1. **Process architecture and client transport** (Summary point 1, §5, Q5-Q6): daemon, forced-command gate, per-client incoming spool. Without this no client flow can be built.
2. **Authority and rebuild rules** (C2, C3, C4, Q1-Q2): decides magcard contents and the revert feature.
3. **Hash timing** (C1, Q3) and **atomic placement via incoming/restore** (D1, Q27).
4. **Folder naming** (C15, Q4) and **relative paths everywhere** (D4): folder names are "never renamed after creation" (PLAN:79), so this is the last chance.
5. **Clip/file model and export layout** (M10, C19, D12, Q14, Q29): fixes `files`/`clips`, proxy names and bin layout.
6. **Card formats** (§8.3, §8.5): relative paths, `|` rule, logging section, `format:` lines. ASC MHL naming and ignore list (D3).
7. **Security boundary**: forced commands and `restrict` (S3), no client write to `mags/` (S4), `AuthorizedKeysCommand` (S5), sudo helper and `timestamp_timeout=0` (S1, S2), input validation (S6, S9).
8. **Mag lifecycle**: eviction to `cold` (M1, Q9), eject (M2, Q10), zmag role (M16, Q11), auto-import vs sudo (C12, Q12).
9. **Schema corrections**: people table (C6), no free-text camera (C5), job tables, `zmags`, cold pull ↔ export link (§7).
10. **Prepare semantics** (M6, Q21): if Prepare formats drives, the gating rules need to be exactly right before any code touches a disk.
11. **Staging location** (P2): never `/tmp`.

### Can be decided during implementation

- Key letters and menu entries (C7 rule set, C8, C16, Q17) — the rules above are enough to start; letters can move.
- Export UI details: name prompt, remove and in/out keys (M3, Q13); project close/reopen placement (M4).
- Dashboard sources for events and pending requests (M23).
- Default profile selection (M14), encoder parallelism (M22), shot types list (M15).
- Logging offline conflict rule (D7, Q24) — needed by the time client logging ships (Phase 6).
- Client read-only vs control of encoder queue; server dashboard over SSH (S10, Q18-Q19).
- Snapshot creation policy (M18, Q25), MHL generations on later copies (Q26).
- Telemetry JSON fields beyond §8.9.
- Free-space margins, retry back-off, log context lengths.
- Stale mockup notes and PLAN open points (C14), roadmap order (M21).
- Layout truncation rules at 120x36 (T7).

### Nice to have later

- Split function (PLAN:84, already "later").
- HQ per-asset toggle (PLAN:651).
- Remote zmag stations (Phase 5; needs S13 first).
- Sidecar matching rules per camera in `.cam`; spanned/chaptered clips as one logical clip.
- RAW decoding via vendor SDKs (P10).
- Clip list filter in Logging (PLAN:517 defers it).
- Project merge (README:25).
- `format:` migrations tooling; `magvault verify --all` scheduled integrity pass.
- Compact copies view for many zmags (P5).
- Desktop egui client (PLAN:618-620).

---

Sources used beyond the repository:
- ASC MHL tooling and supported hash formats: [ascmhl on PyPI](https://pypi.org/project/ascmhl/0.7.1/), [ascmitc/mhl on GitHub](https://www.github.com/ascmitc/mhl)
- FFmpeg 8.0 ProRes RAW decoder: [ubuntuhandbook](https://ubuntuhandbook.org/index.php/2025/08/ffmpeg-8-0-released-apv-prores-raw-decoder/), [LWN](https://lwn.net/Articles/1034813)
