//! A mag as the database knows it, and the magcard rendered from it
//! (database is authoritative for metadata: PLAN.md "Conflict handling").

use crate::db::{Db, Value};
use crate::error::Result;
use crate::magcard::{LoggingRow, MagCard, ManifestRow, FORMAT};

#[derive(Debug, Clone)]
pub struct MagRecord {
    pub mag_uuid: String,
    pub folder: String,
    pub project_uuid: String,
    pub project_name: String,
    pub project_status: String,
    pub shoot_date: String,
    pub camera_profile_id: Option<String>,
    pub file_format: String,
    pub gamma: String,
    pub gamut: String,
    pub source_label: String,
    pub ingested_by: String,
    pub ingested_at: String,
    pub updated_at: String,
    pub notes: String,
    pub status: String,
    pub manifest: Vec<ManifestRow>,
    pub logging: Vec<LoggingRow>,
}

/// Header values that must match between card and database. `camera_model` is derived
/// from the profile file (not stored in the database) and `updated_at` is the card's own
/// write time, so neither counts as a metadata conflict.
pub const COMPARED_KEYS: &[&str] = &[
    "format",
    "mag_uuid",
    "mag_folder",
    "project_uuid",
    "project_name",
    "project_status",
    "shoot_date",
    "camera_profile_id",
    "file_format",
    "gamma",
    "gamut",
    "source_label",
    "ingested_by",
    "ingested_at",
    "notes",
];

pub fn card_from_record(r: &MagRecord, camera_model: Option<&str>) -> MagCard {
    let mut c = MagCard::default();
    c.set("format", FORMAT);
    c.set("mag_uuid", &r.mag_uuid);
    c.set("mag_folder", &r.folder);
    c.set("project_uuid", &r.project_uuid);
    c.set("project_name", &r.project_name);
    c.set("project_status", &r.project_status);
    c.set("shoot_date", &r.shoot_date);
    c.set("camera_profile_id", r.camera_profile_id.clone().unwrap_or_default());
    c.set("camera_model", camera_model.unwrap_or(""));
    c.set("file_format", &r.file_format);
    c.set("gamma", &r.gamma);
    c.set("gamut", &r.gamut);
    c.set("source_label", &r.source_label);
    c.set("ingested_by", &r.ingested_by);
    c.set("ingested_at", &r.ingested_at);
    c.set("updated_at", &r.updated_at);
    c.set("notes", &r.notes);
    c.manifest = r.manifest.clone();
    c.logging = r.logging.clone();
    c
}

/// Find a mag by full UUID or folder name.
pub fn find_mag_id(db: &Db, key: &str) -> Result<Option<String>> {
    let key = key.trim_end_matches('/');
    let key = key.rsplit('/').next().unwrap_or(key);
    Ok(db
        .query_one("SELECT id FROM mags WHERE id = ?1 OR folder = ?1", &[key.into()])?
        .map(|r| r[0].s()))
}

pub fn load_record(db: &Db, mag_id: &str) -> Result<Option<MagRecord>> {
    let Some(m) = db.query_one(
        "SELECT m.id, m.folder, p.id, p.name, p.status, m.shoot_date, m.camera_profile_id, m.file_format, m.gamma, m.gamut,
                m.source_label, m.ingested_by, m.ingested_at, m.updated_at, m.notes, m.status
           FROM mags m JOIN projects p ON p.id = m.project_id WHERE m.id = ?1",
        &[mag_id.into()],
    )?
    else {
        return Ok(None);
    };
    let manifest = db
        .query("SELECT relative_path, size, sha256, mtime FROM files WHERE mag_id = ?1 ORDER BY relative_path", &[mag_id.into()])?
        .into_iter()
        .map(|r| ManifestRow { rel: r[0].s(), size: r[1].int().unwrap_or(0) as u64, sha256: r[2].s(), mtime: r[3].s() })
        .collect();
    Ok(Some(MagRecord {
        mag_uuid: m[0].s(),
        folder: m[1].s(),
        project_uuid: m[2].s(),
        project_name: m[3].s(),
        project_status: m[4].s(),
        shoot_date: m[5].s(),
        camera_profile_id: m[6].text().map(str::to_string).filter(|s| !s.is_empty()),
        file_format: m[7].s(),
        gamma: m[8].s(),
        gamut: m[9].s(),
        source_label: m[10].s(),
        ingested_by: m[11].s(),
        ingested_at: m[12].s(),
        updated_at: m[13].s(),
        notes: m[14].s(),
        status: m[15].s(),
        manifest,
        logging: load_logging(db, mag_id)?,
    }))
}

/// Logging rows for clips that have any logging field set, ordered by path.
fn load_logging(db: &Db, mag_id: &str) -> Result<Vec<LoggingRow>> {
    let clips = db.query(
        "SELECT id, relative_path, rating, slow_motion, shot_type, description, technical_note
           FROM clips WHERE mag_id = ?1 ORDER BY relative_path",
        &[mag_id.into()],
    )?;
    let mut out = Vec::new();
    for c in clips {
        let id = c[0].s();
        let list = |sql: &str| -> Result<String> {
            let rows = db.query(sql, &[Value::from(id.as_str())])?;
            Ok(rows.into_iter().map(|r| r[0].s()).collect::<Vec<_>>().join("; "))
        };
        let tags = list("SELECT tag FROM clip_tags WHERE clip_id = ?1 ORDER BY tag COLLATE NOCASE")?;
        let ops = list("SELECT name FROM clip_people WHERE clip_id = ?1 AND role = 'camera_operator' ORDER BY name COLLATE NOCASE")?;
        let dirs =
            list("SELECT name FROM clip_people WHERE clip_id = ?1 AND role = 'director_producer' ORDER BY name COLLATE NOCASE")?;
        let row = LoggingRow {
            rel: c[1].s(),
            rating: c[2].s(),
            slow_motion: if c[3].int() == Some(1) { "1".into() } else { "0".into() },
            shot_type: c[4].s(),
            tags,
            camera_operator: ops,
            director_producer: dirs,
            description: c[5].s(),
            technical_note: c[6].s(),
        };
        let logged = !row.rating.is_empty()
            || row.slow_motion == "1"
            || !row.shot_type.is_empty()
            || !row.tags.is_empty()
            || !row.camera_operator.is_empty()
            || !row.director_producer.is_empty()
            || !row.description.is_empty()
            || !row.technical_note.is_empty();
        if logged {
            out.push(row);
        }
    }
    Ok(out)
}
