"""Independent checks of one ingested mag (used by tests/e2e.sh).

Hashes are recomputed with Python's hashlib; C4 IDs with a separate Base58 encoder.
Usage: check_ingest.py <data root> <card dir> <mag folder>
"""
import hashlib
import os
import re
import shutil
import sqlite3
import sys
import time
import xml.etree.ElementTree as ET

data, card, folder = sys.argv[1:4]
mag = os.path.join(data, "mags", folder)
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def ok(msg):
    print(f"  ok  {msg}")


def c4(b):
    n = int.from_bytes(hashlib.sha512(b).digest(), "big")
    s = ""
    while n:
        n, r = divmod(n, 58)
        s = B58[r] + s
    return "c4" + s.rjust(88, "1")


def tree(root, skip_internal):
    out = {}
    for dp, dns, fns in os.walk(root):
        rel_dir = os.path.relpath(dp, root)
        if skip_internal and rel_dir.split(os.sep)[0] in ("ascmhl", ".magvault"):
            continue
        for f in fns:
            rel = os.path.normpath(os.path.join(rel_dir, f))
            if skip_internal and rel == "magcard.txt":
                continue
            out[rel] = os.path.join(dp, f)
    return out


# ---- folder name -----------------------------------------------------------------
m = re.fullmatch(r"(\d{8})_([0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12})", folder)
assert m, folder
mag_uuid = m.group(2)
if shutil.which("ffprobe"):
    expected = time.strftime("%Y%m%d", time.localtime(1747749731))  # 2025-05-20T14:02:11Z, earliest clip
    assert m.group(1) == expected, (m.group(1), expected)
ok(f"folder {folder}: shoot day + full v4 UUID")

# ---- copied files ------------------------------------------------------------------
src = tree(card, False)
dst = tree(mag, True)
assert set(src) == set(dst), (set(src) ^ set(dst))
for rel in src:
    assert open(src[rel], "rb").read() == open(dst[rel], "rb").read(), rel
    assert int(os.stat(src[rel]).st_mtime) == int(os.stat(dst[rel]).st_mtime), rel
assert os.path.exists(os.path.join(mag, ".DS_Store")) and os.path.exists(os.path.join(mag, "MISC", "._A001_C001.mov"))
ok(f"{len(src)} files copied byte-identical with mtimes, .DS_Store and ._ files included (no junk filter)")

# ---- magcard -------------------------------------------------------------------------
text = open(os.path.join(mag, "magcard.txt"), encoding="utf-8").read()
header, rest = text.split("\n# manifest\n", 1)
manifest_part, logging_part = rest.split("\n# logging\n", 1)
h = dict(line.split(": ", 1) if ": " in line else (line.rstrip(":"), "") for line in header.strip().splitlines())
for k in ["format", "mag_uuid", "mag_folder", "project_uuid", "project_name", "project_status", "shoot_date",
          "camera_profile_id", "camera_model", "file_format", "gamma", "gamut", "source_label", "ingested_by",
          "ingested_at", "updated_at", "notes"]:
    assert k in h, k
assert h["format"] == "magvault-magcard/1"
assert h["mag_uuid"] == mag_uuid and h["mag_folder"] == folder
assert h["camera_profile_id"] == "sony-a7siii" and h["camera_model"] == "Sony A7S III"
assert (h["file_format"], h["gamma"], h["gamut"]) == ("ProRes RAW", "S-Log3", "S-Gamut3.Cine")
assert h["project_name"] == "ACME_Campaign_2025" and h["project_status"] == "open"
assert h["notes"] == "Second unit, stage left"
assert h["shoot_date"] == f"{folder[0:4]}-{folder[4:6]}-{folder[6:8]}"
assert re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", h["ingested_at"])
rows = [l for l in manifest_part.splitlines() if l and not l.startswith("#")]
manifest = {}
for l in rows:
    f = l.split(" | ")
    assert len(f) == 4 and l.count("|") == 3, l
    rel, size, sha, mtime = f
    assert not rel.startswith("/") and ".." not in rel.split("/"), rel
    manifest[rel] = (int(size), sha, mtime)
assert set(manifest) == set(src), set(manifest) ^ set(src)
for rel, (size, sha, mtime) in manifest.items():
    b = open(dst[rel], "rb").read()
    assert size == len(b), rel
    assert sha == hashlib.sha256(b).hexdigest(), rel
assert " note with spaces .txt" in "".join(manifest), "leading/trailing spaces preserved"
assert [l for l in logging_part.splitlines() if l and not l.startswith("#")] == []
ok("magcard.txt: header complete, manifest = relative paths + correct SHA-256, empty logging section")

