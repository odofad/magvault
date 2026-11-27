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
    for m in $MEDIA/FIELD-L_M $MEDIA/FIELD-L_S $MEDIA/FIELD-T_M $MEDIA/FIELD-T_S; do mountpoint -q "$m" 2>/dev/null && umount "$m"; rmdir "$m" 2>/dev/null || true; done
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
for c in init ingest reconcile serve gate cards sync usb projects ui; do grep -qE "^  $c +" <<<"$top" || fail "--help misses $c"; done
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
Y1="$W/yes1"; mkdir -p "$Y1/VG20/MP_ROOT/100PNV01" "$Y1/VG20/PRIVATE/AVCHD/BDMV/STREAM" "$Y1/VG20/AVF_INFO"
# NEX-VG20 reference layout: MPEG-2 with the model in MVI_DATA; a leftover FDR-AX53 profile in AVF_INFO is ignored.
printf '\x00\x00\x01\xbaMVI_DATA SONY\x00\x00NEX-VG20E\x00HDR-000010G\x00' > "$Y1/VG20/MP_ROOT/100PNV01/M2U00001.MPG"
printf '<MediaProfile><System systemKind="FDR-AX53"/></MediaProfile>' > "$Y1/VG20/AVF_INFO/AVIN0001.INT"
out=$(ms ingest --scan-root "$Y1" --project YesTest --shoot-date 2026-11-20 --yes </dev/null 2>&1) || { echo "$out"; fail "--yes, no path, no TTY, one card"; }
grep -q -- "--yes: one card detected, using $Y1/VG20" <<<"$out" || { echo "$out"; fail "card not picked"; }
grep -q "camera      Sony NEX-VG20 \[sony-vg20\]  (guessed)" <<<"$out" || { echo "$out"; fail "camera not guessed from the live-media model"; }
grep -q "model hint  NEX-VG20E (MPEG MVI_DATA)" <<<"$out" || { echo "$out"; fail "model hint not shown"; }
YF=$(awk '/^ingested /{print $2}' <<<"$out")
[ -d "$SRV/mags/$YF" ] || fail "--yes ingest did not commit"
grep -qx "camera_model_hint: NEX-VG20E (MPEG MVI_DATA)" "$SRV/mags/$YF/magcard.txt" || fail "model hint not on the mag card"
pass "--yes, no path, no TTY, one card: picked; camera guessed from live-media metadata (no --camera), hint on the card"
[ "$(sql "select count(*) from encoder_job_clips jc join encoder_jobs j on j.id=jc.job_id join mags m on m.id=j.mag_id where m.folder='$YF' and jc.status='queued'")" = 1 ] || fail "job clip links"
[ "$(sql "select count(*) from clips c join mags m on m.id=c.mag_id where m.folder='$YF' and c.id not in (select clip_id from encoder_job_clips)")" = 0 ] || fail "clip without a job link"
pass "queued 320p job is linked to the mag's clips (encoder_job_clips)"
mkdir -p "$Y1/NOCAM/DCIM/100XYZ"; head -c 1000 /dev/urandom > "$Y1/NOCAM/DCIM/100XYZ/CLIP0001.MOV"
expect_fail "--yes, no path, no TTY, two cards: refused" "2 cards detected and --yes picks a card only when exactly one is found; pass the path" \
  ms ingest --scan-root "$Y1" --project YesTest --shoot-date 2026-11-20 --yes </dev/null
expect_fail "--yes, no camera guess and no --camera: refused, says what is missing" "camera missing: no camera profile matches this card (layout: Generic DCIM); pass --camera" \
  ms ingest "$Y1/NOCAM" --project YesTest --shoot-date 2026-11-20 --yes </dev/null
Y2="$W/yes2"; mkdir -p "$Y2"; cp -a "$Y1/VG20" "$Y2/"
expect_fail "no path, no TTY, one card, no --yes: refused" "no terminal to confirm the card; re-run with --yes" \
  ms ingest --scan-root "$Y2" --project YesTest --shoot-date 2026-11-20 </dev/null
