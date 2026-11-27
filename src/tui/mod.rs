//! The Phase 1 terminal UI (ratatui): client dashboard, Ingest, the Prepare and Clean
//! popups. It is a view over the same code as the CLI: the screens read a snapshot from
//! `data::gather`, and Pull / Sync / Prepare / Clean call `client_ingest::run`,
//! `sync::run`, `usb::prepare_master`, `usb::prepare_slave` and `usb::clean` with the
//! operator's choices.
//!
//! Those steps run on a worker thread while the TUI stays on screen: what the CLI would
//! print and its progress (hashing, rsync, verify, server wait) arrive through the
//! `out` sink and show as a progress bar and lines. The sudo password is asked in the TUI
//! and handed to `sudo -S` once (never logged). Ctrl-C cancels a running step.

pub mod app;
pub mod data;
pub mod view;
#[cfg(test)]
mod tests;

use crate::client;
use crate::client_ingest;
use crate::config::ClientConfig;
use crate::error::Result;
use crate::fieldset;
use crate::interrupt;
use crate::log::{Level, Log};
use crate::out;
use crate::sync;
use crate::usb;
use app::{Action, App, Secret};
use data::{Preview, Snapshot};
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

enum Bg {
    Snapshot(Snapshot),
    Preview(PathBuf, std::result::Result<Preview, String>),
    /// A step ended; for Prepare master, the set label it prepared.
    Done(std::result::Result<String, String>, Option<String>),
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

/// Run a step on a worker thread. Its output reaches the TUI through the `out` sink
/// (set for the whole TUI session), so nothing is printed to the terminal.
fn spawn_job<F>(tx: &Sender<Bg>, password: Option<Secret>, prepared: Option<String>, f: F)
where
    F: FnOnce() -> Result<String> + Send + 'static,
{
    let tx = tx.clone();
    std::thread::spawn(move || {
        interrupt::reset();
        fieldset::set_sudo_password(password.map(|s| s.0));
        let r = f().map_err(|e| e.0);
        fieldset::set_sudo_password(None);
        let ok = r.is_ok();
        interrupt::reset();
        let _ = tx.send(Bg::Done(r, if ok { prepared } else { None }));
    });
}

pub fn run(cc: &ClientConfig, scan_roots: Vec<PathBuf>) -> Result<()> {
    let (tx, rx): (Sender<Bg>, Receiver<Bg>) = channel();
    let (otx, orx) = channel::<out::Msg>();
    let log = Log::new(&cc.logs);
    let mut app = App::new(Snapshot::empty(cc.staging.clone()), scan_roots.clone());
    app.refreshing = true;
    app.needs_sudo = !fieldset::is_root();
    if cc.server.is_some() {
        app.use_server = true;
    }
    // Everything the shared code prints goes to the TUI while it runs.
    out::set_sink(Some(otx));
    interrupt::install();
    spawn_refresh(&tx, cc, &scan_roots);
    let mut terminal = ratatui::init();
    let res = (|| -> Result<()> {
        loop {
            while let Ok(m) = orx.try_recv() {
                app.job_msg(m);
            }
            while let Ok(m) = rx.try_recv() {
                match m {
                    Bg::Snapshot(s) => app.set_snapshot(s),
                    Bg::Preview(p, r) => app.set_preview(p, r),
                    Bg::Done(r, prepared) => {
                        while let Ok(m) = orx.try_recv() {
                            app.job_msg(m);
                        }
                        let cancelled = app.job.as_ref().is_some_and(|j| j.cancelling);
                        app.job_done(r);
                        if let Some(label) = prepared {
                            app.set_snapshot(data::gather(cc, &scan_roots));
                            app.master_prepared(&label);
                        } else {
                            if !cancelled {
                                app.source = app.source.take().filter(|_| app.popup.is_some());
                            }
                            app.refreshing = true;
                            spawn_refresh(&tx, cc, &scan_roots);
                        }
                    }
                }
            }
            terminal.draw(|f| view::draw(f, &app))?;
            if !event::poll(Duration::from_millis(100))? {
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
                Action::Cancel => interrupt::request(),
                Action::SudoCancelled(what) => {
                    log.warn("usb", &format!("{what}: sudo cancelled by the operator; nothing written"));
                    client::queue_issue(Level::Warn, &format!("sudo cancelled: {what}"));
                }
                Action::Pull(args) => {
                    let src = args.base.source.clone();
                    let label = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    app.start_pull(&format!("Pull {label}"));
                    let cc2 = cc.clone();
                    spawn_job(&tx, None, None, move || {
                        let folder = client_ingest::run(&cc2, *args)?;
                        Ok(format!("ingested {folder}"))
                    });
                }
                Action::Sync => {
                    app.start_job("Sync pending mags");
                    let (cc2, roots) = (cc.clone(), scan_roots.clone());
                    spawn_job(&tx, None, None, move || {
                        let r = sync::run(&cc2, &roots, None)?;
                        let mut s = format!("{} synced, {} failed", r.synced, r.failed);
                        if !r.waiting.is_empty() {
                            s.push_str(&format!(", waiting for both drives: {}", r.waiting.join(", ")));
                        }
                        if r.failed > 0 {
                            return Err(crate::error::Error(s));
                        }
                        Ok(s)
                    });
                }
                Action::PrepareMaster { disk, label, password } => {
                    app.popup = None;
                    app.start_job(&format!("Prepare master of {label}"));
                    let cc2 = cc.clone();
                    let l2 = label.clone();
                    spawn_job(&tx, password, Some(label), move || {
                        usb::prepare_master(&cc2, &disk, &l2, true)?;
                        Ok(format!("master of {l2} ready"))
                    });
                }
                Action::PrepareSlave { disk, label, password } => {
                    app.popup = None;
                    app.start_job(&format!("Prepare slave of {label}"));
                    let (cc2, roots) = (cc.clone(), scan_roots.clone());
                    spawn_job(&tx, password, None, move || {
                        let master = usb::waiting_master(&roots, Some(&label))?;
                        usb::prepare_slave(&cc2, &disk, &master, true)?;
                        Ok(format!("slave of {label} ready; set complete"))
                    });
                }
                Action::Clean { label, password } => {
                    app.popup = None;
                    app.start_job(&format!("Clean {label}"));
                    let (cc2, roots) = (cc.clone(), scan_roots.clone());
                    spawn_job(&tx, password, None, move || {
                        usb::clean(&cc2, Some(&label), true, &roots)?;
                        Ok(format!("{label} cleaned; set still valid and empty"))
                    });
                }
            }
        }
    })();
    ratatui::restore();
    out::set_sink(None);
    res
}
