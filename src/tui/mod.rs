//! The Phase 1 terminal UI (ratatui): client dashboard, Ingest, the Prepare and Clean
//! popups. It is a view over the same code as the CLI: the screens read a snapshot from
//! `data::gather`, and Pull / Prepare / Clean call `client_ingest::run`,
//! `usb::prepare_master`, `usb::prepare_slave` and `usb::clean` with the operator's
//! choices. For those steps the TUI steps aside: the normal terminal shows the copy
//! progress and sudo's own password prompt (the password never passes through Magvault),
//! then Enter returns to the TUI.

pub mod app;
pub mod data;
pub mod view;
#[cfg(test)]
mod tests;

use crate::client_ingest;
use crate::config::ClientConfig;
use crate::error::Result;
use crate::interrupt;
use crate::usb;
use app::{Action, App, Popup};
use data::{Preview, Snapshot};
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

enum Bg {
    Snapshot(Snapshot),
    Preview(PathBuf, std::result::Result<Preview, String>),
}

fn spawn_refresh(tx: &Sender<Bg>, cc: &ClientConfig, roots: &[PathBuf]) {
    let (tx, cc, roots) = (tx.clone(), cc.clone(), roots.to_vec());
    std::thread::spawn(move || {
        let _ = tx.send(Bg::Snapshot(data::gather(&cc, &roots)));
    });
}

fn spawn_probe(tx: &Sender<Bg>, path: PathBuf, cameras: Vec<crate::profiles::CameraProfile>) {
    let tx = tx.clone();
    std::thread::spawn(move || {
        let r = data::preview(&path, &cameras);
        let _ = tx.send(Bg::Preview(path, r));
    });
}

/// Leave the TUI for a step that talks to the operator on the plain terminal (copy
/// progress, sudo prompt), then wait for Enter and come back.
fn step_aside<T>(terminal: &mut ratatui::DefaultTerminal, banner: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    ratatui::restore();
    println!();
    println!("── {banner}");
    interrupt::reset();
    let r = f();
    if let Err(e) = &r {
        eprintln!("error: {}", e.0);
    }
    interrupt::reset();
    print!("\nPress Enter to return to Magvault ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    *terminal = ratatui::init();
    let _ = terminal.clear();
    r
}

pub fn run(cc: &ClientConfig, scan_roots: Vec<PathBuf>) -> Result<()> {
    let (tx, rx): (Sender<Bg>, Receiver<Bg>) = channel();
    let mut app = App::new(Snapshot::empty(cc.staging.clone()), scan_roots.clone());
    app.refreshing = true;
    if cc.server.is_some() {
        app.use_server = true;
    }
    spawn_refresh(&tx, cc, &scan_roots);
    let mut terminal = ratatui::init();
    let res = (|| -> Result<()> {
        loop {
            terminal.draw(|f| view::draw(f, &app))?;
            while let Ok(m) = rx.try_recv() {
                match m {
                    Bg::Snapshot(s) => app.set_snapshot(s),
                    Bg::Preview(p, r) => app.set_preview(p, r),
                }
            }
            if !event::poll(Duration::from_millis(200))? {
                continue;
            }
            let Event::Key(k) = event::read()? else { continue };
            if k.kind != KeyEventKind::Press {
                continue;
            }
            let Some(action) = app.on_key(k) else { continue };
            match action {
                Action::Quit => return Ok(()),
                Action::Refresh => {
                    app.refreshing = true;
                    spawn_refresh(&tx, cc, &scan_roots);
                }
                Action::Probe(path) => spawn_probe(&tx, path, app.snap.cameras.clone()),
                Action::Pull(args) => {
                    let src = args.base.source.display().to_string();
                    app.log(format!("pull started    {src}"));
                    match step_aside(&mut terminal, &format!("Ingest {src} (Ctrl-C aborts; nothing is committed)"), || {
                        client_ingest::run(cc, *args)
                    }) {
                        Ok(folder) => {
                            app.log(format!("ingested        {folder}"));
                            app.source = None;
                            app.preview = None;
                        }
                        Err(e) => {
                            app.log(format!("pull failed     {}", e.0));
                            app.popup = Some(Popup::Message(vec!["Pull failed:".into(), e.0]));
                        }
                    }
                    app.set_snapshot(data::gather(cc, &scan_roots));
                }
                Action::PrepareMaster { disk, label } => {
                    let r = step_aside(&mut terminal, &format!("Prepare master of {label} on {}", disk.path), || {
                        usb::prepare_master(cc, &disk, &label, true)
                    });
                    app.set_snapshot(data::gather(cc, &scan_roots));
                    match r {
                        Ok(_) => {
                            app.log(format!("prepared        master of {label} on {}", disk.path));
                            app.master_prepared(&label);
                        }
                        Err(e) => app.popup = Some(Popup::Message(vec!["Prepare failed (nothing written unless sudo was accepted):".into(), e.0])),
                    }
                }
                Action::PrepareSlave { disk, label } => {
                    let r = step_aside(&mut terminal, &format!("Prepare slave of {label} on {}", disk.path), || {
                        let master = usb::waiting_master(&scan_roots, Some(&label))?;
                        usb::prepare_slave(cc, &disk, &master, true)
                    });
                    app.set_snapshot(data::gather(cc, &scan_roots));
                    match r {
                        Ok(_) => {
                            app.popup = None;
                            app.log(format!("prepared        slave of {label} on {}; set complete", disk.path));
                        }
                        Err(e) => app.popup = Some(Popup::Message(vec!["Prepare failed:".into(), e.0])),
                    }
                }
                Action::Clean { label } => {
                    let r = step_aside(&mut terminal, &format!("Clean field set {label}"), || {
                        usb::clean(cc, Some(&label), true, &scan_roots)
                    });
                    app.set_snapshot(data::gather(cc, &scan_roots));
                    match r {
                        Ok(()) => app.log(format!("cleaned         {label}; set still valid and empty")),
                        Err(e) => app.popup = Some(Popup::Message(vec!["Clean failed:".into(), e.0])),
                    }
                }
            }
        }
    })();
    ratatui::restore();
    res
}
