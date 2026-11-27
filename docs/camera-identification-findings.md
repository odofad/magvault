# Camera identification findings from reference cards

27 November 2026 (Africa/Johannesburg). Consolidated from the reference-card work on that day. Source notes (uncommitted working copies): `docs/camera-profiles-detection-decision.md`, `docs/test-report-camera-profiles-ax3.md`, and `docs/test-report-fx6-usb-ingest.md`.

All cards were read only. Nothing was written, wiped, formatted, or Cleaned on camera media. No detection code was changed as part of this work. The findings and the recommended profile shape have not been deployed.

**Naming (standing note):** The user calls one body a **Sony FDR-AX3**. The camera firmware writes **FDR-AX53** in every readable model field. No card from this body contains the string “FDR-AX3”. In notes and in any future profile, keep both: “FDR-AX3” is the name the user uses; “FDR-AX53” is the metadata key the camera writes. They are not two different bodies in this test set.

## Purpose

Figure out how Magvault can tell cameras apart from a docked card (or a camera mounted as mass storage), and what a profile should look like for each body in the set. Seven reference cards were docked on the client workstation: six Sony bodies and one Panasonic. Each was fingerprinted, copied to a reference archive, and compared with today’s detector (`~/magvault.sh cards`).

This document is the single record of those findings: how to identify a camera, what each card contained, why the current guess is often wrong, and what code and profile work would be needed.

## How to reliably identify a camera

Identify a body with layered evidence, strongest first. A hint must not be a silent hard guess.

### 1. Model string written by the camera (strongest)

Read it from metadata that belongs to the **live media**:

- Sony clip sidecar `Device modelName` (`*M01.XML`, Sony `NonRealTimeMeta`)
- Sony `MEDIAPRO.XML` `systemKind` — **prefix-match**; FS5, FS7, and the FX6 append ` ver.x` (`PXW-FS5 ver.4.020`, `PXW-FS7 ver.4.100`, `ILME-FX6V ver.5.020`)
- Sony MPEG `MVI_DATA` for the VG20 (`NEX-VG20E`)
- Still or thumb EXIF Make/Model (A7 III thumb: `SONY` / `ILCE-7M3`)
- Panasonic `Device ModelName` in the **embedded MOV XML** (`com.panasonic.Semi-Pro.metadata.xml` / `ClipMain`, `AG-CX350`) and in the **P2 sidecar** / embedded MXF XML (`CONTENTS/CLIP/*.XML` / `P2Main`, **`CX350`** — shorter spelling, same body)

AVCHD `INDEX.BDM` and `.MTS` files **never name the camera**. Sony `HDR-000010G_MC1` is shared by VG20, AX53, and FS5; the A7 III empty-AVCHD string is `DSLR-00002G_MC1` (a useful extra tell, not a model name). The CX350 AVCHD index has no Sony-style `HDR-` / `DSLR-` maker string; it carries `LSLX64G`, which also appears in `INDEX.DAT` and looks like a media or card id, not a model.

### 2. Only trust that string from metadata next to live media

Leftover Sony `MEDIAPRO.XML` or `AVF_INFO` twice named the camera that *formatted* the card, not the one that recorded the footage still on it. An early mixed card carried FDR-AX53 and PXW-FS5 identities, with FS5 media in the macOS trash. The real VG20 card still had FDR-AX53 in `AVF_INFO`. `systemId` fingerprints a **body**, not a model line. Some Sony bodies write a dummy serial (`4294967295` / `0xFFFFFFFF` on both AX53 and A7 III). The Panasonic CX350 wrote a real serial (`L3TRB0138`).

### 3. Layout and filename as a family fallback

Use these when the model string is missing or needs confirming:

| Layout + names | Family |
|---|---|
| `PRIVATE/XDROOT` + `Clip????.MXF` | PXW-FS5-class |
| Top-level `XDROOT` + `PDTV-FX6_*.MXF` | ILME-FX6 |
| Top-level `XDROOT` + `A###C###_*.MXF` | PXW-FS7 (same top-level `XDROOT` as the FX6; split by filename or XML) |
| Top-level `MP_ROOT` without `M4ROOT` + `M2U?????.MPG` | NEX-VG20-class |
| `PRIVATE/M4ROOT` + `C????.MP4` | FDR-AX53 or ILCE-7M3 (and possibly other Sony stills/video bodies) — **layout cannot split these two** |
| Top-level `CONTENTS` + `LASTCLIP.TXT` + `PRIVATE/PANA_GRP` + `PRIVATE/AVCHD` | **Panasonic** family (P2 MXF and/or MOV). Layout names the vendor, not CX350 versus another Panasonic P2/AVCCAM body |

Today’s detector only sees **top-level** folders and filename globs; the documented `model:` key is not used. That is why `sony-vg20` (`PRIVATE` + `*.MTS`) wrongly claimed AX53, FS5, A7 III, and Panasonic AG-CX350 cards.

### 4. The operator confirms or overrides

Show the evidence above and let the operator confirm or override, especially on a reused card.

### Log and gamma are per clip, not per body

When present, Sony `AcquisitionRecord` `CaptureGammaEquation` / `CaptureColorPrimaries` and Panasonic `AcquisitionMetadata` `CaptureGamma` / `CaptureGamut` name the transfer and gamut of **that take** (Sony A7 III and FX6: **S-Log3 / S-Gamut3.Cine**; Panasonic CX350 C002: **V-Log / V-Gamut**; CX350 C001 and the P2 clip: `HD` / `BT.709`). The FS7 test card had no such fields; those clips look Rec.709. ffprobe colour tags were often unknown, or still `bt709`, even when the XML said log.

