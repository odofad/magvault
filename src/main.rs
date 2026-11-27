//! Magvault command line (Phases 0 and 1). See `magvault --help`.

mod catalog;
mod client;
mod client_ingest;
mod config;
mod db;
mod discover;
mod fieldset;
#[macro_use]
mod error;
mod gate;
mod hash;
mod hash_consts;
mod help;
mod ingest;
mod init;
mod interrupt;
mod log;
mod out;
mod magcard;
mod mhl;
mod probe;
mod profiles;
mod reconcile;
mod serve;
mod sync;
mod tui;
mod textfile;
mod time;
mod usb;
mod util;

use error::{Error, Result};
use std::path::PathBuf;
use std::process::ExitCode;


struct Cli {
    config: Option<String>,
    command: Option<String>,
    positional: Vec<String>,
    opts: Vec<(String, Option<String>)>,
    /// --verbose / --debug: DEBUG lines for this run (PLAN.md "Debug logging").
    debug: bool,
}

const FLAGS: &[&str] = &[
    "--yes",
    "-y",
    "--all",
    "--verify-hashes",
    "--write-pointer",
    "--help",
    "-h",
    "--version",
    "--once",
    "--mirror-set",
    "--no-sync",
];

fn parse_cli() -> Result<Cli> {
    let mut cli = Cli {
        config: None,
        command: None,
        positional: Vec::new(),
        opts: Vec::new(),
        debug: false,
    };
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
            if (name == "--verbose" || name == "--debug") && inline.is_none() {
                cli.debug = true;
                continue;
            }
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
    fn take_all(&mut self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        while let Some(v) = self.take(name) {
            out.push(v);
        }
        out
    }
    fn take(&mut self, name: &str) -> Option<String> {
        let i = self.opts.iter().position(|(n, v)| n == name && v.is_some())?;
        self.opts.remove(i).1
    }
    fn reject_unknown(&self, allowed: &[&str]) -> Result<()> {
        for (n, _) in &self.opts {
            if !allowed.contains(&n.as_str()) {
                let c = self.command.as_deref().unwrap_or("");
                bail!("unknown option {n} for `{c}` (see `magvault {c} --help`)");
            }
        }
        Ok(())
    }
}

fn base_args(cli: &mut Cli) -> ingest::Args {
    ingest::Args {
        project: cli.take("--project"),
        project_id: cli.take("--project-id"),
        camera: cli.take("--camera"),
        format: cli.take("--format"),
        gamma: cli.take("--gamma"),
        gamut: cli.take("--gamut"),
        shoot_date: cli.take("--shoot-date"),
        notes: cli.take("--notes"),
        yes: cli.flag(&["--yes", "-y"]),
        source: PathBuf::new(),
    }
}

/// Source from the command line, or one of the detected cards (operator picks).
fn source_or_card(
    positional: &[String],
    cams: &[profiles::CameraProfile],
    scan_roots: &[PathBuf],
    yes: bool,
) -> Result<PathBuf> {
    match positional {
        [one] => return Ok(PathBuf::from(one)),
        [] => {}
        _ => bail!("ingest takes exactly one source directory"),
    }
    let cards = discover::find_cards(&discover::volumes(scan_roots), cams);
    let searched = if scan_roots.is_empty() {
        "under /media, /run/media or /mnt".to_string()
    } else {
        format!("in {}", scan_roots.iter().map(|r| r.display().to_string()).collect::<Vec<_>>().join(", "))
    };
    if cards.is_empty() {
        bail!("no camera card detected {searched}; pass the path: magvault ingest /path/to/card --project <name>");
    }
    eprintln!("detected cards:");
    for (i, c) in cards.iter().enumerate() {
        eprintln!("  {:>2}  {}", i + 1, c.line());
    }
    let tty = std::io::IsTerminal::is_terminal(&std::io::stdin());
    if cards.len() == 1 {
        let c = &cards[0];
        if let Some(p) = &c.problem {
            bail!("the detected card cannot be ingested: {p}");
        }
        if yes {
            eprintln!("--yes: one card detected, using {}", c.volume.mount.display());
            return Ok(c.volume.mount.clone());
        }
    } else if yes || !tty {
        bail!(
            "{} cards detected and {}; pass the path of the card: magvault ingest /path/to/card --project <name>{}",
            cards.len(),
            if yes { "--yes picks a card only when exactly one is found" } else { "there is no terminal to pick one" },
            if tty { " (or run without --yes and pick one)" } else { "" }
        );
    }
    if !tty {
        bail!("no terminal to confirm the card; re-run with --yes (one card detected) or pass the path");
    }
    eprint!("pick a card [number; Enter cancels]: ");
    let mut line = String::new();
    std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut line)?;
    let n: usize = line
        .trim()
        .parse()
        .map_err(|_| Error("cancelled; nothing was written".into()))?;
    let c = cards
        .get(n.wrapping_sub(1))
        .ok_or_else(|| Error("no such card".into()))?;
    if let Some(p) = &c.problem {
        bail!("that card cannot be ingested: {p}");
    }
    Ok(c.volume.mount.clone())
}

