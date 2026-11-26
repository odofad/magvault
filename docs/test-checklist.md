# Magvault test checklist

Hardware for early testing:
- Server: HP DL385 G8 (ZFS pools, GPU encoder/decoder)
- Client: Gen8 Intel i5

## Both machines

- [ ] Ubuntu 26.04
- [ ] Packages: `build-essential pkg-config libssl-dev libsqlite3-dev sqlite3 ffmpeg rsync zfsutils-linux openssh-server git curl`
- [ ] Rust via rustup
- [ ] Users created:
  - Server: `magvault` (disabled password)
  - Client: `magvaultclient` (disabled password)

## Server (DL385 G8)

### ZFS datasets
- [ ] `tank/mags` → `/magvault/mags`
- [ ] `tank/proxies` → `/magvault/proxies`
- [ ] `tank/bins` → `/magvault/bins`
- [ ] `tank/db` → `/magvault/db`
- [ ] `tank/logs` → `/magvault/logs`
- [ ] `tank/cameras` → `/magvault/cameras`
- [ ] `tank/encoders` → `/magvault/encoders`
- [ ] `tank/custom_encode` → `/magvault/custom_encode`
- [ ] `tank/staging` → `/magvault/staging`
- [ ] Ownership: `magvault:magvault` on `/magvault`

### Config
- [ ] Config directory (e.g. `/home/magvault/config/` or `/etc/magvault/`)
- [ ] `path.conf` with all roots above plus:
  - `telemetry_live=/dev/shm/magvault-telemetry.json`
  - `telemetry_disk=/magvault/logs/telemetry-latest.json`

### SSH
- [ ] sshd running
- [ ] `magvaultclient` user exists (for later client tests)

### GPU (later)
- [ ] ffmpeg sees the encoder (nvenc / VAAPI / AMF)

## Client (Gen8 i5)

- [ ] `/home/magvaultclient/staging`
- [ ] `/home/magvaultclient/client-proxies`
- [ ] Client `path.conf` with local paths only
- [ ] Can SSH to server as `magvaultclient` (temporary key or password until Key module exists)
- [ ] At least one USB drive available for later Prepare tests

## Phase 0 test (server, as magvault)

- [ ] Config loads and paths are correct
- [ ] SQLite schema is created
- [ ] Ingest a test folder:
  - [ ] Mag folder created as `<YYYYMMDD>_<full-uuid>`
  - [ ] SHA-256 hashes present in magcard and database
  - [ ] ASC MHL contains C4 hashes only: ASC MHL (specification 1.0; files declare hashlist version 2.0, `urn:ASC:MHL:v2.0`); C4 only.
  - [ ] Database has mag and clip rows
  - [ ] 320p job record exists
- [ ] Conflict test: change metadata, confirm old magcard is backed up to `.magvault/` and new card matches database
- [ ] Re-verify a copied file against the stored hash

## Later phases (not required for Phase 0)

- [ ] Prepare formats a USB as ext4
- [ ] Client can pull 320p proxies
- [ ] Zmag create / copy / eject
- [ ] Logging writes back to magcard
- [ ] Export card and cold-pull request
- [ ] Key module generate + USB export