mkdir -p "$W/mounts-empty"
expect_fail "no card at all: refused" "no camera card detected in $W/mounts-empty" ms ingest --scan-root "$W/mounts-empty" --project YesTest --yes </dev/null

ING=(--project Field --camera sony-a7siii --shoot-date 2026-11-20 --yes)
ONE="$W/mounts1"; mkdir -p "$ONE"; cp -a "$MNT/A7SIII_128" "$ONE/"
CARD="$ONE/A7SIII_128"

echo "== FX6 / XDCAM card: M01.XML sidecars paired, SALVAGE.TMP skipped, model hint, no hard guess"
FX6="$W/fx6/FX6"; mkdir -p "$FX6/XDROOT/Clip" "$FX6/XDROOT/General/Sony" "$FX6/SONY"
for n in 3985 3986; do
  mkclip "$FX6/XDROOT/Clip/PDTV-FX6_$n.MXF"
  printf '<NonRealTimeMeta><Device manufacturer="Sony" modelName="ILME-FX6V" serialNo="2509191"/></NonRealTimeMeta>' > "$FX6/XDROOT/Clip/PDTV-FX6_${n}M01.XML"
done
printf '<MediaProfile><System systemKind="ILME-FX6V ver.5.020"/></MediaProfile>' > "$FX6/XDROOT/MEDIAPRO.XML"
head -c 300000 /dev/zero > "$FX6/XDROOT/General/Sony/SALVAGE.TMP"
expect_fail "FX6 without --camera: refused, names the model the card writes" "camera missing: no camera profile has the model this card names: ILME-FX6V (clip sidecar XML)" \
  ms ingest "$FX6" --project Fx6Test --shoot-date 2026-11-27 --yes </dev/null
out=$(ms ingest "$FX6" --project Fx6Test --camera sony-a7siii --format "XAVC S" --shoot-date 2026-11-27 --yes </dev/null 2>&1) || { echo "$out"; fail "FX6 ingest with a stand-in camera"; }
grep -q "files       5 (2 clips, 2 sidecars, 1 other)" <<<"$out" || { echo "$out"; fail "FX6 M01.XML sidecars not paired"; }
grep -q "skipped     1 junk item(s): XDROOT/General/Sony/SALVAGE.TMP" <<<"$out" || { echo "$out"; fail "SALVAGE.TMP not skipped"; }
XF=$(awk '/^ingested /{print $2}' <<<"$out")
[ ! -e "$SRV/mags/$XF/XDROOT/General/Sony/SALVAGE.TMP" ] || fail "SALVAGE.TMP ingested"
grep -qx "camera_model_hint: ILME-FX6V (clip sidecar XML)" "$SRV/mags/$XF/magcard.txt" || fail "FX6 hint not on the card"
[ "$(sql "select count(*) from files f join mags m on m.id=f.mag_id where m.folder='$XF' and f.role='sidecar' and f.clip_id is not null")" = 2 ] || fail "sidecars not recorded as sidecars of their clips"
grep -q "^ok  *$XF" <<<"$(ms reconcile "$XF")" || fail "reconcile FX6 mag (hint kept)"
grep -qx "camera_model_hint: ILME-FX6V (clip sidecar XML)" "$SRV/mags/$XF/magcard.txt" || fail "reconcile dropped the hint"
pass "FX6: 2 clips + 2 paired M01.XML sidecars, SALVAGE.TMP skipped, hint ILME-FX6V on the card, operator picks the camera"

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
[ -z "$(ls -A "$SRV/mags/.incoming/.clients/dit-a" | grep -v "^\.client-log")" ] || fail "spool not empty after commit"
pass "card picked by discovery (no TTY), staged, verified, committed by serve (job linked to clips), staging cleaned, logged on both sides"