**AVCHD is a special case.** Across every reference card that held AVCHD (AX53, FS5, VG20, and CX350), every AVCHD clip looked Rec.709 and **none carried gamma metadata** — no `AcquisitionRecord`, no `CaptureGamma`, no readable log flag in `INDEX.BDM` / `.MTS`. **Default AVCHD to Rec.709**, but only as a default the operator can override. AVCHD never records gamma, so log shot in AVCHD (if a body allows it) would be undetectable.

## Summary comparison

| Camera (user name / firmware) | Model string (this card) | Layout + names | Log on this card | Today's `cards` guess |
|---|---|---|---|---|
| FDR-AX3 / **FDR-AX53** | `modelName` / `systemKind` `FDR-AX53` | `PRIVATE/M4ROOT` `C????.MP4` + `PRIVATE/AVCHD` `*.MTS` | Rec.709 on the reference takes (no AcquisitionRecord gamma). AVCHD: Rec.709, no gamma metadata | sony-vg20 (wrong) |
| PXW-FS5 / **PXW-FS5** | `PXW-FS5` (`systemKind` `ver.4.020`) | `PRIVATE/XDROOT` `Clip????.MXF` + AVCHD | Rec.709. AVCHD: Rec.709, no gamma metadata | sony-vg20 (wrong; empty `M4ROOT` stole the layout label) |
| FX6 / **ILME-FX6V** | `ILME-FX6V` (`systemKind` `ver.5.020`) | top-level `XDROOT` `PDTV-FX6_*.MXF` | **S-Log3 / S-Gamut3.Cine** | camera not guessed |
| NEX-VG20 / **NEX-VG20E** | `NEX-VG20E` in MPEG `MVI_DATA` (no live `M4ROOT` XML) | `MP_ROOT` `M2U?????.MPG` + AVCHD, no `M4ROOT` | Rec.709 (no XML gamma). AVCHD: Rec.709, no gamma metadata | sony-vg20 (right this time; leftover AX53 in `AVF_INFO` must be ignored) |
| PXW-FS7 / **PXW-FS7** | `PXW-FS7` (`systemKind` `ver.4.100`) | top-level `XDROOT` `A###C###_*.MXF` | none in sidecar (looks Rec.709) | camera not guessed |
| A7 III / **ILCE-7M3** | `ILCE-7M3` in XML and thumb EXIF | `PRIVATE/M4ROOT` `C????.MP4` (same as AX53); empty AVCHD `DSLR-00002G_MC1` | **S-Log3 / S-Gamut3.Cine** | sony-vg20 (wrong) |
| AG-CX350 / **AG-CX350** (`CX350` in P2 XML) | `AG-CX350` in embedded MOV XML; `CX350` in P2 sidecar / MXF | top-level `CONTENTS` + `PRIVATE/PANA_GRP` `A###C###_*.MOV` + `PRIVATE/AVCHD` `*.MTS` + P2 `CONTENTS/AVCLIP/*.MXF`; empty `DCIM` | **V-Log / V-Gamut** on C002; Rec.709 on C001 and the P2 clip; AVCHD: Rec.709, no gamma metadata | sony-vg20 (wrong) |

## Reference card: FDR-AX3 / FDR-AX53

Freshly formatted in this camera, with one clip in each format the user listed: XAVC S 4K, XAVC S HD, and AVCHD. Docked as an SD card (exFAT, 59.2 GB).

An earlier mixed card from the same body (same `systemId` `04C461FFFE1806E4`) also held leftover PXW-FS5 `PRIVATE/XDROOT` and macOS trash; that card is evidence of the leftover-metadata trap, not the clean reference. The clean card is the one described here.

**Layout.** Top level: `PRIVATE/`, `AVF_INFO/` only. No `DCIM`, no `XDROOT`.

- `PRIVATE/M4ROOT/` — XAVC S: `CLIP/C0001.MP4` + `C0001M01.XML` + thumb, `CLIP/C0002.MP4` + `C0002M01.XML` + thumb, `MEDIAPRO.XML`, `STATUS.BIN`
- `PRIVATE/AVCHD/BDMV/` — one AVCHD clip: `STREAM/00000.MTS`, CLIPINF, playlist, `INDEX.BDM`, `MOVIEOBJ.BDM`
- `PRIVATE/SONY/SONYCARD.IND` — empty marker
- `AVF_INFO/` — Sony index/preview binaries

**Model evidence.** Firmware writes **FDR-AX53** in `MEDIAPRO.XML` `systemKind` and in both clip sidecars `Device modelName`. `systemId="04C461FFFE1806E4"`, `masterVersion="XAVC-M4@1.10.00"`. Serial in XML is the dummy `4294967295`. AVCHD `INDEX.BDM` / MPL / CPI carry `0002HDR-000010G_MC1` (not a model name). The string “FDR-AX3” appears nowhere. ffprobe has no make/model on these clips.

**Formats.**

