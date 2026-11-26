#!/usr/bin/env bash
# End-to-end test of Phase 0: init → ingest → magcard / MHL / DB checks → conflict
# handling → integrity errors → failure paths → orphan folders → escaping of control
# characters → Ctrl+C / SIGTERM / kill -9 during the copy → concurrent ingests.
#
# Uses the real `rsync` when installed. Without it, tests/fake-bin/rsync (a test double
# that copies files) is put on PATH and a notice is printed. Needs: cargo, python3,
# ffmpeg/ffprobe (for realistic clips; plain bytes are used if ffmpeg is missing), setsid and
# pgrep (util-linux, procps). With the real rsync the interrupt tests copy a large random
# file so the signal lands mid-copy: MAGVAULT_E2E_BIG_MB (default 1536) sets its size.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
W="$REPO/target/e2e-work"          # inside the repo, never /tmp (incoming may not be under /tmp)
rm -rf "$W"
mkdir -p "$W"

cargo build --release --offline --quiet --manifest-path "$REPO/Cargo.toml" 2>/dev/null || cargo build --release --quiet --manifest-path "$REPO/Cargo.toml"
MV="$REPO/target/release/magvault"

if ! command -v rsync >/dev/null || [ "${MAGVAULT_E2E_FAKE_RSYNC:-}" = 1 ]; then
  echo "NOTICE: real rsync not used; tests/fake-bin/rsync (test double) is on PATH"
  export PATH="$REPO/tests/fake-bin:$PATH"
fi

pass() { printf '  ok  %s\n' "$*"; }
fail() { printf '  FAIL %s\n' "$*"; exit 1; }
expect_fail() { # expect_fail "<description>" "<stderr substring>" cmd...
  local what="$1" needle="$2"; shift 2
  if out=$("$@" 2>&1); then fail "$what: command succeeded"; fi
  grep -qF -- "$needle" <<<"$out" || { echo "$out"; fail "$what: expected message containing: $needle"; }
  pass "$what"
}

CFG="$W/config"
DATA="$W/data"
mv_() { "$MV" --config "$CFG" "$@"; }

echo "== init"
"$MV" --config "$CFG" init --root "$DATA" >/dev/null
for d in mags mags/.incoming proxies/db bins db logs custom_encode; do [ -d "$DATA/$d" ] || fail "missing $DATA/$d"; done
[ -f "$CFG/cameras/sony-a7siii.cam" ] && [ -f "$CFG/encoders/default-320p.enc" ] || fail "starter profiles"
[ -f "$DATA/db/magvault.sqlite" ] || fail "database"
grep -qx "incoming: $DATA/mags/.incoming" "$CFG/path.conf" || fail "init must write incoming: <mags>/.incoming"
INC="$DATA/mags/.incoming"
mags_entries() { ls -A "$1" | grep -vx '.incoming' || true; }   # .incoming lives in mags/ and is not a mag
pass "config dir, roots (incoming = mags/.incoming), starter profiles, database"
"$MV" --config "$CFG" init >/dev/null && pass "init is idempotent"

echo "== build a fake card"
CARD="$W/cards/A7SIII_128"
mkdir -p "$CARD/DCIM/100MSDCF" "$CARD/PRIVATE/M4ROOT/CLIP" "$CARD/MISC"
mkclip() { # path creation_time
  if command -v ffmpeg >/dev/null; then
    ffmpeg -v error -y -f lavfi -i testsrc=size=320x180:rate=24000/1001 -f lavfi -i sine=frequency=440:sample_rate=48000 \
      -ac 2 -t 1 -c:v libx264 -c:a aac -metadata creation_time="$2" -f mov "$1"
  else
    head -c 200000 /dev/urandom > "$1"
  fi
}
mkclip "$CARD/PRIVATE/M4ROOT/CLIP/A001_C001.mov" 2025-05-20T14:02:11Z
mkclip "$CARD/PRIVATE/M4ROOT/CLIP/A001_C002.mov" 2025-05-20T15:30:00Z
echo '<xml>sidecar</xml>' > "$CARD/PRIVATE/M4ROOT/CLIP/A001_C001.XML"
echo 'card info' > "$CARD/INFO.TXT"
echo 'spaces in name' > "$CARD/MISC/ note with spaces .txt"
echo 'junk' > "$CARD/.DS_Store"
echo 'junk' > "$CARD/MISC/._A001_C001.mov"
head -c 3000000 /dev/urandom > "$CARD/DCIM/100MSDCF/DSC00001.JPG"
pass "card with 2 clips, 1 sidecar, 3 other files, 2 junk files"

