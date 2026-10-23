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
    /// Selbst in der App eingetragen, nicht aus WebUntis.
    #[serde(default)]
    pub local: bool,
}

/// Eine selbst eingetragene Prüfung.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewExam {
    /// ISO-Datum.
    pub date: String,
    /// `HH:MM`.
    pub start: Option<String>,
    pub subject: Option<String>,
    pub name: String,
    pub note: Option<String>,
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

    async fn exam_query(&self, query: &[(&str, String)]) -> Result<Vec<RemoteExam>> {
        let data: Data<Exams> = self.rest(false, "WebUntis/api/exams", query).await?;
        Ok(data.data.exams)
    }

    /// Prüfungen des letzten Monats und der nächsten sechs Monate. Schulen
    /// liefern sie je nach Einstellung nur für die Person oder nur für die
    /// Klasse, manche tragen sie nur im Stundenplan ein; daher alles zusammen.
    pub async fn exams(&self, today: NaiveDate) -> Result<Listed<Exam>> {
        let person = self.person()?;
        let from = today - Days::new(30);
        let to = today + Days::new(180);
        let fetch = async {
            let range = [
                ("startDate", untis_date(from)),
                ("endDate", untis_date(to)),
                ("withGrades", "true".to_owned()),
            ];
            let mut queries = vec![vec![
                ("studentId", person.id.to_string()),
                ("klasseId", "-1".to_owned()),
            ]];
            if let Some(klasse) = person.klasse_id {
                queries.push(vec![("klasseId", klasse.to_string())]);
            }
            let mut remote: Vec<RemoteExam> = Vec::new();
            let mut first_error = None;
            for mut query in queries {
                query.extend(range.iter().cloned());
                match self.exam_query(&query).await {
                    Ok(found) => {
                        for e in found {
                            if !remote.iter().any(|r| r.id == e.id) {
                                remote.push(e);
                            }
                        }
                    }
                    Err(e) => {
                        first_error.get_or_insert(e);
                    }
                }
            }
            let mut exams: Vec<Exam> = remote
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
                    local: false,
                })
                .collect();
            // Im Stundenplan als Prüfung markierte Stunden, die noch fehlen.
            let lessons = match self.range(today, to.min(today + Days::new(60))).await {
                Ok(lessons) => lessons,
                Err(e) => match (exams.is_empty(), first_error) {
                    (true, Some(first)) => return Err(first),
                    (true, None) => return Err(e),
                    _ => Vec::new(),
                },
            };
            for l in lessons.into_iter().filter(|l| l.status == "exam") {
                let subject = l.subject.as_ref().map(|s| s.long_name.clone());
                if exams
                    .iter()
                    .any(|e| e.date == l.date && e.subject == subject)
                {
                    continue;
                }
                exams.push(Exam {
                    id: -l.id,
                    kind: None,
                    name: l.text.clone().unwrap_or_default(),
                    subject,
                    date: l.date.clone(),
                    start: Some(l.start.clone()),
                    end: Some(l.end.clone()),
                    teachers: l.teachers.iter().map(|t| t.name.clone()).collect(),
                    rooms: l.rooms.iter().map(|r| r.name.clone()).collect(),
                    text: l.info.clone().or(l.substitution.clone()),
                    grade: None,
                    local: false,
                });
            }
            exams.sort_by(|a, b| (&a.date, &a.start).cmp(&(&b.date, &b.start)));
            Ok(exams)
        };
        let mut listed = self.cached("exams", fetch).await.or_else(|e| {
            // Ohne Verbindung und Zwischenstand zumindest die eigenen zeigen.
            match self.local_exams() {
                Ok(local) if !local.is_empty() => Ok(Listed {
                    items: Vec::new(),
                    fetched_at: 0,
                    offline: true,
                }),
                _ => Err(e),
            }
        })?;
        listed.items.extend(self.local_exams()?);
        listed
            .items
            .sort_by(|a, b| (&a.date, &a.start).cmp(&(&b.date, &b.start)));
        Ok(listed)
    }

    fn local_exams(&self) -> Result<Vec<Exam>> {
        let conn = self.db().conn();
        let mut stmt =
            conn.prepare("SELECT id, date, start, subject, name, note FROM untis_local_exams")?;
        let rows = stmt.query_map([], |r| {
            Ok(Exam {
                id: r.get(0)?,
                kind: None,
                date: r.get(1)?,
                start: r.get(2)?,
                end: None,
                subject: r.get(3)?,
                name: r.get(4)?,
                teachers: Vec::new(),
                rooms: Vec::new(),
                text: r.get(5)?,
                grade: None,
                local: true,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn add_exam(&self, exam: NewExam) -> Result<()> {
        let clean = |v: Option<String>| v.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        self.db().conn().execute(
            "INSERT INTO untis_local_exams (date, start, subject, name, note)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                exam.date,
                clean(exam.start),
                clean(exam.subject),
                exam.name.trim(),
                clean(exam.note),
            ),
        )?;
        Ok(())
    }

    pub fn delete_exam(&self, id: i64) -> Result<()> {
        self.db()
            .conn()
            .execute("DELETE FROM untis_local_exams WHERE id = ?1", [id])?;
        Ok(())
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
