//! Wochenstundenplan mit Vertretungen und Ausfällen.

use std::collections::{BTreeSet, HashMap};

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Error, Result, Session, cache, iso_date, time, untis_date};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    pub name: String,
    pub long_name: String,
    /// Ursprünglich geplant (z. B. vertretene Lehrkraft, alter Raum).
    pub previous: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: i64,
    /// ISO-Datum.
    pub date: String,
    /// `HH:MM`.
    pub start: String,
    pub end: String,
    pub subject: Option<Element>,
    pub teachers: Vec<Element>,
    pub rooms: Vec<Element>,
    pub classes: Vec<Element>,
    /// `regular`, `cancelled`, `changed` (Vertretung, Raumänderung …),
    /// `exam` oder `other` (Sprechstunde, Pausenaufsicht …).
    pub status: String,
    pub text: Option<String>,
    pub info: Option<String>,
    pub substitution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub name: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    pub holiday: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Week {
    pub monday: String,
    pub days: Vec<Day>,
    pub lessons: Vec<Lesson>,
    pub grid: Vec<Unit>,
    pub fetched_at: i64,
    pub offline: bool,
}

/// Geänderte Stunde für Benachrichtigungen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub date: String,
    /// Stundenbezeichnung aus dem Zeitraster, sonst die Uhrzeit.
    pub period: String,
    pub subject: String,
    /// `cancelled` oder `changed`.
    pub kind: String,
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Short {
    #[serde(default)]
    name: String,
    #[serde(default)]
    longname: String,
    #[serde(default)]
    orgname: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteLesson {
    id: i64,
    date: i64,
    start_time: i64,
    end_time: i64,
    #[serde(default)]
    kl: Vec<Short>,
    #[serde(default)]
    te: Vec<Short>,
    #[serde(default)]
    su: Vec<Short>,
    #[serde(default)]
    ro: Vec<Short>,
    #[serde(default)]
    lstext: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    info: Option<String>,
    #[serde(default)]
    subst_text: Option<String>,
    #[serde(default)]
    lstype: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteUnit {
    name: String,
    start_time: i64,
    end_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteDay {
    day: i64,
    time_units: Vec<RemoteUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Holiday {
    #[serde(default)]
    long_name: String,
    #[serde(default)]
    name: String,
    start_date: i64,
    end_date: i64,
}

fn element(s: &Short) -> Element {
    Element {
        name: s.name.clone(),
        long_name: if s.longname.is_empty() {
            s.name.clone()
        } else {
            s.longname.clone()
        },
        previous: s.orgname.clone().filter(|o| !o.is_empty() && *o != s.name),
    }
}

fn text(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

fn lesson(l: &RemoteLesson) -> Lesson {
    let teachers: Vec<Element> = l.te.iter().map(element).collect();
    let rooms: Vec<Element> = l.ro.iter().map(element).collect();
    let moved = teachers.iter().chain(&rooms).any(|e| e.previous.is_some());
    let status = match (l.code.as_deref(), l.lstype.as_deref()) {
        (Some("cancelled"), _) => "cancelled",
        (Some("irregular"), _) => "changed",
        (_, Some("ex")) => "exam",
        (_, Some("oh" | "sb" | "bs")) => "other",
        _ if moved || text(&l.subst_text).is_some() => "changed",
        _ => "regular",
    };
    Lesson {
        id: l.id,
        date: iso_date(l.date),
        start: time(l.start_time),
        end: time(l.end_time),
        subject: l.su.first().map(element),
        teachers,
        rooms,
        classes: l.kl.iter().map(element).collect(),
        status: status.to_owned(),
        text: text(&l.lstext),
        info: text(&l.info),
        substitution: text(&l.subst_text),
    }
}

/// Montag der Woche von `date`.
pub fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Days::new(date.weekday().num_days_from_monday() as u64)
}

const MASTER_MAX_AGE: i64 = 7 * 24 * 3600;

impl Session<'_> {
    async fn lessons(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Lesson>> {
        let person = self.person()?;
        let remote: Vec<RemoteLesson> = self
            .rpc(
                "getTimetable",
                json!({ "options": {
                    "element": { "id": person.id, "type": person.kind },
                    "startDate": untis_date(from),
                    "endDate": untis_date(to),
                    "showLsText": true,
                    "showInfo": true,
                    "showSubstText": true,
                    "showStudentgroup": true,
                    "klasseFields": ["id", "name", "longname"],
                    "roomFields": ["id", "name", "longname"],
                    "subjectFields": ["id", "name", "longname"],
                    "teacherFields": ["id", "name", "longname"],
                }}),
            )
            .await?;
        let mut lessons: Vec<Lesson> = remote.iter().map(lesson).collect();
        lessons.sort_by(|a, b| (&a.date, &a.start).cmp(&(&b.date, &b.start)));
        self.remember_subjects(&lessons)?;
        Ok(lessons)
    }

    fn remember_subjects(&self, lessons: &[Lesson]) -> Result<()> {
        let conn = self.db().conn();
        for s in lessons.iter().filter_map(|l| l.subject.as_ref()) {
            conn.execute(
                "INSERT INTO untis_subjects (name, long_name) VALUES (?1, ?2)
                 ON CONFLICT (name) DO UPDATE SET long_name = excluded.long_name",
                (&s.name, &s.long_name),
            )?;
        }
        Ok(())
    }

    /// Fächer aus bisher geladenen Stundenplänen: (Kürzel, Name).
    pub fn subjects(&self) -> Result<Vec<(String, String)>> {
        let conn = self.db().conn();
        let mut stmt =
            conn.prepare("SELECT name, long_name FROM untis_subjects ORDER BY long_name")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Stammdaten, die sich selten ändern: höchstens wöchentlich neu laden.
    async fn master<T>(&self, key: &str, method: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let cached: Option<(T, i64)> = cache::load(self.db(), key)?;
        let fresh = cached
            .as_ref()
            .is_some_and(|(_, at)| crate::now() - at < MASTER_MAX_AGE);
        if !fresh {
            match self.rpc::<T>(method, json!({})).await {
                Ok(value) => {
                    cache::store(self.db(), key, &value)?;
                    return Ok(value);
                }
                Err(e) if cached.is_none() => return Err(e),
                Err(_) => {}
            }
        }
        Ok(cached.expect("geprüft").0)
    }

    async fn grid(&self) -> Vec<Unit> {
        let days: Vec<RemoteDay> = self
            .master("timegrid", "getTimegridUnits")
            .await
            .unwrap_or_default();
        // Die Stunden des Tages mit den meisten Einheiten (meist Montag).
        days.into_iter()
            .max_by_key(|d| (d.time_units.len(), -d.day))
            .map(|d| {
                d.time_units
                    .iter()
                    .map(|u| Unit {
                        name: u.name.clone(),
                        start: time(u.start_time),
                        end: time(u.end_time),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn holidays(&self) -> Vec<Holiday> {
        self.master("holidays", "getHolidays")
            .await
            .unwrap_or_default()
    }

    /// Stundenplan der Woche mit `monday`; offline der zuletzt geladene Stand.
    pub async fn week(&self, monday: NaiveDate) -> Result<Week> {
        let monday = monday_of(monday);
        let sunday = monday + Days::new(6);
        let key = format!("week:{monday}");
        let (lessons, fetched_at, offline) = match self.lessons(monday, sunday).await {
            Ok(lessons) => {
                let at = cache::store(self.db(), &key, &lessons)?;
                (lessons, at, false)
            }
            Err(e @ (Error::Network(_) | Error::SessionExpired)) => {
                match cache::load::<Vec<Lesson>>(self.db(), &key)? {
                    Some((lessons, at)) => (lessons, at, true),
                    None => return Err(e),
                }
            }
            Err(e) => return Err(e),
        };
        let holidays = self.holidays().await;
        let saturday = lessons.iter().any(|l| {
            NaiveDate::parse_from_str(&l.date, "%Y-%m-%d")
                .is_ok_and(|d| d.weekday().num_days_from_monday() >= 5)
        });
        let days = (0..if saturday { 6 } else { 5 })
            .map(|i| {
                let date = monday + Days::new(i);
                let number: i64 = untis_date(date).parse().unwrap_or_default();
                let holiday = holidays
                    .iter()
                    .find(|h| h.start_date <= number && number <= h.end_date)
                    .map(|h| {
                        if h.long_name.is_empty() {
                            h.name.clone()
                        } else {
                            h.long_name.clone()
                        }
                    });
                Day {
                    date: date.to_string(),
                    holiday,
                }
            })
            .collect();
        Ok(Week {
            monday: monday.to_string(),
            days,
            lessons,
            grid: self.grid().await,
            fetched_at,
            offline,
        })
    }

    /// Stunden von `from` bis einschließlich `to` (für die Heute-Ansicht).
    pub async fn range(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Lesson>> {
        self.lessons(from, to).await
    }

    /// Neue Ausfälle und Vertretungen der nächsten sieben Tage seit dem
    /// letzten Aufruf. Beim ersten Aufruf ist nichts neu.
    pub async fn check_changes(&self, today: NaiveDate) -> Result<Vec<Change>> {
        const KEY: &str = "watch";
        let lessons = self.lessons(today, today + Days::new(6)).await?;
        let stored: Option<(HashMap<i64, String>, i64)> = cache::load(self.db(), KEY)?;
        let signature = |l: &Lesson| {
            let names = |e: &[Element]| {
                e.iter()
                    .map(|e| e.name.as_str())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(",")
            };
            format!(
                "{}|{}|{}|{}",
                l.status,
                names(&l.teachers),
                names(&l.rooms),
                l.substitution.as_deref().unwrap_or_default()
            )
        };
        let current: HashMap<i64, String> = lessons.iter().map(|l| (l.id, signature(l))).collect();
        cache::store(self.db(), KEY, &current)?;
        let Some((before, _)) = stored else {
            return Ok(Vec::new());
        };
        let grid = self.grid().await;
        let period = |start: &str| {
            grid.iter()
                .find(|u| u.start == start)
                .map_or_else(|| start.to_owned(), |u| u.name.clone())
        };
        Ok(lessons
            .iter()
            .filter(|l| matches!(l.status.as_str(), "cancelled" | "changed"))
            .filter(|l| before.get(&l.id) != current.get(&l.id))
            .map(|l| Change {
                date: l.date.clone(),
                period: period(&l.start),
                subject: l
                    .subject
                    .as_ref()
                    .map_or_else(String::new, |s| s.long_name.clone()),
                kind: l.status.clone(),
                detail: l.substitution.clone().or_else(|| {
                    l.teachers
                        .iter()
                        .chain(&l.rooms)
                        .find(|e| e.previous.is_some())
                        .map(|e| {
                            format!("{} → {}", e.previous.as_deref().unwrap_or_default(), e.name)
                        })
                }),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(json: serde_json::Value) -> Lesson {
        lesson(&serde_json::from_value(json).unwrap())
    }

    #[test]
    fn classifies_lessons() {
        let base = json!({"id": 1, "date": 20261021, "startTime": 800, "endTime": 845,
            "su": [{"id": 3, "name": "M", "longname": "Mathematik"}],
            "te": [{"id": 4, "name": "MUE"}], "ro": [{"id": 5, "name": "R101"}]});
        let l = remote(base.clone());
        assert_eq!(
            (l.status.as_str(), l.start.as_str(), l.date.as_str()),
            ("regular", "08:00", "2026-10-21")
        );
        assert_eq!(l.subject.unwrap().long_name, "Mathematik");

        let mut cancelled = base.clone();
        cancelled["code"] = json!("cancelled");
        assert_eq!(remote(cancelled).status, "cancelled");

        let mut substituted = base.clone();
        substituted["te"] = json!([{"id": 6, "name": "SCH", "orgid": 4, "orgname": "MUE"}]);
        let l = remote(substituted);
        assert_eq!(l.status, "changed");
        assert_eq!(l.teachers[0].previous.as_deref(), Some("MUE"));
    }

    #[test]
    fn monday() {
        let wed = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
        assert_eq!(monday_of(wed).to_string(), "2026-10-19");
        let sun = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        assert_eq!(monday_of(sun).to_string(), "2026-10-19");
    }
}