| # | User label | Path | Video | Audio | Other | Duration |
|---|---|---|---|---|---|---|
| 1 | XAVC S 4K | `PRIVATE/M4ROOT/CLIP/C0001.MP4` | H.264 High L5.1, 3840×2160p25, ~56.3 Mb/s. XML: `AVC_3840_2160_HP@L51` | LPCM 16-bit BE, 2 ch | `C0001M01.XML`; thumb; no AcquisitionRecord gamma (Rec.709) | 6.240 s |
| 2 | XAVC S HD | `PRIVATE/M4ROOT/CLIP/C0002.MP4` | H.264 High L4.2, 1920×1080p50, ~46.5 Mb/s. XML: `AVC_1920_1080_HP@L42` | LPCM 16-bit BE, 2 ch | `C0002M01.XML`; thumb; Rec.709 | 5.760 s |
| 3 | AVCHD | `PRIVATE/AVCHD/BDMV/STREAM/00000.MTS` | H.264 High L4.0, 1920×1080i25, ~16.6 Mb/s | AC-3 5.1, 448 kb/s | No XML, no gamma metadata, PGS subtitle stream | 5.728 s |

**Current guess.** `Sony XAVC (PRIVATE/M4ROOT)  ·  Sony NEX-VG20 [sony-vg20]`. Layout is fair (`M4ROOT` holds clips). The VG20 guess is wrong: it matches top-level `PRIVATE` and `*.MTS`.

**Archive.** `/wildraid/camtest/Sony-FDR-AX3/` — 18 files, 14 directories, 165,116,307 bytes; all sha256 matched. Metadata (no media): `reference-cards/fdr-ax3/`.

## Reference card: PXW-FS5

SD card, exFAT, 119.1 GB. User formats: XAVC QFHD, XAVC HD, AVCHD. All three present; one extra XAVC HD Intra take (Clip0003), nothing missing.

**Layout.** Top level: `PRIVATE/`, `AVF_INFO/` only. Live media is under **`PRIVATE/XDROOT`**, plus `PRIVATE/AVCHD`. An **empty** `PRIVATE/M4ROOT` is still present and stole today’s layout label.

- `PRIVATE/XDROOT/Clip/` — `Clip0001.MXF`, `Clip0002.MXF`, `Clip0003.MXF` and `*M01.XML` sidecars
- `PRIVATE/AVCHD/` — one `00000.MTS`
- `MEDIAPRO.XML` / `DISCMETA.XML` under XDROOT; empty M4ROOT `MEDIAPRO.XML` also exists

Camera clock appeared unset (media dates 1 January 2015, offset −05:00).

**Model evidence.** `systemKind="PXW-FS5 ver.4.020"`, `modelName="PXW-FS5"`, real serial `1208189`, `systemId="DCEFCAFFFE834F33"`. AVCHD carries the same `HDR-000010G_MC1` as AX53 and VG20. Rec.709 / nxcam-std on these clips (S-Log2 / S-Gamut is available on this body, not used here).

**Formats.**

| # | User label | Clip | Video | Audio | Other | Duration |
|---|---|---|---|---|---|---|
| 1 | XAVC QFHD | `Clip0001.MXF` | XAVC Long GOP, 3840×2160p25, 4:2:0 | 4 ch 24-bit LPCM | `Clip0001M01.XML` | ~5 s class |
| 2 | XAVC HD | `Clip0002.MXF` | XAVC Intra 50, 1920×1080p50, 4:2:2 10-bit | 4 ch 24-bit LPCM | sidecar | |
| — | extra XAVC HD Intra | `Clip0003.MXF` | same family as clip 2 | 4 ch 24-bit LPCM | sidecar | |
| 3 | AVCHD | `00000.MTS` | H.264 High L4.0, 1920×1080i50, ~22.9 Mb/s | pcm_bluray stereo | no XML, no gamma metadata | 5.200 s |

**Current guess.** `Sony XAVC (PRIVATE/M4ROOT)  ·  Sony NEX-VG20 [sony-vg20]`. Both parts are misleading: live media is XDCAM under `PRIVATE/XDROOT`, and the body is an FS5, not a VG20. Empty `M4ROOT` stole the layout line.

**Archive.** `/wildraid/camtest/Sony-PXW-FS5/` — 24 files, 22 directories, 231,990,507 bytes; all sha256 matched. Metadata: `reference-cards/pxw-fs5/`.

## Reference card: Sony FX6 (USB direct)

The camera was connected to the client over USB as mass storage (no card reader). USB `054c:0d21`, model ILME-FX6V, USB 2 High Speed (480 Mbps). Two LUNs: media (exFAT, UUID `ACAE-E116`, 149.1 GiB) and `PMHOME` (64 MiB vfat, licences only — not a second media slot). Only one media slot was occupied.

**Layout.** Top level on the media volume: `SONY/`, **`XDROOT/`** only. No `PRIVATE`. Clip names `PDTV-FX6_3985.MXF` / `PDTV-FX6_3986.MXF`. Also thumbs, `MEDIAPRO.XML`, `DISCMETA.XML`, `CUEUP.XML`, a LUT (`1_SGamut3CineSLog3_To_LC-709.cube`), and `XDROOT/General/Sony/SALVAGE.TMP` (256 MiB, not media).

**Model evidence.** `systemKind="ILME-FX6V ver.5.020"`, `modelName="ILME-FX6V"`, real serial `2509191`, `systemId="5026EFFFFEBC9977"`, `masterVersion="XAVC-XD@2.00.00"`, mediaprofile version 2.30. USB serial `D46D40264987`. Both clips have `*M01.XML` sidecars (stems do not match the MXF for today’s classifier). Firmware / software 5.02; lens FE PZ 28-135mm F4 G OSS.

**Formats.**

| # | Format | Clip | Video | Audio | Other | Duration |
|---|---|---|---|---|---|---|
| 1 | XAVC Intra 4K | `PDTV-FX6_3985.MXF` | Intra 4K 50p, 10-bit 422, ~506 Mb/s | 8 ch 24-bit LPCM | sidecar; **S-Log3 / S-Gamut3.Cine**; LUT present | 5.360 s |
| 2 | XAVC Long 4K | `PDTV-FX6_3986.MXF` | Long GOP 4K 50p, 420, ~169 Mb/s | 4 ch 24-bit LPCM | sidecar; same log/gamut | 5.600 s |

