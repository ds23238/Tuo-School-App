//! Abstimmungen: Optionen, eigene Wahl, Ergebnisse.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::activities::Cached;
use crate::{Error, Result, Session, plain};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: i64,
    pub options: Vec<ChoiceOption>,
    pub multiple: bool,
    /// Eigene Wahl ändern oder zurückziehen.
    pub can_update: bool,
    pub answered: bool,
    pub open: bool,
    pub opens: Option<i64>,
    pub closes: Option<i64>,
    /// Ergebnisse sind sichtbar (Einstellung der Lehrkraft).
    pub results: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub id: i64,
    pub text: String,
    pub checked: bool,
    pub disabled: bool,
    /// Nur bei sichtbaren Ergebnissen.
    pub votes: Option<i64>,
    pub percent: Option<f64>,
}

impl Session<'_> {
    async fn choice_id(&self, course_id: i64, module_id: i64) -> Result<(i64, Value)> {
        let v: Value = self
            .call(
                "mod_choice_get_choices_by_courses",
                &[("courseids[0]", course_id.to_string())],
            )
            .await?;
        let choice = v["choices"]
            .as_array()
            .and_then(|list| {
                list.iter()
                    .find(|c| c["coursemodule"].as_i64() == Some(module_id))
            })
            .cloned()
            .ok_or(Error::FileNotFound)?;
        Ok((choice["id"].as_i64().ok_or(Error::FileNotFound)?, choice))
    }

    pub async fn choice(&self, course_id: i64, module_id: i64) -> Result<Cached<Choice>> {
        self.cached(&format!("choice:{module_id}"), || async {
            let (id, settings) = self.choice_id(course_id, module_id).await?;
            let options: Value = self
                .call(
                    "mod_choice_get_choice_options",
                    &[("choiceid", id.to_string())],
                )
                .await?;
            let options = options["options"].as_array().cloned().unwrap_or_default();
            let answered = options.iter().any(|o| o["checked"] == true);
            let now = crate::contents::now();
            let opens = settings["timeopen"].as_i64().filter(|t| *t > 0);
            let closes = settings["timeclose"].as_i64().filter(|t| *t > 0);
            let open = opens.is_none_or(|t| t <= now) && closes.is_none_or(|t| t > now);
            // 0 nie, 1 nach eigener Wahl, 2 nach Schluss, 3 immer.
            let results = match settings["showresults"].as_i64().unwrap_or(0) {
                1 => answered,
                2 => closes.is_some_and(|t| t <= now),
                3 => true,
                _ => false,
            };
            let counts: Value = if results {
                self.call(
                    "mod_choice_get_choice_results",
                    &[("choiceid", id.to_string())],
                )
                .await
                .unwrap_or_default()
            } else {
                Value::Null
            };
            let count = |option: i64| {
                counts["options"].as_array().and_then(|list| {
                    list.iter()
                        .find(|o| o["id"].as_i64() == Some(option))
                        .map(|o| {
                            (
                                o["numberofuser"].as_i64().unwrap_or(0),
                                o["percentageamount"].as_f64().unwrap_or(0.0),
                            )
                        })
                })
            };
            Ok(Choice {
                id,
                options: options
                    .iter()
                    .filter_map(|o| {
                        let option = o["id"].as_i64()?;
                        let counted = count(option);
                        Some(ChoiceOption {
                            id: option,
                            text: plain(o["text"].as_str().unwrap_or_default()),
                            checked: o["checked"].as_bool().unwrap_or(false),
                            disabled: o["disabled"].as_bool().unwrap_or(false),
                            votes: counted.map(|c| c.0),
                            percent: counted.map(|c| c.1),
                        })
                    })
                    .collect(),
                multiple: settings["allowmultiple"].as_bool().unwrap_or(false),
                can_update: settings["allowupdate"].as_bool().unwrap_or(false),
                answered,
                open,
                opens,
                closes,
                results,
            })
        })
        .await
    }

    /// Stimmt ab; eine leere Auswahl zieht die eigene Wahl zurück.
    pub async fn choice_answer(&self, choice_id: i64, options: &[i64]) -> Result<()> {
        if options.is_empty() {
            let _: Value = self
                .call(
                    "mod_choice_delete_choice_responses",
                    &[("choiceid", choice_id.to_string())],
                )
                .await?;
            return Ok(());
        }
        let keys: Vec<String> = (0..options.len())
            .map(|i| format!("responses[{i}]"))
            .collect();
        let mut params = vec![("choiceid", choice_id.to_string())];
        for (key, option) in keys.iter().zip(options) {
            params.push((key.as_str(), option.to_string()));
        }
        let _: Value = self
            .call("mod_choice_submit_choice_response", &params)
            .await?;
        Ok(())
    }
}
