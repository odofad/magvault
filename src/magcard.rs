//! `magcard.txt`: `key: value` header, `# manifest` section (fixed at ingest) and
//! `# logging` section (rewritten from the database). Format per tech spec §8.3 and
//! PLAN.md "Mag card format", "Logging on the mag card", "Implementation defaults".
//!
//! List sections use ` | ` (space, pipe, space) between fields. Fields are not trimmed,
//! so names with leading/trailing spaces survive. A field may not contain `|` or control
//! characters; such names are refused at probe time.

use crate::error::Result;
use crate::textfile::check_card_field;

pub const FORMAT: &str = "magvault-magcard/1";
pub const FILE_NAME: &str = "magcard.txt";
pub const SEP: &str = " | ";

/// Header keys in the order they are written. Keys found in an existing card but not
/// listed here are kept (after these) when the card is rewritten.
pub const HEADER_KEYS: &[&str] = &[
    "format",
    "mag_uuid",
    "mag_folder",
    "project_uuid",
    "project_name",
    "project_status",
    "shoot_date",
    "camera_profile_id",
    "camera_model",
    "file_format",
    "gamma",
    "gamut",
    "source_label",
    "ingested_by",
    "ingested_at",
    "updated_at",
    "notes",
];

pub const MANIFEST_COLUMNS: &str = "relative_path | size | sha256 | mtime";
pub const LOGGING_COLUMNS: &str =
    "relative_path | rating | slow_motion | shot_type | tags | camera_operator | director_producer | description | technical_note";

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestRow {
    pub rel: String,
    pub size: u64,
    pub sha256: String,
    pub mtime: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoggingRow {
    pub rel: String,
    pub rating: String,
    pub slow_motion: String,
    pub shot_type: String,
    pub tags: String,
    pub camera_operator: String,
    pub director_producer: String,
    pub description: String,
    pub technical_note: String,
}

impl LoggingRow {
    fn fields(&self) -> [&str; 9] {
        [
            &self.rel,
            &self.rating,
            &self.slow_motion,
            &self.shot_type,
            &self.tags,
            &self.camera_operator,
            &self.director_producer,
            &self.description,
            &self.technical_note,
        ]
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MagCard {
    /// Header in file order (known keys first when written).
    pub header: Vec<(String, String)>,
    pub manifest: Vec<ManifestRow>,
    pub logging: Vec<LoggingRow>,
}

impl MagCard {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.header.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        match self.header.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.header.push((key.to_string(), value)),
        }
    }

    pub fn render(&self) -> Result<String> {
        let mut out = String::new();
        let mut written = Vec::new();
        for key in HEADER_KEYS {
            let v = self.get(key).unwrap_or("");
            check_card_field(&format!("magcard {key}"), v)?;
            out.push_str(&format!("{key}: {v}\n"));
            written.push(*key);
        }
        for (k, v) in &self.header {
            if !written.contains(&k.as_str()) {
                check_card_field(&format!("magcard {k}"), v)?;
                out.push_str(&format!("{k}: {v}\n"));
            }
        }
        out.push_str("\n# manifest\n# ");
        out.push_str(MANIFEST_COLUMNS);
        out.push('\n');
        for r in &self.manifest {
            for f in [&r.rel, &r.sha256, &r.mtime] {
                check_card_field("manifest field", f)?;
            }
            out.push_str(&format!("{}{SEP}{}{SEP}{}{SEP}{}\n", r.rel, r.size, r.sha256, r.mtime));
        }
        out.push_str("\n# logging\n# ");
        out.push_str(LOGGING_COLUMNS);
        out.push('\n');
        for r in &self.logging {
            let f = r.fields();
            for v in f {
                check_card_field("logging field", v)?;
            }
            out.push_str(&f.join(SEP));
            out.push('\n');
        }
        Ok(out)
    }

    pub fn parse(text: &str) -> std::result::Result<MagCard, String> {
        #[derive(PartialEq)]
        enum Sec {
            Header,
            Manifest,
            Logging,
            Unknown,
        }
        let mut card = MagCard::default();
        let mut sec = Sec::Header;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            let lineno = n + 1;
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with('#') {
                match line.trim() {
                    "# manifest" => sec = Sec::Manifest,
                    "# logging" => sec = Sec::Logging,
                    t if t.starts_with("# ") && t.contains('|') => {} // column comment
                    t if sec == Sec::Header || t.len() < 3 => {}      // comment
                    _ => sec = Sec::Unknown,                          // future section: ignored
                }
                continue;
            }
            match sec {
                Sec::Header => {
                    let Some((k, v)) = line.split_once(':') else {
                        return Err(format!("line {lineno}: expected `key: value`"));
                    };
                    card.header.push((k.trim().to_string(), v.trim().to_string()));
                }
                Sec::Manifest => {
                    let f: Vec<&str> = line.split(SEP).collect();
                    if f.len() != 4 || line.matches('|').count() != 3 {
                        return Err(format!("line {lineno}: manifest line needs 4 fields separated by ' | '"));
                    }
                    let size = f[1].parse::<u64>().map_err(|_| format!("line {lineno}: size {:?} is not a number", f[1]))?;
                    card.manifest.push(ManifestRow { rel: f[0].into(), size, sha256: f[2].into(), mtime: f[3].into() });
                }
                Sec::Logging => {
                    let f: Vec<&str> = line.split(SEP).collect();
                    if f.len() != 9 || line.matches('|').count() != 8 {
                        return Err(format!("line {lineno}: logging line needs 9 fields separated by ' | '"));
                    }
                    card.logging.push(LoggingRow {
                        rel: f[0].into(),
                        rating: f[1].into(),
                        slow_motion: f[2].into(),
                        shot_type: f[3].into(),
                        tags: f[4].into(),
                        camera_operator: f[5].into(),
                        director_producer: f[6].into(),
                        description: f[7].into(),
                        technical_note: f[8].into(),
                    });
                }
                Sec::Unknown => {}
            }
        }
        Ok(card)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut c = MagCard::default();
        c.set("format", FORMAT);
        c.set("mag_uuid", "3f6c2a1e-9b0d-4c55-8e21-a3f9c2d41b77");
        c.set("notes", "Second unit, stage left");
        c.set("future_key", "kept");
        c.manifest.push(ManifestRow {
            rel: "DCIM/ clip with spaces .MOV".into(),
            size: 42,
            sha256: "ab".repeat(32),
            mtime: "2025-05-20T14:02:11Z".into(),
        });
        c.logging.push(LoggingRow { rel: "DCIM/x.MOV".into(), rating: "4".into(), tags: "a; b".into(), ..Default::default() });
        let text = c.render().unwrap();
        let back = MagCard::parse(&text).unwrap();
        assert_eq!(back.manifest, c.manifest);
        assert_eq!(back.logging, c.logging);
        assert_eq!(back.get("future_key"), Some("kept"));
        assert_eq!(back.get("notes"), Some("Second unit, stage left"));
        assert_eq!(back.get("gamma"), Some(""));
    }

    #[test]
    fn rejects_pipes() {
        let mut c = MagCard::default();
        c.set("notes", "a | b");
        assert!(c.render().is_err());
        assert!(MagCard::parse("format: x\n\n# manifest\na | b | 1 | c | d\n").is_err());
    }
}