**Current guess.** `Sony XDCAM (XDROOT)  ·  camera not guessed`. Layout is fair (top-level `XDROOT`). No loaded profile matches (`sony-vg20` and `sony-a7siii` both score 0).

**Ingest test** (same day, `--via staging`, no field-set, no write to the camera). Command forced `--camera sony-a7siii --format "XAVC S" --gamma S-Log3 --gamut S-Gamut3.Cine` as a **stand-in** (gamma/gamut match only; formats on the card are not XAVC S). Server committed. Mag `/srv/magvault/mags/20261127_f84d40f8-169d-46cc-8a89-fd12e4910d14`, project Fx6UsbTest, status `online`, 12 files, 726,317,214 bytes, 2 clips, ingested 14:47 SAST. Classifier: “2 clips, 0 sidecars, 10 other” — `*M01.XML` stem mismatch; `SALVAGE.TMP` ingested as other. Card mtimes and all 12 sha256 sums were unchanged after ingest. `cards` listed the media LUN and ignored `PMHOME`. The ingest copy itself finished in 0.4 s from page cache after the earlier archive read; that is not a measure of cold USB 2 ingest speed.

**Archive.** `/wildraid/camtest/Sony-FX6/ACAE-E116/` (12 files, 726,317,214 bytes) and `…/PMHOME/` (33 files, 755,159 bytes). Total 45 files, 727,072,373 bytes; all sha256 matched. Metadata: `reference-cards/fx6/`.

## Reference card: Sony NEX-VG20

First card in the set that actually came from a VG20. User did not list formats in advance (AVCHD-era body; possibly also MPEG in `MP_ROOT`, and DCIM stills). SD card, exFAT, 119.1 GB, no volume label.

**Layout.** Top level: `PRIVATE/`, `AVF_INFO/`, **`MP_ROOT/`**. No `DCIM`, no `M4ROOT`, no `XDROOT`.

- `PRIVATE/AVCHD/` — one `00000.MTS`
- `MP_ROOT/100PNV01/M2U00001.MPG` — MPEG-2 program stream (SD), not MP4
- `AVF_INFO/AVIN0001.INT` still holds a leftover `MediaProfile` from an earlier use in the **FDR-AX53** (`systemKind="FDR-AX53"`, same `systemId="04C461FFFE1806E4"`, 29 July 2026). Trust the live media, not this leftover.

File dates on the live trees are 2011-05-01 (clock possibly default or unset).

**Model evidence.** Firmware writes **NEX-VG20E** (European / PAL) in the MPEG `MVI_DATA` private stream (`SONY` + `NEX-VG20E` + `HDR-000010G`). The AVCHD `.MTS` has no model string. AVCHD indexes carry the same `HDR-000010G_MC1` as AX53 and FS5. This is the first card that actually contains the text “NEX-VG20”.

**Formats.**

| # | Format | Path | Video | Audio | Other | Duration |
|---|---|---|---|---|---|---|
| 1 | AVCHD | `PRIVATE/AVCHD/BDMV/STREAM/00000.MTS` | H.264 High L4.2, 1920×1080p50, ~26.3 Mb/s | AC-3 stereo 256 kb/s | no XML, no gamma metadata | 6.240 s |
| 2 | MPEG-2 SD | `MP_ROOT/100PNV01/M2U00001.MPG` | MPEG-2 Main, 720×576i25 PAL, ~8.34 Mb/s | AC-3 stereo 256 kb/s | `MVI_DATA` carries `NEX-VG20E`; no XML gamma | 5.280 s |

No MP4 under `MP_ROOT`, no DCIM stills. An AVCHD-only VG20 card (no `MP_ROOT`) has not been seen; it should still lack `M4ROOT` after a format in this body.

**Current guess.** `AVCHD (PRIVATE/AVCHD)  ·  Sony NEX-VG20 [sony-vg20]`. Right this time, but only because `sony-vg20` matches `PRIVATE` + `*.MTS` — the same rule that mis-labels other cards.

**Archive.** `/wildraid/camtest/Sony-NEX-VG20/` — 12 files, 11 directories, 97,563,602 bytes; all sha256 matched. Metadata: `reference-cards/nex-vg20/`.

## Reference card: Sony PXW-FS7

Sony QD-G32 32 GB XQD card, USB 3 (5000 Mbps), exFAT, 29.1 GiB, UUID `5D7B-80A9`, no volume label. User formats: XAVC-I, XAVC-L, MPEG HD 422. All three present; one clip each; nothing missing or extra.

**Layout.** Top level: **`XDROOT/` only** — same family as the FX6, not the FS5’s `PRIVATE/XDROOT`. No `PRIVATE`, no `SONY`, no AVCHD. Clip names `A019C001_26112750.MXF` (professional `A###C###_` form). Also `*M01.XML` and `*R01.BIM` sidecars; empty `Edit/`, `Sub/`, `Take/`, `UserData/`, `Thmbnl/`. No LUT, no `SALVAGE.TMP`. Card was initialised on this body the same day.

**Model evidence.** `systemKind="PXW-FS7 ver.4.100"`, `modelName="PXW-FS7"`, real serial `0032619`, `systemId="0800460202997F6B"`, `masterVersion="XAVC-XD@1.10.00"`, mediaprofile version 2.10. No `CaptureGamma` / `CaptureGamut` (or LUT) on this card; these takes look Rec.709. Same `*M01.XML` stem-mismatch as the FX6.

