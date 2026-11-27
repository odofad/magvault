//! Minimal SQLite access through the C API (system libsqlite3) and the Magvault schema.
//!
//! Schema: PLAN.md "SQLite schema" as refined by docs/technical-review-and-spec.md §7.1
//! and PLAN.md "Implementation defaults" (camera is a profile id or empty, no `archived`
//! status, cold-pull requests link to their export, relative paths only).

use crate::error::{Error, Result};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::ptr;

#[allow(non_camel_case_types)]
type sqlite3 = c_void;
#[allow(non_camel_case_types)]
type sqlite3_stmt = c_void;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_NULL: c_int = 5;
const SQLITE_TRANSIENT: isize = -1;

#[link(name = "sqlite3")]
extern "C" {
    fn sqlite3_open_v2(filename: *const c_char, db: *mut *mut sqlite3, flags: c_int, vfs: *const c_char) -> c_int;
    fn sqlite3_close_v2(db: *mut sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_busy_timeout(db: *mut sqlite3, ms: c_int) -> c_int;
    fn sqlite3_exec(
        db: *mut sqlite3,
        sql: *const c_char,
        cb: *const c_void,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(p: *mut c_void);
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        sql: *const c_char,
        n: c_int,
        stmt: *mut *mut sqlite3_stmt,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_bind_text(stmt: *mut sqlite3_stmt, i: c_int, v: *const c_char, n: c_int, destructor: isize) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut sqlite3_stmt, i: c_int, v: i64) -> c_int;
    fn sqlite3_bind_null(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_count(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_type(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_column_int64(stmt: *mut sqlite3_stmt, i: c_int) -> i64;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, i: c_int) -> *const u8;
    fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_changes(db: *mut sqlite3) -> c_int;
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Text(String),
}

impl Value {
    pub fn text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Text(s) => s.parse().ok(),
            Value::Null => None,
        }
    }
    /// Text value or empty string.
    pub fn s(&self) -> String {
        match self {
            Value::Text(s) => s.clone(),
            Value::Int(i) => i.to_string(),
            Value::Null => String::new(),
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_string())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}
impl From<&String> for Value {
    fn from(s: &String) -> Self {
        Value::Text(s.clone())
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}
impl From<u64> for Value {
    fn from(i: u64) -> Self {
        Value::Int(i as i64)
    }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Int(b as i64)
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(o: Option<T>) -> Self {
        o.map(Into::into).unwrap_or(Value::Null)
    }
}

pub type Row = Vec<Value>;

pub struct Db {
    raw: *mut sqlite3,
}

impl Drop for Db {
    fn drop(&mut self) {
        // SAFETY: raw came from sqlite3_open_v2 and is closed exactly once.
        unsafe {
            sqlite3_close_v2(self.raw);
        }
    }
}

fn cstring(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| Error("text contains a NUL byte".into()))
}

impl Db {
    pub fn open(path: &Path) -> Result<Db> {
        let p = cstring(&path.to_string_lossy())?;
        let mut raw = ptr::null_mut();
        // SAFETY: valid C string, out-pointer to our local.
        let rc = unsafe { sqlite3_open_v2(p.as_ptr(), &mut raw, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE, ptr::null()) };
        let db = Db { raw };
        if rc != SQLITE_OK {
            return Err(Error(format!("cannot open database {}: {}", path.display(), db.errmsg())));
        }
        // SAFETY: open handle.
        unsafe {
            sqlite3_busy_timeout(raw, 5000);
        }
        db.exec("PRAGMA foreign_keys = ON;")?;
        Ok(db)
    }

    fn errmsg(&self) -> String {
        if self.raw.is_null() {
            return "out of memory".into();
        }
        // SAFETY: sqlite returns a NUL-terminated string owned by the connection.
        unsafe { CStr::from_ptr(sqlite3_errmsg(self.raw)).to_string_lossy().into_owned() }
    }

    /// Run one or more statements without parameters.
    pub fn exec(&self, sql: &str) -> Result<()> {
        let c = cstring(sql)?;
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: valid handle and C string; err is freed with sqlite3_free.
        let rc = unsafe { sqlite3_exec(self.raw, c.as_ptr(), ptr::null(), ptr::null_mut(), &mut err) };
        if rc != SQLITE_OK {
            let msg = if err.is_null() {
                self.errmsg()
            } else {
                // SAFETY: err points to a sqlite-allocated C string.
                let m = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
                unsafe { sqlite3_free(err as *mut c_void) };
                m
            };
            return Err(Error(format!("database error: {msg}")));
        }
        Ok(())
    }

    fn run(&self, sql: &str, params: &[Value], mut on_row: impl FnMut(Row)) -> Result<usize> {
        let c = cstring(sql)?;
        let mut stmt = ptr::null_mut();
        // SAFETY: valid handle and SQL; stmt finalized below on every path.
        let rc = unsafe { sqlite3_prepare_v2(self.raw, c.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(Error(format!("database error: {} (in: {})", self.errmsg(), first_line(sql))));
        }
        let result = (|| {
            for (i, p) in params.iter().enumerate() {
                let idx = (i + 1) as c_int;
                // SAFETY: stmt is a prepared statement; text is copied (TRANSIENT).
                let rc = unsafe {
                    match p {
                        Value::Null => sqlite3_bind_null(stmt, idx),
                        Value::Int(v) => sqlite3_bind_int64(stmt, idx, *v),
                        Value::Text(s) => sqlite3_bind_text(
                            stmt,
                            idx,
                            s.as_ptr() as *const c_char,
                            s.len() as c_int,
                            SQLITE_TRANSIENT,
                        ),
                    }
                };
                if rc != SQLITE_OK {
                    return Err(Error(format!("database bind error: {}", self.errmsg())));
                }
            }
            loop {
                // SAFETY: stepping a prepared statement.
                let rc = unsafe { sqlite3_step(stmt) };
                match rc {
                    SQLITE_ROW => {
                        let n = unsafe { sqlite3_column_count(stmt) };
                        let mut row = Vec::with_capacity(n as usize);
                        for i in 0..n {
                            // SAFETY: i < column count; text pointer valid until next step.
                            let v = unsafe {
                                match sqlite3_column_type(stmt, i) {
                                    SQLITE_NULL => Value::Null,
                                    SQLITE_INTEGER => Value::Int(sqlite3_column_int64(stmt, i)),
                                    _ => {
                                        let p = sqlite3_column_text(stmt, i);
                                        let len = sqlite3_column_bytes(stmt, i) as usize;
                                        if p.is_null() {
                                            Value::Null
                                        } else {
                                            let bytes = std::slice::from_raw_parts(p, len);
                                            Value::Text(String::from_utf8_lossy(bytes).into_owned())
                                        }
                                    }
                                }
                            };
                            row.push(v);
                        }
                        on_row(row);
                    }
                    SQLITE_DONE => break,
                    _ => return Err(Error(format!("database error: {} (in: {})", self.errmsg(), first_line(sql)))),
                }
            }
            Ok(unsafe { sqlite3_changes(self.raw) } as usize)
        })();
        // SAFETY: finalize the statement exactly once.
        unsafe {
            sqlite3_finalize(stmt);
        }
        result
    }

    /// Execute a statement with parameters; returns the number of changed rows.
    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<usize> {
        self.run(sql, params, |_| {})
    }

    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>> {
        let mut rows = Vec::new();
        self.run(sql, params, |r| rows.push(r))?;
        Ok(rows)
    }

    pub fn query_one(&self, sql: &str, params: &[Value]) -> Result<Option<Row>> {
        Ok(self.query(sql, params)?.into_iter().next())
    }

    pub fn begin(&self) -> Result<()> {
        self.exec("BEGIN IMMEDIATE;")
    }
    pub fn commit(&self) -> Result<()> {
        self.exec("COMMIT;")
    }
    pub fn rollback(&self) {
        let _ = self.exec("ROLLBACK;");
    }

    /// Bump and return the catalog change sequence (used later for client sync).
    pub fn next_seq(&self) -> Result<i64> {
        self.execute("UPDATE meta SET value = CAST(value AS INTEGER) + 1 WHERE key = 'catalog_seq'", &[])?;
        let row = self.query_one("SELECT value FROM meta WHERE key = 'catalog_seq'", &[])?;
        row.and_then(|r| r[0].int()).ok_or_else(|| Error("meta.catalog_seq missing".into()))
    }
}

fn first_line(s: &str) -> &str {
    s.trim().lines().next().unwrap_or("")
}

pub const SCHEMA_VERSION: i64 = 2;

/// Create tables if missing. Safe to run on every start.
pub fn ensure_schema(db: &Db) -> Result<()> {
    db.exec("PRAGMA journal_mode = WAL;")?;
    db.exec(SCHEMA)?;
    db.execute("INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', ?1)", &[SCHEMA_VERSION.to_string().into()])?;
    db.execute("INSERT OR IGNORE INTO meta(key, value) VALUES ('catalog_seq', '0')", &[])?;
    let mut v = db
        .query_one("SELECT value FROM meta WHERE key = 'schema_version'", &[])?
        .and_then(|r| r[0].int())
        .unwrap_or(0);
    if v == 1 {
        // 1 → 2: project requirements and the project log (PLAN.md "Project requirements
        // warning"). project_log comes from SCHEMA above.
        db.begin()?;
        let r = db
            .exec("ALTER TABLE projects ADD COLUMN requirements TEXT;")
            .and_then(|_| db.execute("UPDATE meta SET value = '2' WHERE key = 'schema_version'", &[]).map(|_| ()));
        match r {
            Ok(()) => db.commit()?,
            Err(e) => {
                db.rollback();
                return Err(e);
            }
        }
        v = 2;
    }
    if v != SCHEMA_VERSION {
        crate::bail!("database schema version {v} is not supported by this build (expects {SCHEMA_VERSION})");
    }
    Ok(())
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
  id              TEXT PRIMARY KEY,
  name            TEXT NOT NULL,
  season_or_year  TEXT,
  notes           TEXT,
  status          TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','closed')),
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  seq             INTEGER NOT NULL DEFAULT 0,
  requirements    TEXT
);

CREATE TABLE IF NOT EXISTS project_log (
  id          INTEGER PRIMARY KEY,
  project_id  TEXT NOT NULL REFERENCES projects(id),
  mag_id      TEXT,
  at          TEXT NOT NULL,
  level       TEXT NOT NULL CHECK (level IN ('INFO','WARN','ERROR')),
  message     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mags (
  id                TEXT PRIMARY KEY,
  project_id        TEXT NOT NULL REFERENCES projects(id),
  folder            TEXT NOT NULL UNIQUE,
  shoot_date        TEXT NOT NULL,
  camera_profile_id TEXT,
  file_format       TEXT NOT NULL,
  gamma             TEXT NOT NULL,
  gamut             TEXT NOT NULL,
  status            TEXT NOT NULL CHECK (status IN ('ingesting','online','cold','pulling','failed')),
  logging_complete  INTEGER NOT NULL DEFAULT 0,
  size_bytes        INTEGER NOT NULL,
  file_count        INTEGER NOT NULL,
  source_label      TEXT,
  ingested_by       TEXT NOT NULL,
  ingested_at       TEXT NOT NULL,
  notes             TEXT,
  updated_at        TEXT NOT NULL,
  seq               INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS clips (
  id                TEXT PRIMARY KEY,
  mag_id            TEXT NOT NULL REFERENCES mags(id),
  relative_path     TEXT NOT NULL,
  original_filename TEXT NOT NULL,
  size              INTEGER NOT NULL,
  checksum          TEXT NOT NULL,
  duration_ms       INTEGER,
  fps               TEXT,
  width             INTEGER,
  height            INTEGER,
  audio_streams     INTEGER,
  audio_channels    TEXT,
  proxy_status      TEXT NOT NULL DEFAULT 'none' CHECK (proxy_status IN ('none','pending','ready','failed')),
  hq_status         TEXT NOT NULL DEFAULT 'none' CHECK (hq_status IN ('none','pending','ready','failed')),
  description       TEXT,
  rating            INTEGER CHECK (rating BETWEEN 1 AND 5),
  slow_motion       INTEGER NOT NULL DEFAULT 0,
  technical_note    TEXT,
  shot_type         TEXT,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  seq               INTEGER NOT NULL DEFAULT 0,
  UNIQUE (mag_id, relative_path)
);

CREATE TABLE IF NOT EXISTS files (
  mag_id         TEXT NOT NULL REFERENCES mags(id),
  relative_path  TEXT NOT NULL,
  size           INTEGER NOT NULL,
  mtime          TEXT NOT NULL,
  sha256         TEXT NOT NULL,
  role           TEXT NOT NULL CHECK (role IN ('media','sidecar','other')),
  clip_id        TEXT REFERENCES clips(id),
  PRIMARY KEY (mag_id, relative_path)
);

CREATE TABLE IF NOT EXISTS tags (
  name       TEXT PRIMARY KEY COLLATE NOCASE,
  created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS clip_tags (
  clip_id TEXT NOT NULL REFERENCES clips(id),
  tag     TEXT NOT NULL REFERENCES tags(name),
  PRIMARY KEY (clip_id, tag)
);

CREATE TABLE IF NOT EXISTS people (
  name       TEXT PRIMARY KEY COLLATE NOCASE,
  created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS clip_people (
  clip_id TEXT NOT NULL REFERENCES clips(id),
  role    TEXT NOT NULL CHECK (role IN ('camera_operator','director_producer')),
  name    TEXT NOT NULL REFERENCES people(name),
  PRIMARY KEY (clip_id, role, name)
);

CREATE TABLE IF NOT EXISTS clip_field_times (
  clip_id    TEXT NOT NULL REFERENCES clips(id),
  field      TEXT NOT NULL,
  edited_at  TEXT NOT NULL,
  edited_by  TEXT NOT NULL,
  PRIMARY KEY (clip_id, field)
);

CREATE TABLE IF NOT EXISTS zmags (
  id             TEXT PRIMARY KEY,
  role           TEXT NOT NULL CHECK (role IN ('primary','backup')),
  pool_guid      TEXT NOT NULL UNIQUE,
  station        TEXT NOT NULL DEFAULT 'local',
  capacity_bytes INTEGER,
  created_at     TEXT NOT NULL,
  last_seen_at   TEXT,
  notes          TEXT
);

CREATE TABLE IF NOT EXISTS placements (
  id          TEXT PRIMARY KEY,
  mag_id      TEXT NOT NULL REFERENCES mags(id),
  zmag_id     TEXT NOT NULL REFERENCES zmags(id),
  copy_number INTEGER,
  status      TEXT NOT NULL CHECK (status IN ('copying','complete','failed','suspect')),
  copied_at   TEXT,
  verified_at TEXT,
  error       TEXT,
  notes       TEXT,
  UNIQUE (mag_id, zmag_id)
);

CREATE TABLE IF NOT EXISTS exports (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  project_id  TEXT REFERENCES projects(id),
  created_by  TEXT NOT NULL,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  notes       TEXT
);

CREATE TABLE IF NOT EXISTS export_clips (
  export_id   TEXT NOT NULL REFERENCES exports(id),
  clip_id     TEXT NOT NULL REFERENCES clips(id),
  in_point    TEXT,
  out_point   TEXT,
  copy_status TEXT NOT NULL DEFAULT 'pending' CHECK (copy_status IN ('pending','copying','done','failed')),
  PRIMARY KEY (export_id, clip_id)
);

CREATE TABLE IF NOT EXISTS cold_pull_requests (
  id           TEXT PRIMARY KEY,
  mag_id       TEXT NOT NULL REFERENCES mags(id),
  zmag_id      TEXT REFERENCES zmags(id),
  export_id    TEXT REFERENCES exports(id),
  requested_by TEXT NOT NULL,
  status       TEXT NOT NULL CHECK (status IN ('pending','dock_needed','pulling','complete','failed','cancelled')),
  error        TEXT,
  created_at   TEXT NOT NULL,
  completed_at TEXT,
  notes        TEXT
);

CREATE TABLE IF NOT EXISTS encoder_jobs (
  id               TEXT PRIMARY KEY,
  type             TEXT NOT NULL CHECK (type IN ('320p','hq','custom')),
  mag_id           TEXT NOT NULL REFERENCES mags(id),
  profile_id       TEXT NOT NULL,
  priority         INTEGER NOT NULL DEFAULT 0,
  status           TEXT NOT NULL CHECK (status IN ('queued','running','done','failed','cancelled')),
  cancel_requested INTEGER NOT NULL DEFAULT 0,
  clips_total      INTEGER NOT NULL,
  clips_done       INTEGER NOT NULL DEFAULT 0,
  progress         INTEGER NOT NULL DEFAULT 0,
  output_root      TEXT NOT NULL,
  attempts         INTEGER NOT NULL DEFAULT 0,
  requested_by     TEXT NOT NULL,
  created_at       TEXT NOT NULL,
  started_at       TEXT,
  finished_at      TEXT,
  error            TEXT
);

CREATE TABLE IF NOT EXISTS encoder_job_clips (
  job_id  TEXT NOT NULL REFERENCES encoder_jobs(id),
  clip_id TEXT NOT NULL REFERENCES clips(id),
  status  TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued','running','done','failed')),
  error   TEXT,
  PRIMARY KEY (job_id, clip_id)
);

CREATE TABLE IF NOT EXISTS copy_jobs (
  id               TEXT PRIMARY KEY,
  kind             TEXT NOT NULL CHECK (kind IN ('zmag_copy','export_copy','incoming_commit')),
  mag_id           TEXT REFERENCES mags(id),
  zmag_id          TEXT REFERENCES zmags(id),
  export_id        TEXT REFERENCES exports(id),
  status           TEXT NOT NULL CHECK (status IN ('queued','running','done','failed','cancelled')),
  cancel_requested INTEGER NOT NULL DEFAULT 0,
  bytes_total      INTEGER,
  bytes_done       INTEGER NOT NULL DEFAULT 0,
  attempts         INTEGER NOT NULL DEFAULT 0,
  requested_by     TEXT NOT NULL,
  created_at       TEXT NOT NULL,
  started_at       TEXT,
  finished_at      TEXT,
  error            TEXT
);

CREATE INDEX IF NOT EXISTS clips_by_mag      ON clips(mag_id);
CREATE INDEX IF NOT EXISTS files_by_clip     ON files(clip_id);
CREATE INDEX IF NOT EXISTS mags_by_project   ON mags(project_id);
CREATE INDEX IF NOT EXISTS placements_by_mag ON placements(mag_id);
CREATE INDEX IF NOT EXISTS jobs_queue        ON encoder_jobs(status, priority DESC, created_at);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_and_roundtrip() {
        let dir = std::env::temp_dir().join(format!("mv-db-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.sqlite");
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        ensure_schema(&db).unwrap();
        ensure_schema(&db).unwrap(); // idempotent
        db.execute("INSERT INTO projects(id,name,status,created_at,updated_at,requirements) VALUES ('p0','R','open','t','t','fps=25')", &[]).unwrap();
        db.execute(
            "INSERT INTO projects(id,name,status,created_at,updated_at) VALUES (?1,?2,'open',?3,?3)",
            &["p1".into(), "Name with 'quote' | pipe".into(), "2026-01-01T00:00:00Z".into()],
        )
        .unwrap();
        let rows = db.query("SELECT name, seq FROM projects WHERE id = ?1", &["p1".into()]).unwrap();
        assert_eq!(rows[0][0], Value::Text("Name with 'quote' | pipe".into()));
        assert_eq!(rows[0][1], Value::Int(0));
        assert_eq!(db.next_seq().unwrap(), 1);
        assert_eq!(db.next_seq().unwrap(), 2);
        // foreign keys enforced
        assert!(db
            .execute(
                "INSERT INTO mags(id,project_id,folder,shoot_date,file_format,gamma,gamut,status,size_bytes,file_count,ingested_by,ingested_at,updated_at) VALUES ('m','nope','f','2026-01-01','a','b','c','online',0,0,'x','t','t')",
                &[]
            )
            .is_err());
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schema_1_is_upgraded() {
        let dir = std::env::temp_dir().join(format!("mv-db-up-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.sqlite");
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        db.exec("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, season_or_year TEXT, notes TEXT,
                   status TEXT NOT NULL DEFAULT 'open', created_at TEXT NOT NULL, updated_at TEXT NOT NULL, seq INTEGER NOT NULL DEFAULT 0);
                 INSERT INTO meta VALUES ('schema_version','1');
                 INSERT INTO projects(id,name,status,created_at,updated_at) VALUES ('p1','Old','open','t','t');").unwrap();
        ensure_schema(&db).unwrap();
        db.execute("UPDATE projects SET requirements = 'res=HD' WHERE id = 'p1'", &[]).unwrap();
        db.execute("INSERT INTO project_log(project_id, at, level, message) VALUES ('p1','t','WARN','x')", &[]).unwrap();
        assert_eq!(db.query_one("SELECT value FROM meta WHERE key='schema_version'", &[]).unwrap().unwrap()[0].s(), "2");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
