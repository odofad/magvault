//! Field backup sets (PLAN.md "Client ingest and field backups", "Field backup sets",
//! "Prepare popup", "Clean", "Prepare"). Field USB only: unrelated to zmag roles.
//!
//! - A set is exactly one master and one slave, prepared master first. Set size = the
//!   master's usable size; the slave must be the same size or larger and is limited to the
//!   set size (the ext4 filesystem is created with exactly the same number of 4 KiB blocks).
//! - Both drives hold the same mags, always. Each has `magvault-backup.txt` at its root
//!   with role, set id, set size, label, created, host and the mag list (pending / synced).
//! - Prepare (format ext4) and Clean (delete mag folders) run through the root-owned
//!   helper (`magvault-helper`) via sudo; nothing is written or deleted until the sudo
//!   password is accepted. Prepare is refused on a drive that holds pending mags; Clean is
//!   only offered when every mag is synced.

use crate::discover;
use crate::error::{Context, Error, Result};
use crate::ingest::is_mag_folder;
use crate::textfile::atomic_write;
use crate::time;
use crate::util::{self, esc, human};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const FILE: &str = "magvault-backup.txt";
pub const FORMAT: &str = "magvault-backup/1";
pub const BLOCK: u64 = 4096;
/// Root-controlled switch that lets the helper (and the drive list) use loop devices, for
/// tests on machines without spare USB drives. Never present in production.
pub const ALLOW_LOOP: &str = "/etc/magvault/allow-loop-devices";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Master,
    Slave,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Master => "master",
            Role::Slave => "slave",
        }
    }
    /// Filesystem label suffix, so both drives can be mounted side by side.
    pub fn suffix(self) -> &'static str {
        match self {
            Role::Master => "_M",
            Role::Slave => "_S",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MagState {
    Pending,
    Synced,
}

impl MagState {
    pub fn as_str(self) -> &'static str {
        match self {
            MagState::Pending => "pending",
            MagState::Synced => "synced",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MagEntry {
    pub folder: String,
    pub state: MagState,
    pub updated: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackupFile {
    pub role: Role,
    pub set_id: String,
    pub set_size: u64,
    pub label: String,
    pub created: String,
    pub host: String,
    pub mags: Vec<MagEntry>,
}

/// Set label: upper-case letters, digits and `-`, at most 12 characters (the ext4 label
/// adds `_M` / `_S` and must stay within 16).
pub fn valid_label(l: &str) -> bool {
    let b = l.as_bytes();
    !b.is_empty()
        && b.len() <= 12
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'-')
}

impl BackupFile {
    pub fn pending(&self) -> usize {
        self.mags
            .iter()
            .filter(|m| m.state == MagState::Pending)
            .count()
    }

    pub fn render(&self) -> String {
        let mut s = format!(
            "format: {FORMAT}\nrole: {}\nset_id: {}\nset_size: {}\nlabel: {}\ncreated: {}\nhost: {}\n\n# mags\n# mag_folder | state | updated\n",
            self.role.as_str(),
            self.set_id,
            self.set_size,
            self.label,
            self.created,
            self.host
        );
        for m in &self.mags {
            s.push_str(&format!(
                "{} | {} | {}\n",
                m.folder,
                m.state.as_str(),
                m.updated
            ));
        }
        s
    }

    pub fn parse(text: &str) -> std::result::Result<BackupFile, String> {
        let mut header: BTreeMap<String, String> = BTreeMap::new();
        let mut mags = Vec::new();
        let mut in_mags = false;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with('#') {
                if line.trim() == "# mags" {
                    in_mags = true;
                }
                continue;
            }
            if in_mags {
                let f: Vec<&str> = line.split(" | ").collect();
                if f.len() != 3 || line.matches('|').count() != 2 {
                    return Err(format!(
                        "line {}: mag line needs 3 fields separated by ' | '",
                        n + 1
                    ));
                }
                if !is_mag_folder(f[0]) {
                    return Err(format!(
                        "line {}: {:?} is not a mag folder name",
                        n + 1,
                        esc(f[0])
                    ));
                }
                let state = match f[1] {
                    "pending" => MagState::Pending,
                    "synced" => MagState::Synced,
                    other => {
                        return Err(format!(
                            "line {}: state {:?} must be pending or synced",
                            n + 1,
                            esc(other)
                        ))
                    }
                };
                mags.push(MagEntry {
                    folder: f[0].into(),
                    state,
                    updated: f[2].into(),
                });
            } else if let Some((k, v)) = line.split_once(':') {
                header.insert(k.trim().into(), v.trim().into());
            } else {
                return Err(format!("line {}: expected `key: value`", n + 1));
            }
        }
        let get = |k: &str| header.get(k).cloned().unwrap_or_default();
        if get("format") != FORMAT {
            return Err(format!("format must be {FORMAT}"));
        }
        let role = match get("role").as_str() {
            "master" => Role::Master,
            "slave" => Role::Slave,
            other => return Err(format!("role {:?} must be master or slave", esc(other))),
        };
        let set_id = get("set_id");
        if !util::uuid_ok(&set_id) {
            return Err("set_id is not a UUID".into());
        }
        let set_size: u64 = get("set_size")
            .parse()
            .map_err(|_| "set_size is not a number".to_string())?;
        if set_size == 0 || !set_size.is_multiple_of(BLOCK) {
            return Err("set_size must be a positive multiple of 4096".into());
        }
        let label = get("label");
        if !valid_label(&label) {
            return Err(format!("label {:?} is not a valid set label", esc(&label)));
        }
        Ok(BackupFile {
            role,
            set_id,
            set_size,
            label,
            created: get("created"),
            host: get("host"),
            mags,
        })
    }

    /// `Ok(None)` = no `magvault-backup.txt` (a not-valid drive).
    pub fn load(root: &Path) -> Result<Option<BackupFile>> {
        match std::fs::read_to_string(root.join(FILE)) {
            Ok(t) => BackupFile::parse(&t)
                .map(Some)
                .map_err(|e| Error(format!("{}: {e}", root.join(FILE).display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error(format!(
                "cannot read {}: {e}",
                root.join(FILE).display()
            ))),
        }
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        atomic_write(&root.join(FILE), self.render().as_bytes())
    }
}

/// A mounted volume and what its `magvault-backup.txt` says.
#[derive(Debug, Clone)]
pub struct Drive {
    pub mount: PathBuf,
    pub backup: std::result::Result<Option<BackupFile>, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetStatus {
    Valid,
    Incomplete(String),
    Inconsistent(String),
}

#[derive(Debug, Clone)]
pub struct FieldSet {
    pub set_id: String,
    pub label: String,
    pub set_size: u64,
    pub master: Option<PathBuf>,
    pub slave: Option<PathBuf>,
    pub mags: Vec<MagEntry>,
    pub status: SetStatus,
}

impl FieldSet {
    pub fn pending(&self) -> Vec<&MagEntry> {
        self.mags
            .iter()
            .filter(|m| m.state == MagState::Pending)
            .collect()
    }
    pub fn synced(&self) -> usize {
        self.mags
            .iter()
            .filter(|m| m.state == MagState::Synced)
            .count()
    }
    pub fn line(&self) -> String {
        let status = match &self.status {
            SetStatus::Valid => "valid".to_string(),
            SetStatus::Incomplete(r) | SetStatus::Inconsistent(r) => format!("NOT USABLE: {r}"),
        };
        let p = self.pending().len();
        format!(
            "USB set {}  ·  master + slave  ·  {}  ·  {status}  ·  {} mag(s): {} pending, {} synced",
            self.label,
            human(self.set_size),
            self.mags.len(),
            p,
            self.synced()
        )
    }
    /// Both drive roots of a valid set.
    pub fn roots(&self) -> Result<(&Path, &Path)> {
        match (&self.status, &self.master, &self.slave) {
            (SetStatus::Valid, Some(m), Some(s)) => Ok((m, s)),
            (SetStatus::Incomplete(r), ..) | (SetStatus::Inconsistent(r), ..) => Err(Error(
                format!("field set {} is not usable: {r}", self.label),
            )),
            _ => Err(Error(format!("field set {} is not usable", self.label))),
        }
    }
}

/// Mounted volumes with their backup files: real mounts under `/media`, `/run/media`,
/// `/mnt`, plus the folders inside any `--scan-root` (tests and fixtures).
pub fn drives(scan_roots: &[PathBuf]) -> Vec<Drive> {
    let mut vols = discover::volumes(&[]);
    if !scan_roots.is_empty() {
        vols.extend(discover::volumes(scan_roots));
    }
    let mut seen = std::collections::BTreeSet::new();
    vols.into_iter()
        .filter(|v| seen.insert(v.mount.clone()))
        .map(|v| Drive {
            backup: BackupFile::load(&v.mount).map_err(|e| e.0),
            mount: v.mount,
        })
        .collect()
}

fn same_list(a: &[MagEntry], b: &[MagEntry]) -> bool {
    let key = |l: &[MagEntry]| -> Vec<(String, MagState)> {
        let mut v: Vec<_> = l.iter().map(|m| (m.folder.clone(), m.state)).collect();
        v.sort_by(|x, y| x.0.cmp(&y.0));
        v
    };
    key(a) == key(b)
}

/// Group drives into sets by `set_id`.
pub fn sets(drives: &[Drive]) -> Vec<FieldSet> {
    let mut groups: BTreeMap<String, Vec<(&Drive, &BackupFile)>> = BTreeMap::new();
    for d in drives {
        if let Ok(Some(b)) = &d.backup {
            groups.entry(b.set_id.clone()).or_default().push((d, b));
        }
    }
    let mut out = Vec::new();
    for (set_id, members) in groups {
        let masters: Vec<_> = members
            .iter()
            .filter(|(_, b)| b.role == Role::Master)
            .collect();
        let slaves: Vec<_> = members
            .iter()
            .filter(|(_, b)| b.role == Role::Slave)
            .collect();
        let first = members[0].1;
        let mut set = FieldSet {
            set_id,
            label: first.label.clone(),
            set_size: first.set_size,
            master: masters.first().map(|(d, _)| d.mount.clone()),
            slave: slaves.first().map(|(d, _)| d.mount.clone()),
            mags: masters
                .first()
                .map(|(_, b)| b.mags.clone())
                .unwrap_or_else(|| first.mags.clone()),
            status: SetStatus::Valid,
        };
        set.status = if masters.len() > 1 || slaves.len() > 1 {
            SetStatus::Inconsistent("more than one drive with the same role is mounted".into())
        } else if masters.is_empty() {
            SetStatus::Incomplete("master not mounted".into())
        } else if slaves.is_empty() {
            SetStatus::Incomplete("slave not mounted".into())
        } else {
            let (m, s) = (masters[0].1, slaves[0].1);
            if m.set_size != s.set_size || m.label != s.label {
                SetStatus::Inconsistent("master and slave disagree on size or label".into())
            } else if !same_list(&m.mags, &s.mags) {
                SetStatus::Inconsistent("master and slave mag lists differ".into())
            } else {
                SetStatus::Valid
            }
        };
        out.push(set);
    }
    out
}

/// Pick the set to use: by label, or the only valid set mounted.
pub fn choose_set(all: Vec<FieldSet>, label: Option<&str>) -> Result<FieldSet> {
    let describe = |v: &[FieldSet]| v.iter().map(|s| s.line()).collect::<Vec<_>>().join("\n  ");
    if let Some(l) = label {
        return all.iter().find(|s| s.label == l).cloned().ok_or_else(|| {
            Error(format!(
                "no field set labelled {l} is mounted\n  {}",
                describe(&all)
            ))
        });
    }
    let valid: Vec<_> = all
        .iter()
        .filter(|s| s.status == SetStatus::Valid)
        .collect();
    match valid.as_slice() {
        [one] => Ok((*one).clone()),
        [] if all.is_empty() => Err(Error("no field backup set is mounted".into())),
        [] => Err(Error(format!(
            "no usable field backup set is mounted\n  {}",
            describe(&all)
        ))),
        _ => Err(Error(format!(
            "several field sets are mounted; pick one with --set <label>\n  {}",
            describe(&all)
        ))),
    }
}

/// Write the same mag list to both drives (master first).
pub fn save_list(set: &FieldSet, mags: &[MagEntry]) -> Result<()> {
    let (m, s) = set.roots()?;
    for root in [m, s] {
        let mut b = BackupFile::load(root)?
            .ok_or_else(|| Error(format!("{} lost its {FILE}", root.display())))?;
        b.mags = mags.to_vec();
        b.save(root)?;
    }
    Ok(())
}

pub fn add_pending(set: &mut FieldSet, folder: &str) -> Result<()> {
    let mut mags = set.mags.clone();
    mags.retain(|m| m.folder != folder);
    mags.push(MagEntry {
        folder: folder.into(),
        state: MagState::Pending,
        updated: time::iso_utc(time::now_secs()),
    });
    save_list(set, &mags)?;
    set.mags = mags;
    Ok(())
}

pub fn mark_synced(set: &mut FieldSet, folder: &str) -> Result<()> {
    let mut mags = set.mags.clone();
    for m in mags.iter_mut().filter(|m| m.folder == folder) {
        m.state = MagState::Synced;
        m.updated = time::iso_utc(time::now_secs());
    }
    save_list(set, &mags)?;
    set.mags = mags;
    Ok(())
}

// ---- rules (pure, unit-tested) -------------------------------------------------------------

/// Master: the whole drive, in 4 KiB blocks. Set size = blocks × 4096.
pub fn master_blocks(drive_bytes: u64) -> Result<u64> {
    let blocks = drive_bytes / BLOCK;
    if blocks < 4096 {
        crate::bail!("drive is too small ({})", human(drive_bytes));
    }
    Ok(blocks)
}

/// Slave: same size or larger than the set; a larger slave is limited to the set size.
/// Returns (blocks to format, bytes left unused).
pub fn slave_plan(set_size: u64, slave_bytes: u64) -> Result<(u64, u64)> {
    if slave_bytes < set_size {
        crate::bail!(
            "the slave ({}) is smaller than the master ({}). A smaller slave is refused: start a new set with the smaller drive as master",
            human(slave_bytes),
            human(set_size)
        );
    }
    Ok((set_size / BLOCK, slave_bytes - set_size))
}

/// Prepare is blocked on a drive that holds mags pending server sync.
pub fn prepare_allowed(existing: Option<&BackupFile>) -> Result<()> {
    if let Some(b) = existing {
        let p = b.pending();
        if p > 0 {
            crate::bail!(
                "{} ({}) holds {p} mag(s) pending server sync; Prepare is blocked until every mag is synced (run `magvault sync`)",
                b.label,
                b.role.as_str()
            );
        }
    }
    Ok(())
}

/// Clean only on a valid set whose every mag is synced, and only when there is something to
/// clean.
pub fn clean_allowed(set: &FieldSet) -> Result<()> {
    set.roots()?;
    let p = set.pending().len();
    if p > 0 {
        crate::bail!("{} mag(s) on {} are pending server sync; Clean is only offered when every mag is synced", p, set.label);
    }
    if set.mags.is_empty() {
        crate::bail!("{} holds no mags; nothing to clean", set.label);
    }
    Ok(())
}

// ---- drives that can be prepared ------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Disk {
    pub path: String,
    pub size: u64,
    pub label: String,
    pub mounts: Vec<PathBuf>,
    pub backup: Option<BackupFile>,
}

impl Disk {
    pub fn line(&self) -> String {
        let state = match &self.backup {
            Some(b) => format!(
                "set {} {} · {} pending",
                b.label,
                b.role.as_str(),
                b.pending()
            ),
            None => "not valid".into(),
        };
        let mounts: Vec<String> = self
            .mounts
            .iter()
            .map(|m| esc(&m.display().to_string()))
            .collect();
        format!(
            "{}  {}  {}  {}  {}",
            self.path,
            human(self.size),
            if self.label.is_empty() {
                "-".into()
            } else {
                esc(&self.label)
            },
            if mounts.is_empty() {
                "not mounted".into()
            } else {
                mounts.join(", ")
            },
            state
        )
    }
}

fn lsblk_pairs(line: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut rest = line.trim();
    while let Some(eq) = rest.find("=\"") {
        let key = rest[..eq].trim().to_string();
        let after = &rest[eq + 2..];
        let Some(end) = after.find('"') else { break };
        out.insert(key, after[..end].to_string());
        rest = &after[end + 1..];
    }
    out
}

/// A whole disk lsblk reports as removable / USB (or a loop device on test machines),
/// either offered for Prepare or refused with the reason the operator sees.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: String,
    pub disk: std::result::Result<Disk, String>,
}

const LSBLK_COLUMNS: &str = "NAME,PATH,SIZE,TYPE,RM,HOTPLUG,TRAN,FSTYPE,LABEL,MOUNTPOINT,PKNAME,MODEL";

fn lsblk_text() -> Result<String> {
    let out = Command::new("lsblk")
        .args(["-b", "-P", "-o", LSBLK_COLUMNS])
        .env("LC_ALL", "C")
        .output()
        .ctx("cannot run lsblk")?;
    if !out.status.success() {
        crate::bail!("lsblk failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Card readers by model name (USB readers show up as ordinary `sdX` disks).
fn is_card_reader_model(model: &str) -> bool {
    let m = model.to_lowercase();
    ["card reader", "cardreader", "card-reader", "sd/mmc", "sdxc", "sdhc", "microsd", "multi-card", "multi card"]
        .iter()
        .any(|k| m.contains(k))
}

/// Classify the whole disks in `lsblk -P` output. Offered: removable or USB disks
/// (`RM=1`, `HOTPLUG=1` or `TRAN=usb`; USB hard drives report `RM=0`), loop devices when
/// `allow_loop`. Not listed at all: camera cards (SD/MMC slots `mmcblk*`, card readers by
/// model, or a mounted camera layout per `is_camera`). Refused with a reason: disks with a
/// ZFS / LVM / RAID / LUKS signature, disks mounted outside /media, /run/media, /mnt.
pub fn classify_disks(text: &str, allow_loop: bool, is_camera: &dyn Fn(&Path) -> bool) -> Vec<Candidate> {
    let rows: Vec<BTreeMap<String, String>> = text.lines().map(lsblk_pairs).collect();
    let mut out = Vec::new();
    for r in &rows {
        let g = |k: &str| r.get(k).cloned().unwrap_or_default();
        let kind = g("TYPE");
        let name = g("NAME");
        let removable = g("RM") == "1" || g("HOTPLUG") == "1" || g("TRAN") == "usb";
        let ok_kind = (kind == "disk" && removable) || (kind == "loop" && allow_loop);
        let size: u64 = g("SIZE").parse().unwrap_or(0);
        if !ok_kind || size == 0 {
            continue;
        }
        let family: Vec<&BTreeMap<String, String>> = rows
            .iter()
            .filter(|c| c.get("NAME") == Some(&name) || c.get("PKNAME") == Some(&name))
            .collect();
        let mounts: Vec<PathBuf> = family
            .iter()
            .filter_map(|c| c.get("MOUNTPOINT"))
            .filter(|m| !m.is_empty())
            .map(PathBuf::from)
            .collect();
        if name.starts_with("mmcblk") || is_card_reader_model(&g("MODEL")) || mounts.iter().any(|m| is_camera(m)) {
            continue;
        }
        let path = g("PATH");
        let mut sigs: Vec<&str> = Vec::new();
        for c in &family {
            let sig = match c.get("FSTYPE").map(String::as_str) {
                Some("zfs_member") => "ZFS",
                Some("LVM2_member") => "LVM",
                Some("linux_raid_member") => "RAID",
                Some("crypto_LUKS") => "LUKS",
                _ => continue,
            };
            if !sigs.contains(&sig) {
                sigs.push(sig);
            }
        }
        if !sigs.is_empty() {
            out.push(Candidate {
                disk: Err(format!(
                    "{path} has an existing {} signature; wipe it first (wipefs), then Prepare",
                    sigs.join("/")
                )),
                path,
            });
            continue;
        }
        if let Some(m) = mounts.iter().find(|m| !discover::MOUNT_PARENTS.iter().any(|p| m.starts_with(p))) {
            out.push(Candidate {
                disk: Err(format!(
                    "{path} is mounted at {} (outside /media, /run/media, /mnt); unmount it first, then Prepare",
                    esc(&m.display().to_string())
                )),
                path,
            });
            continue;
        }
        let backup = mounts.iter().find_map(|m| BackupFile::load(m).ok().flatten());
        let label = family
            .iter()
            .filter_map(|c| c.get("LABEL"))
            .find(|l| !l.is_empty())
            .cloned()
            .unwrap_or_default();
        out.push(Candidate {
            disk: Ok(Disk { path: path.clone(), size, label, mounts, backup }),
            path,
        });
    }
    out
}

/// Whole disks lsblk reports (offered and refused), camera cards left out.
pub fn candidates() -> Result<Vec<Candidate>> {
    let is_camera = |m: &Path| discover::layout_of(m, &[]).is_some();
    Ok(classify_disks(&lsblk_text()?, Path::new(ALLOW_LOOP).exists(), &is_camera))
}

/// Disks that can be offered for Prepare (see `classify_disks`).
pub fn candidate_disks() -> Result<Vec<Disk>> {
    Ok(candidates()?.into_iter().filter_map(|c| c.disk.ok()).collect())
}

// ---- privileged helper -----------------------------------------------------------------------

extern "C" {
    fn geteuid() -> u32;
}

pub fn is_root() -> bool {
    // SAFETY: plain libc call without arguments.
    unsafe { geteuid() == 0 }
}

/// Run the root helper: directly when already root, else `sudo -k <helper> …` (sudo asks
/// for the password on the terminal; `-k` so it is asked every time, PLAN.md "Sudo helper").
/// Returns the helper's stdout.
pub fn run_helper(helper: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = if is_root() {
        Command::new(helper)
    } else {
        let mut c = Command::new("sudo");
        c.arg("-k").arg(helper);
        c
    };
    let out = cmd
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .ctx(format!("cannot run {}", helper.display()))?;
    if !out.status.success() {
        crate::bail!(
            "{} {} did not run (sudo password not accepted, or the helper refused; see the message above)",
            helper.display(),
            args.first().unwrap_or(&"")
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bf(role: Role, mags: &[(&str, MagState)]) -> BackupFile {
        BackupFile {
            role,
            set_id: "11111111-2222-4333-8444-555555555555".into(),
            set_size: 64 * 1024 * 1024,
            label: "FIELD-A".into(),
            created: "2026-11-26T20:00:00Z".into(),
            host: "laptop".into(),
            mags: mags
                .iter()
                .map(|(f, s)| MagEntry {
                    folder: f.to_string(),
                    state: *s,
                    updated: "2026-11-26T20:00:00Z".into(),
                })
                .collect(),
        }
    }
    const F1: &str = "20261126_aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    const F2: &str = "20261126_aaaaaaaa-bbbb-4ccc-8ddd-ffffffffffff";

    #[test]
    fn backup_file_roundtrip_and_validation() {
        let b = bf(
            Role::Master,
            &[(F1, MagState::Pending), (F2, MagState::Synced)],
        );
        let back = BackupFile::parse(&b.render()).unwrap();
        assert_eq!(back, b);
        assert_eq!(back.pending(), 1);
        assert!(BackupFile::parse(&b.render().replace("role: master", "role: primary")).is_err());
        assert!(BackupFile::parse(&b.render().replace("| pending |", "| maybe |")).is_err());
        assert!(BackupFile::parse(&b.render().replace(F1, "../../etc")).is_err());
        assert!(
            BackupFile::parse(&b.render().replace("format: magvault-backup/1", "format: x"))
                .is_err()
        );
    }

    #[test]
    fn slave_smaller_refused_larger_limited() {
        let set = master_blocks(64 * 1024 * 1024).unwrap() * BLOCK;
        assert_eq!(set, 64 * 1024 * 1024);
        let err = slave_plan(set, 48 * 1024 * 1024).unwrap_err();
        assert!(err.0.contains("smaller slave is refused"), "{}", err.0);
        assert_eq!(slave_plan(set, set).unwrap(), (set / BLOCK, 0));
        assert_eq!(
            slave_plan(set, 96 * 1024 * 1024).unwrap(),
            (set / BLOCK, 32 * 1024 * 1024)
        );
    }

    #[test]
    fn prepare_blocked_while_pending() {
        assert!(prepare_allowed(None).is_ok());
        assert!(prepare_allowed(Some(&bf(Role::Master, &[(F1, MagState::Synced)]))).is_ok());
        let e = prepare_allowed(Some(&bf(Role::Slave, &[(F1, MagState::Pending)]))).unwrap_err();
        assert!(e.0.contains("Prepare is blocked"));
    }

    fn drive(name: &str, b: BackupFile) -> Drive {
        Drive {
            mount: PathBuf::from(format!("/media/op/{name}")),
            backup: Ok(Some(b)),
        }
    }

    #[test]
    fn sets_and_clean_rules() {
        let m = bf(Role::Master, &[(F1, MagState::Pending)]);
        let s = bf(Role::Slave, &[(F1, MagState::Pending)]);
        let all = sets(&[drive("FIELD-A_M", m.clone()), drive("FIELD-A_S", s.clone())]);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, SetStatus::Valid);
        assert!(clean_allowed(&all[0]).unwrap_err().0.contains("pending"));

        let synced = |r| bf(r, &[(F1, MagState::Synced)]);
        let all = sets(&[
            drive("A", synced(Role::Master)),
            drive("B", synced(Role::Slave)),
        ]);
        assert!(clean_allowed(&all[0]).is_ok());

        let empty = sets(&[
            drive("A", bf(Role::Master, &[])),
            drive("B", bf(Role::Slave, &[])),
        ]);
        assert!(clean_allowed(&empty[0])
            .unwrap_err()
            .0
            .contains("nothing to clean"));

        let only_master = sets(&[drive("A", m.clone())]);
        assert_eq!(
            only_master[0].status,
            SetStatus::Incomplete("slave not mounted".into())
        );
        assert!(clean_allowed(&only_master[0]).is_err());

        let differ = sets(&[drive("A", m), drive("B", bf(Role::Slave, &[]))]);
        assert!(matches!(differ[0].status, SetStatus::Inconsistent(_)));
    }

    #[test]
    fn labels() {
        assert!(valid_label("FIELD-A"));
        assert!(!valid_label("field-a"));
        assert!(!valid_label("FIELD-A-TOO-LONG"));
        assert!(!valid_label("A/B"));
    }

    #[test]
    fn lsblk_line() {
        let r = lsblk_pairs(
            r#"NAME="sdb" PATH="/dev/sdb" SIZE="64023257088" TYPE="disk" RM="1" LABEL="NEW 64" MOUNTPOINT="""#,
        );
        assert_eq!(r["SIZE"], "64023257088");
        assert_eq!(r["LABEL"], "NEW 64");
        assert_eq!(r["MOUNTPOINT"], "");
    }
    #[test]
    fn prepare_candidates_from_lsblk() {
        // Modelled on rafiki: OS NVMe, SD slot with a camera card, a USB hard drive (RM=0,
        // TRAN=usb) with an old ZFS pool, one with ext4 + LVM, a clean USB drive, a USB card
        // reader, a USB drive mounted as a system path, and a loop device.
        let t = r#"NAME="nvme0n1" PATH="/dev/nvme0n1" SIZE="512110190592" TYPE="disk" RM="0" HOTPLUG="0" TRAN="nvme" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL="Samsung SSD"
NAME="nvme0n1p2" PATH="/dev/nvme0n1p2" SIZE="511000000000" TYPE="part" RM="0" HOTPLUG="0" TRAN="" FSTYPE="ext4" LABEL="" MOUNTPOINT="/" PKNAME="nvme0n1" MODEL=""
NAME="mmcblk0" PATH="/dev/mmcblk0" SIZE="127865454592" TYPE="disk" RM="0" HOTPLUG="1" TRAN="" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL=""
NAME="mmcblk0p1" PATH="/dev/mmcblk0p1" SIZE="127861260288" TYPE="part" RM="0" HOTPLUG="1" TRAN="" FSTYPE="exfat" LABEL="" MOUNTPOINT="/run/media/rafiki/disk" PKNAME="mmcblk0" MODEL=""
NAME="sda" PATH="/dev/sda" SIZE="320072933376" TYPE="disk" RM="0" HOTPLUG="0" TRAN="usb" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL="ST320LM010"
NAME="sda1" PATH="/dev/sda1" SIZE="320000000000" TYPE="part" RM="0" HOTPLUG="0" TRAN="" FSTYPE="zfs_member" LABEL="wildraid" MOUNTPOINT="" PKNAME="sda" MODEL=""
NAME="sdb" PATH="/dev/sdb" SIZE="500107862016" TYPE="disk" RM="0" HOTPLUG="0" TRAN="usb" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL="ST500LM021"
NAME="sdb1" PATH="/dev/sdb1" SIZE="1000000000" TYPE="part" RM="0" HOTPLUG="0" TRAN="" FSTYPE="ext4" LABEL="boot" MOUNTPOINT="" PKNAME="sdb" MODEL=""
NAME="sdb2" PATH="/dev/sdb2" SIZE="499000000000" TYPE="part" RM="0" HOTPLUG="0" TRAN="" FSTYPE="LVM2_member" LABEL="" MOUNTPOINT="" PKNAME="sdb" MODEL=""
NAME="sdc" PATH="/dev/sdc" SIZE="1000204886016" TYPE="disk" RM="0" HOTPLUG="0" TRAN="usb" FSTYPE="ext4" LABEL="SPARE" MOUNTPOINT="/media/rafiki/SPARE" PKNAME="" MODEL="Elements 25A2"
NAME="sdd" PATH="/dev/sdd" SIZE="63864569856" TYPE="disk" RM="1" HOTPLUG="1" TRAN="usb" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL="SD/MMC Reader"
NAME="sdd1" PATH="/dev/sdd1" SIZE="63860375552" TYPE="part" RM="1" HOTPLUG="1" TRAN="" FSTYPE="exfat" LABEL="EOS" MOUNTPOINT="/media/rafiki/EOS" PKNAME="sdd" MODEL=""
NAME="sde" PATH="/dev/sde" SIZE="2000398934016" TYPE="disk" RM="0" HOTPLUG="0" TRAN="usb" FSTYPE="ext4" LABEL="" MOUNTPOINT="/srv/backup" PKNAME="" MODEL="My Passport"
NAME="sdf" PATH="/dev/sdf" SIZE="31914983424" TYPE="disk" RM="1" HOTPLUG="1" TRAN="usb" FSTYPE="exfat" LABEL="A7S" MOUNTPOINT="/media/rafiki/A7S" PKNAME="" MODEL="Flash Drive"
NAME="loop0" PATH="/dev/loop0" SIZE="67108864" TYPE="loop" RM="0" HOTPLUG="0" TRAN="" FSTYPE="" LABEL="" MOUNTPOINT="" PKNAME="" MODEL=""
"#;
        let camera = |m: &Path| m == Path::new("/media/rafiki/A7S");
        let c = classify_disks(t, false, &camera);
        let paths: Vec<&str> = c.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, vec!["/dev/sda", "/dev/sdb", "/dev/sdc", "/dev/sde"], "no NVMe, SD slot, card reader, camera card, loop");
        assert_eq!(
            c[0].disk.as_ref().unwrap_err(),
            "/dev/sda has an existing ZFS signature; wipe it first (wipefs), then Prepare"
        );
        assert_eq!(
            c[1].disk.as_ref().unwrap_err(),
            "/dev/sdb has an existing LVM signature; wipe it first (wipefs), then Prepare"
        );
        let ok = c[2].disk.as_ref().unwrap();
        assert_eq!((ok.size, ok.label.as_str()), (1000204886016, "SPARE"), "USB drive with RM=0 accepted via TRAN=usb");
        assert!(c[3].disk.as_ref().unwrap_err().contains("mounted at /srv/backup (outside /media"));
        let with_loop = classify_disks(t, true, &camera);
        assert_eq!(with_loop.last().unwrap().path, "/dev/loop0");
    }
}

