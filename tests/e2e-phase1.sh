#!/usr/bin/env bash
# End-to-end test of Phase 1: card discovery, client ingest (staging / field set / direct /
# mirror), server commit of client uploads (`magvault serve`), reconnect-and-sync, the
# client gate's refusals, field-set Clean rules, and — when run as root — Prepare / Clean
# on real ext4 field sets built from loop devices.
#
# Server and client run on this machine. The client uses `server_transport: local` (the
# gate runs as a local process and rsync writes straight into the spool); SSH, rrsync and
# the sudo prompt are covered by the courage checklist in docs/phase1.md.
#
# Uses the real rsync when installed, else tests/fake-bin/rsync. Root section needs
# losetup, mkfs.ext4, wipefs, mount (it creates /etc/magvault/allow-loop-devices for the
# duration of the test and removes it again if it was not there).
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
MEDIA="/media/${SUDO_USER:-root}"   # the helper mounts under /media/<sudo user>
W="$REPO/target/e2e1-work"
cleanup() {
  [ -n "${SERVE_PID:-}" ] && kill "$SERVE_PID" 2>/dev/null || true
  [ -n "${SERVE_PID:-}" ] && { wait "$SERVE_PID" 2>/dev/null || true; }
  if [ -n "${LOOPS:-}" ]; then
    for m in $MEDIA/FIELD-L_M $MEDIA/FIELD-L_S; do mountpoint -q "$m" 2>/dev/null && umount "$m"; rmdir "$m" 2>/dev/null || true; done
    for l in $LOOPS; do losetup -d "$l" 2>/dev/null || true; done
  fi
  [ "${MADE_ALLOW:-}" = 1 ] && rm -f /etc/magvault/allow-loop-devices
  true
}
trap cleanup EXIT
pkill -f -- "--config $W/server serve" 2>/dev/null || true   # a serve left by an aborted run
rm -rf "$W"; mkdir -p "$W"

cargo build --release --offline --quiet --manifest-path "$REPO/Cargo.toml" 2>/dev/null || cargo build --release --quiet --manifest-path "$REPO/Cargo.toml"
MV="$REPO/target/release/magvault"
HELPER="$REPO/target/release/magvault-helper"
if ! command -v rsync >/dev/null || [ "${MAGVAULT_E2E_FAKE_RSYNC:-}" = 1 ]; then
  echo "NOTICE: real rsync not used; tests/fake-bin/rsync (test double) is on PATH"
  export PATH="$REPO/tests/fake-bin:$PATH"
fi

pass() { printf '  ok  %s\n' "$*"; }
fail() { printf '  FAIL %s\n' "$*"; exit 1; }
expect_fail() { local what="$1" needle="$2"; shift 2
  if out=$("$@" 2>&1); then echo "$out"; fail "$what: command succeeded"; fi
  grep -qF -- "$needle" <<<"$out" || { echo "$out"; fail "$what: expected: $needle"; }
  pass "$what"; }
mkclip() { if command -v ffmpeg >/dev/null; then
    ffmpeg -v error -y -f lavfi -i testsrc=size=320x180:rate=25 -f lavfi -i sine=frequency=440:sample_rate=48000 -ac 2 -t 1 \
      -c:v libx264 -c:a aac -metadata creation_time=2026-11-20T10:00:00Z -f mov "$1"
  else head -c 300000 /dev/urandom > "$1"; fi; }
sql() { python3 -I -c "import sqlite3,sys; print('\n'.join('|'.join(map(str,r)) for r in sqlite3.connect(sys.argv[1]).execute(sys.argv[2])))" "$SRV/db/magvault.sqlite" "$1"; }

