> Phase 2 note: this document describes Phase 1 code. Phase 2 replaces name-merge, `--project NEW` creating a real project, and a 320p job on every commit. See PLAN.md Phase 2 decisions.

# Phase 1 — implementation notes

Phase 1 from PLAN.md's roadmap: client ingest and field backups. A client (DIT laptop)
finds a card, copies it with verification, keeps a local or field-set copy until the
server confirms, and sends it to the server through a gate. The server verifies every
upload against the SHA-256 recorded at ingest and commits it with the same transaction as
a Phase 0 ingest. Field backup sets (one master + one slave USB drive) can be prepared,
used as staging, synced and cleaned. Terminal only.

Phase 0 behaviour is unchanged: a config without `mode: client` is a server config and
`magvault ingest <card> --project <name>` works as before (`tests/e2e.sh` still passes).
`src/ingest.rs` was split into reusable steps; the server path runs the same steps in the
same order.

## Build

```sh
sudo apt install build-essential libsqlite3-dev rsync ffmpeg openssh-client   # client and server
sudo apt install openssh-server                                                # server (rrsync ships with rsync ≥ 3.2.4)
# cargo needs crates.io access once to fetch ratatui (the only dependency); Cargo.lock pins it.
# Update the server before the clients (a new client needs the gate's committer line).
cargo build --release      # target/release/magvault and target/release/magvault-helper
cargo test
tests/e2e.sh               # Phase 0 regression
tests/e2e-phase1.sh        # Phase 1; run as root to include the loop-device Prepare/Clean section
```

## Commands

```sh
# server
magvault --config /srv/magvault/config serve            # commits client uploads; Ctrl-C stops
magvault --config /srv/magvault/config serve --once     # one pass (tests, cron)
magvault --config /srv/magvault/config projects         # open projects, requirements, warnings
magvault --config /srv/magvault/config project-log ACME # the project log

# client
magvault --config ~/magvault/config init --mode client --root ~/magvault --server courage
magvault cards                                  # detected camera cards
magvault ingest --project ACME                  # pick a detected card (or give the path)
magvault ingest /media/dit/A7SIII_128 --project ACME --via set
magvault ingest /media/dit/A7SIII_128 --project ACME --mirror-set
magvault ingest /media/dit/A7SIII_128 --project ACME --via direct
magvault sync                                   # pending mags → server
magvault usb status
magvault usb prepare                            # the Prepare popup (master, then slave)
magvault usb clean --set FIELD-A
magvault ingest --project NEW --require-fps 25 --require-resolution HD   # new project with requirements
magvault staging                                # staged mags: waiting / on the server, used / free
magvault staging clean                          # remove staged mags with two backups
magvault projects
```

`magvault --help` lists the commands, grouped (server / client / both), one line each.
`magvault <command> --help` (or `magvault help <command>`) shows one command: purpose,
flags, one example. Server commands are refused in a client config and client commands in
a server config.

## Terminal UI

`magvault` with no command (on a terminal, client config) or `magvault ui` opens the TUI
(ratatui). `magvault ui --scan-root <dir>` uses folders as cards and drives, like the
commands. Without a terminal, `magvault` prints the help and `magvault ui` refuses.

- **Plain by default.** The screens show card label, camera, project, set name,
  pending / synced / empty, the server status and the next action. Paths, UUIDs, device
  nodes and the raw log format appear only after `d` (details; `d` again hides them).
- **Keys** (PLAN.md "Shared key convention"): ←→ or Tab / Shift+Tab move between panes,
  ↑↓ select the item, Enter picks, F1 shows help for the screen, Esc returns to the
  dashboard, `q` quits (dashboard only). No F2.
- **Dashboard** (landing): a "Next" line (the one thing to do now), server status,
  local staging (waiting / on the server and kept, used / free) and field sets, recent
  activity (newest log lines, plain). Modules: `i` Ingest; `g` Logging, `l` Library,
  `h` server dashboard and `s` Settings are stubs for later phases. `y` syncs pending
  mags, `r` refreshes.