Told from the **FS5 without XML** (top-level `XDROOT` vs `PRIVATE`). **Not** told from the **FX6 by layout alone**; that needs a filename pattern (`A###C###_*.MXF` vs `PDTV-FX6_*.MXF`) or XML.

**Formats.**

| # | User label | Clip | Video | Audio | Other | Duration |
|---|---|---|---|---|---|---|
| 1 | XAVC-I | `A019C001_26112750.MXF` | H.264 High 4:2:2 Intra L5.2, 3840×2160p50, 10-bit, ~493.5 Mb/s. XML: `AVC_3840_2160_H422IP@L52` | 8 ch 24-bit LPCM | sidecar; no gamma field | 6.160 s |
| 2 | XAVC-L | `A019C002_261127P3.MXF` | H.264 High Long GOP L5.2, 3840×2160p50, 8-bit 420, ~130.2 Mb/s. XML: `AVC_3840_2160_HP@L52` | 4 ch 24-bit LPCM | sidecar; no gamma field | 5.960 s |
| 3 | MPEG HD 422 | `A019C003_261127DT.MXF` | MPEG-2 4:2:2, 1920×1080i50, 50 Mbps CBR. XML: `MPEG2HD50CBR_1920_1080_422P@HL` | 8 ch 24-bit LPCM | sidecar; no gamma field | 5.080 s |

**Current guess.** `Sony XDCAM (XDROOT)  ·  camera not guessed`. Same situation as the FX6.

**Archive.** `/wildraid/camtest/Sony-PXW-FS7/` — 12 files, 11 directories, 519,297,028 bytes; all sha256 matched. Metadata: `reference-cards/pxw-fs7/`.

## Reference card: Sony A7 III (ILCE-7M3)

SD card, exFAT, 119.1 GB, no volume label. User did not list formats in advance (likely XAVC S 4K/HD, maybe AVCHD, maybe DCIM stills). This card held only one clip.

**Layout.** Top level: `PRIVATE/`, `AVF_INFO/` only. **No `DCIM`.** Same XAVC S tree as the AX53: `PRIVATE/M4ROOT/CLIP/C0001.MP4` + `C0001M01.XML` + thumb. `PRIVATE/AVCHD/` exists but is **empty of media** (INDEX and MOVIEOBJ only, written at format). `AVF_INFO` on this card had no leftover `MediaProfile`.

**Model evidence.** Firmware writes **ILCE-7M3** in `MEDIAPRO.XML` `systemKind`, `C0001M01.XML` `modelName`, and thumb EXIF (`Make=SONY`, `Model=ILCE-7M3`). `systemId="747A90FFFE93F793"`. XML serial is the dummy `4294967295` (same as AX53). Empty AVCHD `INDEX.BDM` maker string is **`DSLR-00002G_MC1`** (not `HDR-000010G_MC1`). No “A7 III” spelling on the card.

**Log.** This take is **S-Log3 / S-Gamut3.Cine**: `CaptureGammaEquation="s-log3-cine"`, `CaptureColorPrimaries="s-gamut3-cine"`. ffprobe colour tags were unknown; the log evidence is in the XML (also embedded in the MP4).

**Formats.**

| # | Format | Path | Video | Audio | Other | Duration |
|---|---|---|---|---|---|---|
| 1 | XAVC S HD (S-Log3) | `PRIVATE/M4ROOT/CLIP/C0001.MP4` | H.264 High L4.2, 1920×1080p50, ~52.4 Mb/s overall. XML: `AVC_1920_1080_HP@L42` | LPCM 16-bit BE, 2 ch | `C0001M01.XML` with AcquisitionRecord; thumb EXIF `ILCE-7M3` | 3.840 s |

No XAVC S 4K, no AVCHD footage, no DCIM stills on this card.

**Distinguishability.** Versus **AX53**: folder layout and `C0001.MP4` names are identical, so nested layout and filename patterns cannot split them. Definitive IDs are XML/EXIF (`ILCE-7M3` vs `FDR-AX53`). On this pair, the empty-AVCHD maker string (`DSLR-…` vs `HDR-…`) is also a tell. Versus the **A7S III profile**: that profile wants `DCIM` + `PRIVATE` + `A001_C*.mov` and scores **0** on this card, even though its S-Log3 / S-Gamut3.Cine defaults would have been a fair ingest stand-in. There is still no A7S III reference card; if that body also writes `C0001.MP4` under `PRIVATE/M4ROOT`, XML/EXIF (`ILCE-7SM3` vs `ILCE-7M3`) will be required.

**Current guess.** `Sony XAVC (PRIVATE/M4ROOT)  ·  Sony NEX-VG20 [sony-vg20]`. Layout fair; VG20 guess wrong (no `*.MTS`; it wins on top-level `PRIVATE` alone).

**Archive.** `/wildraid/camtest/Sony-ILCE-7M3/` — 12 files, 14 directories, 97,553,017 bytes; all sha256 matched. Metadata: `reference-cards/ilce-7m3/`.

## Reference card: Panasonic AG-CX350

First non-Sony card. SD card, exFAT, 59.2 GB, volume label **A039**, UUID `6B09-B8C4`. User did not list formats in advance.

**Layout.** Top level: `CONTENTS/`, `DCIM/` (empty), `LASTCLIP.TXT`, `PRIVATE/`. No leftover Sony trees.

