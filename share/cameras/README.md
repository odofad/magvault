# Camera profiles

One `.cam` file per camera. Filename is `<id>.cam`. Schema is in PLAN.md (`.cam` file schema).

Add a profile here as cameras are confirmed on the test bench. Copy the file into the server `cameras/` root and the client cameras path so both guess the same camera.

Every `.cam` file here is shipped: `src/init.rs` `STARTER_CAMERAS` lists them all (a unit
test fails when a file here is missing from that list), and `magvault init` copies them
into an empty cameras folder. Existing installs: copy new files by hand (`cp -n`).

Detection (docs/camera-identification-findings.md):
- `model:` is the model string the camera writes (comma list for spellings). A model read
  from metadata next to live media (clip sidecar XML, MEDIAPRO.XML beside the clips, MPEG
  MVI_DATA, EXIF, Panasonic XML) that names this model wins over layout and file names.
  It matches exactly or with a letter suffix (`NEX-VG20` ↔ `NEX-VG20E`), never with a
  further digit (`FDR-AX3` does not match `FDR-AX53`).
- `card_layout:` folders must all be present; nested paths such as `PRIVATE/M4ROOT` are
  allowed and must hold media.
- `filename_pattern:` when given, a media file must match.
- `vendor: Generic` profiles (generic-avchd) are a fallback only: they are guessed only
  when no specific profile matches.