fn scan_roots(cli: &mut Cli) -> Vec<PathBuf> {
    discover::scan_roots(
        &cli.take_all("--scan-root")
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    )
}

fn run() -> Result<ExitCode> {
    let mut cli = parse_cli()?;
    if cli.flag(&["--version"]) {
        println!("magvault {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }
    let Some(cmd) = cli.command.clone() else {
        // No command: the TUI on a terminal with a client config; otherwise the help.
        let tty = std::io::IsTerminal::is_terminal(&std::io::stdin()) && std::io::IsTerminal::is_terminal(&std::io::stdout());
        let client = config::resolve_dir(cli.config.as_deref())
            .ok()
            .and_then(|d| config::read_mode(&d).ok().map(|m| (d, m)))
            .filter(|(_, m)| *m == config::Mode::Client);
        if let (false, true, Some((dir, _))) = (cli.flag(&["--help", "-h"]), tty, client) {
            return client_command("ui", cli, &dir);
        }
        print!("{}", help::TOP);
        return Ok(if cli.flag(&["--help", "-h"]) { ExitCode::SUCCESS } else { ExitCode::from(2) });
    };
    // `magvault help [<command>]` and `magvault <command> --help`.
    let help_for = if cmd == "help" {
        Some(cli.positional.first().cloned())
    } else if cli.flag(&["--help", "-h"]) {
        Some(Some(cmd.clone()))
    } else {
        None
    };
    if let Some(topic) = help_for {
        match topic {
            None => print!("{}", help::TOP),
            Some(t) => match help::command(&t) {
                Some(text) => print!("{text}"),
                None => bail!("unknown command {t:?} (see `magvault --help`)"),
            },
        }
        return Ok(ExitCode::SUCCESS);
    }
    match cmd.as_str() {
        "init" => {
            let root = cli.take("--root").map(PathBuf::from);
            let mode = cli.take("--mode");
            let server = cli.take("--server");
            cli.reject_unknown(&["--write-pointer"])?;
            if !cli.positional.is_empty() {
                bail!("init takes no positional arguments");
            }
            let dir = cli
                .config
                .clone()
                .ok_or_else(|| Error("init needs --config <dir>".into()))?;
            let done = match mode.as_deref() {
                None | Some("server") => {
                    if server.is_some() {
                        bail!("--server is only for --mode client");
                    }
                    init::run(
                        &PathBuf::from(&dir),
                        root.as_deref(),
                        cli.flag(&["--write-pointer"]),
                    )?
                }
                Some("client") => init::run_client(
                    &PathBuf::from(&dir),
                    root.as_deref(),
                    server.as_deref(),
                    cli.flag(&["--write-pointer"]),
                )?,
                Some(other) => bail!("--mode {other:?} must be server or client"),
            };
            for p in &done.created {
                println!("created {}", p.display());
            }
            if let Some(p) = done.pointer {
                println!("pointer {} → {dir}", p.display());
            }
            println!("config directory ready: {dir}");
            Ok(ExitCode::SUCCESS)
        }
        "gate" => {
            let spool = cli
                .take("--spool")
                .ok_or_else(|| Error("gate needs --spool".into()))?;
            let results = cli
                .take("--results")
                .ok_or_else(|| Error("gate needs --results".into()))?;
            let client = cli
                .take("--client")
                .ok_or_else(|| Error("gate needs --client".into()))?;
            let rrsync = cli
                .take("--rrsync")
                .unwrap_or_else(|| "/usr/bin/rrsync".into());
            cli.reject_unknown(&[])?;
            gate::run(
                &PathBuf::from(spool),
                &PathBuf::from(results),
                &client,
                &PathBuf::from(rrsync),
            )?;
            Ok(ExitCode::SUCCESS)
        }
        "cards" => {
            let roots = scan_roots(&mut cli);
            cli.reject_unknown(&[])?;
            let cams = match config::resolve_dir(cli.config.as_deref()) {
                Ok(dir) => match config::read_mode(&dir)? {
                    config::Mode::Server => {
                        profiles::load_cameras(&config::load(&dir)?.paths.cameras).profiles
                    }
                    config::Mode::Client => {
                        profiles::load_cameras(&config::load_client(&dir)?.cameras).profiles
                    }
                },
                Err(_) => Vec::new(),
            };
            let cards = discover::find_cards(&discover::volumes(&roots), &cams);
            if cards.is_empty() {
                println!("no camera card detected");
            }
            for (i, c) in cards.iter().enumerate() {
                println!("{:>2}  {}", i + 1, c.line());
            }
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            let dir = config::resolve_dir(cli.config.as_deref())?;
            match config::read_mode(&dir)? {
                config::Mode::Server => server_command(&cmd, cli, &dir),
                config::Mode::Client => client_command(&cmd, cli, &dir),
            }
        }
    }
}

fn server_command(cmd: &str, mut cli: Cli, dir: &std::path::Path) -> Result<ExitCode> {
    let cfg = config::load(dir)?;
    let debug = cli.debug || config::debug_setting(&cfg.settings, &cfg.dir.join("magvault.conf"))?;
    log::configure(&cfg.paths.logs, debug);
    match cmd {
        "ingest" => {
            let mut a = base_args(&mut cli);
            let roots = scan_roots(&mut cli);
            cli.reject_unknown(&["--yes", "-y"])?;
            let cams = profiles::load_cameras(&cfg.paths.cameras).profiles;
            a.source = source_or_card(&cli.positional, &cams, &roots, a.yes)?;
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
            let reports =
                reconcile::run(&cfg, &cli.positional, all, cli.flag(&["--verify-hashes"]))?;
            let mut bad = false;
            for r in &reports {
                let folder = util::esc(&r.folder);
                if r.integrity.is_empty() && r.rewritten.is_empty() && r.notes.is_empty() {
                    println!("ok        {folder}");
                }
                if !r.rewritten.is_empty() {
                    println!("rewritten {folder}  (database wins: {})", r.rewritten.join(", "));
                }
                for n in &r.notes {
                    println!("note      {folder}  {}", util::esc(n));
                }
                for i in &r.integrity {
                    bad = true;
                    println!("INTEGRITY {folder}  {}", util::esc(i));
                }
            }
            if reports.is_empty() {
                println!("no online mags");
            }
            Ok(if bad { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "serve" => {
            cli.reject_unknown(&["--once"])?;
            serve::run(&cfg, cli.flag(&["--once"]))?;
            Ok(ExitCode::SUCCESS)
        }
        "ui" => bail!("the Phase 1 TUI is the client dashboard (config with `mode: client`); the server dashboard comes in Phase 2"),
        "sync" | "usb" | "projects" => bail!(
            "`{cmd}` is a client command; this config directory is a server (no `mode: client`)"
        ),
        other => bail!("unknown command {other:?} (see `magvault --help`)"),
    }
}

fn client_command(cmd: &str, cli: Cli, dir: &std::path::Path) -> Result<ExitCode> {
    let cc = config::load_client(dir)?;
    log::configure(&cc.logs, cli.debug || cc.debug);
    client::start_issues(&cc);
    let r = client_command_inner(cmd, cli, &cc);
    client::finish_issues(&cc);
    r
}

fn client_command_inner(cmd: &str, mut cli: Cli, cc: &config::ClientConfig) -> Result<ExitCode> {
    let cc = cc.clone();
    match cmd {
        "ingest" => {
            let mut base = base_args(&mut cli);
            let roots = scan_roots(&mut cli);
            let via =
                client_ingest::Via::parse(&cli.take("--via").unwrap_or_else(|| "staging".into()))?;
            let set_label = cli.take("--set");
            cli.reject_unknown(&["--yes", "-y", "--mirror-set", "--no-sync"])?;
            let cams = profiles::load_cameras(&cc.cameras).profiles;
            base.source = source_or_card(&cli.positional, &cams, &roots, base.yes)?;
            let a = client_ingest::ClientArgs {
                base,
                via,
                mirror_set: cli.flag(&["--mirror-set"]),
                set_label,
                no_sync: cli.flag(&["--no-sync"]),
                scan_roots: roots,
            };
            client_ingest::run(&cc, a)?;
            Ok(ExitCode::SUCCESS)
        }
        "sync" => {
            let roots = scan_roots(&mut cli);
            let set = cli.take("--set");
            cli.reject_unknown(&[])?;
            let rep = sync::run(&cc, &roots, set.as_deref())?;
            println!(
                "{} synced, {} failed{}",
                rep.synced,
                rep.failed,
                if rep.waiting.is_empty() {
                    String::new()
                } else {
                    format!(", waiting for both drives: {}", rep.waiting.join(", "))
                }
            );
            Ok(if rep.failed > 0 || !rep.waiting.is_empty() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        "usb" => {
            let roots = scan_roots(&mut cli);
            let sub = cli
                .positional
                .first()
                .cloned()
                .unwrap_or_else(|| "status".into());
            let yes = cli.flag(&["--yes", "-y"]);
            match sub.as_str() {
                "status" => {
                    cli.reject_unknown(&[])?;
                    usb::status(&roots)?;
                }
                "prepare" => {
                    cli.reject_unknown(&[])?;
                    usb::prepare_interactive(&cc, &roots)?;
                }
                "prepare-master" => {
                    let device = cli.take("--device").ok_or_else(|| Error("prepare-master needs --device".into()))?;
                    let label = cli.take("--label").ok_or_else(|| Error("prepare-master needs --label".into()))?;
                    cli.reject_unknown(&["--yes", "-y"])?;
                    usb::prepare_master_cmd(&cc, &device, &label, yes)?;
                }
                "prepare-slave" => {
                    let device = cli.take("--device").ok_or_else(|| Error("prepare-slave needs --device".into()))?;
                    let set = cli.take("--set");
                    cli.reject_unknown(&["--yes", "-y"])?;
                    usb::prepare_slave_cmd(&cc, &device, set.as_deref(), yes, &roots)?;
                }
                "clean" => {
                    let set = cli.take("--set");
                    cli.reject_unknown(&["--yes", "-y"])?;
                    usb::clean(&cc, set.as_deref(), yes, &roots)?;
                }
                other => bail!("unknown usb command {other:?} (status, prepare, prepare-master, prepare-slave, clean)"),
            }
            Ok(ExitCode::SUCCESS)
        }
        "ui" => {
            let roots = scan_roots(&mut cli);
            cli.reject_unknown(&[])?;
            if !(std::io::IsTerminal::is_terminal(&std::io::stdin()) && std::io::IsTerminal::is_terminal(&std::io::stdout())) {
                bail!("the TUI needs a terminal; scripts use the commands (see `magvault --help`)");
            }
            tui::run(&cc, roots)?;
            Ok(ExitCode::SUCCESS)
        }
        "projects" => {
            cli.reject_unknown(&[])?;
            let t = client::Transport::new(&cc);
            for (id, name) in client::projects(&cc, t.as_ref()) {
                println!("{id}  {}", util::esc(&name));
            }
            Ok(ExitCode::SUCCESS)
        }
        "reconcile" | "serve" => {
            bail!("`{cmd}` is a server command; this config directory is a client (`mode: client`)")
        }
        other => bail!("unknown command {other:?} (see `magvault --help`)"),
    }
}

fn main() -> ExitCode {
    interrupt::default_sigpipe();
    match run() {
        Ok(code) => code,
        Err(e) => {
            // Errors can quote card file names and labels: escape control characters per line.
            let mut lines = e.0.lines();
            eprintln!("error: {}", util::esc(lines.next().unwrap_or("")));
            for l in lines {
                eprintln!("{}", util::esc(l));
            }
            ExitCode::from(1)
        }
    }
}