# ---------------------------------------------------------------------------------------
echo "== setup: server, client (local test transport), serve"
SCFG="$W/server"; SRV="$W/srv"
"$MV" --config "$SCFG" init --root "$SRV" >/dev/null
CCFG="$W/client"; CLI="$W/cli"
"$MV" --config "$CCFG" init --mode client --root "$CLI" >/dev/null
grep -qx "mode: client" "$CCFG/path.conf" || fail "client path.conf"
cat >> "$CCFG/path.conf" <<EOF
# test only: run the gate locally instead of over SSH
server_transport: local
local_spool: $SRV/mags/.incoming/.clients
local_results: $SRV/mags/.incoming/.results
local_label: dit-a
usb_helper: $HELPER
EOF
ms() { "$MV" --config "$SCFG" "$@"; }
mc() { "$MV" --config "$CCFG" "$@"; }
"$MV" --config "$SCFG" serve > "$W/serve.log" 2>&1 &   # not via ms(): $! must be magvault itself
SERVE_PID=$!
sleep 0.5
pass "server + client config, serve running (pid $SERVE_PID)"
expect_fail "server command refused in client config" "is a server command" mc reconcile --all
expect_fail "client command refused in server config" "is a client command" ms sync

echo "== help"
top=$("$MV" --help)
for c in init ingest reconcile serve gate cards sync usb projects; do grep -qE "^  $c +" <<<"$top" || fail "--help misses $c"; done
grep -qE -- "--(project|camera|via|device|spool|verify-hashes|once|scan-root)" <<<"$top" && { echo "$top"; fail "--help dumps command flags"; }
[ "$(wc -l <<<"$top")" -lt 40 ] || fail "--help is long ($(wc -l <<<"$top") lines)"
pass "magvault --help: commands only, grouped, no flag lists"
ih=$("$MV" ingest --help)
grep -q "^magvault ingest — " <<<"$ih" && grep -q -- "--camera" <<<"$ih" && grep -q -- "--via" <<<"$ih" && grep -q "^EXAMPLE" <<<"$ih" || { echo "$ih"; fail "ingest --help"; }
grep -qE -- "--(device|spool|verify-hashes|once|label)" <<<"$ih" && { echo "$ih"; fail "ingest --help shows other commands' flags"; }
[ "$("$MV" help ingest)" = "$ih" ] || fail "help ingest differs from ingest --help"
grep -q "^magvault usb — " <<<"$("$MV" usb clean --help)" || fail "usb clean --help"
expect_fail "help for an unknown command refused" "unknown command \"bogus\"" "$MV" help bogus
pass "ingest --help: ingest flags only; help <command> is the same; every command has help"

echo "== card discovery (fixtures)"
MNT="$W/mounts"
mkdir -p "$MNT/A7SIII_128/DCIM/100MSDCF" "$MNT/A7SIII_128/PRIVATE/M4ROOT/CLIP" "$MNT/C300/CONTENTS/CLIPS001" "$MNT/GENERIC_64/DCIM/100ABCDE" "$MNT/Documents"
mkclip "$MNT/A7SIII_128/PRIVATE/M4ROOT/CLIP/A001_C001.mov"
echo '<xml/>' > "$MNT/A7SIII_128/PRIVATE/M4ROOT/CLIP/A001_C001.XML"
head -c 200000 /dev/urandom > "$MNT/A7SIII_128/DCIM/100MSDCF/DSC00001.JPG"
head -c 1000 /dev/urandom > "$MNT/C300/CONTENTS/CLIPS001/A001C001_X.MXF"
head -c 1000 /dev/urandom > "$MNT/GENERIC_64/DCIM/100ABCDE/IMG_0001.MOV"
echo notes > "$MNT/Documents/notes.txt"
out=$(mc cards --scan-root "$MNT")
grep -q "A7SIII_128  ·  Sony XAVC (PRIVATE/M4ROOT)  ·  Sony A7S III \[sony-a7siii\]" <<<"$out" || { echo "$out"; fail "Sony card"; }
grep -q "C300  ·  Canon XF (CONTENTS/CLIPS001)" <<<"$out" || fail "Canon card"
grep -q "GENERIC_64  ·  Generic DCIM" <<<"$out" || fail "generic DCIM card"
grep -q Documents <<<"$out" && fail "a non-card folder was listed"
[ "$(wc -l <<<"$out")" = 3 ] || fail "expected 3 cards"
pass "Sony, Canon XF and generic DCIM detected with camera guess; non-card folder ignored"
expect_fail "several cards and --yes: operator must pick" "pass the path" mc ingest --scan-root "$MNT" --project Field --yes </dev/null

