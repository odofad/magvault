//! magvault-helper — the root-owned helper for privileged field-USB operations (PLAN.md
//! "Sudo helper"). Installed as root:root 0755 (e.g. /usr/lib/magvault/magvault-helper) and
//! allowed in sudoers for the Magvault user, with `timestamp_timeout=0`:
//!
//!   Defaults:magvaultclient timestamp_timeout=0
//!   magvaultclient ALL=(root) /usr/lib/magvault/magvault-helper
//!
//! Verbs (nothing else is accepted):
//!
//!   usb-format <device> <fs-label> <blocks>
//!       Format a removable USB disk as ext4 with exactly <blocks> 4 KiB blocks (a larger
//!       slave is limited to the master's size), owned by the calling user, and mount it at
//!       /media/<user>/<fs-label>. Prints the mount point.
//!   usb-clean <master-mount> <slave-mount>
//!       Delete the mag folders listed on a field backup set. Both drives must carry the
//!       same set id, roles master/slave, identical mag lists, and every mag must be synced.
//!
//! Every argument is validated here; the caller is not trusted. Loop devices are accepted
//! only when /etc/magvault/allow-loop-devices exists (test machines).

use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

const ALLOW_LOOP: &str = "/etc/magvault/allow-loop-devices";
const MOUNT_PARENTS: &[&str] = &["/media/", "/run/media/", "/mnt/"];
const BACKUP_FILE: &str = "magvault-backup.txt";

extern "C" {
    fn geteuid() -> u32;
}

fn die(msg: &str) -> ! {
    eprintln!("magvault-helper: {msg}");
    exit(1)
}

fn tool(name: &str) -> PathBuf {
    for dir in ["/usr/sbin", "/sbin", "/usr/bin", "/bin"] {
        let p = Path::new(dir).join(name);
        if p.exists() {
            return p;
        }
    }
    die(&format!("{name} not found"))
}

