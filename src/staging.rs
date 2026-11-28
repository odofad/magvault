//! Client staging cache and card dump status (PLAN.md "Client staging cache", "Card dump
//! status").
//!
//! Staging holds mags between the card and confirmed backups. A staged mag is *pending*
//! until the server confirms it; then it is *on the server* and listed in
//! `<staging>/.magvault/synced`. It is removed (autoclean) only once two backups exist
//! besides staging: the server and a field backup set. With one backup (server only) it
//! stays. If staging is the only copy (pending, no set) it is never cleaned. Autoclean can
//! be switched off (`staging_autoclean: no`); then `magvault staging clean` clears mags by
//! hand, with the same rule unless one mag is named.
//!
//! Dump status: every mag ingested on this client is recorded in `<config>/dumps.txt` with
//! a fingerprint of the card it came from (relative path, size and mtime of every file)
//! and the identity of that volume (`discover::volume_identity`). A detected card is
//! "dumped" only when both match a recorded mag: this client ingested this card. It is
//! "safe" (green) when that mag has two copies and one of them is the server. Files that
//! match a mag ingested from another volume (a card image and its card, a copied folder)
//! are shown yellow with a note, never green.

use crate::config::ClientConfig;
use crate::error::{Context, Result};
use crate::fieldset::FieldSet;
use crate::ingest::is_mag_folder;
use crate::log::Log;
use crate::textfile::atomic_write;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const STATE_DIR: &str = ".magvault";
const SYNCED_FILE: &str = "synced";
const DUMPS_FILE: &str = "dumps.txt";

// ---- staged mags ---------------------------------------------------------------------------

/// Every mag folder in staging, sorted.
pub fn mags(staging: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(staging)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| is_mag_folder(n))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn synced_path(staging: &Path) -> PathBuf {
    staging.join(STATE_DIR).join(SYNCED_FILE)
}

/// Staged mags the server has confirmed (only those still in staging).
pub fn synced(staging: &Path) -> BTreeSet<String> {
    let listed: BTreeSet<String> = std::fs::read_to_string(synced_path(staging))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|f| is_mag_folder(f))
        .map(String::from)
        .collect();
    mags(staging).into_iter().filter(|m| listed.contains(m)).collect()
}

/// Staged mags waiting for the server.
pub fn pending(staging: &Path) -> Vec<String> {
    let s = synced(staging);
    mags(staging).into_iter().filter(|m| !s.contains(m)).collect()
}

fn write_synced(staging: &Path, set: &BTreeSet<String>) -> Result<()> {
    let dir = staging.join(STATE_DIR);
    std::fs::create_dir_all(&dir).ctx(format!("cannot create {}", dir.display()))?;
    let text: String = set.iter().map(|f| format!("{f}\n")).collect();
    atomic_write(&synced_path(staging), text.as_bytes())
}

/// The server confirmed this staged mag.
pub fn mark_synced(staging: &Path, folder: &str) -> Result<()> {
    let mut s = synced(staging);
    s.insert(folder.to_string());
    write_synced(staging, &s)
}

/// Remove a staged mag and forget its state.
pub fn remove(staging: &Path, folder: &str) -> Result<()> {
    std::fs::remove_dir_all(staging.join(folder)).ctx(format!("cannot remove {folder} from local staging"))?;
    let mut s = synced(staging);
    s.remove(folder);
    write_synced(staging, &s)
}

/// Bytes used by staging (every file below it) and free on its filesystem.
pub fn usage(staging: &Path) -> (u64, u64) {
    fn size(dir: &Path, depth: usize) -> u64 {
        let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
        let mut n = 0;
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() && depth < 32 {
                n += size(&e.path(), depth + 1);
            } else if m.is_file() {
                n += m.len();
            }
        }
        n
    }
    (size(staging, 0), crate::util::free_bytes(staging).unwrap_or(0))
}

/// The backups of a staged mag besides staging itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Backups {
    /// The server confirmed the mag.
    pub server: bool,
    /// A field backup set (master + slave) holds the mag.
    pub field_set: Option<String>,
}

impl Backups {
    pub fn count(&self) -> usize {
        self.server as usize + self.field_set.is_some() as usize
    }
}

/// What autoclean did with a staged mag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clean {
    Removed,
    /// Kept: the reason, for the operator.
    Kept(String),
}