echo "== ingest --yes: camera guess, card pick without a terminal"
Y1="$W/yes1"; mkdir -p "$Y1/SONY_XAVC/PRIVATE/M4ROOT/CLIP"          # Sony layout, no DCIM, camera file names
mkclip "$Y1/SONY_XAVC/PRIVATE/M4ROOT/CLIP/C0001.MP4"
out=$(ms ingest --scan-root "$Y1" --project YesTest --shoot-date 2026-11-20 --yes </dev/null 2>&1) || { echo "$out"; fail "--yes, no path, no TTY, one card"; }
grep -q -- "--yes: one card detected, using $Y1/SONY_XAVC" <<<"$out" || { echo "$out"; fail "card not picked"; }
grep -q "camera      Sony A7S III \[sony-a7siii\]  (guessed)" <<<"$out" || { echo "$out"; fail "camera not guessed from the Sony layout"; }
YF=$(awk '/^ingested /{print $2}' <<<"$out")
[ -d "$SRV/mags/$YF" ] || fail "--yes ingest did not commit"
pass "--yes, no path, no TTY, one card: picked; camera guessed from the layout (no --camera)"
[ "$(sql "select count(*) from encoder_job_clips jc join encoder_jobs j on j.id=jc.job_id join mags m on m.id=j.mag_id where m.folder='$YF' and jc.status='queued'")" = 1 ] || fail "job clip links"
[ "$(sql "select count(*) from clips c join mags m on m.id=c.mag_id where m.folder='$YF' and c.id not in (select clip_id from encoder_job_clips)")" = 0 ] || fail "clip without a job link"
pass "queued 320p job is linked to the mag's clips (encoder_job_clips)"
mkdir -p "$Y1/NOCAM/DCIM/100XYZ"; head -c 1000 /dev/urandom > "$Y1/NOCAM/DCIM/100XYZ/CLIP0001.MOV"
expect_fail "--yes, no path, no TTY, two cards: refused" "2 cards detected and --yes picks a card only when exactly one is found; pass the path" \
  ms ingest --scan-root "$Y1" --project YesTest --shoot-date 2026-11-20 --yes </dev/null
expect_fail "--yes, no camera guess and no --camera: refused, says what is missing" "camera missing: no camera profile matches this card (layout: Generic DCIM); pass --camera" \
  ms ingest "$Y1/NOCAM" --project YesTest --shoot-date 2026-11-20 --yes </dev/null
Y2="$W/yes2"; mkdir -p "$Y2"; cp -a "$Y1/SONY_XAVC" "$Y2/"
expect_fail "no path, no TTY, one card, no --yes: refused" "no terminal to confirm the card; re-run with --yes" \
  ms ingest --scan-root "$Y2" --project YesTest --shoot-date 2026-11-20 </dev/null
mkdir -p "$W/mounts-empty"
expect_fail "no card at all: refused" "no camera card detected in $W/mounts-empty" ms ingest --scan-root "$W/mounts-empty" --project YesTest --yes </dev/null

ING=(--project Field --camera sony-a7siii --shoot-date 2026-11-20 --yes)
ONE="$W/mounts1"; mkdir -p "$ONE"; cp -a "$MNT/A7SIII_128" "$ONE/"
CARD="$ONE/A7SIII_128"