echo "== refusals before anything is written"
expect_fail "no confirmation without a terminal" "re-run with --yes" mv_ ingest "$CARD" --project ACME_Campaign_2025 </dev/null
expect_fail "gamma not in profile" "is not offered by camera profile" mv_ ingest "$CARD" --project ACME_Campaign_2025 --gamma HLG --yes
expect_fail "bad shoot date" "must be a real date" mv_ ingest "$CARD" --project X --shoot-date 2025-02-30 --yes
expect_fail "source inside a Magvault root" "overlaps the Magvault root" mv_ ingest "$DATA/mags" --project X --yes
[ -z "$(mags_entries "$DATA/mags")" ] && [ -z "$(ls -A "$INC")" ] || fail "something was written"
pass "mags/ and incoming/ still empty"

echo "== ingest"
mv_ ingest "$CARD" --project ACME_Campaign_2025 --notes "Second unit, stage left" --yes > "$W/ingest1.out" 2> "$W/ingest1.err"
cat "$W/ingest1.out"
FOLDER=$(awk '/^ingested /{print $2}' "$W/ingest1.out")
python3 -I "$REPO/tests/check_ingest.py" "$DATA" "$CARD" "$FOLDER"
[ -z "$(ls -A "$INC")" ] || fail "incoming not empty after ingest (incl. .files/.lock)"
pass "incoming/ empty after commit"
grep -q "INFO   ingest    Mag $FOLDER ingested" "$DATA"/logs/*.log && pass "log line written" || fail "log line"

echo "== second ingest reuses the open project"
mv_ ingest "$CARD" --project ACME_Campaign_2025 --camera sony-a7siii --shoot-date 2025-05-21 --yes > "$W/ingest2.out" 2>/dev/null
FOLDER2=$(awk '/^ingested /{print $2}' "$W/ingest2.out")
[[ "$FOLDER2" == 20250521_* ]] || fail "shoot date flag not used: $FOLDER2"
python3 -I - "$DATA" <<'EOF' || fail "project reuse"
import sqlite3, sys
db = sqlite3.connect(sys.argv[1] + "/db/magvault.sqlite")
assert db.execute("select count(*) from projects").fetchone()[0] == 1
assert db.execute("select count(distinct project_id) from mags").fetchone()[0] == 1
assert db.execute("select count(*) from encoder_jobs where status='queued'").fetchone()[0] == 2
EOF
pass "one project, two mags, two queued 320p jobs"

echo "== conflict handling: database wins"
out=$(mv_ reconcile "$FOLDER"); grep -q "^ok  *$FOLDER" <<<"$out" || { echo "$out"; fail "clean mag not ok"; }
pass "unchanged mag reports ok"
python3 -I - "$DATA" "$FOLDER" <<'EOF'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1] + "/db/magvault.sqlite")
db.execute("update mags set notes='Corrected in database', gamma='S-Log2' where folder=?", (sys.argv[2],))
db.execute("update projects set name='ACME Campaign 2025'")
cid = db.execute("select c.id from clips c join mags m on m.id=c.mag_id where m.folder=? order by relative_path limit 1", (sys.argv[2],)).fetchone()[0]
db.execute("update clips set description='Sun setting behind the stage', rating=4 where id=?", (cid,))
db.execute("insert into tags(name, created_at) values ('sunset','x')")
db.execute("insert into clip_tags(clip_id, tag) values (?, 'sunset')", (cid,))
db.commit()
EOF
cp "$DATA/mags/$FOLDER/magcard.txt" "$W/card-before.txt"
out=$(mv_ reconcile "$FOLDER"); echo "$out"
grep -q "rewritten $FOLDER" <<<"$out" || fail "card not rewritten"
CARDF="$DATA/mags/$FOLDER/magcard.txt"
grep -q '^notes: Corrected in database$' "$CARDF" && grep -q '^gamma: S-Log2$' "$CARDF" && grep -q '^project_name: ACME Campaign 2025$' "$CARDF" || fail "card not from DB"
grep -q '^PRIVATE/M4ROOT/CLIP/A001_C001.mov | 4 | 0 |  | sunset |  |  | Sun setting behind the stage | $' "$CARDF" || { sed -n '/# logging/,$p' "$CARDF"; fail "logging section"; }
BK=( "$DATA/mags/$FOLDER/.magvault/"magcard.*.backup )
[ "${#BK[@]}" = 1 ] && cmp -s "${BK[0]}" "$W/card-before.txt" || fail "backup"
pass "metadata and logging rewritten from DB; old card backed up as ${BK[0]##*/}"
diff <(sed -n '/^# manifest/,/^# logging/p' "$W/card-before.txt") <(sed -n '/^# manifest/,/^# logging/p' "$CARDF") >/dev/null || fail "manifest changed"
pass "manifest section untouched"
out=$(mv_ reconcile "$FOLDER"); grep -q "^ok  *$FOLDER" <<<"$out" || fail "second reconcile not ok"
pass "second reconcile: ok"
sleep 1
python3 -I -c "import sqlite3,sys; d=sqlite3.connect(sys.argv[1]+'/db/magvault.sqlite'); d.execute(\"update mags set notes='Again'\"); d.commit()" "$DATA"
mv_ reconcile --all >/dev/null
n=$(ls "$DATA/mags/$FOLDER/.magvault/" | grep -c '^magcard\..*\.backup$')
[ "$n" = 1 ] || fail "expected a single backup, found $n"
pass "single backup kept after another rewrite"
sed -i 's/^shoot_date: .*/shoot_date: 1999-01-01/' "$CARDF"
out=$(mv_ reconcile "$FOLDER"); grep -q "rewritten $FOLDER.*shoot_date" <<<"$out" || { echo "$out"; fail "edited card not restored"; }
pass "hand-edited card field restored from DB"