/// The rule (PLAN.md "Client staging cache"): remove only when two backups exist.
pub fn clean_rule(b: &Backups) -> std::result::Result<(), String> {
    match (b.server, &b.field_set) {
        (true, Some(_)) => Ok(()),
        (true, None) => Err("on the server; kept in staging until a second backup exists (field set)".into()),
        (false, Some(s)) => Err(format!("on field set {s}, not yet on the server; kept in staging")),
        (false, None) => Err("staging is the only copy (besides the card); never cleaned".into()),
    }
}

/// Autoclean one staged mag (when `staging_autoclean` is on and two backups exist).
pub fn autoclean(cc: &ClientConfig, folder: &str, b: &Backups, log: &Log) -> Result<Clean> {
    if let Err(why) = clean_rule(b) {
        return Ok(Clean::Kept(why));
    }
    if !cc.staging_autoclean {
        return Ok(Clean::Kept("two backups exist; staging autoclean is off (staging_autoclean: no)".into()));
    }
    remove(&cc.staging, folder)?;
    log.info(
        "staging",
        &format!(
            "Mag {folder} removed from local staging: two backups exist (server and field set {})",
            b.field_set.as_deref().unwrap_or("")
        ),
    );
    Ok(Clean::Removed)
}

/// The field set (mounted, valid) whose list holds `folder`, if any.
pub fn set_holding<'a>(sets: &'a [FieldSet], folder: &str) -> Option<&'a FieldSet> {
    sets.iter().find(|s| s.status == crate::fieldset::SetStatus::Valid && s.mags.iter().any(|m| m.folder == folder))
}

// ---- dump ledger ---------------------------------------------------------------------------

/// Fingerprint of a card's content: every file's relative path, size and mtime (seconds).
/// The same card read again gives the same fingerprint; a card with new clips does not.
pub fn fingerprint(files: &[(String, u64, i64)]) -> String {
    let mut v: Vec<&(String, u64, i64)> = files.iter().collect();
    v.sort();
    let mut h = crate::hash::Sha256::new();
    for (rel, size, mtime) in v {
        h.update(format!("{rel}\t{size}\t{mtime}\n").as_bytes());
    }
    crate::hash::hex(&h.finalize())
}

pub fn probe_fingerprint(files: &[crate::probe::ProbedFile]) -> String {
    let v: Vec<(String, u64, i64)> = files.iter().map(|f| (f.rel.clone(), f.size, f.mtime)).collect();
    fingerprint(&v)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dump {
    pub fingerprint: String,
    pub folder: String,
    pub server: bool,
    /// Field set label, or None.
    pub set: Option<String>,
    /// Volume the mag was ingested from (None: recorded before identities were kept).
    pub identity: Option<String>,
}

fn dumps_path(cc: &ClientConfig) -> PathBuf {
    cc.dir.join(DUMPS_FILE)
}

pub fn dumps(cc: &ClientConfig) -> Vec<Dump> {
    std::fs::read_to_string(dumps_path(cc))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.split(" | ").collect();
            let (fp, folder, server, set, identity) = match p.as_slice() {
                [fp, folder, server, set] => (fp, folder, server, set, None),
                [fp, folder, server, set, id] => (fp, folder, server, set, Some(id.to_string()).filter(|i| i != "-")),
                _ => return None,
            };
            if !is_mag_folder(folder) || fp.len() != 64 {
                return None;
            }
            Some(Dump {
                fingerprint: fp.to_string(),
                folder: folder.to_string(),
                server: *server == "server",
                set: (*set != "-").then(|| set.to_string()),
                identity,
            })
        })
        .collect()
}

fn save_dumps(cc: &ClientConfig, v: &[Dump]) -> Result<()> {
    let text: String = v
        .iter()
        .map(|d| {
            format!(
                "{} | {} | {} | {} | {}\n",
                d.fingerprint,
                d.folder,
                if d.server { "server" } else { "-" },
                d.set.as_deref().unwrap_or("-"),
                d.identity.as_deref().map(|i| i.replace('|', "/")).unwrap_or_else(|| "-".into())
            )
        })
        .collect();
    atomic_write(&dumps_path(cc), text.as_bytes())
}

