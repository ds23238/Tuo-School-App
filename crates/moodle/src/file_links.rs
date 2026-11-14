//! Resolve Moodle browser links against the sync index, never by filename.
use tsa_core::Database;
use url::Url;

use crate::Result;

/// Canonical identity of a pluginfile URL. Authentication and download flags
/// do not change the file. Moodle supports both slash arguments and `?file=`.
fn identity(url: &Url, base: &Url) -> Option<String> {
    if url.origin() != base.origin() || !url.path().starts_with(base.path()) {
        return None;
    }
    let relative = url.path().strip_prefix(base.path())?;
    let relative = relative.strip_prefix("webservice/").unwrap_or(relative);
    let file = relative.strip_prefix("pluginfile.php")?;
    let path = if file.starts_with('/') {
        file.to_owned()
    } else if file.is_empty() {
        url.query_pairs().find(|(k, _)| k == "file")?.1.into_owned()
    } else {
        return None;
    };
    // Parsing encodes spaces in Moodle's query-argument variant as in normal URLs.
    let normalized = Url::parse(&format!("https://file.invalid/{path}")).ok()?;
    let mut parts: Vec<_> = normalized
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect();
    // Moodle uses both a concrete revision and 0/-1 for the latest content.
    // The context, component, file area and full path identify the same file.
    if parts.len() >= 5
        && matches!(parts[1], "mod_resource" | "mod_folder")
        && parts[2] == "content"
        && parts[3].parse::<i64>().is_ok()
    {
        parts.remove(3);
    }
    Some(parts.join("/"))
}

/// Only an exact, unique file identity is eligible. Resource activity links
/// are supported when the activity contains exactly one current file.
pub fn resolve(db: &Database, base: &Url, url: &Url) -> Result<Option<i64>> {
    if url.origin() != base.origin() || !url.path().starts_with(base.path()) {
        return Ok(None);
    }
    let conn = db.conn();
    if url.path().strip_prefix(base.path()) == Some("mod/resource/view.php") {
        let module = url
            .query_pairs()
            .find(|(k, _)| k == "id")
            .and_then(|(_, v)| v.parse::<i64>().ok());
        if let Some(module) = module {
            let mut stmt =
                conn.prepare("SELECT id FROM moodle_files WHERE module_id = ?1 AND removed = 0")?;
            let ids = stmt
                .query_map([module], |r| r.get::<_, i64>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            return Ok(if ids.len() == 1 { Some(ids[0]) } else { None });
        }
        return Ok(None);
    }
    let Some(wanted) = identity(url, base) else {
        return Ok(None);
    };
    let mut stmt = conn.prepare("SELECT id, file_url FROM moodle_files WHERE removed = 0")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    let mut found = None;
    for row in rows {
        let (id, stored) = row?;
        if Url::parse(&stored)
            .ok()
            .and_then(|u| identity(&u, base))
            .as_ref()
            == Some(&wanted)
        {
            if found.is_some() {
                return Ok(None);
            }
            found = Some(id);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{RemoteFile, record};

    fn fixture() -> (Database, Url) {
        let db = Database::open_in_memory().unwrap();
        crate::init(&db).unwrap();
        let file = RemoteFile {
            module_id: 42, remote_path: "/sheet.pdf".into(),
            url: "https://school.example/moodle/webservice/pluginfile.php/123/mod_resource/content/7/sheet.pdf?token=secret".into(),
            modified: None, size: 20, target: "course/sheet.pdf".into(),
        };
        record(&db, 1, &[file]).unwrap();
        (db, Url::parse("https://school.example/moodle/").unwrap())
    }

    #[test]
    fn resolves_browser_download_and_resource_links() {
        let (db, base) = fixture();
        for path in [
            "pluginfile.php/123/mod_resource/content/7/sheet.pdf?forcedownload=1#page=2",
            "webservice/pluginfile.php/123/mod_resource/content/7/sheet.pdf?token=other",
            "pluginfile.php/123/mod_resource/content/0/sheet.pdf",
            "pluginfile.php/123/mod_resource/content/8/sheet.pdf",
            "pluginfile.php?file=%2F123%2Fmod_resource%2Fcontent%2F7%2Fsheet.pdf",
            "mod/resource/view.php?id=42&redirect=1",
        ] {
            assert!(
                resolve(&db, &base, &base.join(path).unwrap())
                    .unwrap()
                    .is_some(),
                "{path}"
            );
        }
    }

    #[test]
    fn rejects_foreign_sites_same_filename_and_other_activities() {
        let (db, base) = fixture();
        for url in [
            "https://evil.example/moodle/pluginfile.php/123/mod_resource/content/7/sheet.pdf",
            "https://school.example/other/pluginfile.php/123/mod_resource/content/7/sheet.pdf",
            "https://school.example/moodle/pluginfile.php/999/mod_resource/content/7/sheet.pdf",
            "https://school.example/moodle/mod/folder/view.php?id=42",
            "https://school.example/moodle/mod/resource/view.php?id=99",
        ] {
            assert_eq!(
                resolve(&db, &base, &Url::parse(url).unwrap()).unwrap(),
                None
            );
        }
    }

    #[test]
    fn ambiguous_and_removed_files_do_not_intercept() {
        let (db, base) = fixture();
        db.conn()
            .execute("UPDATE moodle_files SET removed = 1", [])
            .unwrap();
        assert_eq!(
            resolve(
                &db,
                &base,
                &base.join("mod/resource/view.php?id=42").unwrap()
            )
            .unwrap(),
            None
        );
        db.conn()
            .execute("UPDATE moodle_files SET removed = 0", [])
            .unwrap();
        db.conn().execute("INSERT INTO moodle_files (course_id, module_id, remote_path, file_url, remote_size, target_path) SELECT course_id, module_id, '/other.pdf', file_url, remote_size, target_path FROM moodle_files", []).unwrap();
        for path in [
            "mod/resource/view.php?id=42",
            "pluginfile.php/123/mod_resource/content/7/sheet.pdf",
        ] {
            assert_eq!(
                resolve(&db, &base, &base.join(path).unwrap()).unwrap(),
                None
            );
        }
    }
}