echo "== integrity problems are reported, never auto-fixed"
victim="$DATA/mags/$FOLDER/INFO.TXT"
printf 'CARD INFO' > "$victim.new" && mv "$victim.new" "$victim"   # 9 bytes instead of 10
before=$(cat "$CARDF")
set +e; out=$(mv_ reconcile "$FOLDER" --verify-hashes); rc=$?; set -e
[ $rc = 1 ] && grep -q "INTEGRITY $FOLDER" <<<"$out" || { echo "$out"; fail "integrity not reported"; }
[ "$before" = "$(cat "$CARDF")" ] || fail "card was rewritten despite integrity problem"
pass "modified file → INTEGRITY, exit 1, card untouched"
python3 -I -c "import sqlite3,sys; d=sqlite3.connect(sys.argv[1]+'/db/magvault.sqlite'); d.execute(\"update mags set notes='x' where folder=?\", (sys.argv[2],)); d.commit()" "$DATA" "$FOLDER2"
f2="$DATA/mags/$FOLDER2/PRIVATE/M4ROOT/CLIP/A001_C001.XML"
python3 -I -c "import sys; p=sys.argv[1]; b=bytearray(open(p,'rb').read()); b[0]^=1; open(p,'wb').write(b)" "$f2"
set +e; out=$(mv_ reconcile "$FOLDER2"); rc=$?; set -e
[ $rc = 0 ] || fail "size-equal change should not be caught without --verify-hashes"
set +e; out=$(mv_ reconcile "$FOLDER2" --verify-hashes); rc=$?; set -e
[ $rc = 1 ] && grep -q "SHA-256 differs from manifest: PRIVATE/M4ROOT/CLIP/A001_C001.XML" <<<"$out" || { echo "$out"; fail "bit flip"; }
pass "same-size bit flip caught by --verify-hashes"

echo "== failure paths leave nothing behind"
count_mags() { python3 -I -c "import sqlite3,sys; print(sqlite3.connect(sys.argv[1]+'/db/magvault.sqlite').execute('select count(*) from mags').fetchone()[0])" "$DATA"; }
n_before=$(count_mags); ls_before=$(ls "$DATA/mags")
if [ "$(command -v rsync)" = "$REPO/tests/fake-bin/rsync" ]; then
  expect_fail "copy corrupted in transit" "integrity error" env FAKE_RSYNC_CORRUPT=1 "$MV" --config "$CFG" ingest "$CARD" --project ACME_Campaign_2025 --yes
  expect_fail "rsync exits non-zero" "rsync failed (exit 23)" env FAKE_RSYNC_FAIL=23 "$MV" --config "$CFG" ingest "$CARD" --project ACME_Campaign_2025 --yes
else
  echo "  (skipping injected rsync faults: real rsync in use)"
