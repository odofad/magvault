//! `magvault usb …` — field backup set status, Prepare (master, then slave) and Clean
//! (PLAN.md "USB drive diagnostics and prepare", "Prepare popup", "Clean"). Terminal
//! version of the popups: each step is shown, Enter / empty input cancels, and nothing is
//! written or deleted until the sudo password for the helper is accepted.

use crate::config::ClientConfig;
use crate::error::{Error, Result};
use crate::fieldset::{self, BackupFile, Disk, FieldSet, Role, SetStatus};
use crate::ingest::is_mag_folder;
use crate::log::Log;
use crate::time;
use crate::util::{self, esc, human};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

pub const SMALLER_FIRST: &str = "Mount the smaller drive first and prepare it as master.";

pub fn status(scan_roots: &[PathBuf]) -> Result<()> {
    let drives = fieldset::drives(scan_roots);
    let sets = fieldset::sets(&drives);
    if sets.is_empty() {
        println!("no field backup set mounted");
    }
    for s in &sets {
        let actions = match (&s.status, s.pending().len(), s.mags.len()) {
            (SetStatus::Valid, 0, 0) => "empty, ready for new mags".to_string(),
            (SetStatus::Valid, 0, _) => "all synced · [usb clean] available".to_string(),
            (SetStatus::Valid, p, _) => {
                format!("{p} pending · selectable · Prepare blocked until synced (`magvault sync`)")
            }
            _ => "mount both drives of the set".to_string(),
        };
        println!("{}", s.line());
        println!("    set id {}  ·  {actions}", s.set_id);
        for (role, root) in [("master", &s.master), ("slave", &s.slave)] {
            if let Some(r) = root {
                let free = util::free_bytes(r)
                    .map(human)
                    .unwrap_or_else(|_| "?".into());
                println!(
                    "    {role:<6} {}  ({free} free)",
                    esc(&r.display().to_string())
                );
            }
        }
    }
    // Drives that are not members of a set: removable disks (real), or scan-root folders
    // (fixtures). Only these can be prepared.
    if scan_roots.is_empty() {
        for c in fieldset::candidates().unwrap_or_default() {
            match &c.disk {
                Ok(d) if d.backup.is_none() => println!("USB drive {}  ·  not valid  ·  [usb prepare]", d.line()),
                Ok(_) => {}
                Err(why) => println!("USB drive {why}"),
            }
        }
    } else {
        let in_roots = |m: &Path| scan_roots.iter().any(|r| m.starts_with(r));
        for d in drives
            .iter()
            .filter(|d| !matches!(d.backup, Ok(Some(_))) && in_roots(&d.mount))
        {
            let why = match &d.backup {
                Err(e) => format!("not valid ({})", esc(e)),
                _ => "not valid".into(),
            };
            println!(
                "USB drive {}  ·  {why}  ·  [usb prepare]",
                esc(&d.mount.display().to_string())
            );
        }
    }
    Ok(())
}

