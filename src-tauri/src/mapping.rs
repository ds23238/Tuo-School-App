//! Zuordnung Untis-Fach ↔ Moodle-Kurs: Vorschläge über Namensähnlichkeit,
//! der Nutzer bestätigt oder korrigiert.

use std::collections::HashMap;

use serde::Serialize;
use tsa_core::Database;
use tsa_core::settings::{get_value, set_value};
use tsa_moodle::Course;

const KEY: &str = "mapping";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub subject: String,
    pub long_name: String,
    pub course_id: Option<i64>,
    /// Vorschlag, noch nicht vom Nutzer bestätigt.
    pub suggested: bool,
}

/// Bestätigte Zuordnungen; `None` = ausdrücklich kein Kurs.
fn stored(db: &Database) -> HashMap<String, Option<i64>> {
    get_value(db, KEY)
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn set(db: &Database, subject: &str, course_id: Option<i64>) -> tsa_core::Result<()> {
    let mut map = stored(db);
    map.insert(subject.to_owned(), course_id);
    set_value(db, KEY, &serde_json::to_string(&map)?)
}

/// Kleinbuchstaben, Umlaute ausgeschrieben, nur Buchstaben und Ziffern.
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Gängige Fachkürzel, die im Kursnamen meist ausgeschrieben sind.
const ALIASES: &[(&str, &[&str])] = &[
    ("m", &["mathematik", "mathe"]),
    ("d", &["deutsch"]),
    ("e", &["englisch", "english"]),
    ("f", &["franzoesisch", "french"]),
    ("l", &["latein"]),
    ("s", &["spanisch"]),
    ("bio", &["biologie"]),
    ("bi", &["biologie"]),
    ("ch", &["chemie"]),
    ("ph", &["physik"]),
    ("inf", &["informatik"]),
    ("if", &["informatik"]),
    ("ge", &["geschichte"]),
    ("ek", &["erdkunde", "geographie"]),
    ("geo", &["geographie", "erdkunde"]),
    ("ku", &["kunst"]),
    ("mu", &["musik"]),
    ("sp", &["sport"]),
    ("spo", &["sport"]),
    ("pw", &["politik"]),
    ("po", &["politik"]),
    ("sk", &["sozialkunde"]),
    ("wi", &["wirtschaft"]),
    ("eth", &["ethik"]),
    ("et", &["ethik"]),
    ("rel", &["religion"]),
    ("er", &["religion", "evangelisch"]),
    ("kr", &["religion", "katholisch"]),
    ("phil", &["philosophie"]),
    ("pl", &["philosophie"]),
];

/// Wie gut ein Kurs zu einem Fach passt; 0 = gar nicht.
fn score(subject: &str, long_name: &str, course: &(i64, String)) -> u32 {
    let course_words: Vec<String> = words(&course.1);
    let has = |w: &str| {
        course_words
            .iter()
            .any(|c| c == w || (w.len() >= 5 && c.starts_with(w)))
    };
    let mut points = 0;
    let long = words(long_name);
    if !long.is_empty() && long.iter().all(|w| has(w)) {
        points += 3;
    }
    let short = subject.to_lowercase();
    if has(&short) && short.len() >= 2 {
        points += 2;
    }
    if let Some((_, full)) = ALIASES.iter().find(|(a, _)| *a == short) {
        if full.iter().any(|w| has(w)) {
            points += 2;
        }
    }
    points
}

/// Bester Kurs, wenn er eindeutig vorne liegt.
/// `courses`: Kurs-ID und Kursname samt Kurzname.
fn suggest(subject: &str, long_name: &str, courses: &[(i64, String)]) -> Option<i64> {
    let mut scored: Vec<(u32, i64)> = courses
        .iter()
        .map(|c| (score(subject, long_name, c), c.0))
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    match scored.as_slice() {
        [(_, id)] => Some(*id),
        [(best, id), (second, _), ..] if best > second => Some(*id),
        _ => None,
    }
}

/// Alle bekannten Fächer mit bestätigter oder vorgeschlagener Zuordnung.
pub fn list(db: &Database, subjects: &[(String, String)]) -> Vec<Entry> {
    let stored = stored(db);
    let courses: Vec<(i64, String)> = Course::list(db)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.enrolled)
        .map(|c| (c.id, format!("{} {}", c.full_name, c.short_name)))
        .collect();
    subjects
        .iter()
        .map(|(subject, long_name)| match stored.get(subject) {
            Some(course_id) => Entry {
                subject: subject.clone(),
                long_name: long_name.clone(),
                course_id: *course_id,
                suggested: false,
            },
            None => Entry {
                subject: subject.clone(),
                long_name: long_name.clone(),
                course_id: suggest(subject, long_name, &courses),
                suggested: true,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn course(id: i64, full: &str, short: &str) -> (i64, String) {
        (id, format!("{full} {short}"))
    }

    #[test]
    fn suggests_by_name_and_alias() {
        let courses = [
            course(1, "Mathematik 10b (Müller)", "M10b"),
            course(2, "Biologie 10b", "Bio10"),
            course(3, "Deutsch 10b", "D10b"),
            course(4, "Physik AG", "PhAG"),
        ];
        assert_eq!(suggest("M", "Mathematik", &courses), Some(1));
        assert_eq!(suggest("BI", "Biologie", &courses), Some(2));
        assert_eq!(suggest("D", "", &courses), Some(3));
        assert_eq!(suggest("CH", "Chemie", &courses), None);
    }
}
