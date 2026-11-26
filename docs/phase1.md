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

- **Dashboard** (landing): server connection (ping through the gate), local staging
  (pending mags), field sets and not-valid drives, recent local activity (the newest log
  lines). Modules: `i` Ingest; `g` Logging, `l` Library and `s` server dashboard are
  stubs for later phases. `r` refreshes, `q` quits (dashboard only).
- **Ingest**: panes 1 Detected (cards, plus "type a path…"), 2 Files (the probed card),
  3 Project (open projects; `n` new project by name, same rule as `--project`),
  4 Details (camera, format, gamma, gamut from the camera profile only; shoot day; notes),
  5 Targets (local staging, server, each field set as one row, each not-valid USB drive
  with `[ o Prepare ]`; camera cards are never listed there). Enter or Space toggles a
  target. The checkboxes map to the CLI routes: staging = default, staging + set =
  `--mirror-set`, set only = `--via set`, server only = `--via direct`, server unticked
  = `--no-sync`. The warning line shows when no field set is ticked (and says so more
  strongly when the server is not confirmed). `p` pulls once card, camera, project, shoot
  day and a target are set; otherwise it lists what is missing.
- **Prepare popup** (`o`): the smaller-first warning, the drives with sizes (refused ones
  with the reason), Enter picks, Enter again confirms (`e` edits the set label). On a set
  row waiting for its slave, `o` opens the slave step; a smaller slave is refused, a larger
  one shows "Will use X to match master. Rest unused." Esc cancels.
- **Clean popup** (`c`): only on a fully synced set; otherwise the reason is shown.

Pull, Prepare and Clean call `client_ingest::run`, `usb::prepare_master`,
`usb::prepare_slave` and `usb::clean` — the code behind `ingest`, `usb prepare-*` and
`usb clean` — with the operator's choices and `--yes` (the TUI popup is the
confirmation). For these steps the TUI steps aside: the plain terminal shows the copy
progress or sudo's own password prompt, then "Press Enter to return to Magvault".
Ctrl-C there aborts an ingest (nothing committed) or cancels sudo (nothing written).
This differs from the mockups, which draw the password field and the progress bar inside
the TUI: the password never passes through Magvault, and progress inside the TUI is left
for later.

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
| default (`--via staging`) | card → local staging | rsync to the server, server verifies and commits, then the staging copy is removed (autoclean) |
| `--mirror-set` | card → local staging, then staging → master → slave | as above; the set copy is marked `synced` and kept until Clean |
| `--via set` | card → master, master → slave (verified) | uploaded from the master; marked `synced` on both drives |
| `--via direct` | card → server only (refused when the server is not reachable) | committed before the card is released |

When the server cannot be reached (or `--no-sync`), the mag stays pending in staging or on
the set. With no field-set copy and no server confirmation, ingest prints a WARNING ("keep
the card") and logs it.

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
- **Camera profiles on the client** are local copies made by `init --mode client`; keep
  them in step with the server by hand. The server checks every upload against its own
  profile.
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
generic DCIM, non-card folder ignored, several cards need a pick); staging → server →
autoclean with logs on both sides; offline ingest pending + sync; a corrupted local copy is
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
synced, pickers, resize message; `MAGVAULT_TUI_DUMP=<dir>` writes the screens as text).
`tests/e2e-phase1.sh` also drives the real binary in a pseudo-terminal
(`tests/tui_pty.py`, a small VT100 emulator): a full pull from the TUI through the
client ingest to a server commit, the Prepare popup and Esc, and as root Clean (refused
while pending, then done) and Prepare (master with an edited label, larger slave limited)
on loop devices.

Not covered by the script (manual on courage): SSH + rrsync, the sudo prompt, real USB
drives and automount, real ZFS/LVM signatures (lsblk needs udev to report them), the TUI
on a real terminal with sudo's prompt.
