//! ASC MHL for a mag folder: ASC MHL (specification 1.0; files declare hashlist version
//! 2.0, `urn:ASC:MHL:v2.0`); C4 only.
//!
//! - Manifest file name: `<NNNN>_<foldername>_<YYYY-MM-DD>_<HHMMSS>Z.mhl` (UTC).
//! - Chain file `ascmhl_chain.xml` lists every generation with its C4 hash.
//! - The specification allows md5, sha1, c4, xxh64, xxh3, xxh128 — not SHA-256. Magvault
//!   writes C4 (SHA-512 based, computed in the same first read as the SHA-256 that goes
//!   into the database and magcard). The C4 is also required for the chain file.
//! - `magcard.txt` and `.magvault/` are listed as ignore patterns: they are rewritten
//!   from the database later and must not break MHL verification.

use crate::error::{Context, Result};
use crate::hash::c4_of_bytes;
use crate::textfile::atomic_write;
use crate::time;
use std::path::Path;

pub const DIR: &str = "ascmhl";
pub const IGNORE: &[&str] = &[".DS_Store", "ascmhl", "ascmhl/", "magcard.txt", ".magvault", ".magvault/"];

pub struct Entry<'a> {
    pub rel: &'a str,
    pub size: u64,
    pub mtime: i64,
    pub c4: &'a str,
}

pub fn xml_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            _ => o.push(c),
        }
    }
    o
}

pub fn manifest_xml(entries: &[Entry], created: i64, hashed_at: i64, hostname: &str) -> String {
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str("<hashlist version=\"2.0\" xmlns=\"urn:ASC:MHL:v2.0\">\n");
    x.push_str("  <creatorinfo>\n");
    x.push_str(&format!("    <creationdate>{}</creationdate>\n", time::iso_utc(created)));
    x.push_str(&format!("    <hostname>{}</hostname>\n", xml_escape(hostname)));
    x.push_str(&format!("    <tool version=\"{}\">magvault</tool>\n", env!("CARGO_PKG_VERSION")));
    x.push_str("  </creatorinfo>\n");
    x.push_str("  <processinfo>\n");
    x.push_str("    <process>in-place</process>\n");
    x.push_str("    <ignore>\n");
    for p in IGNORE {
        x.push_str(&format!("      <pattern>{}</pattern>\n", xml_escape(p)));
    }
    x.push_str("    </ignore>\n");
    x.push_str("  </processinfo>\n");
    x.push_str("  <hashes>\n");
    let hashdate = time::iso_utc(hashed_at);
    for e in entries {
        x.push_str("    <hash>\n");
        x.push_str(&format!(
            "      <path size=\"{}\" lastmodificationdate=\"{}\">{}</path>\n",
            e.size,
            time::iso_utc(e.mtime),
            xml_escape(e.rel)
        ));
        x.push_str(&format!("      <c4 action=\"original\" hashdate=\"{hashdate}\">{}</c4>\n", e.c4));
        x.push_str("    </hash>\n");
    }
    x.push_str("  </hashes>\n");
    x.push_str("</hashlist>\n");
    x
}

pub fn chain_xml(generations: &[(String, String)]) -> String {
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str("<ascmhldirectory xmlns=\"urn:ASC:MHL:DIRECTORY:v2.0\">\n");
    for (i, (path, c4)) in generations.iter().enumerate() {
        x.push_str(&format!("  <hashlist sequencenr=\"{}\">\n", i + 1));
        x.push_str(&format!("    <path>{}</path>\n", xml_escape(path)));
        x.push_str(&format!("    <c4>{c4}</c4>\n"));
        x.push_str("  </hashlist>\n");
    }
    x.push_str("</ascmhldirectory>\n");
    x
}

pub fn manifest_name(seq: u32, folder: &str, created: i64) -> String {
    format!("{seq:04}_{folder}_{}.mhl", time::mhl_name_stamp(created))
}

/// Write generation 0001 and the chain file into `<mag_dir>/ascmhl/`.
/// Returns the manifest file name.
pub fn write_initial(mag_dir: &Path, folder: &str, entries: &[Entry], created: i64, hashed_at: i64, hostname: &str) -> Result<String> {
    let dir = mag_dir.join(DIR);
    std::fs::create_dir_all(&dir).ctx(format!("cannot create {}", dir.display()))?;
    let name = manifest_name(1, folder, created);
    let xml = manifest_xml(entries, created, hashed_at, hostname);
    atomic_write(&dir.join(&name), xml.as_bytes())?;
    let chain = chain_xml(&[(name.clone(), c4_of_bytes(xml.as_bytes()))]);
    atomic_write(&dir.join("ascmhl_chain.xml"), chain.as_bytes())?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_xml() {
        assert_eq!(
            manifest_name(1, "20250520_3f6c2a1e-9b0d-4c55-8e21-a3f9c2d41b77", 1_763_839_263),
            "0001_20250520_3f6c2a1e-9b0d-4c55-8e21-a3f9c2d41b77_2025-11-22_192103Z.mhl"
        );
        let c4 = c4_of_bytes(b"x");
        let xml = manifest_xml(&[Entry { rel: "A&B/c.mov", size: 1, mtime: 0, c4: &c4 }], 0, 0, "host");
        assert!(xml.contains("<path size=\"1\" lastmodificationdate=\"1970-01-01T00:00:00Z\">A&amp;B/c.mov</path>"));
        assert!(xml.contains("<pattern>magcard.txt</pattern>"));
        let chain = chain_xml(&[("0001_x.mhl".into(), c4.clone())]);
        assert!(chain.contains("<hashlist sequencenr=\"1\">"));
    }
}