- **Ingest** (layout from docs/mockups/ingest.md): panes 1 Detected (cards with a dump
  status dot: red not dumped, yellow dumped but not yet two copies with one on the
  server, green two copies and on the server; plus "type a path…"), 2 Card (a summary,
  no file list: shoot date and clip time range, clips, total size, frame rate,
  resolution and GOP for the card, the dominant value plus "mixed" when clips differ,
  the dump status), System time (wall clock, 24-hour, DD/MM/YYYY by default;
  `date_format` / `time_format` in magvault.conf), 3 Project (Enter opens the open
  projects; `n` new project by name, then its requirements from the fixed lists; the
  project's requirements and, when the card does not meet them, "Camera footage does
  not meet project requirements." — not a hard stop), 4 Details (camera, format, gamma,
  gamut from the camera profile only; shoot day; notes; model; the confirm checklist),
  5 Targets (local staging with a used / free bar, server, each field set as one row,
  each not-valid USB drive by model and size with `[ o Prepare ]`; camera cards are
  never listed there; the pane scrolls when there are more rows than fit). Enter or
  Space toggles a target. `l` shows the whole ingest log. A card whose clips name more
  than one camera shows a warning, and `p` asks before pulling it as one mag. The checkboxes map to the CLI routes: staging = default,
  staging + set = `--mirror-set`, set only = `--via set`, server only = `--via direct`,
  server unticked = `--no-sync`. The warning line shows when no field set is ticked (and
  says so more strongly when the server is not confirmed). `p` pulls once card, camera,
  project, shoot day and a target are set; otherwise it lists what is missing.
- **Prepare popup** (`o`): the smaller-first warning, the drives with model and size
  (refused ones with the reason), Enter picks, Enter again confirms (`e` edits the set
  label). On a set row waiting for its slave, `o` opens the slave step; a smaller slave
  is refused, a larger one shows "Will use X to match master. Rest unused." Esc cancels.
- **Clean popup** (`c`): only on a fully synced set; otherwise the reason is shown.

**Steps run inside the TUI.** Pull, Sync (`y`), Prepare and Clean call
`client_ingest::run`, `sync::run`, `usb::prepare_master`, `usb::prepare_slave` and
`usb::clean` — the code behind `ingest`, `sync`, `usb prepare-*` and `usb clean` — with
the operator's choices and `--yes` (the TUI popup is the confirmation). They run on a
worker thread; what the CLI would print and its progress come back through one output
sink (`src/out.rs`). A pull runs in the Ingest bar ("Ingesting": `x ABORT`, a bar with
bytes and percent, the stages hash → copy → verify → server commit) while the panes stay
visible and locked; Sync, Prepare and Clean show a progress popup. When the step ends,
the TUI stays where it was: one line in the log, a message on failure. `x` (pull) or
Ctrl-C while a step runs aborts it (an ingest commits nothing; a mag already copied
stays pending).

**Sudo in the TUI.** Prepare and Clean ask for the sudo password in a focused prompt
(dots on screen). The password is handed once to `sudo -k -S` for that helper call,
then dropped; it is never logged or shown, and the `Secret` type hides it from debug
output. Esc or Ctrl-C at the prompt cancels: nothing is written, and "sudo cancelled" is
logged locally and reported to the server log. As root there is no prompt.

## Server status: ready, committer down, offline

