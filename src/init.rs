//! `magvault init`: create a config directory (path.conf, magvault.conf, starter
//! profiles), the data roots and the database. Existing files are never overwritten.

use crate::config;
use crate::db::{self, Db};
use crate::error::{Context, Result};
use crate::textfile::atomic_write;
use std::path::{Path, PathBuf};

const STARTER_CAMERAS: &[(&str, &str)] = &[("sony-a7siii.cam", include_str!("../share/cameras/sony-a7siii.cam"))];
const STARTER_ENCODERS: &[(&str, &str)] = &[
    ("default-320p.enc", include_str!("../share/encoders/default-320p.enc")),
    ("hevc-hq.enc", include_str!("../share/encoders/hevc-hq.enc")),
];

pub struct Done {
    pub created: Vec<PathBuf>,
    pub pointer: Option<PathBuf>,
}

fn path_conf(root: &Path) -> String {
    let r = |p: &str| root.join(p).display().to_string();
    format!(
        "format: magvault-path/1\n\
         # All roots used by Magvault. Relative paths resolve against this config directory.\n\
         mags: {}\n\
         # Ingest copies land in `incoming` and are renamed into `mags` once verified.\n\
         # It lives inside the mags dataset (same filesystem, atomic rename). Never /tmp.\n\
         incoming: {}\n\
         proxies: {}\n\
         custom_encode: {}\n\
         bins: {}\n\
         db: {}\n\
         logs: {}\n\
         cameras: cameras\n\
         encoders: encoders\n\
         telemetry_live: /dev/shm/magvault-telemetry.json\n\
         telemetry_disk: {}\n",
        r("mags"),
        r("mags/.incoming"),
        r("proxies"),
        r("custom_encode"),
        r("bins"),
        r("db"),
        r("logs"),
        r("state/telemetry-latest.json"),
    )
}

const MAGVAULT_CONF: &str = "format: magvault/1\n\
# Non-path settings.\n\
log_retention_days: 30\n\
default_profile_320p: default-320p\n";

pub fn run(dir: &Path, root: Option<&Path>, write_pointer: bool) -> Result<Done> {
    let dir = if dir.is_absolute() { dir.to_path_buf() } else { std::env::current_dir()?.join(dir) };
    std::fs::create_dir_all(&dir).ctx(format!("cannot create {}", dir.display()))?;
    let mut created = Vec::new();

    let pc = dir.join("path.conf");
    if !pc.exists() {
        let Some(root) = root else {
            crate::bail!("{} does not exist yet; pass --root <data directory> so init can write it", pc.display());
        };
        let root = if root.is_absolute() { root.to_path_buf() } else { std::env::current_dir()?.join(root) };
        atomic_write(&pc, path_conf(&root).as_bytes())?;
        created.push(pc);
    }
    let mc = dir.join("magvault.conf");
    if !mc.exists() {
        atomic_write(&mc, MAGVAULT_CONF.as_bytes())?;
        created.push(mc);
    }

    let cfg = config::load(&dir)?;
    config::ensure_roots(&cfg.paths)?;
    for (folder, files) in [(&cfg.paths.cameras, STARTER_CAMERAS), (&cfg.paths.encoders, STARTER_ENCODERS)] {
        let empty = std::fs::read_dir(folder).map(|mut d| d.next().is_none()).unwrap_or(true);
        if empty {
            for (name, text) in files {
                let p = folder.join(name);
                atomic_write(&p, text.as_bytes())?;
                created.push(p);
            }
        }
    }
    let db_file = cfg.paths.db_file();
    let fresh = !db_file.exists();
    let db = Db::open(&db_file)?;
    db::ensure_schema(&db)?;
    if fresh {
        created.push(db_file);
    }

    let mut pointer = None;
    if write_pointer {
        let p = config::home_pointer().ok_or_else(|| crate::error::Error("HOME is not set".into()))?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        atomic_write(&p, format!("{}\n", dir.display()).as_bytes())?;
        pointer = Some(p);
    }
    Ok(Done { created, pointer })
}

fn client_path_conf(root: &Path, server: Option<&str>) -> String {
    let r = |p: &str| root.join(p).display().to_string();
    format!(
        "format: magvault-client-path/1\n\
         mode: client\n\
         # Local staging: card → staging → server. Mags here are pending until the server\n\
         # confirms, then removed. Never /tmp.\n\
         staging: {}\n\
         logs: {}\n\
         # Copies of the server's camera profiles (served by the server in a later phase).\n\
         cameras: cameras\n\
         # Server reached over SSH as server_user; the server forces `magvault gate` for this key.\n\
         server_host: {}\n\
         server_user: magvaultclient\n\
         server_key:\n\
         # Root helper for field USB prepare / clean (sudo, password every time).\n\
         usb_helper: {}\n",
        r("staging"),
        r("logs"),
        server.unwrap_or(""),
        crate::config::DEFAULT_HELPER
    )
}

/// `magvault init --mode client`: client config directory, staging, logs, starter profiles.
pub fn run_client(
    dir: &Path,
    root: Option<&Path>,
    server: Option<&str>,
    write_pointer: bool,
) -> Result<Done> {
    let dir = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()?.join(dir)
    };
    std::fs::create_dir_all(&dir).ctx(format!("cannot create {}", dir.display()))?;
    let mut created = Vec::new();
    let pc = dir.join("path.conf");
    if !pc.exists() {
        let Some(root) = root else {
            crate::bail!(
                "{} does not exist yet; pass --root <data directory> so init can write it",
                pc.display()
            );
        };
        let root = if root.is_absolute() {
            root.to_path_buf()
        } else {
            std::env::current_dir()?.join(root)
        };
        atomic_write(&pc, client_path_conf(&root, server).as_bytes())?;
        created.push(pc);
    }
    let cc = crate::config::load_client(&dir)?;
    crate::config::check_not_tmp("staging", &cc.staging)?;
    std::fs::create_dir_all(&cc.logs).ctx(format!("cannot create {}", cc.logs.display()))?;
    std::fs::create_dir_all(&cc.cameras).ctx(format!("cannot create {}", cc.cameras.display()))?;
    let empty = std::fs::read_dir(&cc.cameras)
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    if empty {
        for (name, text) in STARTER_CAMERAS {
            let p = cc.cameras.join(name);
            atomic_write(&p, text.as_bytes())?;
            created.push(p);
        }
    }
    let mut pointer = None;
    if write_pointer {
        let p =
            config::home_pointer().ok_or_else(|| crate::error::Error("HOME is not set".into()))?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        atomic_write(&p, format!("{}\n", dir.display()).as_bytes())?;
        pointer = Some(p);
    }
    Ok(Done { created, pointer })
}
