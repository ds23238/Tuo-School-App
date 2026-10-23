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

/// Token aus `api/token/new`: je nach Version roher Text, JSON-String
/// oder ein Objekt mit `accessToken`.
fn token_from(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('<') {
        return None;
    }
    let token = match serde_json::from_str::<Value>(raw) {
        Ok(Value::String(s)) => s,
        Ok(Value::Object(map)) => ["accessToken", "token", "access_token", "jwt"]
            .iter()
            .find_map(|k| map.get(*k).and_then(Value::as_str))?
            .to_owned(),
        _ => raw.trim_matches('"').to_owned(),
    };
    let token = token.trim().to_owned();
    (!token.is_empty()).then_some(token)
}

/// Mandant (Schule) aus dem Token; manche Schnittstellen verlangen ihn.
fn tenant_id(token: &str) -> Option<String> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let tenant = value.get("tenant_id").or(value.get("tenantId"))?;
    match tenant {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Sucht Schulen nach Name oder Ort.
pub async fn search_schools(query: &str) -> Result<Vec<School>> {
    // Für Tests gegen einen nachgebauten Server.
    let endpoint =
        std::env::var("TSA_UNTIS_SCHOOL_SEARCH").unwrap_or_else(|_| SCHOOL_SEARCH.into());
    search_schools_at(&endpoint, query).await
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
        self.rest_with(session, bearer, path, query, &[]).await
    }

    /// Kopfzeilen für `api/rest/view/…`: Sitzung, Token und Mandant.
    fn view_request(
        &self,
        builder: reqwest::RequestBuilder,
        session: &str,
        bearer: Option<&str>,
        extra: &[(&str, String)],
    ) -> reqwest::RequestBuilder {
        let mut request = builder
            .header(reqwest::header::COOKIE, self.cookies(session))
            .header(reqwest::header::ACCEPT, "application/json");
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
            if let Some(tenant) = tenant_id(token) {
                request = request.header("Tenant-Id", tenant);
            }
        }
        for (k, v) in extra {
            request = request.header(*k, v);
        }
        request
    }

    pub async fn rest_with<T: DeserializeOwned>(
        &self,
        session: &str,
        bearer: Option<&str>,
        path: &str,
        query: &[(&str, String)],
        extra: &[(&str, String)],
    ) -> Result<T> {
        let mut url = self.url(path);
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        let request = self.view_request(self.http.get(url), session, bearer, extra);
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

    /// Sendet ein Formular wie die Weboberfläche: ein JSON-Teil `request`
    /// und Dateien als `attachments`. Liefert Status und Antworttext.
    pub async fn post_form(
        &self,
        session: &str,
        bearer: Option<&str>,
        path: &str,
        extra: &[(&str, String)],
        request_json: String,
        files: Vec<(String, Vec<u8>)>,
    ) -> Result<(u16, String)> {
        let json = reqwest::multipart::Part::text(request_json)
            .mime_str("application/json; charset=utf-8")
            .expect("gültiger MIME-Typ");
        let mut form = reqwest::multipart::Form::new().part("request", json);
        for (name, bytes) in files {
            form = form.part(
                "attachments",
                reqwest::multipart::Part::bytes(bytes).file_name(name),
            );
        }
        let request = self.view_request(self.http.post(self.url(path)), session, bearer, extra);
        let response = request.multipart(form).send().await?;
        let status = response.status().as_u16();
        if matches!(status, 401 | 403) {
            return Err(Error::SessionExpired);
        }
        Ok((status, response.text().await.unwrap_or_default()))
    }

    /// Lädt eine Datei. Auf dem Untis-Server mit Sitzung und Token, fremde
    /// Speicher-Links (vorsigniert) ohne Kopfzeilen.
    pub async fn get_bytes(
        &self,
        session: &str,
        bearer: Option<&str>,
        url: &str,
    ) -> Result<(u16, Option<String>, Vec<u8>)> {
        let url = self
            .base
            .join(url.trim_start_matches('/'))
            .map_err(|e| Error::Response(e.to_string()))?;
        let own = url.host_str() == self.base.host_str();
        let request = if own {
            self.view_request(self.http.get(url), session, bearer, &[])
        } else {
            self.http.get(url)
        };
        let response = request.send().await?;
        let status = response.status().as_u16();
        if own && matches!(status, 401 | 403) {
            return Err(Error::SessionExpired);
        }
        let kind = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        Ok((status, kind, response.bytes().await?.to_vec()))
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
        let raw = response.error_for_status()?.text().await?;
        token_from(&raw).ok_or(Error::SessionExpired)
    }

    pub fn base(&self) -> &Url {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_in_all_shapes() {
        assert_eq!(token_from("abc.def.ghi").as_deref(), Some("abc.def.ghi"));
        assert_eq!(token_from("\"abc\"").as_deref(), Some("abc"));
        assert_eq!(
            token_from(r#"{"accessToken":"xyz"}"#).as_deref(),
            Some("xyz")
        );
        assert_eq!(token_from("<html>"), None);
        assert_eq!(token_from(""), None);
    }

    #[test]
    fn tenant_from_jwt() {
        // {"tenant_id":"4711"} als Base64url
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let token = format!("x.{}.y", URL_SAFE_NO_PAD.encode(r#"{"tenant_id":"4711"}"#));
        assert_eq!(tenant_id(&token).as_deref(), Some("4711"));
        assert_eq!(tenant_id("kein-jwt"), None);
    }
}