PLAN.md "Client ↔ server status". `magvault serve` writes a heartbeat
(`mags/.incoming/.results/serve.alive`: `alive <time> <pid> <state>` every 2 s, `stopped
<time>` when it stops). The gate's `ping` (and `status`) add a second line, `serve
alive` or `serve down: <why>` (heartbeat older than 10 s, stopped, never started). The
client calls the server **ready** only on `serve alive`; gate up with serve down is
**server up, committer down**, never "reachable"; no answer is **offline**. The
dashboard, Ingest and `magvault projects` use the same check.

Ingest and sync wait for a committer that is down: they print the warning, retry, and
stop after `committer_wait` seconds (client `magvault.conf`, default 120) with a clear
message; the mag stays pending in staging or on the set. `sync` stops at the first such
timeout instead of waiting again for every mag. `--via direct` waits before copying
anything (the card is the only copy). A commit already queued keeps waiting while serve
is alive (busy verifying a large mag still sends heartbeats).

Update the server (gate + serve) before the clients: a new client treats an old gate
(no committer line) as "committer down".

## Client issues on the server log

PLAN.md "Client issues on the server log". The client queues operational problems in
`<config>/issues.pending`: cannot reach the server, committer down, timeouts,
ingest / sync errors, sudo cancelled (plus DEBUG lines when debug is on). They are sent
with the gate's new `log` verb (lines on stdin) whenever the gate answers (every ping)
and at the end of each client command; a connection failure is reported when the
connection is back, with its original time. Limits: the client drops repeats of the
same line within 60 s and keeps at most 200 queued lines; the gate keeps at most
256 KiB per client; serve writes at most 30 operational and 300 DEBUG lines per client
per minute and reports what it dropped. Every line is made printable, one line, at
most 300 characters. Messages are Magvault's own; no passwords or key material.

Server log line: `2026-11-27T08:30:10Z  WARN   client    rafiki  [2026-11-27T08:30:03Z]
server up, committer down; sync waiting` (the bracket is the client's time).

## Debug logging

PLAN.md "Debug logging". `debug: yes` in `magvault.conf` (server: the existing file;
client: `<config>/magvault.conf`, created by `init --mode client` with `debug: no`), or
`--verbose` / `--debug` for one run (`magvault --debug sync`, `magvault --debug serve`).
DEBUG lines go into the same daily log files. Covered: gate verbs (the gate queues one
line per verb; serve writes them only when its debug is on), gate calls on the client,
serve accept / commit / reject, rsync start and end (destination, file count, status,
duration), hash at ingest, every verify (start / end, ok or MISMATCH), the rename from
incoming into place, helper calls (arguments and result; never the sudo password). On a
client, DEBUG lines are also sent to the server log through the client-issues path.

## Camera identification (fixes from docs/camera-identification-findings.md)

- **Sidecars; no junk filter.** Sony `<clip>M01.XML` and `<clip>R01.BIM` next to a clip
  are its sidecars (FX6, FS7, FS5, XAVC S). PLAN.md "Mag rules": every file on the card
  is copied, including `SALVAGE.TMP`, Panasonic `BACKUP.TMP`, `.DS_Store` and AppleDouble
  `._*` files (an AppleDouble `._X.mov` is copied as "other", not counted as a clip). The
  only files told apart are Magvault's own (`magcard.txt`, `ascmhl/`, `.magvault/`) inside
  a mag folder. `.MPG` is media (NEX-VG20 `MP_ROOT`).
- **Model hint.** `src/camera_meta.rs` reads the model only from metadata next to live
  media: clip sidecar XML (`modelName`), `MEDIAPRO.XML` `systemKind` only when its folder
  holds clips (prefix; ` ver.x` dropped), P2 `CONTENTS/CLIP/<stem>.XML`, XML inside
  MOV / MXF / MP4 clips (first and last MiB, up to 8 clips), MPEG `MVI_DATA` (`SONY` …
  `NEX-VG20E`), thumbnail / DCIM EXIF Model. `AVF_INFO` and a `MEDIAPRO.XML` beside no
  clip are ignored (they name the camera that formatted the card). Two spellings of one
  body (`CX350` / `AG-CX350`) count once; different models are shown as "more than one
  camera on this card". The hint is shown in `cards`, the ingest summary ("model hint …
  — confirm, or pass --model <name>") and the TUI Details ("Model … (from card)",
  editable). It is stored on the mag card as `camera_model_hint: <model> (<source>)`
  (kept by reconcile; no database column yet) and never replaces the camera profile.
- **Camera guess.** A profile whose `model:` names the hint scores highest (exact, or a
  letter suffix: `NEX-VG20` ↔ `NEX-VG20E`; never a further digit, so `FDR-AX3` does not
  match `FDR-AX53` or `FDR-AX33`). A specific profile whose `model:` names none of the
  hints is ruled out. Otherwise `card_layout` is required (nested paths such as
  `PRIVATE/M4ROOT` allowed and must hold media) and, when the profile has
  `filename_pattern`, a media file must match. `vendor: Generic` (generic-avchd) is a
  fallback only when no specific profile scores. When the card names a model that no
  profile has, ingest refuses without `--camera` and says which model the card names; the
  layout-vendor fallback is not used then. `--model <name>` gives the operator's name for
  the camera (stored as the hint, used for the guess).
- **Layout label.** The card is labelled by the media trees that hold media, joined with
  " + " (`Sony XDCAM (PRIVATE/XDROOT) + AVCHD (PRIVATE/AVCHD)`): Panasonic P2
  (`CONTENTS/AVCLIP`), Panasonic (`PRIVATE/PANA_GRP`), Sony XAVC (`PRIVATE/M4ROOT`), Sony
  XDCAM (`PRIVATE/XDROOT`, `XDROOT`), Sony MPEG (`MP_ROOT`), Canon XF, AVCHD
  (`PRIVATE/AVCHD`). An empty `M4ROOT` no longer steals the label. AVCHD alone names no
  vendor.
- **Profiles shipped.** `init` installs every `share/cameras/*.cam`: `build.rs` lists the
  folder at build time, so a new file needs no code change (a unit test checks the list).
  Today: generic-avchd, panasonic-ag-cx350, sony-a7siii, sony-fdr-ax33, sony-fdr-ax53,
  sony-ilce-7m3, sony-ilme-fx6, sony-pxw-fs5, sony-pxw-fs7, sony-vg20. `#` in
  `filename_pattern` matches one digit (`A###C###_*.MXF`). A P2 hint `CX350` matches
  `model: AG-CX350` (same body without the maker prefix).
- **Reference cards.** `cargo test reference_cards` rebuilds the seven reference cards in
  miniature (folders, names, metadata strings) and checks label, guess, hint, pairing and
  that nothing is dropped, with every shipped profile: AX3 → sony-fdr-ax53 (hint
  FDR-AX53); FS5 → sony-pxw-fs5; FX6 → sony-ilme-fx6 (ILME-FX6V); VG20 → sony-vg20
  (NEX-VG20E); FS7 → sony-pxw-fs7; A7 III → sony-ilce-7m3; CX350 → panasonic-ag-cx350
  (AG-CX350, or CX350 from the P2 XML).

## What the pieces do

### Card discovery (`magvault cards`, `ingest` without a path)

Mounted filesystems under `/media`, `/run/media` and `/mnt` (from `/proc/self/mountinfo`)
are checked for camera layouts: Sony XAVC (`PRIVATE/M4ROOT`), AVCHD, XDCAM (`XDROOT`),
Canon XF (`CONTENTS/CLIPS001`), RED (`.RDM`), Blackmagic BRAW, and `DCIM` (GoPro, Canon,
DJI, Insta360, generic), plus any `card_layout` in a camera profile. Field drives and mag
folders are never offered as cards. A device mounted twice (automount plus a manual
mount) is listed once, preferring `/media`. The operator picks the card. With `--yes`
(no terminal needed) the card is used when exactly one is detected; zero or several cards
are refused with a message to pass the path (or, in a terminal, run without `--yes` and
pick). Without `--yes` and without a terminal, ingest refuses instead of guessing.

The camera guess is the same in `cards` and `ingest`: the profile guess (`card_layout`
folders, `filename_pattern`); if that finds nothing, the recognised layout's vendor
(Sony, Canon, RED, Blackmagic, GoPro, DJI, Insta360) when exactly one camera profile has
that `vendor`. `--yes` accepts the guess; only with no guess and no `--camera` is ingest
refused, with the reason (`camera missing: no camera profile matches this card (layout:
Generic DCIM); pass --camera <profile id> (available: …)`). `--scan-root <dir>` (repeatable) or
`MAGVAULT_SCAN_ROOTS` replaces the mount scan with "every folder in <dir> is a volume",
which is how the fixtures are tested.

### Client ingest routes

All routes probe, confirm, hash once at ingest (SHA-256 + C4), copy with rsync, verify the
copy against the hashes and write `magcard.txt` + ASC MHL with relative paths, exactly as
Phase 0.

| Route | Copies | Server |
| --- | --- | --- |
| default (`--via staging`) | card → local staging | rsync to the server, server verifies and commits; the staging copy is kept (the server is one backup) |
| `--mirror-set` | card → local staging, then staging → master → slave | as above; the set copy is marked `synced` and kept until Clean; staging is removed (two backups: server + set) |
| `--via set` | card → master, master → slave (verified) | uploaded from the master; marked `synced` on both drives |
| `--via direct` | card → server only (refused when the server is not reachable) | committed before the card is released |

When the server cannot be reached (or `--no-sync`), the mag stays pending in staging or on
the set. With no field-set copy and no server confirmation, ingest prints a WARNING ("keep
the card") and logs it.

**Staging autoclean** (PLAN.md "Client staging cache"). A staged mag the server confirmed
is listed in `<staging>/.magvault/synced` and kept until a second backup exists: a mounted
field set that holds it. Then ingest or `sync` removes it and logs that. Staging + server
alone is kept; staging alone (pending) is never removed. `staging_autoclean: no` in the
client's magvault.conf keeps everything; `magvault staging` lists staged mags with their
state and used / free space, `magvault staging clean` removes the mags with two backups,
`magvault staging clean <mag>` removes one server-confirmed mag (asks when the server is
its only backup). `sync` uploads a mag once: a set copy of a mag already confirmed from
staging is marked synced without a second upload.

**Server camera check before upload.** `serve` publishes its camera profiles
(`results/cameras.txt`, gate verb `cameras`). Online, ingest refuses before copying when
the chosen camera / format / gamma / gamut is not in the server's profile; offline, the
cached list (`<config>/cameras.cache`) gives a warning. `sync` checks each mag's magcard
against the list and never uploads one the server would refuse (it stays pending, FAILED
with the reason). A server that does not publish the list yet (older serve) is checked
after the upload as before.

**Project requirements.** `ingest --require-fps 24|25|30|50|60 --require-resolution
4K|HD|SD --require-gop intra|long-GOP` set a NEW project's requirements (TUI: the New
project popup). The summary shows the card's frame rate / resolution / GOP and, when the
project has requirements, "Camera footage does not meet project requirements." with what
differs. Not a hard stop. The mag card gets `requirements_warning` (and, for a new client
project, `project_requirements`, so the server creates the project with them); the server
re-checks every upload against its own `projects.requirements` and writes the mismatch to
`project_log` and its daily log. `magvault projects` shows requirements (server: with the
warning count); `magvault project-log <project>` (server) shows the log. Requirements
reach clients through the gate verb `requirements` (cached in `requirements.cache`).

**Mixed-camera cards.** When the live media names more than one camera, ingest warns and
asks "Ingest the whole card as one mag anyway?"; `--yes` refuses unless `--mixed-ok` is
given; the mag card records `mixed_cameras`. The TUI shows the warning in 2 Card and asks
before Pull.

**Dump status.** Each ingested mag is recorded in `<config>/dumps.txt` with a fingerprint
of its card (relative path, size, mtime of every file) and the identity of the volume it
came from (device: image (loop) or disk, filesystem UUID, size; folder: its path). A
detected card is dumped only when both match: this client ingested this card. Copies
counted are the server (confirmed), staging and the field set (a mounted set is checked;
an unmounted one is trusted); green needs two copies with one on the server. Files that
match a mag from another volume (a card image and its card, a copied folder) are yellow
with the note "files match an ingested mag", never green; ledger lines from before the
identity was kept count the same way. Cleaning a set updates the ledger. `magvault cards`
on a client prints the status of each card.

**Already dumped.** Ingest of a card that is dumped (or whose files match a mag) warns
"already dumped: …". `--yes` refuses ("pass --force to do it anyway"; for a card not yet
safe it points to `magvault sync`); without `--yes` it asks "Ingest it again as a new
mag?"; the TUI asks in a popup before Pull. When the operator goes ahead (`--force` or
the TUI yes), the client log and the server log get a WARN line, and the new mag card
records `reingest_of: <earlier mag>`. Server-side `ingest` has no dump ledger and does not
check this.

**ffprobe.** Found on PATH, else at `ffprobe:` in magvault.conf (client and server), else
in /usr/bin, /usr/local/bin, /snap/bin (non-interactive sessions often have a short PATH).
When it is missing, ingest prints a WARNING, the client log and the server log get a WARN
line, `serve` warns at start, and the TUI shows it in 2 Card: frame rate, resolution, GOP
and clip times are not read and requirements cannot be checked.

**Unmounted field sets.** A drive whose ext4 filesystem carries a set label (`<SET>_M` /
`<SET>_S`, given by Prepare) is never offered for Prepare unless it is mounted with its
`magvault-backup.txt`: unmounted, `usb status`, the TUI and the Prepare popup show "is the
master of field set X: set not mounted"; mounted without a readable list, it is refused
with "check it before anything else". The root helper refuses to format an unmounted set
drive too. Drives that were never a set are offered as before.

### Client → server

The client never writes into `mags/`. Each client key on the server is locked to
`magvault gate` (SSH forced command), which allows only:

- `rsync --server …` (upload only) → `rrsync -wo <spool>/<client>`: the client's own spool
  folder under `mags/.incoming/.clients/`;
- `commit <folder>`, `status <folder>`, `projects`, `ping`.

`magvault serve` (server user) picks up commit requests, verifies the upload against the
manifest in its magcard (SHA-256 per file, nothing missing or extra), checks the camera
values against the server's profile, and then runs the Phase 0 commit (database rows,
queued 320p job with its clips listed in `encoder_job_clips`, rename into `mags/`). It writes the result into `mags/.incoming/.results/`,
which the client can read but not write, so a client cannot forge "committed". A failed
upload is deleted from the spool and reported; the client copy stays pending.

`magvault sync` re-verifies the local copy against the stored SHA-256 before sending, and
marks a mag synced (set) or removes it (staging) only after the server answered
`committed`.

### Field backup sets

- Exactly one master and one slave. Filesystem labels `<SET>_M` and `<SET>_S`; each drive
  carries `magvault-backup.txt` (role, set_id, set_size, label, created, host, mag list with
  `pending` / `synced`). Field USB roles are not zmag roles.
- **Prepare** (`usb prepare`, or `usb prepare-master` / `usb prepare-slave`): shows "Mount
  the smaller drive first and prepare it as master.", lists the drives with sizes, master
  first. After confirmation `sudo` runs the helper, which formats the **whole drive as
  ext4** (never FAT32) and mounts it at `/media/<user>/<label>`; only then is
  `magvault-backup.txt` written. The master's size is the set size. A slave smaller than
  the master is refused; a larger slave gets a filesystem limited to the set size ("Will use
  X to match master. Rest unused."). Esc or Ctrl-C cancels at any prompt (Enter also
  cancels the drive pick and the y/N questions); nothing is written before sudo is accepted. A drive with pending mags cannot be prepared again (the
  set stays selectable for ingest and sync).
- **Which drives are offered.** Whole disks that lsblk reports as removable or USB
  (`RM=1`, `HOTPLUG=1` or `TRAN=usb`; USB hard drives report `RM=0` and are accepted via
  `TRAN=usb`). Camera cards are never offered: SD/MMC slots (`mmcblk*`), card readers by
  model name, and any drive with a mounted camera layout are left out of `usb status` and
  `usb prepare`, and `prepare-master --device` refuses them (the helper also refuses
  `mmcblk*`). A disk that still carries a ZFS, LVM, RAID or LUKS signature is refused with
  `/dev/sdX has an existing ZFS signature; wipe it first (wipefs), then Prepare` (shown in
  `usb status` too); Magvault never wipes such a disk itself. A disk mounted outside
  `/media`, `/run/media`, `/mnt` is refused with "unmount it first".
- **Clean** (`usb clean`): only when both drives are mounted, belong to the same set, list
  the same mags, and every mag is `synced`. After confirmation and sudo, the helper deletes
  the listed mag folders on both drives and nothing else; `magvault-backup.txt`, set id,
  size and roles stay, the set is valid and empty, and the clean is logged.

### Root helper (`magvault-helper`)

Installed root-owned at `/usr/lib/magvault/magvault-helper` and run with `sudo -k` (the
password is asked every time). Two verbs, both strictly validated:

- `usb-format <device> <fs-label> <blocks>`: whole disk only, removable/USB (loop devices
  only when `/etc/magvault/allow-loop-devices` exists), no partitions in use, mounted only
  under `/media`, `/run/media` or `/mnt`, refuses a drive whose `magvault-backup.txt` lists
  pending mags; `wipefs`, `mkfs.ext4 -b 4096 -m 0 -L <label>` with the root directory owned
  by the sudo user, mount `nosuid,nodev,noexec`.
- `usb-clean <master-mount> <slave-mount>`: the Clean rules above, checked again as root.

Client sudo is used for nothing else.

### 320p job clips

Every queued 320p job (server ingest and client commit) lists the mag's clips in
`encoder_job_clips` (one `queued` row per clip, same transaction as the job). Jobs queued
before this change keep an empty list.

## Decisions taken while implementing

Each is the smallest choice consistent with PLAN.md; each is easy to change.

- **Slave limit instead of a partition.** PLAN.md allows "partition or limit". The slave
  gets an ext4 filesystem of exactly the set size on the whole device (`mkfs.ext4 … <blocks>`);
  no partition table tools are needed.
- **A slave is only added to an empty master.** Copying existing mags onto a new slave is
  not in PLAN.md, so the slave step is refused once the master holds mags.
- **Sync and Clean need both drives.** A set with one drive mounted is shown as incomplete;
  sync uploads from the master and must mark both drives.
- **`--via direct --mirror-set`** is the same as `--via set` (card → set → server).
- **Projects on a client.** The client chooses from the server's open projects (cached in
  `<config>/projects.cache` for offline use). A project the client creates is remembered in
  `<config>/projects.local` so later offline ingests reuse its UUID. On commit the server
  uses the project with that UUID; if the UUID is unknown it applies the Phase 0 name rule
  (one open project with that name is reused, a closed-only or ambiguous name is refused,
  otherwise the project is created). When the server's project wins, the uploaded
  `magcard.txt` is rewritten with the server's project before it moves into `mags/`
  (`magcard.txt` is an MHL ignore pattern). The client drops a local entry once the server
  lists that name. Copies still on a field set keep the client's project UUID until Clean.
- **Card discovery fallback.** If one card is found and `--yes` is given, it is used; with
  several, the operator must pick or pass the path.

## Temporary in Phase 1

- **Hand-written forced command.** Each client key goes into `~magvaultclient/.ssh/authorized_keys`
  by hand (below). PLAN.md's AuthorizedKeysCommand / Keys page comes later.
- **`magvault serve` is the first piece of `magvaultd`.** It is started by hand (or a
  systemd unit you write); it polls every 2 s.
- **Camera profiles on the client** are local copies made by `init --mode client`.
  Running `init --mode client` again installs shipped profiles that are missing and never
  overwrites a local file (a different one is reported). Before copying, the client
  checks its choice against the server's published list; the server checks every upload
  against its own profile.
- **Ownership after commit.** Uploaded files keep the `magvaultclient` owner and the
  `magvault-spool` group (0660/2770) after they move into `mags/`; the server reads and
  manages them through the group.
- **`server_transport: local`** in a client `path.conf` runs the gate as a local process;
  it exists for `tests/e2e-phase1.sh` and is not for production.

## Setup on courage

A fresh server data root `/srv/mvtest` owned by your own account (it stands in for the
`magvault` service user), the SSH account `magvaultclient` for clients, and courage also
acting as the client over SSH to localhost. Use a second machine as the client if you have
one; only `--server` changes. Keep the server data root outside home directories:
`magvaultclient` must be able to traverse to the spool.

```sh
# build and install
cd ~/magvault && git pull && cargo build --release
sudo install -m 0755 target/release/magvault /usr/local/bin/magvault
sudo install -d -m 0755 /usr/lib/magvault
sudo install -o root -g root -m 0755 target/release/magvault-helper /usr/lib/magvault/magvault-helper

# server
sudo install -d -o "$USER" -g "$USER" -m 0755 /srv/mvtest
magvault --config /srv/mvtest/config init --root /srv/mvtest
sudo groupadd -f magvault-spool
id magvaultclient >/dev/null 2>&1 || sudo useradd -m -s /bin/sh magvaultclient
sudo usermod -aG magvault-spool magvaultclient && sudo usermod -aG magvault-spool "$USER"
magvault --config /srv/mvtest/config serve --once          # creates mags/.incoming/.clients and .results
sudo chgrp magvault-spool /srv/mvtest/mags/.incoming/.clients /srv/mvtest/mags/.incoming/.results
chmod 2770 /srv/mvtest/mags/.incoming/.clients && chmod 2750 /srv/mvtest/mags/.incoming/.results
sg magvault-spool -c "magvault --config /srv/mvtest/config serve"   # leave running in its own terminal

# client key, locked to the gate
ssh-keygen -t ed25519 -f ~/.ssh/magvault_dit-a -N ''
sudo install -d -o magvaultclient -g magvaultclient -m 0700 ~magvaultclient/.ssh
echo "restrict,command=\"/usr/local/bin/magvault gate --spool /srv/mvtest/mags/.incoming/.clients --results /srv/mvtest/mags/.incoming/.results --client dit-a\" $(cat ~/.ssh/magvault_dit-a.pub)" \
  | sudo tee ~magvaultclient/.ssh/authorized_keys >/dev/null
sudo chown magvaultclient: ~magvaultclient/.ssh/authorized_keys && sudo chmod 0600 ~magvaultclient/.ssh/authorized_keys
ssh-keyscan -H localhost >> ~/.ssh/known_hosts      # the client uses StrictHostKeyChecking=yes

# client config (your account as the operator)
magvault --config ~/mvclient/config init --mode client --root ~/mvclient --server localhost
sed -i "s|^server_key:.*|server_key: $HOME/.ssh/magvault_dit-a|" ~/mvclient/config/path.conf
export MAGVAULT_CONFIG=~/mvclient/config
magvault projects                                    # first contact through the gate

# sudo for the helper only (sudo -k asks for the password every time)
echo "$USER ALL=(root) /usr/lib/magvault/magvault-helper" | sudo tee /etc/sudoers.d/magvault-helper
sudo chmod 0440 /etc/sudoers.d/magvault-helper && sudo visudo -c
```

## Tests

`tests/e2e-phase1.sh` runs the server, a client with the local test transport and a
background `serve` on one machine. It covers: discovery fixtures (Sony XAVC, Canon XF,
generic DCIM, non-card folder ignored, several cards need a pick); staging → server with
staging kept (one backup), staging + set → server with staging removed (two backups),
offline staging + set synced once and cleaned, `magvault staging` list / clean / named
clean / pending never removed, `staging_autoclean: no`; the server camera check (refused
before copying online, warned offline, never uploaded by sync); project requirements
(fixed lists, stored at creation on server and client, warning, mag card, project log,
existing project keeps its values); a mixed-camera card (`--yes` refuses, warning before
the question, `--mixed-ok`); offline ingest pending + sync; a corrupted local copy is
not sent; a tampered upload is refused by the server and the spool cleaned; gate refusals
(download, shell, paths, missing folder, bad label); field set as folders (pending on both
drives, Clean refused while pending, sync marks synced and keeps mags, helper refuses
folders that are not mounts); mirror and direct routes; one project for all client mags;
offline projects merged by name. As root it adds a real ext4 set on loop devices: master
64 MiB, 48 MiB slave refused, 96 MiB slave limited to 64 MiB, Prepare blocked while
pending (Magvault and helper), Clean refused while pending, sync, Clean, set valid and
empty, set reused. It uses the real rsync when installed, else `tests/fake-bin/rsync`.

It also checks the help output (`--help` lists commands only; `ingest --help` shows ingest
flags only) and, with a fake `lsblk` (`tests/fake-lsblk/`), that a disk with a ZFS
signature is refused with the wipe hint while an SD slot and a USB card reader are never
offered. The root section mounts under `/media/${SUDO_USER:-root}`, so it runs under
`sudo` as well as in a root shell; the helper-refusal check needs root and is skipped
otherwise.

TUI: `cargo test` runs key-script tests on ratatui's TestBackend (dashboard → Ingest,
one card picked and probed, Pull arguments, target routes, Prepare popup open / Esc /
confirm, refused ZFS disk and smaller slave, Clean hidden while pending and offered when
synced, pickers, resize message, sudo prompt (dots, Esc reports, the password never on
screen or in Debug), progress popup and Ctrl-C, committer down never "reachable", plain
wording, ←→ panes and F1 help on both screens, `h` / `s` stubs, card summary without a
file list, requirements warning and the New project requirements popup, mixed-camera
confirmation, dump colours (red / yellow / green), staging bar and autoclean note, the
System time block, the inline pull with `x` abort; `MAGVAULT_TUI_DUMP=<dir>` writes the
screens as text).
`tests/e2e-phase1.sh` also drives the real binary in a pseudo-terminal
(`tests/tui_pty.py`, a small VT100 emulator): a full pull inside the TUI with the
progress bar (the rsync stand-in prints progress2 lines; `FAKE_RSYNC_FILE_DELAY` slows
it) through the client ingest to a server commit, the Prepare popup and Esc, the
committer-down status, and as root Clean (refused while pending, then done) and Prepare
(master with an edited label, larger slave limited) on loop devices. The committer-down
section stops serve: ingest warns, waits `committer_wait` (3 s in the test) and leaves
the mag pending; sync times out; the TUI bot also opens F1 help and moves panes with →; after a restart with `--debug` the client issues and
DEBUG lines (gate verbs, serve accept, rsync, verify, rename, forwarded client lines)
are checked in both logs.

Not covered by the script (manual on courage): SSH + rrsync, the sudo prompt, real USB
drives and automount, real ZFS/LVM signatures (lsblk needs udev to report them), the TUI
sudo prompt against real sudo (the tests run as root, so `sudo -S` is not exercised),
real rsync progress in the TUI.
