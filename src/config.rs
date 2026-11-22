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
    /// Must be on the same filesystem as `mags` (rename must be atomic). Never /tmp.
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
        None => mags.parent().map(|p| p.join("incoming")).unwrap_or_else(|| dir.join("incoming")),
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