/// A new mag from a card (best effort: the ledger drives the status colour and the
/// "already dumped" check).
pub fn record_dump(cc: &ClientConfig, fingerprint: &str, identity: &str, folder: &str, set: Option<&str>) {
    let mut v = dumps(cc);
    v.retain(|d| d.folder != folder);
    v.push(Dump {
        fingerprint: fingerprint.into(),
        folder: folder.into(),
        server: false,
        set: set.map(String::from),
        identity: Some(identity.replace('|', "/")),
    });
    let _ = save_dumps(cc, &v);
}

/// The server confirmed `folder`; `set` (when Some) holds it too.
pub fn record_server(cc: &ClientConfig, folder: &str, set: Option<&str>) {
    let mut v = dumps(cc);
    for d in v.iter_mut().filter(|d| d.folder == folder) {
        d.server = true;
        if let Some(s) = set {
            d.set = Some(s.to_string());
        }
    }
    let _ = save_dumps(cc, &v);
}

/// A field set was cleaned: its mags are no longer copies.
pub fn record_set_cleaned(cc: &ClientConfig, label: &str) {
    let mut v = dumps(cc);
    for d in v.iter_mut().filter(|d| d.set.as_deref() == Some(label)) {
        d.set = None;
    }
    let _ = save_dumps(cc, &v);
}

/// Card dump status (PLAN.md "Card dump status").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpStatus {
    /// Red: no Magvault copy of this card.
    NotDumped,
    /// Yellow: dumped, but not yet two copies with one on the server.
    Dumped { copies: usize, server: bool },
    /// Green: two copies and on the server.
    Safe { copies: usize },
    /// Yellow at most: the files match a mag this client ingested from another volume
    /// (e.g. an image of this card); this card itself was not ingested here.
    Matches { folder: String },
}

impl DumpStatus {
    pub fn text(&self) -> String {
        match self {
            DumpStatus::NotDumped => "not dumped".into(),
            DumpStatus::Dumped { server: false, .. } => "dumped · not yet safe".into(),
            DumpStatus::Dumped { server: true, .. } => "on server · 1 copy only".into(),
            DumpStatus::Safe { copies } => format!("{copies} copies · on server"),
            DumpStatus::Matches { .. } => "files match an ingested mag".into(),
        }
    }
}

/// Copies of a recorded mag now: server, staging, field set (a mounted set is checked;
/// an unmounted one is trusted from the ledger).
pub fn status_of(d: &Dump, staging: &Path, sets: &[FieldSet]) -> DumpStatus {
    let on_set = match &d.set {
        Some(label) => match sets.iter().find(|s| &s.label == label) {
            Some(s) => s.mags.iter().any(|m| m.folder == d.folder),
            None => true,
        },
        None => set_holding(sets, &d.folder).is_some(),
    };
    let in_staging = staging.join(&d.folder).is_dir();
    let copies = d.server as usize + in_staging as usize + on_set as usize;
    if d.server && copies >= 2 {
        DumpStatus::Safe { copies }
    } else {
        DumpStatus::Dumped { copies, server: d.server }
    }
}

/// Status of a detected card: its fingerprint and its volume identity must both match a
/// recorded mag for "dumped"; a fingerprint match alone is `Matches` (yellow, with a note).
pub fn card_status(ledger: &[Dump], fingerprint: Option<&str>, identity: Option<&str>, staging: &Path, sets: &[FieldSet]) -> DumpStatus {
    let Some(fp) = fingerprint else { return DumpStatus::NotDumped };
    let same = ledger.iter().rev().filter(|d| d.fingerprint == fp);
    if let Some(d) = same.clone().find(|d| identity.is_some() && d.identity.as_deref() == identity) {
        return status_of(d, staging, sets);
    }
    match same.into_iter().next() {
        Some(d) => DumpStatus::Matches { folder: d.folder.clone() },
        None => DumpStatus::NotDumped,
    }
}

