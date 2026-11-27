//! Client → server transport (Phase 1).
//!
//! The client only ever talks to `magvault gate` on the server (forced command for its SSH
//! key): `ping`, `projects`, upload with rsync into its own spool, `commit <folder>`,
//! `status <folder>`. The server (`magvault serve`) verifies and commits; the client marks a
//! mag synced (field set) or removes it (local staging) only after `status` says
//! `committed`.
//!
//! `server_transport: local` (test only) runs the gate as a local process and rsyncs
//! straight into the spool folder; everything else is identical.
//!
//! Server status (PLAN.md "Client ↔ server status"): the server is ready only when the
//! gate answers *and* reports `serve alive`; SSH up with serve down is "server up,
//! committer down". Ingest and sync wait for a committer that is down for at most
//! `committer_wait` seconds, then stop with a clear message (the mag stays pending).
//!
//! Client issues (PLAN.md "Client issues on the server log"): operational problems and,
//! with debug on, DEBUG lines are queued in `<config>/issues.pending` and sent with the
//! gate's `log` verb when the gate answers. Bounded, identical lines rate-limited.

use crate::config::{ClientConfig, ServerLink};
use crate::error::{Context, Error, Result};
use crate::ingest;
use crate::interrupt;
use crate::log::Level;
use crate::textfile::atomic_write;
use crate::util::esc;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Transport {
    link: ServerLink,
}

/// What `ping` found when the gate answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerStatus {
    /// Gate up and `magvault serve` alive: commits are accepted.
    Ready,
    /// Gate up, committer down (the reason from the heartbeat).
    CommitterDown(String),
}

pub const COMMITTER_DOWN: &str = "server up, committer down";

const SSH_OPTS: &[&str] = &[
    "-o",
    "BatchMode=yes",
    "-o",
    "ConnectTimeout=10",
    "-o",
    "StrictHostKeyChecking=yes",
];

impl Transport {
    pub fn new(cc: &ClientConfig) -> Option<Transport> {
        cc.server.clone().map(|link| Transport { link })
    }

    pub fn describe(&self) -> String {
        match &self.link {
            ServerLink::Ssh { user, host, .. } => format!("{user}@{host}"),
            ServerLink::Local { label, .. } => format!("local test gate ({label})"),
        }
    }

    fn ssh_base(user: &str, host: &str, key: &Option<PathBuf>) -> Vec<String> {
        let mut v: Vec<String> = SSH_OPTS.iter().map(|s| s.to_string()).collect();
        if let Some(k) = key {
            v.push("-i".into());
            v.push(k.display().to_string());
        }
        v.push(format!("{user}@{host}"));
        v
    }

    /// Run one gate command; stdout on success.
    pub fn call(&self, cmd: &str) -> Result<String> {
        self.call_input(cmd, None)
    }