echo "== client ingest: card → local staging → server → autoclean"
mc ingest --scan-root "$ONE" "${ING[@]}" > "$W/i1.out" 2> "$W/i1.err" </dev/null || { cat "$W/i1.err"; fail "ingest via staging"; }
F1=$(awk '/^ingested /{print $2}' "$W/i1.out")
grep -q "server  committed" "$W/i1.out" || { cat "$W/i1.out" "$W/i1.err"; fail "not committed"; }
[ -d "$SRV/mags/$F1" ] || fail "mag not in server mags/"
[ ! -e "$CLI/staging/$F1" ] || fail "staging not autocleaned"
[ "$(sql "select status, ingested_by like '%@%' from mags where folder='$F1'")" = "online|1" ] || fail "server DB row"
[ "$(sql "select count(*) from encoder_jobs e join mags m on m.id=e.mag_id where m.folder='$F1' and e.status='queued'")" = 1 ] || fail "320p job"
[ "$(sql "select count(*) from encoder_job_clips jc join encoder_jobs e on e.id=jc.job_id join mags m on m.id=e.mag_id where m.folder='$F1'")" = "$(sql "select count(*) from clips c join mags m on m.id=c.mag_id where m.folder='$F1'")" ] || fail "client mag: job clip links"
grep -q "^ok  *$F1" <<<"$(ms reconcile "$F1")" || fail "server reconcile of client mag"
grep -q "INFO   ingest    Mag $F1 committed from client dit-a" "$SRV"/logs/*.log || fail "server log"
grep -q "Mag $F1 committed on the server" "$CLI"/logs/*.log || fail "client log"
[ -z "$(ls -A "$SRV/mags/.incoming/.clients/dit-a")" ] || fail "spool not empty after commit"
pass "card picked by discovery (no TTY), staged, verified, committed by serve (job linked to clips), staging cleaned, logged on both sides"

echo "== offline ingest stays pending; sync later"
mc ingest "$CARD" "${ING[@]}" --no-sync > "$W/i2.out" 2> "$W/i2.err"
F2=$(awk '/^ingested /{print $2}' "$W/i2.out")
[ -d "$CLI/staging/$F2" ] || fail "offline mag not in staging"
grep -q "WARNING: no field-set backup and the server has not confirmed $F2" "$W/i2.err" || { cat "$W/i2.err"; fail "no-backup warning"; }
grep -q "WARN   ingest    no field-set backup" "$CLI"/logs/*.log || fail "no-backup warning not logged"
[ -z "$(sql "select id from mags where folder='$F2'")" ] || fail "offline mag reached the server"
pass "pending in local staging, warning shown and logged"
mc sync > "$W/s1.out" 2>&1 || { cat "$W/s1.out"; fail "sync"; }
grep -q "synced    $F2  (local staging, cleaned)" "$W/s1.out" || fail "sync output"
[ -d "$SRV/mags/$F2" ] && [ ! -e "$CLI/staging/$F2" ] || fail "sync result"
pass "sync: verified, committed, staging cleaned"

echo "== a corrupted local copy is not sent"
mc ingest "$CARD" "${ING[@]}" --no-sync > "$W/i3.out" 2>/dev/null
F3=$(awk '/^ingested /{print $2}' "$W/i3.out")
python3 -I -c "import sys; p=sys.argv[1]; b=bytearray(open(p,'rb').read()); b[0]^=1; open(p,'wb').write(b)" "$CLI/staging/$F3/INFO.TXT" 2>/dev/null \
  || python3 -I -c "import sys; p=sys.argv[1]; b=bytearray(open(p,'rb').read()); b[0]^=1; open(p,'wb').write(b)" "$CLI/staging/$F3/DCIM/100MSDCF/DSC00001.JPG"
set +e; mc sync > "$W/s2.out" 2>&1; rc=$?; set -e
[ $rc = 1 ] && grep -q "FAILED    $F3" "$W/s2.out" || { cat "$W/s2.out"; fail "corrupt copy synced"; }
[ -d "$CLI/staging/$F3" ] && [ -z "$(sql "select id from mags where folder='$F3'")" ] || fail "corrupt mag state"
pass "local re-verify against stored SHA-256 fails; mag stays pending, nothing sent"
rm -rf "${CLI:?}/staging/$F3"

echo "== server refuses an upload that does not match its manifest"
mc ingest "$CARD" "${ING[@]}" --no-sync > "$W/i4.out" 2>/dev/null
F4=$(awk '/^ingested /{print $2}' "$W/i4.out")
SP="$SRV/mags/.incoming/.clients/dit-a"
mkdir -p "$SP"; cp -a "$CLI/staging/$F4" "$SP/"
python3 -I -c "import sys; p=sys.argv[1]; b=bytearray(open(p,'rb').read()); b[-1]^=1; open(p,'wb').write(b)" "$SP/$F4/DCIM/100MSDCF/DSC00001.JPG"
gate() { SSH_ORIGINAL_COMMAND="$1" "$MV" gate --spool "$SRV/mags/.incoming/.clients" --results "$SRV/mags/.incoming/.results" --client dit-a; }
[ "$(gate "commit $F4")" = queued ] || fail "commit not queued"
for _ in $(seq 1 50); do st=$(gate "status $F4"); [[ "$st" == error* ]] && break; sleep 0.2; done
[[ "$st" == "error: integrity error"* ]] || fail "server status: $st"
[ ! -e "$SP/$F4" ] && [ -z "$(sql "select id from mags where folder='$F4'")" ] || fail "tampered upload committed or kept"
pass "tampered upload → error result, spool cleaned, nothing committed"
mc sync >/dev/null 2>&1 || fail "honest re-sync after refused upload"
[ -d "$SRV/mags/$F4" ] || fail "re-sync"
pass "the same mag syncs cleanly afterwards"

echo "== gate refuses everything else"
expect_fail "rsync download refused" "only uploads" gate "rsync --server --sender -t . x"
expect_fail "shell command refused" "refused by the Magvault gate" gate "cat /etc/passwd"
expect_fail "path in status refused" "not a mag folder" gate "status ../../etc"
expect_fail "commit of a missing folder refused" "has not been uploaded" gate "commit 20261120_00000000-0000-4000-8000-000000000000"
expect_fail "bad client label refused" "invalid client label" env SSH_ORIGINAL_COMMAND=ping "$MV" gate --spool "$W" --results "$W" --client "../x"
grep -q "magvault-gate ok dit-a" <<<"$(gate ping)" || fail "ping"
pass "ping answers"

echo "== field set as staging (folders standing in for drives)"
USB="$W/usb"; mkdir -p "$USB/FIELD-B_M" "$USB/FIELD-B_S"
python3 -I - "$USB" <<'EOF'
import sys, uuid
root = sys.argv[1]; sid = str(uuid.uuid4())
for role, d in (("master", "FIELD-B_M"), ("slave", "FIELD-B_S")):
    open(f"{root}/{d}/magvault-backup.txt", "w").write(
        f"format: magvault-backup/1\nrole: {role}\nset_id: {sid}\nset_size: 1073741824\nlabel: FIELD-B\n"
        "created: 2026-11-26T20:00:00Z\nhost: test\n\n# mags\n# mag_folder | state | updated\n")
EOF
grep -q "USB set FIELD-B  ·  master + slave  ·  1.1 GB  ·  valid  ·  0 mag(s)" <<<"$(mc usb status --scan-root "$USB")" || fail "set status"
mc ingest "$CARD" "${ING[@]}" --via set --set FIELD-B --scan-root "$USB" --no-sync > "$W/i5.out" 2> "$W/i5.err" || { cat "$W/i5.err"; fail "ingest via set"; }
F5=$(awk '/^ingested /{print $2}' "$W/i5.out")
diff -r "$USB/FIELD-B_M/$F5" "$USB/FIELD-B_S/$F5" >/dev/null || fail "master and slave copies differ"
for d in FIELD-B_M FIELD-B_S; do grep -q "^$F5 | pending | " "$USB/$d/magvault-backup.txt" || fail "$d not pending"; done
grep -q "WARNING: no field-set backup" "$W/i5.err" && fail "no-backup warning shown although the set holds a copy"
[ -z "$(ls -A "$USB/FIELD-B_M/.incoming" | grep -v '^$' || true)" ] || fail "leftovers in master .incoming"
pass "same mag on master and slave, pending on both, no false warning"
grep -q "1 pending · selectable · Prepare blocked" <<<"$(mc usb status --scan-root "$USB")" || fail "pending status"
expect_fail "clean refused while pending" "pending server sync; Clean is only offered" mc usb clean --set FIELD-B --yes --scan-root "$USB"
mc sync --scan-root "$USB" > "$W/s3.out" 2>&1 || { cat "$W/s3.out"; fail "sync of set"; }
grep -q "synced    $F5  (field set FIELD-B)" "$W/s3.out" || fail "set sync output"
for d in FIELD-B_M FIELD-B_S; do grep -q "^$F5 | synced | " "$USB/$d/magvault-backup.txt" || fail "$d not synced"; [ -d "$USB/$d/$F5" ] || fail "synced mag removed from $d"; done
[ -d "$SRV/mags/$F5" ] || fail "set mag not on server"
pass "sync from the master: committed, marked synced on both drives, mags kept until Clean"
grep -q "all synced · \[usb clean\] available" <<<"$(mc usb status --scan-root "$USB")" || fail "clean not offered"
if [ "$(id -u)" = 0 ]; then
  expect_fail "helper refuses to clean folders that are not mounted drives" "not a mounted drive" mc usb clean --set FIELD-B --yes --scan-root "$USB"
else
  echo "  --  helper refusal check skipped (not root: the helper would need sudo; run the script as root to include it)"
fi
for d in FIELD-B_M FIELD-B_S; do [ -d "$USB/$d/$F5" ] || fail "helper deleted something after refusing"; done

echo "== Prepare candidates (fake lsblk): signatures refused with the fix, camera cards never offered"
cat > "$W/lsblk.txt" <<'LSBLK'
NAME="sdy" PATH="/dev/zero" SIZE="320072933376" TYPE="disk" RM="0" HOTPLUG="0" TRAN="usb" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL="ST320LM010"
NAME="sdy1" PATH="/dev/zero1" SIZE="320000000000" TYPE="part" RM="0" HOTPLUG="0" TRAN="" FSTYPE="zfs_member" LABEL="wildraid" MOUNTPOINT="" PKNAME="sdy" MODEL=""
NAME="mmcblk9" PATH="/dev/full" SIZE="127865454592" TYPE="disk" RM="0" HOTPLUG="1" TRAN="" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL=""
NAME="mmcblk9p1" PATH="/dev/full1" SIZE="127861260288" TYPE="part" RM="0" HOTPLUG="1" TRAN="" FSTYPE="exfat" LABEL="" MOUNTPOINT="/run/media/op/disk" PKNAME="mmcblk9" MODEL=""
NAME="sdx" PATH="/dev/urandom" SIZE="63864569856" TYPE="disk" RM="1" HOTPLUG="1" TRAN="usb" FSTYPE="exfat" LABEL="EOS" MOUNTPOINT="/media/op/EOS" PKNAME="" MODEL="SD/MMC Reader"
NAME="sdw" PATH="/dev/random" SIZE="500107862016" TYPE="disk" RM="0" HOTPLUG="0" TRAN="usb" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL="ST500LM021"
LSBLK
fl() { PATH="$REPO/tests/fake-lsblk:$PATH" FAKE_LSBLK_FILE="$W/lsblk.txt" "$@"; }
st=$(fl mc usb status)
grep -q "^USB drive /dev/zero has an existing ZFS signature; wipe it first (wipefs), then Prepare$" <<<"$st" || { echo "$st"; fail "status: ZFS disk reason"; }
grep -q "^USB drive /dev/random .*not valid  ·  \[usb prepare\]" <<<"$st" || { echo "$st"; fail "status: clean USB disk (RM=0, TRAN=usb) not offered"; }
grep -qE "/dev/full|mmcblk|/dev/urandom|EOS" <<<"$st" && { echo "$st"; fail "status offers a camera card / card reader"; }
pass "usb status: ZFS disk shown with the fix, USB disk with RM=0 offered, SD slot and card reader not listed"
expect_fail "prepare-master refuses a ZFS-signature disk with the fix (before sudo)" "/dev/zero has an existing ZFS signature; wipe it first (wipefs), then Prepare" \
  fl mc usb prepare-master --device /dev/zero --label NOPE --yes
expect_fail "prepare-master refuses the SD card slot" "camera cards and SD slots are never prepared" \
  fl mc usb prepare-master --device /dev/full --label NOPE --yes
grep -q "NOPE" "$CLI"/logs/*.log && fail "refused prepare was logged as prepared"

echo "== mirror to the set + direct to the server"
mc ingest "$CARD" "${ING[@]}" --mirror-set --set FIELD-B --scan-root "$USB" > "$W/i6.out" 2>/dev/null
F6=$(awk '/^ingested /{print $2}' "$W/i6.out")
[ -d "$SRV/mags/$F6" ] && [ ! -e "$CLI/staging/$F6" ] || fail "mirror route server/staging"
for d in FIELD-B_M FIELD-B_S; do grep -q "^$F6 | synced | " "$USB/$d/magvault-backup.txt" || fail "mirror not synced on $d"; done
pass "staging + mirror: committed, staging cleaned, set copy marked synced"
mc ingest "$CARD" "${ING[@]}" --via direct > "$W/i7.out" 2>"$W/i7.err" || { cat "$W/i7.err"; fail "direct"; }
F7=$(awk '/^ingested /{print $2}' "$W/i7.out")
[ -d "$SRV/mags/$F7" ] && [ ! -e "$CLI/staging/$F7" ] || fail "direct route"
[ -z "$(ls -A "$CLI/staging/.incoming")" ] || fail "direct left files in staging/.incoming"
pass "direct: card → server, no local copy left"
expect_fail "direct refused when offline" "needs the server" mc ingest "$CARD" "${ING[@]}" --via direct --no-sync
n_projects=$(sql "select count(*) from projects where name='Field'")
[ "$n_projects" = 1 ] || fail "project duplicated ($n_projects)"
pass "all client mags in one project (project list served by the server)"

echo "== offline client project: one UUID locally, merged into the server's project"
ms ingest "$CARD" --project Night --camera sony-a7siii --shoot-date 2026-11-20 --yes >/dev/null 2>&1 || fail "server ingest Night"
NIGHT=$(sql "select id from projects where name='Night'")
NI=(--project Night --camera sony-a7siii --shoot-date 2026-11-20 --yes --no-sync)
mc ingest "$CARD" "${NI[@]}" > "$W/n1.out" 2>/dev/null || fail "offline Night 1"
mc ingest "$CARD" "${NI[@]}" > "$W/n2.out" 2>/dev/null || fail "offline Night 2"
N1=$(awk '/^ingested /{print $2}' "$W/n1.out"); N2=$(awk '/^ingested /{print $2}' "$W/n2.out")
U1=$(awk -F': ' '/^project_uuid:/{print $2}' "$CLI/staging/$N1/magcard.txt")
U2=$(awk -F': ' '/^project_uuid:/{print $2}' "$CLI/staging/$N2/magcard.txt")
[ -n "$U1" ] && [ "$U1" = "$U2" ] && [ "$U1" != "$NIGHT" ] || fail "offline ingests did not share one local project ($U1 / $U2)"
pass "two offline ingests reuse the client's own project UUID"
mc sync > "$W/n3.out" 2>&1 || { cat "$W/n3.out"; fail "sync Night"; }
[ "$(sql "select count(*) from projects where name='Night'")" = 1 ] || fail "Night duplicated on the server"
[ "$(sql "select count(*) from mags m where m.project_id='$NIGHT' and m.folder in ('$N1','$N2')")" = 2 ] || fail "Night mags not in the server's project"
grep -qx "project_uuid: $NIGHT" "$SRV/mags/$N1/magcard.txt" || fail "magcard not rewritten to the server's project"
grep -q "^ok  *$N1" <<<"$(ms reconcile "$N1")" || fail "reconcile after project merge"
mc projects >/dev/null
grep -q "Night" "$CCFG/projects.local" 2>/dev/null && fail "local project kept after the server listed the name"
pass "server merges by name into its open project, rewrites magcard, client drops its local entry"

# ---------------------------------------------------------------------------------------
if [ "$(id -u)" = 0 ] && command -v losetup >/dev/null && command -v mkfs.ext4 >/dev/null && losetup -f >/dev/null 2>&1; then
  echo "== root: Prepare and Clean on loop devices (real ext4 field set)"
  mkdir -p /etc/magvault
  [ -e /etc/magvault/allow-loop-devices ] || { touch /etc/magvault/allow-loop-devices; MADE_ALLOW=1; }
  for m in $MEDIA/FIELD-L_M $MEDIA/FIELD-L_S; do [ -e "$m" ] && fail "$m already exists; remove it first"; done
  truncate -s 64M "$W/m64.img"; truncate -s 96M "$W/s96.img"; truncate -s 48M "$W/s48.img"
  L64=$(losetup -f --show "$W/m64.img"); L96=$(losetup -f --show "$W/s96.img"); L48=$(losetup -f --show "$W/s48.img")
  LOOPS="$L64 $L96 $L48"
  mc usb prepare-master --device "$L64" --label FIELD-L --yes > "$W/p1.out" 2>&1 || { cat "$W/p1.out"; fail "prepare master"; }
  grep -q "Mount the smaller drive first and prepare it as master." "$W/p1.out" || fail "smaller-first warning"
  MM=$MEDIA/FIELD-L_M
  mountpoint -q "$MM" && grep -q "^role: master" "$MM/magvault-backup.txt" && grep -q "^set_size: 67108864$" "$MM/magvault-backup.txt" || fail "master prepared"
  [ "$(findmnt -n -o FSTYPE "$MM")" = ext4 ] || fail "master is not ext4"
  pass "master: ext4, mounted at $MM, set size 64 MiB"
  expect_fail "smaller slave refused" "smaller slave is refused" mc usb prepare-slave --device "$L48" --yes
  [ ! -e $MEDIA/FIELD-L_S ] || fail "something was prepared on the smaller slave"
  mc usb prepare-slave --device "$L96" --yes > "$W/p2.out" 2>&1 || { cat "$W/p2.out"; fail "prepare slave"; }
  grep -q "Will use 67.1 MB to match master. Rest unused." "$W/p2.out" || { cat "$W/p2.out"; fail "limit message"; }
  MS=$MEDIA/FIELD-L_S
  [ "$(df -B1 --output=size "$MM" | tail -1)" = "$(df -B1 --output=size "$MS" | tail -1)" ] || fail "slave filesystem not limited to the master size"
  grep -q "^role: slave" "$MS/magvault-backup.txt" || fail "slave file"
  [ "$(grep ^set_id "$MM/magvault-backup.txt")" = "$(grep ^set_id "$MS/magvault-backup.txt")" ] || fail "set ids differ"
  pass "larger slave limited to the master size (same filesystem size), same set id"
  mc ingest "$CARD" "${ING[@]}" --via set --set FIELD-L --no-sync > "$W/i8.out" 2>/dev/null || fail "ingest to loop set"
  F8=$(awk '/^ingested /{print $2}' "$W/i8.out")
  expect_fail "Prepare blocked while pending (Magvault)" "Prepare is blocked" mc usb prepare-master --device "$L64" --label FIELD-X --yes
  expect_fail "Prepare blocked while pending (helper)" "pending server sync" "$HELPER" usb-format "$L64" FIELD-X_M 16384
  expect_fail "Clean refused while pending" "pending server sync" mc usb clean --set FIELD-L --yes
  mc sync --set FIELD-L > "$W/s4.out" 2>&1 || { cat "$W/s4.out"; fail "sync loop set"; }
  grep -q "^$F8 | synced | " "$MM/magvault-backup.txt" && grep -q "^$F8 | synced | " "$MS/magvault-backup.txt" || fail "not synced"
  id_before=$(grep ^set_id "$MM/magvault-backup.txt")
  mc usb clean --set FIELD-L --yes > "$W/c1.out" 2>&1 || { cat "$W/c1.out"; fail "clean"; }
  [ ! -e "$MM/$F8" ] && [ ! -e "$MS/$F8" ] || fail "mag folders not deleted"
  for r in "$MM" "$MS"; do
    [ "$(grep ^set_id "$r/magvault-backup.txt")" = "$id_before" ] || fail "set id changed"
    grep -q "^set_size: 67108864$" "$r/magvault-backup.txt" || fail "size limit lost"
    [ -z "$(grep -v '^#' "$r/magvault-backup.txt" | grep '|' || true)" ] || fail "mag list not empty"
  done
  grep -q "^role: master" "$MM/magvault-backup.txt" && grep -q "^role: slave" "$MS/magvault-backup.txt" || fail "roles lost"
  grep -q "USB set FIELD-L  ·  master + slave  ·  67.1 MB  ·  valid  ·  0 mag(s)" <<<"$(mc usb status)" || fail "set not valid and empty after clean"
  grep -q "INFO   fieldset  Field set FIELD-L cleaned: 1 mag(s) removed" "$CLI"/logs/*.log || fail "clean not logged"
  pass "clean: mags removed from both drives; set id, size, roles kept; set valid and empty; logged"
  mc ingest "$CARD" "${ING[@]}" --via set --set FIELD-L --no-sync >/dev/null 2>&1 || fail "set not reusable after clean"
  pass "cleaned set takes new mags without re-prepare"
else
  echo "== root section skipped (needs root, losetup, mkfs.ext4): Prepare/Clean on real drives not run"
fi

echo
echo "ALL PHASE 1 E2E CHECKS PASSED"