echo "== TUI (a bot at the keyboard in a pseudo-terminal): dashboard → Ingest → pull → back"
cat > "$W/tui1.script" <<'TUI'
wait 1 Modules
wait ● server ready
send i
wait 1 Detected
wait A7SIII_128
# Enter picks the one card; the probe fills Files and Details.
send \r
wait probe done
send nTuiTest\r
wait TuiTest  (new
send p
# The copy runs inside the TUI: progress bar with bytes and percent, no raw terminal.
wait Pull A7SIII_128
wait copying
wait  %)
wait Pull A7SIII_128: ingested mag
send \e
wait 1 Modules
send q
exit 0
TUI
FAKE_RSYNC_FILE_DELAY=0.6 python3 "$REPO/tests/tui_pty.py" "$W/tui1.script" -- "$MV" --config "$CCFG" ui --scan-root "$ONE" > "$W/tui1.out" 2>&1 || { cat "$W/tui1.out"; fail "TUI pull"; }
TF=$(sql "select m.folder from mags m join projects p on p.id=m.project_id where p.name='TuiTest'")
[ -n "$TF" ] && [ -d "$SRV/mags/$TF" ] && [ ! -e "$CLI/staging/$TF" ] || fail "TUI pull did not commit through the client ingest path"
grep -q "Mag $TF committed on the server" "$CLI"/logs/*.log || fail "TUI pull not in the client log"
pass "TUI: card picked with Enter, new project, p pulls inside the TUI with a progress bar (committed, staging cleaned), Esc, q"
cat > "$W/tui2.script" <<'TUI'
wait 1 Modules
send i
wait 1 Detected
send \t\t\t\t
send o
wait Mount the smaller drive first and prepare it as master.
send \e
wait 5 Targets
send p
wait Pull needs:
send x
send \e
wait 1 Modules
send q
exit 0
TUI
python3 "$REPO/tests/tui_pty.py" "$W/tui2.script" -- "$MV" --config "$CCFG" ui --scan-root "$ONE" > "$W/tui2.out" 2>&1 || { cat "$W/tui2.out"; fail "TUI prepare popup"; }
pass "TUI: Prepare popup opens with the smaller-first warning, Esc closes it; Pull refuses without a card"
expect_fail "ui refuses without a terminal" "the TUI needs a terminal" mc ui </dev/null
[ "$("$MV" --config "$CCFG" </dev/null >/dev/null 2>&1; echo $?)" = 2 ] || fail "no command without a terminal should print help (exit 2)"

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

echo "== committer down: gate up, serve down → 'server up, committer down', bounded waits, client issues on the server log"
kill "$SERVE_PID"; wait "$SERVE_PID" 2>/dev/null || true
grep -q "^stopped " "$SRV/mags/.incoming/.results/serve.alive" || fail "serve did not record its stop in the heartbeat"
sed -i 's/^committer_wait:.*/committer_wait: 3/' "$CCFG/magvault.conf"
grep -qx "serve down: stopped at .*" <<<"$(SSH_ORIGINAL_COMMAND=ping "$MV" gate --spool "$SRV/mags/.incoming/.clients" --results "$SRV/mags/.incoming/.results" --client dit-a | sed -n 2p)" || fail "gate ping does not report the committer"
out=$(mc ingest "$CARD" "${ING[@]}" 2>&1) || { echo "$out"; fail "ingest with the committer down"; }
grep -q "WARNING: server up, committer down (stopped at" <<<"$out" || { echo "$out"; fail "no committer-down warning"; }
grep -q "server up, committer down for 3 s" <<<"$out" || { echo "$out"; fail "no timeout message"; }
grep -q "  server  pending" <<<"$out" || { echo "$out"; fail "ingest should leave the mag pending"; }
FD=$(awk '/^ingested /{print $2}' <<<"$out"); [ -d "$CLI/staging/$FD" ] || fail "pending mag not in staging"
pass "ingest: warning 'server up, committer down', waited 3 s (committer_wait), mag stays pending"
start=$(date +%s)
expect_fail "sync times out with a clear message (does not spin)" "server up, committer down for 3 s" mc sync
[ $(( $(date +%s) - start )) -lt 30 ] || fail "sync waited too long"
cat > "$W/tui6.script" <<'TUI'
wait ◐ server up, committer down
wait Its committer is down
send i
wait Server          server up, committer down — the mag waits locally
send \e
wait 1 Modules
send q
exit 0
TUI
python3 "$REPO/tests/tui_pty.py" "$W/tui6.script" -- "$MV" --config "$CCFG" ui --scan-root "$W/mounts-none" > "$W/tui6.out" 2>&1 || { cat "$W/tui6.out"; fail "TUI committer down"; }
pass "TUI dashboard and Ingest show 'server up, committer down' (never 'reachable')"
"$MV" --config "$SCFG" --debug serve > "$W/serve2.log" 2>&1 &   # restarted with DEBUG lines for this run
SERVE_PID=$!
sleep 1
mc --debug sync > "$W/s9.out" 2>&1 || { cat "$W/s9.out"; fail "sync after serve restart"; }
grep -q "synced    $FD" "$W/s9.out" || fail "pending mag not synced after restart"
sleep 3   # serve drains the client lines every 2 s
grep -qE "WARN   client    dit-a  \[[0-9T:Z-]+\] server up, committer down" "$SRV"/logs/*.log || fail "client issue (committer down) not on the server log"
grep -qE "ERROR  client    dit-a  \[[0-9T:Z-]+\] timeout: server up, committer down for 3 s" "$SRV"/logs/*.log || fail "client timeout not on the server log"
grep -qE "ERROR  client    dit-a  \[[0-9T:Z-]+\] sync: Mag $FD failed, stays pending" "$SRV"/logs/*.log || fail "client sync error not on the server log"
pass "client issues (committer down, timeout, sync error) on the server log with the client label"
grep -q "DEBUG  client    dit-a  gate commit $FD" "$SRV"/logs/*.log || fail "gate verb not in the server DEBUG log"
grep -q "DEBUG  serve     accepted commit request $FD from client dit-a" "$SRV"/logs/*.log || fail "serve accept not in DEBUG"
grep -q "DEBUG  ingest    rename .*$FD" "$SRV"/logs/*.log || fail "incoming rename not in DEBUG"
grep -q "DEBUG  verify    start" "$SRV"/logs/*.log || fail "server hash verify not in DEBUG"
for w in "DEBUG  rsync     start copying" "DEBUG  rsync     end copying" "DEBUG  verify    start verifying local copy" "DEBUG  gate      call commit"; do
  grep -q "$w" "$CLI"/logs/*.log || fail "client DEBUG line missing: $w"
done
grep -qE "DEBUG  client    dit-a  \[[0-9T:Z-]+\] rsync: start copying" "$SRV"/logs/*.log || fail "client DEBUG lines not forwarded to the server log"
[ ! -s "$CCFG/issues.pending" ] || fail "client issue queue not flushed"
sed -i 's/^committer_wait:.*/committer_wait: 120/' "$CCFG/magvault.conf"
pass "--debug: gate verbs, serve accept, rsync start/end, hash verify, rename; client DEBUG forwarded to the server"
before=$(grep -c "DEBUG" "$CLI"/logs/*.log | awk -F: '{s+=$NF} END {print s}')
mc projects > /dev/null
after=$(grep -c "DEBUG" "$CLI"/logs/*.log | awk -F: '{s+=$NF} END {print s}')
[ "$before" = "$after" ] || fail "DEBUG lines written without debug on"
pass "no DEBUG lines without debug: yes / --debug"

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
  for m in $MEDIA/FIELD-L_M $MEDIA/FIELD-L_S $MEDIA/FIELD-T_M $MEDIA/FIELD-T_S; do [ -e "$m" ] && fail "$m already exists; remove it first"; done
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

  echo "== root: the TUI drives Prepare and Clean (same functions as the CLI; root needs no sudo prompt)"
  cat > "$W/tui3.script" <<TUI
wait 1 Modules
send i
wait 5 Targets
send \\t\\t\\t\\t
down
down
send c
wait Clean is not offered
wait pending server sync
send x
send \\e
wait 1 Modules
send q
exit 0
TUI
  python3 "$REPO/tests/tui_pty.py" "$W/tui3.script" -- "$MV" --config "$CCFG" ui > "$W/tui3.out" 2>&1 || { cat "$W/tui3.out"; fail "TUI clean refused while pending"; }
  pass "TUI: Clean not offered while a mag is pending"
  mc sync --set FIELD-L > /dev/null 2>&1 || fail "sync before TUI clean"
  cat > "$W/tui4.script" <<TUI
wait 1 Modules
send i
wait 5 Targets
send \\t\\t\\t\\t
down
down
wait [ c Clean ]
send c
wait All mags on FIELD-L are on the server. Wipe master and slave?
send \\r
wait Clean FIELD-L: FIELD-L cleaned; set still valid and empty
send \\e
wait 1 Modules
send q
exit 0
TUI
  python3 "$REPO/tests/tui_pty.py" "$W/tui4.script" -- "$MV" --config "$CCFG" ui > "$W/tui4.out" 2>&1 || { cat "$W/tui4.out"; fail "TUI clean"; }
  [ -z "$(ls "$MM" | grep -E '^[0-9]{8}_')" ] && [ -z "$(ls "$MS" | grep -E '^[0-9]{8}_')" ] || fail "TUI clean left mag folders"
  grep -q "^set_id" "$MM/magvault-backup.txt" || fail "TUI clean removed the set file"
  pass "TUI: Clean offered once synced; popup, Enter, mags removed, set kept"
  truncate -s 64M "$W/t64.img"; truncate -s 96M "$W/t96.img"
  LT64=$(losetup -f --show "$W/t64.img"); LT96=$(losetup -f --show "$W/t96.img"); LOOPS="$LOOPS $LT64 $LT96"
  cat > "$W/tui5.script" <<TUI
wait 1 Modules
send i
wait 5 Targets
send \\t\\t\\t\\t
send o
wait Mount the smaller drive first and prepare it as master.
# d: device names (hidden by default) so the bot can find its loop device.
send d
seek $LT64 
send \\r
wait WILL BE DESTROYED.
send e
send \\x7f\\x7f\\x7f\\x7f\\x7f\\x7f\\x7fFIELD-T\\r
wait label FIELD-T
send \\r
wait step 2 of 2 · slave of FIELD-T
seek $LT96 
wait Will use 67.1 MB to match master. Rest unused.
send \\r\\r
wait slave of FIELD-T ready; set complete
send \\e
wait 1 Modules
send q
exit 0
TUI
  python3 "$REPO/tests/tui_pty.py" "$W/tui5.script" -- "$MV" --config "$CCFG" ui > "$W/tui5.out" 2>&1 || { cat "$W/tui5.out"; fail "TUI prepare"; }
  for r in M S; do
    mountpoint -q "$MEDIA/FIELD-T_$r" && [ "$(findmnt -n -o FSTYPE "$MEDIA/FIELD-T_$r")" = ext4 ] || fail "TUI prepare: FIELD-T_$r"
  done
  [ "$(df -B1 --output=size "$MEDIA/FIELD-T_M" | tail -1)" = "$(df -B1 --output=size "$MEDIA/FIELD-T_S" | tail -1)" ] || fail "TUI slave not limited"
  grep -q "USB set FIELD-T  ·  master + slave  ·  67.1 MB  ·  valid" <<<"$(mc usb status)" || fail "TUI-prepared set not valid"
  pass "TUI: Prepare popup → master (label edited) → slave (larger, limited) through the same Prepare code"
else
  echo "== root section skipped (needs root, losetup, mkfs.ext4): Prepare/Clean on real drives not run"
fi

echo
echo "ALL PHASE 1 E2E CHECKS PASSED"