    /// Run one gate command with `input` on its stdin (the `log` verb).
    pub fn call_input(&self, cmd: &str, input: Option<&str>) -> Result<String> {
        let mut c = match &self.link {
            ServerLink::Ssh { user, host, key } => {
                let mut c = Command::new("ssh");
                c.args(Self::ssh_base(user, host, key)).arg(cmd);
                c
            }
            ServerLink::Local {
                spool,
                results,
                label,
            } => {
                let mut c =
                    Command::new(std::env::current_exe().ctx("cannot find the magvault binary")?);
                c.arg("gate")
                    .arg("--spool")
                    .arg(spool)
                    .arg("--results")
                    .arg(results)
                    .arg("--client")
                    .arg(label);
                c.env("SSH_ORIGINAL_COMMAND", cmd);
                c
            }
        };
        let verb = cmd.split(' ').next().unwrap_or("");
        if verb != "log" {
            crate::log::dbg("gate", &format!("call {verb}"));
        }
        let mut child = c
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ctx("cannot reach the server")?;
        if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let out = child.wait_with_output().ctx("cannot reach the server")?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            crate::bail!("server gate refused `{verb}`: {}", esc(err.trim()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// `Err`: the gate cannot be reached (offline). Otherwise ready or committer down.
    pub fn ping(&self) -> Result<ServerStatus> {
        let out = match self.call("ping") {
            Ok(o) => o,
            Err(e) => {
                queue_issue(Level::Warn, &format!("cannot reach the server: {}", first_line(&e.0)));
                return Err(e);
            }
        };
        let mut lines = out.lines();
        if !lines.next().unwrap_or("").starts_with("magvault-gate ok") {
            crate::bail!("unexpected answer from the server: {}", esc(&out));
        }
        let st = parse_serve(lines.next().unwrap_or(""));
        if let ServerStatus::CommitterDown(why) = &st {
            queue_issue(Level::Warn, &format!("{COMMITTER_DOWN} ({why})"));
        }
        flush_issues(self);
        Ok(st)
    }

    /// Ready, or wait up to `wait` seconds for a committer that is down. `what` names the
    /// waiting step for the messages ("ingest", "sync").
    pub fn wait_ready(&self, wait: u64, what: &str) -> Result<()> {
        let start = Instant::now();
        let mut said = false;
        loop {
            match self.ping()? {
                ServerStatus::Ready => {
                    if said {
                        crate::say!("server committer is back; continuing");
                    }
                    return Ok(());
                }
                ServerStatus::CommitterDown(why) => {
                    if !said {
                        crate::say!("WARNING: {COMMITTER_DOWN} ({why}); {what} waits up to {wait} s");
                        queue_issue(Level::Warn, &format!("{COMMITTER_DOWN}; {what} waiting"));
                        said = true;
                    }
                    if start.elapsed() >= Duration::from_secs(wait) {
                        let msg = format!(
                            "{COMMITTER_DOWN} for {wait} s ({why}); {what} stopped waiting, the mag stays pending here (run `magvault sync` once `magvault serve` runs on the server)"
                        );
                        queue_issue(Level::Error, &format!("timeout: {COMMITTER_DOWN} for {wait} s; {what} gave up, mag pending"));
                        crate::bail!("{msg}");
                    }
                }
            }
            sleep_checked(Duration::from_secs(wait.clamp(1, 5)))?;
        }
    }

    /// Open projects on the server: (uuid, name).
    pub fn projects(&self) -> Result<Vec<(String, String)>> {
        Ok(parse_projects(&self.call("projects")?))
    }

    /// rsync `rels` (relative to `src_root`) into the spool folder `<folder>/` on the server.
    pub fn upload(
        &self,
        src_root: &Path,
        rels: &[&str],
        folder: &str,
        list_file: &Path,
    ) -> Result<()> {
        match &self.link {
            ServerLink::Ssh { user, host, key } => {
                let mut e = String::from("ssh");
                for o in SSH_OPTS {
                    e.push(' ');
                    e.push_str(o);
                }
                if let Some(k) = key {
                    e.push_str(&format!(" -i {}", k.display()));
                }
                ingest::rsync_to(
                    src_root,
                    rels,
                    &format!("{user}@{host}:{folder}/"),
                    list_file,
                    &["-e".into(), e],
                )
            }
            ServerLink::Local { spool, label, .. } => {
                let dest = spool.join(label).join(folder);
                std::fs::create_dir_all(&dest).ctx(format!("cannot create {}", dest.display()))?;
                ingest::rsync_copy(src_root, rels, &dest, list_file)
            }
        }
    }

    pub fn commit(&self, folder: &str) -> Result<()> {
        let out = self.call(&format!("commit {folder}"))?;
        if out != "queued" {
            crate::bail!("server did not queue {folder}: {}", esc(&out));
        }
        Ok(())
    }

    /// The commit state of `folder` and the committer state.
    pub fn status(&self, folder: &str) -> Result<(String, ServerStatus)> {
        let out = self.call(&format!("status {folder}"))?;
        let mut lines = out.lines();
        let st = lines.next().unwrap_or("").trim().to_string();
        Ok((st, parse_serve(lines.next().unwrap_or(""))))
    }

    /// Wait until the server has verified and committed `folder`. Ctrl+C stops waiting (the
    /// mag stays pending and is picked up by the next sync). A committer that stays down
    /// for `wait` seconds ends the wait with a clear message.
    pub fn wait_committed(&self, folder: &str, wait: u64) -> Result<()> {
        let start = Instant::now();
        let mut last_note = Instant::now();
        let mut down_since: Option<Instant> = None;
        loop {
            interrupt::check()?;
            let (st, serve) = self.status(folder)?;
            match st.as_str() {
                "committed" => return Ok(()),
                "queued" | "verifying" => {}
                s if s.starts_with("error") => {
                    crate::bail!("the server refused {folder}: {}", esc(s))
                }
                other => crate::bail!(
                    "the server has no commit request for {folder} ({})",
                    esc(other)
                ),
            }
            match &serve {
                ServerStatus::Ready => down_since = None,
                ServerStatus::CommitterDown(why) => {
                    let since = *down_since.get_or_insert_with(|| {
                        crate::say!("WARNING: {COMMITTER_DOWN} ({why}); waiting up to {wait} s for the commit of {folder}");
                        queue_issue(Level::Warn, &format!("{COMMITTER_DOWN}; commit of {folder} waiting"));
                        Instant::now()
                    });
                    if since.elapsed() >= Duration::from_secs(wait) {
                        queue_issue(Level::Error, &format!("timeout: {COMMITTER_DOWN} for {wait} s; {folder} uploaded but not committed"));
                        crate::bail!(
                            "{COMMITTER_DOWN} for {wait} s ({why}); {folder} is uploaded and queued on the server but not committed; it stays pending here (run `magvault sync` once `magvault serve` runs)"
                        );
                    }
                }
            }
            if start.elapsed() > Duration::from_secs(6 * 3600) {
                crate::bail!(
                    "no answer from the server for {folder} after 6 hours; it stays pending"
                );
            }
            if last_note.elapsed() > Duration::from_secs(30) {
                crate::say!("waiting for the server to verify {folder} ({st}) …");
                last_note = Instant::now();
            }
            crate::out::send(crate::out::Msg::Progress { label: format!("server {st}"), done: 0, total: 0 });
            sleep_checked(Duration::from_secs(2))?;
        }
    }
}

fn sleep_checked(d: Duration) -> Result<()> {
    let end = Instant::now() + d;
    while Instant::now() < end {
        interrupt::check()?;
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

/// The gate's committer line: `serve alive` or `serve down: …` (an old gate without the
/// line counts as down: nobody vouches for the committer).
pub fn parse_serve(line: &str) -> ServerStatus {
    let l = line.trim();
    if l == "serve alive" {
        ServerStatus::Ready
    } else if let Some(why) = l.strip_prefix("serve down: ") {
        ServerStatus::CommitterDown(why.to_string())
    } else {
        ServerStatus::CommitterDown("the server does not report its committer".into())
    }
}

// ---- client issues → server log ----------------------------------------------------------

const ISSUES_FILE: &str = "issues.pending";
const ISSUES_MAX_LINES: usize = 200;
const SAME_LINE_SECONDS: u64 = 60;

struct Issues {
    path: PathBuf,
    recent: std::collections::HashMap<String, Instant>,
    dropped: usize,
}

static ISSUES: std::sync::Mutex<Option<Issues>> = std::sync::Mutex::new(None);

/// Start the client-issue queue for this run; DEBUG lines are queued too (log forwarder).
pub fn start_issues(cc: &ClientConfig) {
    *ISSUES.lock().unwrap() = Some(Issues {
        path: cc.dir.join(ISSUES_FILE),
        recent: Default::default(),
        dropped: 0,
    });
    crate::log::set_forward(Some(|level, module, msg| queue_issue(level, &format!("{module}: {msg}"))));
}

/// Queue an operational problem (or a DEBUG line) for the server log. Never secrets: the
/// callers pass Magvault's own messages; the server cleans each line again.
pub fn queue_issue(level: Level, msg: &str) {
    let mut g = ISSUES.lock().unwrap();
    let Some(q) = g.as_mut() else { return };
    let key = format!("{} {msg}", level.as_str());
    if q.recent.get(&key).is_some_and(|t| t.elapsed() < Duration::from_secs(SAME_LINE_SECONDS)) {
        return;
    }
    q.recent.insert(key, Instant::now());
    let existing = std::fs::read_to_string(&q.path).map(|t| t.lines().count()).unwrap_or(0);
    if existing >= ISSUES_MAX_LINES {
        q.dropped += 1;
        return;
    }
    let line = format!(
        "{} [{}] {}\n",
        level.as_str(),
        crate::time::iso_utc(crate::time::now_secs()),
        crate::gate::clean_line(msg)
    );
    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&q.path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
}

/// Send queued lines with the gate's `log` verb (best effort; kept on failure).
pub fn flush_issues(t: &Transport) {
    let (path, dropped) = {
        let mut g = ISSUES.lock().unwrap();
        let Some(q) = g.as_mut() else { return };
        (q.path.clone(), std::mem::take(&mut q.dropped))
    };
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if dropped > 0 {
        text.push_str(&format!("WARN {dropped} more client line(s) dropped (client queue full)\n"));
    }
    if text.trim().is_empty() {
        return;
    }
    let sending = path.with_extension("sending");
    if std::fs::rename(&path, &sending).is_err() && dropped == 0 {
        return;
    }
    // Not through `call` (its DEBUG line would queue another line while sending).
    let ok = t.call_input("log", Some(&text)).is_ok();
    if ok {
        let _ = std::fs::remove_file(&sending);
    } else {
        // Put the lines back in front of anything queued meanwhile.
        let newer = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::write(&path, format!("{}{newer}", std::fs::read_to_string(&sending).unwrap_or_default()));
        let _ = std::fs::remove_file(&sending);
    }
}

/// End of a client command: send what is still queued if the server answers.
pub fn finish_issues(cc: &ClientConfig) {
    let has = std::fs::metadata(cc.dir.join(ISSUES_FILE)).map(|m| m.len() > 0).unwrap_or(false);
    if has {
        if let Some(t) = Transport::new(cc) {
            // Without the forwarder: a failed send must not queue more lines.
            crate::log::set_forward(None);
            flush_issues(&t);
        }
    }
}

pub fn parse_projects(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| l.split_once(" | "))
        .filter(|(id, name)| crate::util::uuid_ok(id) && !name.is_empty())
        .map(|(id, name)| (id.to_string(), name.to_string()))
        .collect()
}

fn cache_path(cc: &ClientConfig) -> PathBuf {
    cc.dir.join("projects.cache")
}

fn local_path(cc: &ClientConfig) -> PathBuf {
    cc.dir.join("projects.local")
}

/// Open projects: from the server when reachable (and cached), else from the cache, plus the
/// projects this client created itself that the server does not list yet (`projects.local`).
/// A local project is dropped once the server lists a project with that name: the server's
/// project wins (the server merges the client's mags into it on commit).
pub fn projects(cc: &ClientConfig, t: Option<&Transport>) -> Vec<(String, String)> {
    let mut server = None;
    if let Some(t) = t {
        if let Ok(p) = t.projects() {
            let text: String = p.iter().map(|(id, n)| format!("{id} | {n}\n")).collect();
            let _ = atomic_write(&cache_path(cc), text.as_bytes());
            server = Some(p);
        }
    }
    let online = server.is_some();
    let mut all = server.unwrap_or_else(|| {
        std::fs::read_to_string(cache_path(cc))
            .map(|t| parse_projects(&t))
            .unwrap_or_default()
    });
    let local = std::fs::read_to_string(local_path(cc))
        .map(|t| parse_projects(&t))
        .unwrap_or_default();
    let keep: Vec<(String, String)> = local
        .into_iter()
        .filter(|(id, n)| !all.iter().any(|(i, m)| i == id || m == n))
        .collect();
    if online {
        let text: String = keep.iter().map(|(id, n)| format!("{id} | {n}\n")).collect();
        let _ = atomic_write(&local_path(cc), text.as_bytes());
    }
    all.extend(keep);
    all
}

/// Remember a project this client created, so the next ingest (online or not) reuses its UUID
/// instead of creating another project with the same name.
pub fn remember_local_project(cc: &ClientConfig, project: &ingest::ProjectChoice) -> Result<()> {
    if let ingest::ProjectChoice::New { id, name } = project {
        let mut text = std::fs::read_to_string(local_path(cc)).unwrap_or_default();
        if !parse_projects(&text).iter().any(|(i, _)| i == id) {
            text.push_str(&format!("{id} | {name}\n"));
            atomic_write(&local_path(cc), text.as_bytes())?;
        }
    }
    Ok(())
}

/// Client project choice: `--project-id` must be a known open project; `--project` reuses the
/// open project with that name or creates a new one (UUID now; the server creates it on
/// commit).
pub fn choose_project(
    a: &ingest::Args,
    known: &[(String, String)],
) -> Result<ingest::ProjectChoice> {
    if let Some(id) = &a.project_id {
        let (id, name) = known
            .iter()
            .find(|(i, _)| i == id)
            .ok_or_else(|| Error(format!("project {id} is not an open project known to this client (connect once to refresh)")))?;
        return Ok(ingest::ProjectChoice::Existing {
            id: id.clone(),
            name: name.clone(),
            status: "open".into(),
        });
    }
    let name = ingest::project_name_arg(a)?;
    let rows: Vec<(String, String, String)> = known
        .iter()
        .filter(|(_, n)| *n == name)
        .map(|(i, n)| (i.clone(), n.clone(), "open".into()))
        .collect();
    ingest::pick_project(&name, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committer_line_decides_ready() {
        assert_eq!(parse_serve("serve alive"), ServerStatus::Ready);
        assert_eq!(
            parse_serve("serve down: no heartbeat for 40 s"),
            ServerStatus::CommitterDown("no heartbeat for 40 s".into())
        );
        // An answer without the committer line is never "ready".
        assert!(matches!(parse_serve(""), ServerStatus::CommitterDown(_)));
    }

    #[test]
    fn rsync_progress2_lines() {
        assert_eq!(ingest::parse_progress2("     1,000,000  50%    1.00MB/s    0:00:01 (xfr#1, to-chk=1/2)"), Some((1_000_000, 2_000_000)));
        assert_eq!(ingest::parse_progress2("        32,768 100%  1.00MB/s"), Some((32_768, 32_768)));
        assert_eq!(ingest::parse_progress2("sending incremental file list"), None);
    }

    #[test]
    fn debug_setting_yes_no() {
        let f = Path::new("magvault.conf");
        let m = |v: &str| std::collections::BTreeMap::from([("debug".to_string(), v.to_string())]);
        assert!(!crate::config::debug_setting(&Default::default(), f).unwrap());
        assert!(crate::config::debug_setting(&m("yes"), f).unwrap());
        assert!(!crate::config::debug_setting(&m("no"), f).unwrap());
        assert!(crate::config::debug_setting(&m("maybe"), f).is_err());
    }
}
