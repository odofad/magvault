# Settings module mockup (ratatui)

> **Mockup only.** Design reference for the Settings module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values (including every path) are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Settings module"). Settings is admin-only on the server. The client shows only client-relevant settings. No other non-path settings are defined today; they are added when a module needs them.

## Server settings (admin only), 150x42

```text
 MAGVAULT  ·  Settings  ·  server (admin)                                                                       running as magvault · unsaved changes
╔═ 1 path.conf roots ══════════════════════════════════════════════════════════════════════════════════╗┌─ 2 Selected setting ───────────────────────┐
║                                                                                                      ║│                                            │
║ setting           value                                                                              ║│ file      path.conf                        │
║   mags             /srv/magvault/mags                                      ✓ exists                  ║│ key       logs                             │
║   proxies          /srv/magvault/proxies                                   ✓ exists                  ║│ on disk   /srv/magvault/logs               │
║   bins             /srv/bins                                               ✓ exists                  ║│ edited    /var/log/magvault                │
║   db               /srv/magvault/db                                        ✓ exists                  ║│                                            │
║ ▶ logs             /srv/magvault/logs▏                                     ✖ not writable            ║│ Validation                                 │
║   cameras          /srv/magvault/cameras                                   ✓ exists                  ║│ ✖ path exists but is not                   │
║   encoders         /srv/magvault/encoders                                  ✓ exists                  ║│   writable by magvault                     │
║   custom_encode    /srv/magvault/custom_encode                             + will be created         ║│                                            │
║   telemetry_live   /dev/shm/magvault                                       ✓ exists                  ║│ Save is blocked while any root             │
║   telemetry_disk   /srv/magvault/telemetry                                 ✓ exists                  ║│ has an error.                              │
║                                                                                                      ║│                                            │
║ ✓ exists   + can be created   ✖ error                                                                ║│ Other roots                                │
║                                                                                                      ║│   9 ✓ exist or can be created              │
║ magvault.conf (non-path settings)                                                                    ║│                                            │
║   none defined                                                                                       ║│ Writes                                     │
║   Future settings appear here when a module needs them.                                              ║│   path.conf                                │
║                                                                                                      ║│   magvault.conf (nothing yet)              │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║ Ctrl+S writes the files · r reload from disk                                                         ║│                                            │
║ Roots are read from path.conf.                                                                       ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│ Enter edit · Esc dashboard (asks if unsave │
╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝└────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · Ctrl+S save · r reload from disk · Esc dashboard
```

## Server settings (admin only), 120x36 (minimum)

```text
 MAGVAULT  ·  Settings  ·  server (admin)                                         running as magvault · unsaved changes
╔═ 1 path.conf roots ══════════════════════════════════════════════════════════════╗┌─ 2 Selected setting ─────────────┐
║                                                                                  ║│                                  │
║ setting           value                                                          ║│ file      path.conf              │
║   mags             /srv/magvault/mags                  ✓ exists                  ║│ key       logs                   │
║   proxies          /srv/magvault/proxies               ✓ exists                  ║│ on disk   /srv/magvault/logs     │
║   bins             /srv/bins                           ✓ exists                  ║│ edited    /var/log/magvault      │
║   db               /srv/magvault/db                    ✓ exists                  ║│                                  │
║ ▶ logs             /srv/magvault/logs▏                 ✖ not writable            ║│ Validation                       │
║   cameras          /srv/magvault/cameras               ✓ exists                  ║│ ✖ path exists but is not         │
║   encoders         /srv/magvault/encoders              ✓ exists                  ║│   writable by magvault           │
║   custom_encode    /srv/magvault/custom_encode         + will be created         ║│                                  │
║   telemetry_live   /dev/shm/magvault                   ✓ exists                  ║│ Save is blocked while any root   │
║   telemetry_disk   /srv/magvault/telemetry             ✓ exists                  ║│ has an error.                    │
║                                                                                  ║│                                  │
║ ✓ exists   + can be created   ✖ error                                            ║│ Other roots                      │
║                                                                                  ║│   9 ✓ exist or can be created    │
║ magvault.conf (non-path settings)                                                ║│                                  │
║   none defined                                                                   ║│ Writes                           │
║   Future settings appear here when a module needs them.                          ║│   path.conf                      │
║                                                                                  ║│   magvault.conf (nothing yet)    │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║ Ctrl+S writes the files · r reload from disk                                     ║│                                  │
║ Roots are read from path.conf.                                                   ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│ Enter edit · Esc dashboard (asks │
╚══════════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · Ctrl+S save · r reload from disk · Esc dashboard
```