- `PRIVATE/PANA_GRP/001RBQAM/` — two QuickTime MOVs: `A039C001_261127_L34B.MOV`, `A039C002_261127_L34B.MOV` (reel `A039` matches the volume label)
- `PRIVATE/AVCHD/` — one `00000.MTS`, plus Panasonic extras `AVCHDTN/` and `PANA_EXT/`
- `CONTENTS/` — P2 layout: `AVCLIP/001GW5GN.MXF`, sidecar `CLIP/001GW5GN.XML`, icon BMP; `LASTCLIP.TXT` points at this P2 clip
- `PRIVATE/PANA_GRP/BACKUP.TMP` is not a clip

**Model evidence.** Next to live media, same rule as Sony. Embedded MOV XML: `Manufacturer=Panasonic`, **`ModelName=AG-CX350`**, serial **`L3TRB0138`**. P2 sidecar (and XML embedded in the MXF): **`ModelName=CX350`** (no `AG-` prefix), same serial. AVCHD does not name the camera and has no Sony `HDR-000010G_MC1`. `INDEX.BDM` / `INDEX.DAT` carry `LSLX64G` (media/card id). Compatible brand on the MOVs: `pana`.

Layout alone identifies a **Panasonic** family, not CX350 versus another Panasonic P2/AVCCAM body.

**Formats.**

| # | Format | Path | Video | Audio | Gamma | Duration |
|---|---|---|---|---|---|---|
| 1 | HEVC 4K Long GOP | `A039C001_261127_L34B.MOV` | HEVC Main 10 L5.1, 3840×2160p50, 10-bit, ~117.4 Mb/s. XML: `H265_420_LongGOP` | 4 ch 24-bit LPCM | `CaptureGamma=HD`, `CaptureGamut=BT.709` | 4.320 s |
| 2 | HEVC 4K Long GOP | `A039C002_261127_L34B.MOV` | same geometry, ~119.2 Mb/s | 4 ch 24-bit LPCM | **`V-Log` / `V-Gamut`** (ffprobe still tagged `bt709`, `color_range=pc`) | 5.280 s |
| 3 | AVCHD | `00000.MTS` | H.264 High L4.0, 1920×1080i50, ~20.5 Mb/s | AC-3 stereo 384 kb/s | no XML, no gamma metadata | 3.840 s |
| 4 | P2 AVC-LongG 422 | `CONTENTS/AVCLIP/001GW5GN.MXF` | H.264 High 4:2:2, 1920×1080i50, 10-bit, ~30.3 Mb/s. XML: `AVC-LongG422` | 4 ch 24-bit LPCM | `CaptureGamma=HD`, `CaptureGamut=BT.709` | 3.840 s |

No MP4 (the 4K takes are `.MOV`), no DCIM stills, no real HLG on this card.

**Current guess.** `AVCHD (PRIVATE/AVCHD)  ·  Sony NEX-VG20 [sony-vg20]`. Wrong. The layout line also missed `PANA_GRP` and `CONTENTS`. A Sony AVCHD matcher must not claim every `PRIVATE` + `*.MTS` card.

**Archive.** `/wildraid/camtest/Panasonic-AG-CX350/` — 18 files, 20 directories, 176,626,709 bytes; all sha256 matched. Metadata: `reference-cards/ag-cx350/`.

## How detection works today, and why it fails

Code read: `src/probe.rs`, `src/discover.rs`, `src/profiles.rs`. No code was changed for these tests.

1. **Score each profile**
   - **+1** if every name in `card_layout` is present as a **top-level** folder on the card.
   - **+2** if any `filename_pattern` glob matches a media filename (for many of these cards, `*.MTS`).
2. **Highest score wins.** A **tie means no guess** (“camera not guessed”).
3. If scoring gives nothing, the recognised **layout’s vendor** is used only when **exactly one** loaded profile has that `vendor`. Two Sony profiles also fail this fallback.

Limits that mattered on every card in this set:

- The schema documents a `model:` key, but it is **not parsed or used**. `CameraProfile` has no model field. Nothing reads `MEDIAPRO.XML`, `NonRealTimeMeta`, Panasonic `ClipMain` / `P2Main`, `INDEX.BDM`, EXIF, or ffprobe tags to identify a camera.
- `card_layout` only sees **top-level** folders, so it cannot express `PRIVATE/M4ROOT` vs `PRIVATE/AVCHD` vs `PRIVATE/XDROOT` vs `PRIVATE/PANA_GRP`, nor top-level `CONTENTS`.
- An empty `M4ROOT` is enough for the layout line to say “Sony XAVC” even when the live media is elsewhere (FS5).
- A loaded `sony-vg20` profile that matches `PRIVATE` + `*.MTS` will claim Sony AX53, FS5, A7 III, and Panasonic CX350 cards, and will also correctly (but for the wrong reason) claim a real VG20.

A temporary config on the client (deleted afterwards) showed the practical result of adding a second same-rule Sony AVCHD profile: **`camera not guessed` (tie)**. Two profiles that both match `PRIVATE` + `*.MTS` cannot be told apart today. Adding a second one makes the guess worse, not better.

The shipped `sony-a7siii` profile (`card_layout: DCIM, PRIVATE`, `filename_pattern: A001_C*.mov`, default ProRes RAW, default S-Log3 / S-Gamut3.Cine) scored **0** on every card in this set except when it was forced as an FX6 ingest stand-in.

## Options A–D and the recommendation

### A — Grouped family profile (no code change)