fi
expect_fail "closed project" "is closed" bash -c "python3 -I -c \"import sqlite3; d=sqlite3.connect('$DATA/db/magvault.sqlite'); d.execute(\\\"update projects set status='closed'\\\"); d.commit()\" && '$MV' --config '$CFG' ingest '$CARD' --project 'ACME Campaign 2025' --yes"
[ "$(count_mags)" = "$n_before" ] && [ "$(ls "$DATA/mags")" = "$ls_before" ] && [ -z "$(ls -A "$INC")" ] || fail "leftovers"
pass "no new mags, no leftovers in incoming/"
grep -q "ERROR  ingest" "$DATA"/logs/*.log && pass "failures logged as ERROR" || true

# ---------------------------------------------------------------------------------------
# Second environment: orphan folders, escaping, interrupts, concurrency.
CFG2="$W/config2"
DATA2="$W/data2"
INC2="$DATA2/mags/.incoming"
"$MV" --config "$CFG2" init --root "$DATA2" >/dev/null
m2() { "$MV" --config "$CFG2" "$@"; }
log2() { cat "$DATA2"/logs/*.log; }
count_mags2() { python3 -I -c "import sqlite3,sys; print(sqlite3.connect(sys.argv[1]+'/db/magvault.sqlite').execute('select count(*) from mags').fetchone()[0])" "$DATA2"; }
ING=(ingest --camera sony-a7siii --shoot-date 2026-11-26 --project "Interrupt tests" --yes)
SMALL="$W/cards/SMALL"; mkdir -p "$SMALL/DCIM"; head -c 100000 /dev/urandom > "$SMALL/DCIM/A001_C001.mov"
m2 "${ING[@]}" "$SMALL" >/dev/null 2>&1

echo "== orphan folders in mags/"
out=$(m2 reconcile --all) || { echo "$out"; fail "clean environment not ok"; }
pass "reconcile --all ok; mags/.incoming is not reported"
ORPHAN_NAME="20261126_00000000-0000-4000-8000-000000000000"
mkdir "$DATA2/mags/$ORPHAN_NAME"
set +e; out=$(m2 reconcile --all); rc=$?; set -e
[ $rc = 1 ] && grep -q "^INTEGRITY $ORPHAN_NAME  folder in mags/ has no database record" <<<"$out" || { echo "$out"; fail "orphan not reported"; }
[ -d "$DATA2/mags/$ORPHAN_NAME" ] || fail "orphan folder was touched"
log2 | grep -q "ERROR  system    Folder in mags/ with no database record: $ORPHAN_NAME" || fail "orphan not logged"
pass "orphan folder → INTEGRITY, exit 1, logged, left in place"
rmdir "$DATA2/mags/$ORPHAN_NAME"
m2 reconcile --all >/dev/null || fail "still failing after the orphan was removed"

echo "== control characters are escaped on screen and in the log"
ESC=$'\033'
ESCCARD="$W/cards/ESC"; mkdir -p "$ESCCARD/DCIM"; head -c 1000 /dev/urandom > "$ESCCARD/DCIM/A001_C001.mov"
touch "$ESCCARD/._${ESC}[2Jboom"
m2 "${ING[@]}" "$ESCCARD" > "$W/esc.out" 2> "$W/esc.err"
if grep -q "$ESC" "$W/esc.out" "$W/esc.err" "$DATA2"/logs/*.log; then fail "raw ESC byte on screen or in the log"; fi
grep -qF '._\u{1b}[2Jboom' "$W/esc.err" || { cat "$W/esc.err"; fail "escaped junk name not shown"; }
log2 | grep -qF '  skipped junk: ._\u{1b}[2Jboom' || fail "escaped junk name not logged"
BAD="$W/cards/BAD"; mkdir -p "$BAD"; touch "$BAD/bad${ESC}[31mname.mov"
set +e; m2 "${ING[@]}" "$BAD" 2> "$W/bad.err"; set -e
if grep -q "$ESC" "$W/bad.err"; then fail "raw ESC in refusal message"; fi
grep -qF 'bad\u{1b}[31mname.mov' "$W/bad.err" || { cat "$W/bad.err"; fail "refusal message not escaped"; }
pass "junk name with an escape sequence shown and logged as \\u{1b}; refusal message escaped"

echo "== Ctrl+C, SIGTERM and kill -9 during the copy"
BIG="$W/cards/BIG"; mkdir -p "$BIG/DCIM"
if [ "$(command -v rsync)" = "$REPO/tests/fake-bin/rsync" ]; then
  head -c 2000000 /dev/urandom > "$BIG/DCIM/A001_C001.mov"
  SLOW_LONG="FAKE_RSYNC_DELAY=60"; SLOW_SHORT="FAKE_RSYNC_DELAY=3"
else
  head -c "${MAGVAULT_E2E_BIG_MB:-1536}M" /dev/urandom > "$BIG/DCIM/A001_C001.mov"
  SLOW_LONG="MAGVAULT_E2E=1"; SLOW_SHORT="MAGVAULT_E2E=1"
fi
start_bg() { # start_bg <env assignment> <card>: ingest in its own process group (setsid), like a terminal job
  setsid env "$1" "$MV" --config "$CFG2" "${ING[@]}" "$2" > "$W/bg.out" 2> "$W/bg.err" &
  BG_PID=$!
}
wait_rsync() { # until rsync is a running child of the ingest
  for _ in $(seq 1 1800); do
    pgrep -P "$BG_PID" -f rsync >/dev/null && return 0
    kill -0 "$BG_PID" 2>/dev/null || return 1
    sleep 0.1
  done
  return 1
}
n0=$(count_mags2)

start_bg "$SLOW_LONG" "$BIG"; wait_rsync || fail "rsync did not start"; sleep 0.5
kill -INT -- -"$BG_PID"                       # whole process group, as Ctrl+C at a terminal does
set +e; { wait "$BG_PID"; } 2>/dev/null; rc=$?; set -e
[ $rc != 0 ] || fail "ingest finished before the signal; raise MAGVAULT_E2E_BIG_MB"
grep -q "interrupted by operator (SIGINT)" "$W/bg.err" || { cat "$W/bg.err"; fail "no interrupt message"; }
[ -z "$(ls -A "$INC2")" ] || fail "incoming not clean after Ctrl+C: $(ls -A "$INC2")"
log2 | grep -A2 "ERROR  ingest" | grep -q "interrupted by operator (SIGINT)" || fail "Ctrl+C not logged as ERROR"
[ "$(count_mags2)" = "$n0" ] || fail "a mag was committed"
pass "Ctrl+C: staging, .files and .lock removed, ERROR logged, nothing committed"

start_bg "$SLOW_LONG" "$BIG"; wait_rsync || fail "rsync did not start"; sleep 0.5
kill -TERM "$BG_PID"                          # Magvault only: it must stop rsync itself
set +e; { wait "$BG_PID"; } 2>/dev/null; rc=$?; set -e
[ $rc != 0 ] && grep -q "interrupted by operator (SIGTERM)" "$W/bg.err" || { cat "$W/bg.err"; fail "SIGTERM not handled"; }
log2 | grep -A2 "ERROR  ingest" | grep -q "interrupted by operator (SIGTERM)" || fail "SIGTERM not logged as ERROR"
sleep 0.5
pgrep -f "rsync.*$INC2" >/dev/null && fail "rsync still running after SIGTERM"
[ -z "$(ls -A "$INC2")" ] || fail "incoming not clean after SIGTERM: $(ls -A "$INC2")"
pass "SIGTERM to Magvault alone: rsync stopped, incoming clean, ERROR logged"

start_bg "$SLOW_LONG" "$BIG"; wait_rsync || fail "rsync did not start"; sleep 0.5
kill -KILL -- -"$BG_PID"                      # crash / power loss: nothing can clean up
set +e; { wait "$BG_PID"; } 2>/dev/null; set -e
LEFT=$(ls "$INC2")
[ -n "$LEFT" ] && [ -e "$INC2/.$LEFT.files" ] && [ -e "$INC2/.$LEFT.lock" ] || fail "expected a leftover folder, .files and .lock"
m2 "${ING[@]}" "$SMALL" > /dev/null 2> "$W/after.err"
grep -q "from an interrupted ingest" "$W/after.err" && grep -qF "$INC2/$LEFT/ + $INC2/.$LEFT.files" "$W/after.err" || { cat "$W/after.err"; fail "no warning on screen"; }
log2 | grep -A1 "WARN   ingest" | grep -qF "$INC2/$LEFT/ + $INC2/.$LEFT.files" || fail "WARN not logged with folder and list"
[ -d "$INC2/$LEFT" ] || fail "leftover must not be cleaned automatically"
pass "kill -9: next ingest warns on screen and logs WARN naming $LEFT and its .files list; left for manual cleanup"
rm -rf "${INC2:?}/$LEFT" "$INC2/.$LEFT.files" "$INC2/.$LEFT.lock"

echo "== two ingests at the same time: no false warning"
warns_before=$(log2 | grep -c "WARN   ingest" || true)
start_bg "$SLOW_SHORT" "$BIG"; wait_rsync || fail "rsync did not start"
m2 "${ING[@]}" "$SMALL" > /dev/null 2> "$W/concurrent.err"
set +e; { wait "$BG_PID"; } 2>/dev/null; rc=$?; set -e
[ $rc = 0 ] || { cat "$W/bg.err"; fail "background ingest failed"; }
if grep -q "interrupted ingest" "$W/concurrent.err"; then fail "live ingest reported as interrupted"; fi
[ "$(log2 | grep -c "WARN   ingest" || true)" = "$warns_before" ] || fail "WARN logged for a live ingest"
[ -z "$(ls -A "$INC2")" ] || fail "incoming not clean after both ingests"
pass "the running ingest's staging folder is not reported; both ingests committed"

echo
echo "ALL E2E CHECKS PASSED"