# ---- ASC MHL -------------------------------------------------------------------------
names = sorted(os.listdir(os.path.join(mag, "ascmhl")))
assert len(names) == 2 and names[1] == "ascmhl_chain.xml", names
assert re.fullmatch(r"0001_" + re.escape(folder) + r"_\d{4}-\d\d-\d\d_\d{6}Z\.mhl", names[0]), names[0]
mhl_bytes = open(os.path.join(mag, "ascmhl", names[0]), "rb").read()
ns = {"m": "urn:ASC:MHL:v2.0"}
root = ET.fromstring(mhl_bytes)
assert root.tag == "{urn:ASC:MHL:v2.0}hashlist" and root.get("version") == "2.0"
assert root.find("m:creatorinfo/m:creationdate", ns) is not None and root.find("m:creatorinfo/m:hostname", ns) is not None
assert root.find("m:processinfo/m:process", ns).text == "in-place"
patterns = [p.text for p in root.findall("m:processinfo/m:ignore/m:pattern", ns)]
assert "magcard.txt" in patterns and "ascmhl" in patterns and ".magvault" in patterns
hashes = root.findall("m:hashes/m:hash", ns)
assert len(hashes) == len(src)
for hh in hashes:
    p = hh.find("m:path", ns)
    rel = p.text
    b = open(dst[rel], "rb").read()
    assert int(p.get("size")) == len(b)
    assert p.get("lastmodificationdate").endswith("Z")
    c = hh.find("m:c4", ns)
    assert c.get("action") == "original" and c.text == c4(b), rel
    assert len(c.text) == 90
chain = ET.parse(os.path.join(mag, "ascmhl", "ascmhl_chain.xml")).getroot()
assert chain.tag == "{urn:ASC:MHL:DIRECTORY:v2.0}ascmhldirectory"
entries = chain.findall("{urn:ASC:MHL:DIRECTORY:v2.0}hashlist")
assert len(entries) == 1 and entries[0].get("sequencenr") == "1"
assert entries[0].find("{urn:ASC:MHL:DIRECTORY:v2.0}path").text == names[0]
assert entries[0].find("{urn:ASC:MHL:DIRECTORY:v2.0}c4").text == c4(mhl_bytes)
ok("ASC MHL: spec file name, C4 per file (independently recomputed), chain file with C4 of the manifest")

# ---- database -----------------------------------------------------------------------
db = sqlite3.connect(os.path.join(data, "db", "magvault.sqlite"))
db.row_factory = sqlite3.Row
assert db.execute("pragma journal_mode").fetchone()[0] == "wal"
p = db.execute("select * from projects").fetchall()
assert len(p) == 1 and p[0]["name"] == "ACME_Campaign_2025" and p[0]["status"] == "open" and p[0]["id"] == h["project_uuid"]
mrow = db.execute("select * from mags where id = ?", (mag_uuid,)).fetchone()
assert mrow["folder"] == folder and mrow["status"] == "online" and mrow["project_id"] == p[0]["id"]
assert mrow["camera_profile_id"] == "sony-a7siii" and mrow["gamma"] == "S-Log3"
assert mrow["file_count"] == len(src) and mrow["size_bytes"] == sum(m[0] for m in manifest.values())
assert mrow["shoot_date"] == h["shoot_date"] and mrow["notes"] == h["notes"]
files = db.execute("select * from files where mag_id = ?", (mag_uuid,)).fetchall()
assert {f["relative_path"] for f in files} == set(src)
for f in files:
    assert f["sha256"] == manifest[f["relative_path"]][1]
    assert not f["relative_path"].startswith("/")
roles = {f["relative_path"]: f["role"] for f in files}
assert roles["PRIVATE/M4ROOT/CLIP/A001_C001.mov"] == "media"
assert roles["PRIVATE/M4ROOT/CLIP/A001_C001.XML"] == "sidecar"
assert roles["INFO.TXT"] == "other" and roles["DCIM/100MSDCF/DSC00001.JPG"] == "other"
assert roles[".DS_Store"] == "other" and roles["MISC/._A001_C001.mov"] == "other", "copied, but AppleDouble is not a clip"
clips = db.execute("select * from clips where mag_id = ? order by relative_path", (mag_uuid,)).fetchall()
assert [c["relative_path"] for c in clips] == ["PRIVATE/M4ROOT/CLIP/A001_C001.mov", "PRIVATE/M4ROOT/CLIP/A001_C002.mov"]
side = [f for f in files if f["role"] == "sidecar"][0]
assert side["clip_id"] == clips[0]["id"]
for c in clips:
    assert c["proxy_status"] == "pending" and c["checksum"] == manifest[c["relative_path"]][1]
    if shutil.which("ffprobe"):
        assert c["fps"] == "24000/1001" and c["width"] == 320 and c["height"] == 180, dict(c)
        assert c["audio_streams"] == 1 and c["audio_channels"] == "2" and abs(c["duration_ms"] - 1001) < 50, dict(c)
jobs = db.execute("select * from encoder_jobs where mag_id = ?", (mag_uuid,)).fetchall()
assert len(jobs) == 1
j = jobs[0]
assert (j["type"], j["status"], j["profile_id"], j["clips_total"]) == ("320p", "queued", "default-320p", 2)
assert j["output_root"] == f"proxies/db/{folder}"
assert not os.listdir(os.path.join(data, "proxies", "db")), "Phase 0 must not encode"
ok("database: project, mag (online), 2 clips with ffprobe data, files with roles, queued 320p job, no proxies written")