## Client settings, 150x42

```text
 MAGVAULT  ·  Settings  ·  CLIENT VIEW                                                                    client · no server paths, no admin settings
╔═ 1 Client settings ══════════════════════════════════════════════════════════════════════════════════╗┌─ 2 Selected setting ───────────────────────┐
║                                                                                                      ║│                                            │
║ setting       value                                                                                  ║│ file      client path.conf                 │
║ ▶ proxy cache  client-proxies▏                                                                       ║│ key       proxy cache                      │
║                ✓ exists                                                                              ║│ value     client-proxies                   │
║   server host  server                                                                                ║│                                            │
║                limited edit                                                                          ║│ Validation                                 │
║   server user  magvaultclient                                                                        ║│ ✓ folder exists                            │
║                limited edit                                                                          ║│                                            │
║                                                                                                      ║│ Holds the local 320p proxy                 │
║ Not shown on the client                                                                              ║│ cache.                                     │
║   server paths, admin settings                                                                       ║│                                            │
║                                                                                                      ║│                                            │
║ Proxy cache path comes from the client path.conf.                                                    ║│ Server host / user                         │
║                                                                                                      ║│   limited edit                             │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║ Ctrl+S saves · r reload from disk                                                                    ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│                                            │
║                                                                                                      ║│ Enter edit · Esc dashboard                 │
╚══════════════════════════════════════════════════════════════════════════════════════════════════════╝└────────────────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · Ctrl+S save · r reload from disk · Esc dashboard
```

## Client settings, 120x36 (minimum)

```text
 MAGVAULT  ·  Settings  ·  CLIENT VIEW                                      client · no server paths, no admin settings
╔═ 1 Client settings ══════════════════════════════════════════════════════════════╗┌─ 2 Selected setting ─────────────┐
║                                                                                  ║│                                  │
║ setting       value                                                              ║│ file      client path.conf       │
║ ▶ proxy cache  client-proxies▏                                                   ║│ key       proxy cache            │
║                ✓ exists                                                          ║│ value     client-proxies         │
║   server host  server                                                            ║│                                  │
║                limited edit                                                      ║│ Validation                       │
║   server user  magvaultclient                                                    ║│ ✓ folder exists                  │
║                limited edit                                                      ║│                                  │
║                                                                                  ║│ Holds the local 320p proxy       │
║ Not shown on the client                                                          ║│ cache.                           │
║   server paths, admin settings                                                   ║│                                  │
║                                                                                  ║│                                  │
║ Proxy cache path comes from the client path.conf.                                ║│ Server host / user               │
║                                                                                  ║│   limited edit                   │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║ Ctrl+S saves · r reload from disk                                                ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│                                  │
║                                                                                  ║│ Enter edit · Esc dashboard       │
╚══════════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────┘
 Tab pane · ↑↓ move · Enter edit · Ctrl+S save · r reload from disk · Esc dashboard
```

## Layout

A single list of `setting = value` rows with inline edit on the left, and the selected setting with its validation on the right. The server list is the `path.conf` roots, followed by a `magvault.conf` section. The client header says **CLIENT VIEW** so it cannot be mistaken for the server screen.

## Server (admin only, running as `magvault`)

- Visible only when running as the `magvault` user. Normal operators and clients never see it.
- `path.conf` roots listed with their current values as editable text fields: mags, proxies, bins, db, logs, cameras, encoders, custom_encode, telemetry_live, telemetry_disk. (`PLAN.md` says "etc."; the list follows the roots named there.)
- Each row shows a validation result: path exists, can be created, or error. The selected setting shows the same check in more detail.
- **Save writes the files** (`path.conf`, and `magvault.conf` once it has content). Save is blocked while any root has an error.
- `magvault.conf` section: **none defined** today. Future non-path settings appear here when a module needs them.
- The sample screens show one root being edited with an error (`logs`, not writable) and one root that will be created (`custom_encode`).

## Client

- Local proxy cache path (editable; lives in the client `path.conf`, for example `client-proxies/`).
- Server SSH host and user (limited edit).
- No server paths. No admin settings. The screen says so.

## Keys (proposals)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Up` / `Down` | Move between settings |
| `Enter` | Edit the selected value |
| `Ctrl+S` | Save (writes the files) |
| `r` | Reload from disk |
| `Esc` | Return to the dashboard. If there are unsaved changes, prompt "Discard changes?" (same as the Profile Editor) |

All sample values are placeholders.