/// One answer from the terminal; `None` when the operator pressed Esc (then Enter) or closed
/// the input (Ctrl-D): the popup is cancelled.
fn ask(prompt: &str) -> Result<Option<String>> {
    if !std::io::stdin().is_terminal() {
        crate::bail!("this step needs a terminal; use `magvault usb prepare-master` / `prepare-slave` with --device and --yes");
    }
    eprint!("{prompt}");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line)? == 0 || line.contains('\u{1b}') {
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

fn pick_disk(disks: &[Disk], exclude: Option<&str>, what: &str) -> Result<Option<Disk>> {
    let list: Vec<&Disk> = disks
        .iter()
        .filter(|d| Some(d.path.as_str()) != exclude)
        .collect();
    if list.is_empty() {
        crate::bail!("no removable drive found");
    }
    eprintln!("  Mounted drives");
    for (i, d) in list.iter().enumerate() {
        eprintln!("  {:>2}  {}", i + 1, d.line());
    }
    let a = ask(&format!(
        "  Pick the {what} [number; Enter or Esc cancels]: "
    ))?
    .unwrap_or_default();
    if a.is_empty() || a.eq_ignore_ascii_case("q") {
        return Ok(None);
    }
    let n: usize = a.parse().map_err(|_| Error("not a number".into()))?;
    list.get(n.wrapping_sub(1))
        .map(|d| Some((*d).clone()))
        .ok_or_else(|| Error("no such drive".into()))
}

fn disk_by_path(path: &str) -> Result<Disk> {
    let want = std::fs::canonicalize(path).map_err(|_| Error(format!("{path} not found")))?;
    let found = fieldset::candidates()?
        .into_iter()
        .find(|c| std::fs::canonicalize(&c.path).ok().as_ref() == Some(&want));
    match found {
        Some(c) => c.disk.map_err(Error),
        None => Err(Error(format!(
            "{path} is not a USB or removable whole disk that can be prepared (camera cards and SD slots are never prepared)"
        ))),
    }
}

/// Master: format the whole drive; its size becomes the set size.
pub fn prepare_master(cc: &ClientConfig, disk: &Disk, label: &str, yes: bool) -> Result<PathBuf> {
    let log = Log::new(&cc.logs);
    if !fieldset::valid_label(label) {
        crate::bail!(
            "set label {label:?}: use A-Z, 0-9 and '-', at most 12 characters (e.g. FIELD-A)"
        );
    }
    fieldset::prepare_allowed(disk.backup.as_ref())?;
    let blocks = fieldset::master_blocks(disk.size)?;
    let set_size = blocks * fieldset::BLOCK;
    eprintln!();
    eprintln!("  {SMALLER_FIRST}");
    eprintln!("  Master  {} · {}", disk.path, human(disk.size));
    eprintln!("  Set size {}", human(set_size));
    eprintln!(
        "  ALL DATA ON {} WILL BE DESTROYED. Then: format ext4, write {} (master).",
        disk.path,
        fieldset::FILE
    );
    eprintln!("  Nothing is written until the sudo password is accepted.");
    if !yes && !util::confirm("  Confirm size and prepare the master?")? {
        crate::bail!("cancelled; nothing was written");
    }
    let fs_label = format!("{label}{}", Role::Master.suffix());
    let mount = fieldset::run_helper(
        &cc.usb_helper,
        &["usb-format", &disk.path, &fs_label, &blocks.to_string()],
    )?;
    let root = PathBuf::from(mount.lines().last().unwrap_or(""));
    let b = BackupFile {
        role: Role::Master,
        set_id: util::uuid_v4()?,
        set_size,
        label: label.into(),
        created: time::iso_utc(time::now_secs()),
        host: util::hostname(),
        mags: Vec::new(),
    };
    b.save(&root)?;
    log.info(
        "fieldset",
        &format!(
            "Prepared master of field set {label} on {} ({}), set size {}",
            disk.path,
            root.display(),
            human(set_size)
        ),
    );
    eprintln!(
        "  master ready: {label} at {} (set size {})",
        root.display(),
        human(set_size)
    );
    Ok(root)
}

/// Slave: same size or larger than the master; a larger slave is limited to the set size.
pub fn prepare_slave(
    cc: &ClientConfig,
    disk: &Disk,
    master: &FieldSet,
    yes: bool,
) -> Result<PathBuf> {
    let log = Log::new(&cc.logs);
    let Some(mroot) = &master.master else {
        crate::bail!("a slave cannot be prepared before its master exists (mount the master)")
    };
    if master.slave.is_some() {
        crate::bail!("field set {} already has a slave mounted", master.label);
    }
    if !master.mags.is_empty() {
        crate::bail!(
            "the master of {} already holds mags; a slave can only be added to an empty master",
            master.label
        );
    }
    if disk.mounts.iter().any(|m| m == mroot) {
        crate::bail!("{} is the master of this set", disk.path);
    }
    fieldset::prepare_allowed(disk.backup.as_ref())?;
    let (blocks, unused) = fieldset::slave_plan(master.set_size, disk.size)?;
    eprintln!();
    eprintln!(
        "  The slave must be the same size or larger than the master ({}).",
        human(master.set_size)
    );
    eprintln!("  Slave  {} · {}", disk.path, human(disk.size));
    if unused > 0 {
        eprintln!(
            "  Will use {} to match master. Rest unused.",
            human(master.set_size)
        );
    }
    eprintln!(
        "  ALL DATA ON {} WILL BE DESTROYED. Then: format ext4 limited to {}, write {} (slave).",
        disk.path,
        human(master.set_size),
        fieldset::FILE
    );
    eprintln!("  Nothing is written until the sudo password is accepted.");
    if !yes && !util::confirm("  Confirm and prepare the slave?")? {
        crate::bail!("cancelled; nothing was written to the slave");
    }
    let fs_label = format!("{}{}", master.label, Role::Slave.suffix());
    let mount = fieldset::run_helper(
        &cc.usb_helper,
        &["usb-format", &disk.path, &fs_label, &blocks.to_string()],
    )?;
    let root = PathBuf::from(mount.lines().last().unwrap_or(""));
    let mb = BackupFile::load(mroot)?
        .ok_or_else(|| Error("the master lost its magvault-backup.txt".into()))?;
    let b = BackupFile {
        role: Role::Slave,
        created: time::iso_utc(time::now_secs()),
        host: util::hostname(),
        ..mb
    };
    b.save(&root)?;
    log.info(
        "fieldset",
        &format!(
            "Prepared slave of field set {} on {} ({}), limited to {}",
            master.label,
            disk.path,
            root.display(),
            human(master.set_size)
        ),
    );
    eprintln!(
        "  slave ready: set {} is complete (master + slave, {})",
        master.label,
        human(master.set_size)
    );
    Ok(root)
}

/// The master of an incomplete set (master mounted, no slave), by label or the only one.
fn waiting_master(scan_roots: &[PathBuf], label: Option<&str>) -> Result<FieldSet> {
    let sets: Vec<FieldSet> = fieldset::sets(&fieldset::drives(scan_roots))
        .into_iter()
        .filter(|s| s.master.is_some() && s.slave.is_none() && label.is_none_or(|l| l == s.label))
        .collect();
    match sets.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(Error(
            "no master without a slave is mounted; prepare the master first (smaller drive)".into(),
        )),
        _ => Err(Error(
            "several masters are waiting for a slave; pick one with --set <label>".into(),
        )),
    }
}