One profile per family, matched only on today’s layout + filename rules. A single `sony-avchd` / `PRIVATE` + `*.MTS` group is **too narrow** for the AX3 (it also shoots XAVC S) and **too wide** for the real VG20 (MPEG-2 in `MP_ROOT`) and for Panasonic (same `*.MTS` path). It would also not distinguish A7 III from AX53. A grouped profile can still be the right shape for *ingest defaults inside one family*, but it is not a substitute for model metadata.

### B — Per-model profiles, plus a code change for model matching

Keep one `.cam` per body and teach the detector to use `model:` against the metadata listed above, with a score above layout and filename so ties break. This can name a body when the live-media XML or EXIF is present. It will be wrong if leftover `systemKind` is trusted, and it cannot name an AVCHD-only take. A hard guess still needs an operator confirm.

### C — Family (or body-shaped) profile plus an optional model hint (preferred)

Detect a card as a family or body-shaped profile from layout + filename (with nested paths once option D exists). Separately, record an optional **model hint** on the mag from live-media metadata, which the operator can confirm or override (including “we call it AX3” or “A7 III”). That hint should be distinct from `camera_profile_id`. Today `magcard.txt` already has a `camera_model` line, but it is just a copy of the profile’s display name, and the database has no matching column.

This fits reused cards better than a silent hard guess, and it is almost required for A7 III vs AX53 (shared `PRIVATE/M4ROOT` + `C????.MP4`).

### D — Nested-path `card_layout` (code change, complementary)

Allow `card_layout` to mean paths such as `PRIVATE/AVCHD`, `PRIVATE/M4ROOT`, `PRIVATE/XDROOT`, or `PRIVATE/PANA_GRP`, not only top-level names, and label the card by trees that actually hold media. That splits FS5 from FX6/FS7, VG20 from AX53 when `MP_ROOT` is present, and Panasonic from Sony, and it stops an empty `M4ROOT` stealing the layout line. It does **not** split FS7 from FX6 (both top-level `XDROOT`) or A7 III from AX53 (same `C????.MP4` tree). Those pairs still need filename patterns and/or XML (and for A7 III / AX53, XML, EXIF, or the `DSLR-` vs `HDR-` maker string).

### Recommendation

Do **not** adopt a single AVCHD-only `sony-avchd` profile, and do **not** leave the current `sony-vg20` deployed as a generic `PRIVATE` + `*.MTS` matcher. Prefer **option C**, with **option D** as a separate improvement, and per-body (or tight family) profiles:

- **FDR-AX3 / FDR-AX53** — XAVC S + AVCHD, `C????.MP4` + `*.MTS`, `PRIVATE/M4ROOT`, Rec.709 defaults on the reference takes
- **NEX-VG20** — AVCHD + MPEG-2 SD, `*.MTS` + `M2U?????.MPG` under `MP_ROOT/100PNV01`; no XAVC S
- **ILCE-7M3** — XAVC S, `C????.MP4` under `PRIVATE/M4ROOT`; S-Log3 / S-Gamut3.Cine is available and was used on this card. Do **not** share this profile with the AX53 despite the identical folder and filename layout
- **PXW-FS5** — XAVC QFHD, XAVC HD / Intra, AVCHD; `Clip????.MXF` under `PRIVATE/XDROOT`
- **PXW-FS7** — XAVC Intra, XAVC Long, MPEG HD 422; top-level `XDROOT`; `A###C###_*.MXF`. Do not share one “top-level XDROOT / `*.MXF`” profile with the FX6
- **ILME-FX6** — top-level `XDROOT`, `PDTV-FX6_*.MXF`, XAVC Intra + XAVC Long, default S-Log3 / S-Gamut3.Cine. Do not rely on `sony-a7siii` to detect it
- **AG-CX350** — HEVC 4K MOV under `PRIVATE/PANA_GRP`, P2 AVC-LongG MXF under `CONTENTS`, AVCHD; V-Log / V-Gamut is available. Prefix-match should accept both `AG-CX350` and `CX350`

**AVCHD default:** Rec.709, as a default the operator can override. No AVCHD clip in this set carried gamma metadata, so a log AVCHD take (if a body allows it) would not be detectable automatically.

When going ahead: keep chosen `.cam` files in `share/cameras`, but do not leave the current `sony-vg20` `PRIVATE` + `*.MTS` matcher in live config dirs (or add it to `STARTER_CAMERAS`) until detection can use model metadata and/or nested layout. `sony-vg20.cam` is already in git; this commit does not change it. No existing mag needs a `sony-vg20` id migration today. The two early test mags tagged Sony A7S III / ProRes RAW / S-Log3 do not match their footage; leaving or retagging them is a separate cleanup.

## Code changes needed

None of this has been written yet.

1. **Metadata model and gamma reader**, used only on metadata next to live media (ignore leftover `AVF_INFO` / stale `MEDIAPRO` that is not beside a live tree):
   - Sony `NonRealTimeMeta` `Device modelName` and `AcquisitionRecord` (`CaptureGammaEquation`, `CaptureColorPrimaries`)
   - Sony `MEDIAPRO.XML` `systemKind` / `systemId` (prefix-match, strip ` ver.x`)
   - Sony MPEG `MVI_DATA` (`NEX-VG20E`)
   - EXIF Make/Model on thumbs or DCIM stills
   - Panasonic embedded MOV XML (`com.panasonic.Semi-Pro.metadata.xml` / `ClipMain`) and P2 `P2Main` (`Device ModelName`, `CaptureGamma`, `CaptureGamut`)
