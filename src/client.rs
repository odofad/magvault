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

use crate::config::{ClientConfig, ServerLink};
use crate::error::{Context, Error, Result};
use crate::ingest;
use crate::interrupt;
use crate::textfile::atomic_write;
use crate::util::esc;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Transport {
    link: ServerLink,
}

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
        let out = c
            .stdin(Stdio::null())
            .output()
            .ctx("cannot reach the server")?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            crate::bail!(
                "server gate refused `{}`: {}",
                cmd.split(' ').next().unwrap_or(""),
                esc(err.trim())
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    pub fn ping(&self) -> Result<()> {
        let out = self.call("ping")?;
        if !out.starts_with("magvault-gate ok") {
            crate::bail!("unexpected answer from the server: {}", esc(&out));
        }
        Ok(())
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

    pub fn status(&self, folder: &str) -> Result<String> {
        self.call(&format!("status {folder}"))
    }

    /// Wait until the server has verified and committed `folder`. Ctrl+C stops waiting (the
    /// mag stays pending and is picked up by the next sync).
    pub fn wait_committed(&self, folder: &str) -> Result<()> {
        let start = Instant::now();
        let mut last_note = Instant::now();
        loop {
            interrupt::check()?;
            let st = self.status(folder)?;
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
            if start.elapsed() > Duration::from_secs(6 * 3600) {
                crate::bail!(
                    "no answer from the server for {folder} after 6 hours; it stays pending"
                );
            }
            if last_note.elapsed() > Duration::from_secs(30) {
                eprintln!("waiting for the server to verify {folder} ({st}) …");
                last_note = Instant::now();
            }
            for _ in 0..20 {
                if interrupt::requested() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
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
