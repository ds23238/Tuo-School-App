//! Hausaufgaben und Prüfungen.

use std::collections::HashMap;

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, Session, cache, iso_date, time, untis_date};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Homework {
    pub id: i64,
    pub subject: Option<String>,
    pub teacher: Option<String>,
    /// ISO-Datum, an dem sie aufgegeben wurde.
    pub assigned: String,
    pub due: String,
    pub text: String,
    pub remark: Option<String>,
    /// In WebUntis erledigt oder in der App abgehakt.
    pub done: bool,
    pub attachments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    pub id: i64,
    pub kind: Option<String>,
    pub name: String,
    pub subject: Option<String>,
    pub date: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub teachers: Vec<String>,
    pub rooms: Vec<String>,
    pub text: Option<String>,
    pub grade: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listed<T> {
    pub items: Vec<T>,
    pub fetched_at: i64,
    pub offline: bool,
}

#[derive(Debug, Deserialize)]
struct Data<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteHomework {
    id: i64,
    lesson_id: i64,
    date: i64,
    due_date: i64,
    #[serde(default)]
    text: String,
    #[serde(default)]
    remark: Option<String>,
    #[serde(default)]
    completed: bool,
    #[serde(default)]
    attachments: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    homework_id: i64,
    #[serde(default)]
    teacher_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct Named {
    id: i64,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct RemoteLesson {
    id: i64,
    #[serde(default)]
    subject: String,
}

#[derive(Debug, Deserialize)]
struct Homeworks {
    #[serde(default)]
    records: Vec<Record>,
    #[serde(default)]
    homeworks: Vec<RemoteHomework>,
    #[serde(default)]
    teachers: Vec<Named>,
    #[serde(default)]
    lessons: Vec<RemoteLesson>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteExam {
    id: i64,
    #[serde(default)]
    exam_type: Option<String>,
    #[serde(default)]
    name: String,
    exam_date: i64,
    #[serde(default)]
    start_time: Option<i64>,
    #[serde(default)]
    end_time: Option<i64>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    teachers: Vec<String>,
    #[serde(default)]
    rooms: Vec<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    grade: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Exams {
    #[serde(default)]
    exams: Vec<RemoteExam>,
}

fn some(text: Option<String>) -> Option<String> {
    text.map(|t| crate::plain(&t)).filter(|t| !t.is_empty())
}

fn homeworks(data: Homeworks, done: &[i64]) -> Vec<Homework> {
    let teachers: HashMap<i64, String> =
        data.teachers.into_iter().map(|t| (t.id, t.name)).collect();
    let by_homework: HashMap<i64, i64> = data
        .records
        .iter()
        .filter_map(|r| Some((r.homework_id, r.teacher_id?)))
        .collect();
    let subjects: HashMap<i64, String> = data
        .lessons
        .into_iter()
        .map(|l| (l.id, l.subject))
        .collect();
    let mut list: Vec<Homework> = data
        .homeworks
        .into_iter()
        .map(|h| Homework {
            id: h.id,
            subject: subjects
                .get(&h.lesson_id)
                .cloned()
                .filter(|s| !s.is_empty()),
            teacher: by_homework
                .get(&h.id)
                .and_then(|t| teachers.get(t))
                .cloned(),
            assigned: iso_date(h.date),
            due: iso_date(h.due_date),
            text: crate::plain(&h.text),
            remark: some(h.remark),
            done: h.completed || done.contains(&h.id),
            attachments: h
                .attachments
                .iter()
                .filter_map(|a| a["name"].as_str().map(str::to_owned))
                .collect(),
        })
        .collect();
    list.sort_by(|a, b| a.due.cmp(&b.due));
    list
}

impl Session<'_> {
    /// Lädt neu oder nimmt offline den zuletzt geladenen Stand.
    async fn cached<T, F>(&self, key: &str, fetch: F) -> Result<Listed<T>>
    where
        T: Serialize + serde::de::DeserializeOwned,
        F: Future<Output = Result<Vec<T>>>,
    {
        match fetch.await {
            Ok(items) => {
                let at = cache::store(self.db(), key, &items)?;
                Ok(Listed {
                    items,
                    fetched_at: at,
                    offline: false,
                })
            }
            Err(e @ (Error::Network(_) | Error::SessionExpired)) => {
                match cache::load::<Vec<T>>(self.db(), key)? {
                    Some((items, at)) => Ok(Listed {
                        items,
                        fetched_at: at,
                        offline: true,
                    }),
                    None => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }

    fn done_ids(&self) -> Result<Vec<i64>> {
        let conn = self.db().conn();
        let mut stmt = conn.prepare("SELECT id FROM untis_homework_done")?;
        let ids = stmt.query_map([], |r| r.get(0))?;
        Ok(ids.collect::<Result<_, _>>()?)
    }

    /// Hausaufgaben von zwei Wochen zurück bis fünf Wochen voraus.
    pub async fn homework(&self, today: NaiveDate) -> Result<Listed<Homework>> {
        let done = self.done_ids()?;
        let fetch = async {
            let data: Data<Homeworks> = self
                .rest(
                    false,
                    "WebUntis/api/homeworks/lessons",
                    &[
                        ("startDate", untis_date(today - Days::new(14))),
                        ("endDate", untis_date(today + Days::new(35))),
                    ],
                )
                .await?;
            Ok(homeworks(data.data, &done))
        };
        let mut listed = self.cached("homework", fetch).await?;
        for h in &mut listed.items {
            h.done |= done.contains(&h.id);
        }
        Ok(listed)
    }

    /// Abhaken gilt nur in der App; WebUntis bleibt unverändert.
    pub fn set_homework_done(&self, id: i64, done: bool) -> Result<()> {
        let sql = if done {
            "INSERT OR IGNORE INTO untis_homework_done (id) VALUES (?1)"
        } else {
            "DELETE FROM untis_homework_done WHERE id = ?1"
        };
        self.db().conn().execute(sql, [id])?;
        Ok(())
    }

    /// Prüfungen der letzten Woche und der nächsten vier Monate.
    pub async fn exams(&self, today: NaiveDate) -> Result<Listed<Exam>> {
        let person = self.person()?;
        let fetch = async {
            let data: Data<Exams> = self
                .rest(
                    false,
                    "WebUntis/api/exams",
                    &[
                        ("studentId", person.id.to_string()),
                        ("klasseId", person.klasse_id.unwrap_or(-1).to_string()),
                        ("startDate", untis_date(today - Days::new(7))),
                        ("endDate", untis_date(today + Days::new(120))),
                        ("withGrades", "true".into()),
                    ],
                )
                .await?;
            let mut exams: Vec<Exam> = data
                .data
                .exams
                .into_iter()
                .map(|e| Exam {
                    id: e.id,
                    kind: e.exam_type.filter(|t| !t.is_empty()),
                    name: e.name,
                    subject: e.subject.filter(|s| !s.is_empty()),
                    date: iso_date(e.exam_date),
                    start: e.start_time.map(time),
                    end: e.end_time.map(time),
                    teachers: e.teachers,
                    rooms: e.rooms,
                    text: some(e.text),
                    grade: e.grade.filter(|g| !g.is_empty()),
                })
                .collect();
            exams.sort_by(|a, b| (&a.date, &a.start).cmp(&(&b.date, &b.start)));
            Ok(exams)
        };
        self.cached("exams", fetch).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_homework_with_lessons_and_teachers() {
        let data: Homeworks = serde_json::from_value(serde_json::json!({
            "records": [{"homeworkId": 9, "teacherId": 2, "elementIds": [1]}],
            "homeworks": [{"id": 9, "lessonId": 4, "date": 20261020, "dueDate": 20261027,
                "text": "S. 12 Nr. 3<br>und 4", "remark": "", "completed": false,
                "attachments": [{"id": 1, "name": "Blatt.pdf"}]}],
            "teachers": [{"id": 2, "name": "Müller"}],
            "lessons": [{"id": 4, "subject": "Mathematik", "lessonType": "Unterricht"}],
        }))
        .unwrap();
        let list = homeworks(data, &[]);
        let h = &list[0];
        assert_eq!(h.subject.as_deref(), Some("Mathematik"));
        assert_eq!(h.teacher.as_deref(), Some("Müller"));
        assert_eq!(
            (h.assigned.as_str(), h.due.as_str()),
            ("2026-10-20", "2026-10-27")
        );
        assert_eq!(h.text, "S. 12 Nr. 3\nund 4");
        assert_eq!(h.remark, None);
        assert_eq!(h.attachments, ["Blatt.pdf"]);
    }
}