fn run(name: &str, args: &[&str]) {
    let st = Command::new(tool(name))
        .args(args)
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .status();
    match st {
        Ok(s) if s.success() => {}
        Ok(s) => die(&format!("{name} {} failed ({s})", args.join(" "))),
        Err(e) => die(&format!("cannot run {name}: {e}")),
    }
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Entries of /proc/self/mounts: (source, mount point), octal escapes decoded.
fn mounts() -> Vec<(String, String)> {
    let unesc = |s: &str| {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\'
                && i + 3 < b.len()
                && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
            {
                out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
                i += 4;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    };
    read(Path::new("/proc/self/mounts"))
        .lines()
        .filter_map(|l| {
            let mut f = l.split(' ');
            Some((unesc(f.next()?), unesc(f.next()?)))
        })
        .collect()
}

fn under_mount_parents(mp: &str) -> bool {
    MOUNT_PARENTS.iter().any(|p| mp.starts_with(p))
}

fn valid_fs_label(l: &str) -> bool {
    let b = l.as_bytes();
    !b.is_empty()
        && b.len() <= 16
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'-' || *c == b'_')
}

fn valid_user(u: &str) -> bool {
    let b = u.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && (b[0].is_ascii_lowercase() || b[0] == b'_')
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-')
}

fn is_mag_folder(name: &str) -> bool {
    let b = name.as_bytes();
    if b.len() != 45 || !b[..8].iter().all(u8::is_ascii_digit) || b[8] != b'_' {
        return false;
    }
    b[9..].iter().enumerate().all(|(i, c)| match i {
        8 | 13 | 18 | 23 => *c == b'-',
        _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
    })
}

struct Backup {
    role: String,
    set_id: String,
    mags: Vec<(String, String)>,
}

fn parse_backup(root: &Path) -> Backup {
    let text = std::fs::read_to_string(root.join(BACKUP_FILE))
        .unwrap_or_else(|_| die(&format!("{} has no {BACKUP_FILE}", root.display())));
    let mut b = Backup {
        role: String::new(),
        set_id: String::new(),
        mags: Vec::new(),
    };
    let mut in_mags = false;
    let mut format_ok = false;
    for line in text.lines().map(|l| l.trim_end_matches('\r')) {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('#') {
            in_mags |= line.trim() == "# mags";
            continue;
        }
        if in_mags {
            let f: Vec<&str> = line.split(" | ").collect();
            if f.len() != 3 || !is_mag_folder(f[0]) || !(f[1] == "pending" || f[1] == "synced") {
                die(&format!("{}: malformed mag line", root.display()));
            }
            b.mags.push((f[0].to_string(), f[1].to_string()));
        } else if let Some((k, v)) = line.split_once(':') {
            match k.trim() {
                "format" => format_ok = v.trim() == "magvault-backup/1",
                "role" => b.role = v.trim().to_string(),
                "set_id" => b.set_id = v.trim().to_string(),
                _ => {}
            }
        }
    }
    if !format_ok || b.set_id.is_empty() {
        die(&format!("{}: not a valid {BACKUP_FILE}", root.display()));
    }
    b
}

/// A whole disk that may be formatted: /dev/sdX (USB or removable) or, on test machines, a
/// loop device. Returns (canonical device path, kernel name, size in bytes).
fn check_device(arg: &str) -> (PathBuf, String, u64) {
    if !arg.starts_with("/dev/") {
        die("device must be under /dev");
    }
    let dev = std::fs::canonicalize(arg).unwrap_or_else(|_| die("device not found"));
    let name = dev
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.is_empty() || name.contains('/') || !dev.starts_with("/dev") {
        die("not a block device");
    }
    let sys = PathBuf::from("/sys/class/block").join(&name);
    if !sys.exists() {
        die("not a block device");
    }
    if sys.join("partition").exists() {
        die("give the whole disk, not a partition");
    }
    let is_loop = name.starts_with("loop");
    if is_loop {
        if !Path::new(ALLOW_LOOP).exists() {
            die("loop devices are not allowed (no /etc/magvault/allow-loop-devices)");
        }
    } else {
        let removable = read(&sys.join("removable")) == "1";
        let usb = std::fs::canonicalize(&sys)
            .map(|p| p.to_string_lossy().contains("/usb"))
            .unwrap_or(false);
        if !(removable || usb) {
            die("only removable USB disks can be prepared");
        }
    }
    let holders = std::fs::read_dir(sys.join("holders"))
        .map(|d| d.count())
        .unwrap_or(0);
    if holders > 0 {
        die("disk is in use by device-mapper, LVM or RAID");
    }
    let sectors: u64 = read(&sys.join("size")).parse().unwrap_or(0);
    if sectors == 0 {
        die("disk has no size");
    }
    (dev, name, sectors * 512)
}

fn usb_format(args: &[String]) {
    let [device, label, blocks] = args else {
        die("usage: usb-format <device> <fs-label> <blocks>")
    };
    let (dev, name, size) = check_device(device);
    if !valid_fs_label(label) {
        die("invalid filesystem label");
    }
    let blocks: u64 = blocks
        .parse()
        .unwrap_or_else(|_| die("blocks must be a number"));
    if blocks < 4096 || blocks.saturating_mul(4096) > size {
        die("block count does not fit the disk");
    }
    // Mounts of this disk or its partitions: all must be under /media, /run/media or /mnt,
    // and none may hold mags pending server sync.
    let is_ours = |src: &str| {
        let Ok(c) = std::fs::canonicalize(src) else {
            return false;
        };
        let n = c
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        n == name
            || (n.starts_with(&name)
                && n[name.len()..]
                    .trim_start_matches('p')
                    .chars()
                    .all(|c| c.is_ascii_digit())
                && n.len() > name.len())
    };
    let ours: Vec<String> = mounts()
        .into_iter()
        .filter(|(s, _)| s.starts_with("/dev/") && is_ours(s))
        .map(|(_, m)| m)
        .collect();
    for mp in &ours {
        if !under_mount_parents(mp) {
            die(&format!(
                "disk is mounted at {mp}, outside /media, /run/media, /mnt"
            ));
        }
        let p = Path::new(mp);
        if p.join(BACKUP_FILE).exists() && parse_backup(p).mags.iter().any(|(_, s)| s == "pending")
        {
            die("drive holds mags pending server sync; Prepare is blocked until every mag is synced");
        }
    }
    for mp in &ours {
        run("umount", &[mp]);
    }
    let uid: u32 = std::env::var("SUDO_UID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let gid: u32 = std::env::var("SUDO_GID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let user = std::env::var("SUDO_USER")
        .ok()
        .filter(|u| valid_user(u))
        .unwrap_or_else(|| "root".into());
    let devs = dev.to_string_lossy().into_owned();
    run("wipefs", &["-a", "-q", &devs]);
    run(
        "mkfs.ext4",
        &[
            "-F",
            "-q",
            "-b",
            "4096",
            "-m",
            "0",
            "-L",
            label,
            "-E",
            &format!("root_owner={uid}:{gid}"),
            &devs,
            &blocks.to_string(),
        ],
    );
    let base = PathBuf::from("/media").join(&user);
    let mp = base.join(label);
    if !base.exists() {
        std::fs::create_dir_all(&base)
            .unwrap_or_else(|e| die(&format!("cannot create {}: {e}", base.display())));
    }
    if mp.exists() {
        let busy = mounts().iter().any(|(_, m)| Path::new(m) == mp);
        let nonempty = std::fs::read_dir(&mp)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true);
        if busy || nonempty {
            die(&format!("{} is in use", mp.display()));
        }
    } else {
        std::fs::create_dir(&mp)
            .unwrap_or_else(|e| die(&format!("cannot create {}: {e}", mp.display())));
    }
    let mps = mp.to_string_lossy().into_owned();
    run(
        "mount",
        &["-t", "ext4", "-o", "nosuid,nodev,noexec", &devs, &mps],
    );
    println!("{mps}");
}

fn usb_clean(args: &[String]) {
    let [master, slave] = args else {
        die("usage: usb-clean <master-mount> <slave-mount>")
    };
    let all = mounts();
    let check_mount = |arg: &str| -> PathBuf {
        let p = std::fs::canonicalize(arg).unwrap_or_else(|_| die("mount point not found"));
        let s = p.to_string_lossy().into_owned();
        if !under_mount_parents(&s) || !all.iter().any(|(_, m)| *m == s) {
            die(&format!(
                "{s} is not a mounted drive under /media, /run/media or /mnt"
            ));
        }
        p
    };
    let (m, s) = (check_mount(master), check_mount(slave));
    if std::fs::metadata(&m).map(|x| x.dev()).ok() == std::fs::metadata(&s).map(|x| x.dev()).ok() {
        die("master and slave are the same drive");
    }
    let (bm, bs) = (parse_backup(&m), parse_backup(&s));
    if bm.role != "master" || bs.role != "slave" || bm.set_id != bs.set_id {
        die("the two drives are not the master and slave of one set");
    }
    let lm: BTreeSet<_> = bm.mags.iter().cloned().collect();
    let ls: BTreeSet<_> = bs.mags.iter().cloned().collect();
    if lm != ls {
        die("master and slave mag lists differ");
    }
    if lm.iter().any(|(_, st)| st != "synced") {
        die("mags pending server sync; Clean is only allowed when every mag is synced");
    }
    let mut removed = 0;
    for root in [&m, &s] {
        for (folder, _) in &lm {
            let p = root.join(folder);
            match std::fs::symlink_metadata(&p) {
                Ok(md) if md.is_dir() => {
                    std::fs::remove_dir_all(&p)
                        .unwrap_or_else(|e| die(&format!("cannot remove {}: {e}", p.display())));
                    removed += 1;
                }
                Ok(_) => die(&format!("{} is not a folder", p.display())),
                Err(_) => {}
            }
        }
    }
    println!("removed {removed}");
}

fn main() {
    // SAFETY: plain libc call.
    if unsafe { geteuid() } != 0 {
        die("must run as root (through sudo)");
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("usb-format") => usb_format(&args[1..]),
        Some("usb-clean") => usb_clean(&args[1..]),
        _ => die("usage: magvault-helper usb-format <device> <fs-label> <blocks> | usb-clean <master-mount> <slave-mount>"),
    }
}
