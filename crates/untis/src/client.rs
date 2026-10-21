//! WebUntis: JSON-RPC (`jsonrpc.do`) für Stundenplan und Stammdaten, REST
//! (`api/…`) für Hausaufgaben, Prüfungen und Mitteilungen.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::{Error, Result};

const USER_AGENT: &str = concat!("TSA-Sync/", env!("CARGO_PKG_VERSION"));
const SCHOOL_SEARCH: &str = "https://mobile.webuntis.com/ms/schoolquery2";

/// Eine Schule aus der Schulsuche.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct School {
    /// Host, z. B. `nessa.webuntis.com`.
    pub server: String,
    /// Kennung der Schule für Anfragen (`?school=`).
    #[serde(alias = "loginName")]
    pub login_name: String,
    #[serde(alias = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub address: String,
}

/// Angemeldete Person: Schüler (5), Eltern (12) o. Ä.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub kind: i64,
    pub id: i64,
    pub klasse_id: Option<i64>,
    pub name: Option<String>,
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("HTTP-Client mit Standardeinstellungen")
}

fn rpc_error(error: &Value) -> Error {
    let code = error["code"].as_i64().unwrap_or_default();
    let message = error["message"].as_str().unwrap_or_default().to_owned();
    match code {
        -8520 => Error::SessionExpired,
        -8504 | -8502 | -8998 => Error::InvalidLogin,
        -6003 => Error::TooManySchools,
        _ => Error::Untis { code, message },
    }
}

fn rpc_body(method: &str, params: Value) -> Value {
    json!({ "id": "tsa-sync", "method": method, "params": params, "jsonrpc": "2.0" })
}

fn rpc_result(value: Value) -> Result<Value> {
    if let Some(error) = value.get("error").filter(|e| !e.is_null()) {
        return Err(rpc_error(error));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| Error::Response("ohne result".into()))
}

/// Sucht Schulen nach Name oder Ort.
pub async fn search_schools(query: &str) -> Result<Vec<School>> {
    search_schools_at(SCHOOL_SEARCH, query).await
}

