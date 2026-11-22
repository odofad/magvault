//! Magvault command line (Phase 0).
//!
//!   magvault init --config <dir> [--root <data dir>] [--write-pointer]
//!   magvault ingest <source> --project <name> [options]
//!   magvault reconcile <mag folder or uuid>... | --all [--verify-hashes]

mod catalog;
mod config;
mod db;
#[macro_use]
mod error;
mod hash;
mod hash_consts;
mod ingest;
mod init;
mod log;
mod magcard;
mod mhl;
mod probe;
mod profiles;
mod reconcile;
mod textfile;
mod time;
mod util;

use error::{Error, Result};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
magvault — archive and retrieval for camera media mags (Phase 0)

USAGE
  magvault [--config <dir>] <command> [options]

COMMANDS
  init        Create a config directory, data roots, starter profiles and the database
                --root <dir>        data directory for a new path.conf
                --write-pointer     remember the config dir in ~/.config/magvault/config-dir

  ingest <source>
              Probe a card or folder, confirm, hash, verified rsync copy into
              mags/<YYYYMMDD>_<uuid>/, write magcard.txt + ASC MHL, record in the
              database, queue a 320p proxy job.
                --project <name>      open project to add to (created if none has this name)
                --project-id <uuid>   pick a project by UUID instead
                --camera <id>         camera profile id (default: guessed from the card)
                --format <value>      file format  (default: the profile's default_format)
                --gamma <value>       gamma        (default: the profile's default_gamma)
                --gamut <value>       gamut        (default: the profile's default_gamut)
                --shoot-date <date>   YYYY-MM-DD   (default: guessed from the media dates)
                --notes <text>        mag-level notes
                --yes                 do not ask for confirmation

  reconcile <mag>... | --all
              Compare magcard.txt with the database. Metadata: database wins (old card
              backed up to .magvault/). Manifest or file differences are reported only.
                --verify-hashes       also re-hash every file against the stored SHA-256

CONFIG
  The config directory comes from --config, else MAGVAULT_CONFIG, else the pointer file
  ~/.config/magvault/config-dir or /etc/magvault/config-dir.
";

struct Cli {
    config: Option<String>,
    command: Option<String>,
    positional: Vec<String>,
    opts: Vec<(String, Option<String>)>,
}

const FLAGS: &[&str] = &["--yes", "-y", "--all", "--verify-hashes", "--write-pointer", "--help", "-h", "--version"];

fn parse_cli() -> Result<Cli> {
    let mut cli = Cli { config: None, command: None, positional: Vec::new(), opts: Vec::new() };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--" {
            cli.positional.extend(args.by_ref());
            break;
        }
        if a.starts_with('-') && a.len() > 1 {
            let (name, inline) = match a.split_once('=') {
                Some((n, v)) => (n.to_string(), Some(v.to_string())),
                None => (a.clone(), None),
            };
            if FLAGS.contains(&name.as_str()) {
                if inline.is_some() {
                    bail!("{name} does not take a value");
                }
                cli.opts.push((name, None));
                continue;
            }
            let value = match inline {
                Some(v) => v,
                None => args.next().ok_or_else(|| Error(format!("{name} needs a value")))?,
            };
            if name == "--config" {
                cli.config = Some(value);
            } else {
                cli.opts.push((name, Some(value)));
            }
        } else if cli.command.is_none() {
            cli.command = Some(a);
        } else {
            cli.positional.push(a);
        }
    }
    Ok(cli)
}

impl Cli {
    fn flag(&self, names: &[&str]) -> bool {
        self.opts.iter().any(|(n, _)| names.contains(&n.as_str()))
    }
    fn take(&mut self, name: &str) -> Option<String> {
        let i = self.opts.iter().position(|(n, v)| n == name && v.is_some())?;
        self.opts.remove(i).1
    }
    fn reject_unknown(&self, allowed: &[&str]) -> Result<()> {
        for (n, _) in &self.opts {
            if !allowed.contains(&n.as_str()) {
                bail!("unknown option {n} for `{}` (see `magvault --help`)", self.command.as_deref().unwrap_or(""));
            }
        }
        Ok(())
    }
}

fn run() -> Result<ExitCode> {
    let mut cli = parse_cli()?;
    if cli.flag(&["--version"]) {
        println!("magvault {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }
    let Some(cmd) = cli.command.clone() else {
        print!("{USAGE}");
        return Ok(if cli.flag(&["--help", "-h"]) { ExitCode::SUCCESS } else { ExitCode::from(2) });
    };
    if cli.flag(&["--help", "-h"]) || cmd == "help" {
        print!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    }
    match cmd.as_str() {
        "init" => {
            let root = cli.take("--root").map(PathBuf::from);
            cli.reject_unknown(&["--write-pointer"])?;
            if !cli.positional.is_empty() {
                bail!("init takes no positional arguments");
            }
            let dir = cli.config.clone().ok_or_else(|| Error("init needs --config <dir>".into()))?;
            let done = init::run(&PathBuf::from(&dir), root.as_deref(), cli.flag(&["--write-pointer"]))?;
            for p in &done.created {
                println!("created {}", p.display());
            }
            if let Some(p) = done.pointer {
                println!("pointer {} → {dir}", p.display());
            }
            println!("config directory ready: {dir}");
            Ok(ExitCode::SUCCESS)
        }
        "ingest" => {
            let a = ingest::Args {
                project: cli.take("--project"),
                project_id: cli.take("--project-id"),
                camera: cli.take("--camera"),
                format: cli.take("--format"),
                gamma: cli.take("--gamma"),
                gamut: cli.take("--gamut"),
                shoot_date: cli.take("--shoot-date"),
                notes: cli.take("--notes"),
                yes: cli.flag(&["--yes", "-y"]),
                source: match cli.positional.as_slice() {
                    [one] => PathBuf::from(one),
                    [] => bail!("ingest needs a source directory: magvault ingest /path/to/card --project <name>"),
                    _ => bail!("ingest takes exactly one source directory"),
                },
            };
            cli.reject_unknown(&["--yes", "-y"])?;
            let cfg = config::load(&config::resolve_dir(cli.config.as_deref())?)?;
            let out = ingest::run(&cfg, a)?;
            println!("ingested {}", out.folder);
            println!("  folder  {}", out.mag_dir.display());
            println!("  files   {} ({} clips) · {}", out.files, out.clips, util::human(out.bytes));
            if out.clips > 0 {
                println!("  proxy   320p job queued");
            }
            Ok(ExitCode::SUCCESS)
        }
        "reconcile" => {
            cli.reject_unknown(&["--all", "--verify-hashes"])?;
            let all = cli.flag(&["--all"]);
            if !all && cli.positional.is_empty() {
                bail!("reconcile needs one or more mag folders/UUIDs, or --all");
            }
            let cfg = config::load(&config::resolve_dir(cli.config.as_deref())?)?;
            let reports = reconcile::run(&cfg, &cli.positional, all, cli.flag(&["--verify-hashes"]))?;
            let mut bad = false;
            for r in &reports {
                if r.integrity.is_empty() && r.rewritten.is_empty() && r.notes.is_empty() {
                    println!("ok        {}", r.folder);
                }
                if !r.rewritten.is_empty() {
                    println!("rewritten {}  (database wins: {})", r.folder, r.rewritten.join(", "));
                }
                for n in &r.notes {
                    println!("note      {}  {n}", r.folder);
                }
                for i in &r.integrity {
                    bad = true;
                    println!("INTEGRITY {}  {i}", r.folder);
                }
            }
            if reports.is_empty() {
                println!("no online mags");
            }
            Ok(if bad { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        other => bail!("unknown command {other:?} (see `magvault --help`)"),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