pub fn prepare_master_cmd(cc: &ClientConfig, device: &str, label: &str, yes: bool) -> Result<()> {
    prepare_master(cc, &disk_by_path(device)?, label, yes).map(|_| ())
}

pub fn prepare_slave_cmd(
    cc: &ClientConfig,
    device: &str,
    set: Option<&str>,
    yes: bool,
    scan_roots: &[PathBuf],
) -> Result<()> {
    let master = waiting_master(scan_roots, set)?;
    prepare_slave(cc, &disk_by_path(device)?, &master, yes).map(|_| ())
}

/// The whole popup: warning, pick master, confirm, sudo; then pick slave (smaller refused),
/// confirm, sudo. Enter at a prompt cancels.
pub fn prepare_interactive(cc: &ClientConfig, scan_roots: &[PathBuf]) -> Result<()> {
    eprintln!("Prepare a field backup set · step 1 of 2 · master");
    eprintln!("  {SMALLER_FIRST}");
    eprintln!("  Pick the master first. Its size becomes the set size.");
    let disks = fieldset::candidate_disks()?;
    let Some(master_disk) = pick_disk(&disks, None, "master")? else {
        eprintln!("cancelled; nothing was written");
        return Ok(());
    };
    let Some(label) = ask("  Set label [FIELD-A; Esc cancels]: ")? else {
        eprintln!("cancelled; nothing was written");
        return Ok(());
    };
    let label = if label.is_empty() {
        "FIELD-A".to_string()
    } else {
        label.to_uppercase()
    };
    prepare_master(cc, &master_disk, &label, false)?;
    loop {
        eprintln!();
        eprintln!("Prepare a field backup set · step 2 of 2 · slave");
        let disks = fieldset::candidate_disks()?;
        let Some(slave_disk) = pick_disk(&disks, Some(&master_disk.path), "slave")? else {
            eprintln!("cancelled; the master is ready, the set has no slave yet (`magvault usb prepare-slave`)");
            return Ok(());
        };
        let master = waiting_master(scan_roots, Some(&label))?;
        match prepare_slave(cc, &slave_disk, &master, false) {
            Ok(_) => return Ok(()),
            Err(e) if e.0.contains("smaller slave is refused") => eprintln!("  REFUSED: {}", e.0),
            Err(e) => return Err(e),
        }
    }
}

/// Mag folders on a drive that are not in the set's list (could be unsynced data).
fn unlisted(root: &Path, set: &FieldSet) -> Vec<String> {
    std::fs::read_dir(root)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| is_mag_folder(n) && !set.mags.iter().any(|m| &m.folder == n))
                .collect()
        })
        .unwrap_or_default()
}

pub fn clean(
    cc: &ClientConfig,
    label: Option<&str>,
    yes: bool,
    scan_roots: &[PathBuf],
) -> Result<()> {
    let log = Log::new(&cc.logs);
    let set = fieldset::choose_set(fieldset::sets(&fieldset::drives(scan_roots)), label)?;
    fieldset::clean_allowed(&set)?;
    let (m, s) = set.roots()?;
    let stray: Vec<String> = [m, s].iter().flat_map(|r| unlisted(r, &set)).collect();
    if !stray.is_empty() {
        crate::bail!(
            "{} holds mag folder(s) that are not in its list ({}); they may not be on the server. Not cleaning.",
            set.label,
            stray.join(", ")
        );
    }
    eprintln!("Clean field backup set · {}", set.label);
    eprintln!(
        "  All mags on {} are on the server. Wipe master and slave?",
        set.label
    );
    eprintln!(
        "  Deletes the {} mag folder(s) on both drives.",
        set.mags.len()
    );
    eprintln!(
        "  Keeps the set: {}, set id, size limit and the master / slave roles. No re-prepare.",
        fieldset::FILE
    );
    eprintln!(
        "  After clean the set is empty, still valid, ready for new mags. This action is logged."
    );
    eprintln!("  Nothing is deleted until the sudo password is accepted.");
    if !yes && !util::confirm("  Wipe master and slave?")? {
        crate::bail!("cancelled; nothing was deleted");
    }
    let out = fieldset::run_helper(
        &cc.usb_helper,
        &[
            "usb-clean",
            &m.display().to_string(),
            &s.display().to_string(),
        ],
    )?;
    fieldset::save_list(&set, &[])?;
    let n = set.mags.len();
    log.info(
        "fieldset",
        &format!(
            "Field set {} cleaned: {n} mag(s) removed from master and slave ({})",
            set.label, out
        ),
    );
    println!(
        "cleaned   {}  ({n} mag(s) removed; set still valid and empty)",
        set.label
    );
    Ok(())
}