pub async fn search_schools_at(endpoint: &str, query: &str) -> Result<Vec<School>> {
    #[derive(Deserialize)]
    struct Found {
        schools: Vec<School>,
    }
    let value: Value = http()
        .post(endpoint)
        .json(&rpc_body(
            "searchSchool",
            json!([{ "search": query.trim() }]),
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let found: Found = serde_json::from_value(rpc_result(value)?)?;
    Ok(found.schools)
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    /// `https://server/`, endet auf `/`.
    base: Url,
    school: String,
}

impl Client {
    pub fn new(base: Url, school: &str) -> Self {
        Self {
            http: http(),
            base,
            school: school.to_owned(),
        }
    }

    pub fn for_school(school: &School) -> Result<Self> {
        // Die Schulsuche liefert nur den Host; mit Schema nur in Tests.
        let base = if school.server.contains("://") {
            format!("{}/", school.server.trim_end_matches('/'))
        } else {
            format!("https://{}/", school.server)
        };
        let base = Url::parse(&base)
            .map_err(|_| Error::Response(format!("Ungültiger Server: {}", school.server)))?;
        Ok(Self::new(base, &school.login_name))
    }

    fn url(&self, path: &str) -> Url {
        self.base.join(path).expect("relativer Pfad")
    }

    fn cookies(&self, session: &str) -> String {
        format!(
            "JSESSIONID={session}; schoolname=\"_{}\"",
            STANDARD.encode(&self.school)
        )
    }

    /// JSON-RPC-Aufruf; ohne `session` nur für die Anmeldung.
    pub async fn rpc<T: DeserializeOwned>(
        &self,
        session: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<T> {
        let mut url = self.url("WebUntis/jsonrpc.do");
        url.query_pairs_mut().append_pair("school", &self.school);
        let mut request = self.http.post(url).json(&rpc_body(method, params));
        if let Some(session) = session {
            request = request.header(reqwest::header::COOKIE, self.cookies(session));
        }
        let value: Value = request.send().await?.error_for_status()?.json().await?;
        Ok(serde_json::from_value(rpc_result(value)?)?)
    }

    /// Anmeldung mit Passwort; liefert Sitzung und Person.
    pub async fn authenticate(&self, user: &str, password: &str) -> Result<(String, Person)> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Auth {
            session_id: String,
            #[serde(default)]
            person_type: i64,
            #[serde(default)]
            person_id: i64,
            #[serde(default)]
            klasse_id: Option<i64>,
        }
        let auth: Auth = self
            .rpc(
                None,
                "authenticate",
                json!({ "user": user, "password": password, "client": "TSA-Sync" }),
            )
            .await?;
        Ok((
            auth.session_id,
            Person {
                kind: auth.person_type,
                id: auth.person_id,
                klasse_id: auth.klasse_id.filter(|&k| k > 0),
                name: None,
            },
        ))
    }

    async fn intern(
        &self,
        method: &str,
        version: &str,
        params: Value,
    ) -> Result<reqwest::Response> {
        let mut url = self.url("WebUntis/jsonrpc_intern.do");
        url.query_pairs_mut()
            .append_pair("m", method)
            .append_pair("school", &self.school)
            .append_pair("v", version);
        Ok(self
            .http
            .post(url)
            .json(&rpc_body(method, params))
            .send()
            .await?
            .error_for_status()?)
    }

    /// App-Schlüssel, wie ihn die Untis-App statt des Passworts speichert.
    pub async fn app_secret(&self, user: &str, password: &str) -> Result<String> {
        let value: Value = self
            .intern(
                "getAppSharedSecret",
                "i3.5",
                json!([{ "userName": user, "password": password }]),
            )
            .await?
            .json()
            .await?;
        rpc_result(value)?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| Error::Response("leerer App-Schlüssel".into()))
    }

    /// Anmeldung mit Einmalcode aus dem App-Schlüssel; liefert die Sitzung.
    pub async fn otp_login(&self, user: &str, secret: &str, unix_millis: u64) -> Result<String> {
        let otp = crate::totp::code(secret, unix_millis / 1000)
            .ok_or_else(|| Error::Response("ungültiger App-Schlüssel".into()))?;
        let response = self
            .intern(
                "getUserData2017",
                "i2.2",
                json!([{ "auth": { "clientTime": unix_millis, "user": user, "otp": otp } }]),
            )
            .await?;
        let session = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .find_map(|part| part.trim().strip_prefix("JSESSIONID="))
            .map(str::to_owned);
        let value: Value = response.json().await?;
        rpc_result(value)?;
        session.ok_or_else(|| Error::Response("keine Sitzung".into()))
    }

    /// Wer angemeldet ist (nach Anmeldung mit App-Schlüssel nötig).
    pub async fn person(&self, session: &str) -> Result<Person> {
        let config: Value = self
            .rest(session, None, "WebUntis/api/app/config", &[])
            .await?;
        let user = &config["data"]["loginServiceConfig"]["user"];
        let id = user["personId"]
            .as_i64()
            .ok_or_else(|| Error::Response("ohne personId".into()))?;
        let person = user["persons"]
            .as_array()
            .and_then(|p| p.iter().find(|p| p["id"].as_i64() == Some(id)));
        let mut result = Person {
            kind: person.and_then(|p| p["type"].as_i64()).unwrap_or(5),
            id,
            klasse_id: None,
            name: person
                .and_then(|p| p["displayName"].as_str())
                .map(str::to_owned),
        };
        if let Ok(day) = self
            .rest::<Value>(session, None, "WebUntis/api/daytimetable/config", &[])
            .await
        {
            result.klasse_id = day["data"]["klasseId"].as_i64().filter(|&k| k > 0);
        }
        Ok(result)
    }

    /// GET auf die REST-Schnittstelle der Weboberfläche.
    pub async fn rest<T: DeserializeOwned>(
        &self,
        session: &str,
        bearer: Option<&str>,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let mut url = self.url(path);
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        let mut request = self
            .http
            .get(url)
            .header(reqwest::header::COOKIE, self.cookies(session));
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(Error::SessionExpired);
        }
        let response = response.error_for_status()?;
        let text = response.text().await?;
        // Abgelaufene Sitzungen landen teils auf der Login-Seite (HTML).
        serde_json::from_str(&text).map_err(|_| {
            if text.trim_start().starts_with('<') {
                Error::SessionExpired
            } else {
                Error::Response(text.chars().take(200).collect())
            }
        })
    }

    /// Bearer-Token für `api/rest/view/…`.
    pub async fn jwt(&self, session: &str) -> Result<String> {
        let response = self
            .http
            .get(self.url("WebUntis/api/token/new"))
            .header(reqwest::header::COOKIE, self.cookies(session))
            .send()
            .await?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(Error::SessionExpired);
        }
        let token = response.error_for_status()?.text().await?;
        let token = token.trim();
        if token.is_empty() || token.starts_with('<') {
            return Err(Error::SessionExpired);
        }
        Ok(token.to_owned())
    }

    pub fn base(&self) -> &Url {
        &self.base
    }
}