2. **`model:` matching in profiles**, including prefix and alias lists (for example `AG-CX350` and `CX350`; `PXW-FS5` matching `PXW-FS5 ver.4.020`).
3. **Nested `card_layout`** paths, and layout labels that follow trees that actually hold media.
4. **`*M01.XML` sidecar pairing.** Today a sidecar is only paired when the stem matches, so `PDTV-FX6_3985.MXF` and `PDTV-FX6_3985M01.XML` (and the same shape on the FS7, and `C0001.MP4` vs `C0001M01.XML`) are not paired. The FX6 ingest reported 0 sidecars for that reason.
5. **Ignore junk that is not media**, at least Sony `SALVAGE.TMP` and Panasonic `BACKUP.TMP` (the FX6 ingest stored a 256 MiB `SALVAGE.TMP` as “other”).

## Profiles in git

- **Canonical location:** `share/cameras/*.cam` in the magvault repository. Encoders: `share/encoders/`.
- **Shipping:** `src/init.rs` `STARTER_CAMERAS` embeds those files and `magvault init` copies them into the config `cameras/` directory, never overwriting. **A file in `share/cameras` is not shipped to new installs until it is also listed in `STARTER_CAMERAS`.**
- **Live copies** (what `cards` actually loads): server `/srv/magvault/cameras` (from `path.conf`), client `~/mvclient/config/cameras`.
- **Today in git** (this branch, after `git pull --ff-only` on 27 November 2026): `share/cameras/sony-a7siii.cam`, plus (commits `7c9df05` and `bd84d7f`, which landed on `main` while these reference cards were being taken) `share/cameras/sony-vg20.cam`, `share/cameras/sony-fdr-ax33.cam`, and `share/cameras/README.md`. `STARTER_CAMERAS` still lists only the A7S III. The README names `sony-a7siii.cam` and `sony-vg20.cam` as the starter set, which does not yet match the code.
- The committed `sony-vg20.cam` still matches `PRIVATE` + `*.MTS` only. That is the rule this document shows will also claim AX53, FS5, A7 III, and Panasonic CX350 cards. It is not changed in this commit.
- `sony-fdr-ax33.cam` is a different model (`id: sony-fdr-ax33`; notes: “Not FDR-AX3”). It was not tested against the FDR-AX53 reference card in this set (`card_layout: PRIVATE, DCIM`; the AX53 reference card has no `DCIM`).

## Reference archives and metadata

Full card copies live on the test server under the `wildraid/camtest` dataset, owner `root:magvault`, directories `2775`, files `664`, readable by the `magvault` user. Copies after the first FX6 archive were rsynced from the client directly to the server over the LAN, with sha256 computed on each host and compared as lists only. Later card copies should keep that path and not route media through a third machine.

| Archive | Files | Bytes | Notes |
|---|---:|---:|---|
| `/wildraid/camtest/Sony-FDR-AX3/` | 18 | 165,116,307 | Clean AX3/AX53 reference |
| `/wildraid/camtest/Sony-PXW-FS5/` | 24 | 231,990,507 | |
| `/wildraid/camtest/Sony-FX6/` | 45 | 727,072,373 | `ACAE-E116/` media + `PMHOME/` licences |
| `/wildraid/camtest/Sony-NEX-VG20/` | 12 | 97,563,602 | |
| `/wildraid/camtest/Sony-PXW-FS7/` | 12 | 519,297,028 | XQD |
| `/wildraid/camtest/Sony-ILCE-7M3/` | 12 | 97,553,017 | |
| `/wildraid/camtest/Panasonic-AG-CX350/` | 18 | 176,626,709 | |

Small metadata only (XML, indexes, listings, ffprobe text — no media) was also kept under `reference-cards/` (`fdr-ax3`, `pxw-fs5`, `fx6`, `nex-vg20`, `pxw-fs7`, `ilce-7m3`, `ag-cx350`) for the working notes.

The FX6 ingest mag is `/srv/magvault/mags/20261127_f84d40f8-169d-46cc-8a89-fd12e4910d14` (project Fx6UsbTest, tagged with the A7S III stand-in profile).

## Open questions

1. Optional later check: what is printed on the FDR-AX3 / AX53 camera badge? That does not change the metadata key. Do not create a second “AX3” profile that tries to detect a string the camera never writes.
2. For the AX3 body, is a family profile with several `file_formats` enough, or is the exact model required in the catalogue? Ingest defaults are not identical across XAVC S 4K, XAVC S HD, and AVCHD, but they can live on one profile.
3. How should a reused card be treated? One mag with a primary camera, a “mixed / reused card” flag, or something else? Conflicting leftover `systemKind`s should not become a silent hard guess.
4. The two early test mags labelled Sony A7S III / ProRes RAW / S-Log3 do not match their footage. Leave them as test artefacts, or retag later?
5. Should the layout label always follow the tree that actually holds media? (Option D.)
6. Would a VG20 card that only holds AVCHD (no `MP_ROOT`) still be distinguishable from a clean AX53? Almost certainly yes, because a VG20 format should not create `M4ROOT` — but that card has not been seen. A VG20 card formatted only in that body would also show whether `AVF_INFO` is cleared.
7. When should a chosen profile be added to `STARTER_CAMERAS`? A file in `share/cameras` alone will not appear on a fresh `magvault init`.
8. There is still no **A7S III** reference card. The current `sony-a7siii` detection rules (`DCIM` + `A001_C*.mov`) did not match the A7 III XAVC S card.
9. Optional later cards: A7 III with 4K, AVCHD, or DCIM stills; further non-Sony bodies.

Nothing in the recommended profile set has been deployed. This document records the evidence so a decision can be made, then implemented.
