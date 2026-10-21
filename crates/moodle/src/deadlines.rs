//! Anstehende Fristen aus dem Moodle-Kalender (Abgaben, Tests …).

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Result, Session, plain};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deadline {
    pub id: i64,
    pub name: String,
    /// Moodle-Modulname, z. B. `assign` oder `quiz`.
    pub kind: String,
    pub course_id: i64,
    pub course: String,
    /// Kurs-Modul-ID (`cmid`), falls bekannt.
    pub module_id: Option<i64>,
    /// Unix-Zeit der Frist.
    pub time: i64,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteCourse {
    id: i64,
    #[serde(default)]
    fullname: String,
}

#[derive(Debug, Deserialize)]
struct RemoteEvent {
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    activityname: Option<String>,
    #[serde(default)]
    modulename: Option<String>,
    timesort: i64,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    course: Option<RemoteCourse>,
}

#[derive(Debug, Deserialize)]
struct Events {
    events: Vec<RemoteEvent>,
}

/// `…/mod/assign/view.php?id=12` → 12.
fn module_id(url: &str) -> Option<i64> {
    let url = Url::parse(url).ok()?;
    if !url.path().contains("/mod/") {
        return None;
    }
    url.query_pairs()
        .find(|(k, _)| k == "id")
        .and_then(|(_, v)| v.parse().ok())
}

impl Session<'_> {
    /// Offene Fristen ab zwei Wochen in der Vergangenheit (überfällig) bis
    /// zu den nächsten 50. Erledigtes liefert Moodle hier nicht mehr.
    pub async fn deadlines(&self) -> Result<Vec<Deadline>> {
        let from = crate::contents::now() - 14 * 24 * 3600;
        let events: Events = self
            .call(
                "core_calendar_get_action_events_by_timesort",
                &[
                    ("timesortfrom", from.to_string()),
                    ("limitnum", "50".into()),
                    ("limittononsuspendedevents", "1".into()),
                ],
            )
            .await?;
        Ok(events
            .events
            .into_iter()
            .map(|e| Deadline {
                id: e.id,
                name: plain(e.activityname.as_deref().unwrap_or(&e.name)),
                kind: e.modulename.unwrap_or_default(),
                course_id: e.course.as_ref().map_or(0, |c| c.id),
                course: e.course.map(|c| plain(&c.fullname)).unwrap_or_default(),
                module_id: e.url.as_deref().and_then(module_id),
                time: e.timesort,
                url: e.url,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_ids() {
        assert_eq!(
            module_id("https://m.example/mod/assign/view.php?id=12"),
            Some(12)
        );
        assert_eq!(module_id("https://m.example/course/view.php?id=3"), None);
        assert_eq!(module_id("kaputt"), None);
    }
}
