//! Config directory, `path.conf` and `magvault.conf`.
//!
//! Resolution order (tech spec §5.3): `--config <dir>` → `MAGVAULT_CONFIG` → pointer file
//! (`~/.config/magvault/config-dir`, then `/etc/magvault/config-dir`) containing one
//! absolute path. Relative paths inside `path.conf` resolve against the config directory.

use crate::error::{Context, Error, Result};
use crate::textfile;
use std::collections::BTreeMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub const REQUIRED_ROOTS: &[&str] = &[
    "mags",
    "proxies",
    "bins",
    "db",
    "logs",
    "cameras",
    "encoders",
    "custom_encode",
    "telemetry_live",
    "telemetry_disk",
];

#[derive(Debug, Clone)]
pub struct Paths {
    pub mags: PathBuf,
    /// Ingest copies land here first and are renamed into `mags/` once verified.
    /// Default `<mags>/.incoming` (PLAN.md "Incoming staging"): on the same filesystem as
    /// `mags` so the rename is atomic. Never /tmp.
    pub incoming: PathBuf,
    pub proxies: PathBuf,
    pub bins: PathBuf,
    pub db_dir: PathBuf,
    pub logs: PathBuf,
    pub cameras: PathBuf,
    pub encoders: PathBuf,
    pub custom_encode: PathBuf,
    pub telemetry_live: PathBuf,
    pub telemetry_disk: PathBuf,
}

impl Paths {
    pub fn db_file(&self) -> PathBuf {
        self.db_dir.join("magvault.sqlite")
    }

    /// Roots that hold data and must never contain an ingest source.
    pub fn data_roots(&self) -> Vec<&Path> {
        vec![
            &self.mags,
            &self.incoming,
            &self.proxies,
            &self.bins,
            &self.db_dir,
            &self.logs,
            &self.custom_encode,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    #[allow(dead_code)] // used by later phases (Settings)
    pub dir: PathBuf,
    pub paths: Paths,
    /// `magvault.conf` key/values.
    pub settings: BTreeMap<String, String>,
}

impl Config {
    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str()).filter(|s| !s.is_empty())
    }
}

pub fn home_pointer() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("magvault").join("config-dir"))
}

pub const SYSTEM_POINTER: &str = "/etc/magvault/config-dir";

pub fn resolve_dir(arg: Option<&str>) -> Result<PathBuf> {
    if let Some(a) = arg {
        return Ok(PathBuf::from(a));
    }
    if let Some(v) = std::env::var_os("MAGVAULT_CONFIG") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    let mut candidates = Vec::new();
    if let Some(p) = home_pointer() {
        candidates.push(p);
    }
    candidates.push(PathBuf::from(SYSTEM_POINTER));
    for p in candidates {
        if let Ok(text) = std::fs::read_to_string(&p) {
            let line = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'));
            if let Some(l) = line {
                let path = PathBuf::from(l);
                if !path.is_absolute() {
                    crate::bail!("pointer file {} must contain an absolute path, found {l:?}", p.display());
                }
                return Ok(path);
            }
        }
    }
    Err(Error(
        "no config directory: pass --config <dir>, set MAGVAULT_CONFIG, or write the path into \
         ~/.config/magvault/config-dir (see `magvault init`)"
            .into(),
    ))
}

fn resolve(dir: &Path, value: &str) -> PathBuf {
    let p = if let Some(rest) = value.strip_prefix("~/") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(rest)).unwrap_or_else(|| PathBuf::from(value))
    } else {
        PathBuf::from(value)
    };
    if p.is_absolute() {
        p
    } else {
        dir.join(p)
    }
}

pub fn load(dir: &Path) -> Result<Config> {
    let dir = if dir.is_absolute() { dir.to_path_buf() } else { std::env::current_dir()?.join(dir) };
    let path_conf = dir.join("path.conf");
    let text = std::fs::read_to_string(&path_conf).ctx(format!("cannot read {}", path_conf.display()))?;
    let kv: BTreeMap<String, String> = textfile::parse_config(&text).into_iter().collect();

    let missing: Vec<&str> = REQUIRED_ROOTS.iter().copied().filter(|k| kv.get(*k).is_none_or(|v| v.is_empty())).collect();
    if !missing.is_empty() {
        crate::bail!("{} is missing required keys: {}", path_conf.display(), missing.join(", "));
    }
    let get = |k: &str| resolve(&dir, &kv[k]);
    let mags = get("mags");
    let incoming = match kv.get("incoming").filter(|v| !v.is_empty()) {
        Some(v) => resolve(&dir, v),
        None => mags.join(".incoming"),
    };
    let paths = Paths {
        mags,
        incoming,
        proxies: get("proxies"),
        bins: get("bins"),
        db_dir: get("db"),
        logs: get("logs"),
        cameras: get("cameras"),
        encoders: get("encoders"),
        custom_encode: get("custom_encode"),
        telemetry_live: get("telemetry_live"),
        telemetry_disk: get("telemetry_disk"),
    };

    let mut settings = BTreeMap::new();
    let mv_conf = dir.join("magvault.conf");
    if mv_conf.exists() {
        let text = std::fs::read_to_string(&mv_conf).ctx(format!("cannot read {}", mv_conf.display()))?;
        settings.extend(textfile::parse_config(&text));
    }
    crate::probe::configure_ffprobe(settings.get("ffprobe").map(String::as_str));
    Ok(Config { dir, paths, settings })
}

