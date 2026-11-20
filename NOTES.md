# Notes and borrowed ideas

Ideas worth taking from other open-source projects while building Magvault. All open source; attribution is welcome.

## From BitMatch
https://github.com/BitmatchApp/Bitmatch

- Verified copy model: hash while copying (SHA-256), read the destination back, only mark the mag committed when verification passes.
- Camera/card layout detection list (Sony, Canon, ARRI, RED, Blackmagic, Panasonic, Fujifilm, GoPro, DJI, Insta360, generic DCIM) as the seed for auto-detect guesses shown to the operator.
- Keep original filenames and camera sidecars; only skip junk like AppleDouble.
- Optional ASC MHL (or equivalent) inventory written next to the dump so the complete manifest is independently checkable.
- Clear failure marking (e.g. rename bad files so they are obvious) and "unsafe until verified" status.
- Queue idea for later multi-mag or multi-destination backups.

## From other ingest tools
CosechaMedia, CameraCopy, cambak, WrangleBot

- Selective / incremental dump options (copy only new or date-filtered content).
- Device/mount detection and persistent camera labels.
- Simple progress and report export (CSV/JSON) after a dump.
- Proxy generation hooks at the end of a successful ingest (we already plan 320p).

## From egui examples (for Phase 4)
lwa_fm and general egui patterns

- Split-panel layouts: file/mag list on one side, details/preview on the other.
- Confirmation dialogs and inline editable fields for the operator sign-off step.
- Lightweight progress bars and status lines that do not block the UI.

## Explicitly not taking

- Any web/Docker/Plex-style architecture.
- Renaming files into house patterns.
- Free-text project names.
- Treating the card as authoritative over the database.