/// The mag a status refers to (None: not dumped).
pub fn matched_folder(ledger: &[Dump], fingerprint: &str, identity: &str) -> Option<String> {
    let same: Vec<&Dump> = ledger.iter().rev().filter(|d| d.fingerprint == fingerprint).collect();
    same.iter().find(|d| d.identity.as_deref() == Some(identity)).or(same.first()).map(|d| d.folder.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    const F1: &str = "20261126_aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    const F2: &str = "20261126_aaaaaaaa-bbbb-4ccc-8ddd-ffffffffffff";

    #[test]
    fn two_backups_rule() {
        let b = |server, set: Option<&str>| Backups { server, field_set: set.map(String::from) };
        assert!(clean_rule(&b(true, Some("FIELD-A"))).is_ok());
        assert!(clean_rule(&b(true, None)).unwrap_err().contains("until a second backup"));
        assert!(clean_rule(&b(false, Some("FIELD-A"))).unwrap_err().contains("not yet on the server"));
        assert!(clean_rule(&b(false, None)).unwrap_err().contains("only copy"));
    }

    #[test]
    fn synced_list_and_pending() {
        let d = std::env::temp_dir().join(format!("mv-staging-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join(F1)).unwrap();
        std::fs::create_dir_all(d.join(F2)).unwrap();
        std::fs::create_dir_all(d.join(".incoming")).unwrap();
        assert_eq!(pending(&d), vec![F1.to_string(), F2.to_string()]);
        mark_synced(&d, F1).unwrap();
        assert_eq!(pending(&d), vec![F2.to_string()]);
        assert!(synced(&d).contains(F1));
        remove(&d, F1).unwrap();
        assert!(synced(&d).is_empty());
        assert_eq!(mags(&d), vec![F2.to_string()]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn fingerprint_is_order_free_and_content_bound() {
        let a = vec![("A.MP4".to_string(), 10, 1), ("B.MP4".to_string(), 20, 2)];
        let b = vec![("B.MP4".to_string(), 20, 2), ("A.MP4".to_string(), 10, 1)];
        assert_eq!(fingerprint(&a), fingerprint(&b));
        let c = vec![("A.MP4".to_string(), 10, 1), ("B.MP4".to_string(), 21, 2)];
        assert_ne!(fingerprint(&a), fingerprint(&c));
    }

    #[test]
    fn dump_colours() {
        let staging = std::env::temp_dir().join(format!("mv-dump-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(staging.join(F1)).unwrap();
        let d = |server, set: Option<&str>, folder: &str| Dump { fingerprint: "f".repeat(64), folder: folder.into(), server, set: set.map(String::from), identity: Some("disk:AAAA-1111:64000".into()) };
        // Staging only: yellow, not on the server.
        assert_eq!(status_of(&d(false, None, F1), &staging, &[]), DumpStatus::Dumped { copies: 1, server: false });
        // Staging + server: two copies, one on the server → green.
        assert_eq!(status_of(&d(true, None, F1), &staging, &[]), DumpStatus::Safe { copies: 2 });
        // Server only (staging cleaned by hand): yellow.
        assert_eq!(status_of(&d(true, None, F2), &staging, &[]), DumpStatus::Dumped { copies: 1, server: true });
        // Server + set not mounted (trusted from the ledger): green.
        assert_eq!(status_of(&d(true, Some("FIELD-A"), F2), &staging, &[]), DumpStatus::Safe { copies: 2 });
        assert_eq!(card_status(&[], Some("x"), Some("disk:AAAA-1111:64000"), &staging, &[]), DumpStatus::NotDumped);
        // Same files, same volume: this card was ingested here → its real status.
        let ledger = vec![d(true, None, F1)];
        let fp = "f".repeat(64);
        assert_eq!(card_status(&ledger, Some(&fp), Some("disk:AAAA-1111:64000"), &staging, &[]), DumpStatus::Safe { copies: 2 });
        // Same files from another volume (an image of the card, a copy): yellow at most.
        assert_eq!(card_status(&ledger, Some(&fp), Some("image:AAAA-1111:64000"), &staging, &[]), DumpStatus::Matches { folder: F1.into() });
        assert_eq!(card_status(&ledger, Some(&fp), None, &staging, &[]), DumpStatus::Matches { folder: F1.into() });
        // An old ledger line without an identity never gives green.
        let mut old = d(true, None, F1);
        old.identity = None;
        assert_eq!(card_status(&[old], Some(&fp), Some("disk:AAAA-1111:64000"), &staging, &[]), DumpStatus::Matches { folder: F1.into() });
        assert_eq!(matched_folder(&ledger, &fp, "image:x"), Some(F1.to_string()));
        let _ = std::fs::remove_dir_all(&staging);
    }
}