/// Create the directory roots that Phase 0 writes to (and the others, so a fresh install
/// is complete). Telemetry entries are files; only their parent folders are created.
pub fn ensure_roots(p: &Paths) -> Result<()> {
    for d in [
        &p.mags,
        &p.incoming,
        &p.proxies,
        &p.proxies.join("db"),
        &p.proxies.join("hq"),
        &p.bins,
        &p.db_dir,
        &p.logs,
        &p.cameras,
        &p.encoders,
        &p.custom_encode,
    ] {
        std::fs::create_dir_all(d).ctx(format!("cannot create {}", d.display()))?;
    }
    for f in [&p.telemetry_live, &p.telemetry_disk] {
        if let Some(parent) = f.parent() {
            std::fs::create_dir_all(parent).ctx(format!("cannot create {}", parent.display()))?;
        }
    }
    Ok(())
}

/// Ingest staging rules (PLAN.md Implementation defaults): never /tmp, and on the same
/// filesystem as `mags` so the final rename is atomic.
pub fn check_incoming(p: &Paths) -> Result<()> {
    let inc = std::fs::canonicalize(&p.incoming).ctx(format!("incoming root {}", p.incoming.display()))?;
    for bad in ["/tmp", "/var/tmp", "/dev/shm"] {
        if inc.starts_with(bad) {
            crate::bail!("incoming root {} is under {bad}; staging must not live in a temporary filesystem", inc.display());
        }
    }
    let a = std::fs::metadata(&p.mags).ctx(format!("mags root {}", p.mags.display()))?;
    let b = std::fs::metadata(&inc)?;
    if a.dev() != b.dev() {
        crate::bail!(
            "incoming root {} is not on the same filesystem as mags root {}; set `incoming:` in path.conf to a folder on the mags dataset",
            inc.display(),
            p.mags.display()
        );
    }
    Ok(())
}

// ---- client mode (Phase 1) -------------------------------------------------------------

/// Which kind of installation a config directory describes (`mode: client` in path.conf).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Server,
    Client,
}

pub fn read_mode(dir: &Path) -> Result<Mode> {
    let path_conf = dir.join("path.conf");
    let text =
        std::fs::read_to_string(&path_conf).ctx(format!("cannot read {}", path_conf.display()))?;
    let mode = textfile::parse_config(&text)
        .into_iter()
        .find(|(k, _)| k == "mode")
        .map(|(_, v)| v);
    match mode.as_deref() {
        None | Some("") | Some("server") => Ok(Mode::Server),
        Some("client") => Ok(Mode::Client),
        Some(other) => Err(Error(format!(
            "{}: mode {other:?} must be server or client",
            path_conf.display()
        ))),
    }
}

/// How the client reaches the server.
#[derive(Debug, Clone)]
pub enum ServerLink {
    /// `ssh <user>@<host>`; the server forces `magvault gate` for this key.
    Ssh {
        user: String,
        host: String,
        key: Option<PathBuf>,
    },
    /// Test transport: run `magvault gate` locally and rsync straight into the spool.
    /// Only for tests on one machine; documented in docs/phase1.md.
    Local {
        spool: PathBuf,
        results: PathBuf,
        label: String,
    },
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub dir: PathBuf,
    /// Local staging (card → staging → server). Mags here are pending until the server
    /// confirms; they are removed (autoclean) once two backups exist (server + field set).
    /// Never /tmp.
    pub staging: PathBuf,
    pub logs: PathBuf,
    pub cameras: PathBuf,
    /// None = no server configured (offline only).
    pub server: Option<ServerLink>,
    pub usb_helper: PathBuf,
    /// `debug: yes` in the client's magvault.conf.
    pub debug: bool,
    /// How long ingest and sync wait for a server committer that is down
    /// (`committer_wait` seconds in magvault.conf, default 120).
    pub committer_wait: u64,
    /// `staging_autoclean: yes|no` (default yes): remove a staged mag once two backups
    /// exist (PLAN.md "Client staging cache"). Never when staging is the only copy.
    pub staging_autoclean: bool,
    /// `date_format` / `time_format` (PLAN.md "Time display").
    pub display: crate::time::DisplayStyle,
}

pub const DEFAULT_COMMITTER_WAIT: u64 = 120;

/// `debug: yes|no` (default no).
pub fn debug_setting(settings: &BTreeMap<String, String>, file: &Path) -> Result<bool> {
    match settings.get("debug").map(|s| s.trim()) {
        None | Some("") | Some("no") => Ok(false),
        Some("yes") => Ok(true),
        Some(other) => crate::bail!("{}: debug must be yes or no, not {other:?}", file.display()),
    }
}

pub const DEFAULT_HELPER: &str = "/usr/lib/magvault/magvault-helper";

pub fn load_client(dir: &Path) -> Result<ClientConfig> {
    let dir = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()?.join(dir)
    };
    let path_conf = dir.join("path.conf");
    let text =
        std::fs::read_to_string(&path_conf).ctx(format!("cannot read {}", path_conf.display()))?;
    let kv: BTreeMap<String, String> = textfile::parse_config(&text).into_iter().collect();
    let get = |k: &str| kv.get(k).filter(|v| !v.is_empty()).cloned();
    let missing: Vec<&str> = ["staging", "logs", "cameras"]
        .into_iter()
        .filter(|k| get(k).is_none())
        .collect();
    if !missing.is_empty() {
        crate::bail!(
            "{} is missing required keys: {}",
            path_conf.display(),
            missing.join(", ")
        );
    }
    let server = match get("server_transport").as_deref() {
        Some("local") => {
            let need = |k: &str| {
                get(k).ok_or_else(|| {
                    Error(format!(
                        "{}: server_transport local needs {k}",
                        path_conf.display()
                    ))
                })
            };
            Some(ServerLink::Local {
                spool: resolve(&dir, &need("local_spool")?),
                results: resolve(&dir, &need("local_results")?),
                label: need("local_label")?,
            })
        }
        None | Some("ssh") => get("server_host").map(|host| ServerLink::Ssh {
            user: get("server_user").unwrap_or_else(|| "magvaultclient".into()),
            host,
            key: get("server_key").map(|k| resolve(&dir, &k)),
        }),
        Some(other) => crate::bail!(
            "{}: server_transport {other:?} must be ssh",
            path_conf.display()
        ),
    };
    let mv_conf = dir.join("magvault.conf");
    let mut settings = BTreeMap::new();
    if mv_conf.exists() {
        let text = std::fs::read_to_string(&mv_conf).ctx(format!("cannot read {}", mv_conf.display()))?;
        settings.extend(textfile::parse_config(&text));
    }
    let debug = debug_setting(&settings, &mv_conf)?;
    crate::probe::configure_ffprobe(settings.get("ffprobe").map(String::as_str));
    let committer_wait = match settings.get("committer_wait").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => DEFAULT_COMMITTER_WAIT,
        Some(v) => v.parse().map_err(|_| {
            Error(format!("{}: committer_wait must be a number of seconds, not {v:?}", mv_conf.display()))
        })?,
    };
    let staging_autoclean = match settings.get("staging_autoclean").map(|s| s.trim()) {
        None | Some("") | Some("yes") => true,
        Some("no") => false,
        Some(other) => crate::bail!("{}: staging_autoclean must be yes or no, not {other:?}", mv_conf.display()),
    };
    let display = crate::time::DisplayStyle::parse(
        settings.get("date_format").map(String::as_str),
        settings.get("time_format").map(String::as_str),
    )
    .map_err(|e| Error(format!("{}: {e}", mv_conf.display())))?;
    let cfg = ClientConfig {
        debug,
        committer_wait,
        staging_autoclean,
        display,
        staging: resolve(&dir, &get("staging").unwrap()),
        logs: resolve(&dir, &get("logs").unwrap()),
        cameras: resolve(&dir, &get("cameras").unwrap()),
        server,
        usb_helper: get("usb_helper")
            .map(|h| resolve(&dir, &h))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_HELPER)),
        dir,
    };
    Ok(cfg)
}

/// Staging rule (PLAN.md Implementation defaults): never a temporary filesystem.
pub fn check_not_tmp(what: &str, p: &Path) -> Result<()> {
    std::fs::create_dir_all(p).ctx(format!("cannot create {}", p.display()))?;
    let c = std::fs::canonicalize(p).ctx(format!("{what} {}", p.display()))?;
    for bad in ["/tmp", "/var/tmp", "/dev/shm"] {
        if c.starts_with(bad) {
            crate::bail!(
                "{what} {} is under {bad}; staging must not live in a temporary filesystem",
                c.display()
            );
        }
    }
    Ok(())
}
