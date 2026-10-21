use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use url::Url;

use crate::{Error, Result};

/// Name des Webservice, den auch die offizielle Moodle-App nutzt.
const SERVICE: &str = "moodle_mobile_app";

/// Pfadteile, ab denen eine eingefügte Adresse nicht mehr zur Basis gehört,
/// z. B. `https://moodle.example/login/index.php`.
const PAGE_SEGMENTS: &[&str] = &["admin", "course", "index.php", "login", "mod", "my", "user"];

/// Öffentliche Angaben der Seite, abrufbar ohne Anmeldung.
#[derive(Debug, Clone, Deserialize)]
pub struct PublicConfig {
    pub wwwroot: String,
    pub httpswwwroot: Option<String>,
    pub sitename: String,
    /// 1 = Login in der App, 2/3 = Login im Browser (SSO).
    #[serde(default = "one")]
    pub typeoflogin: i64,
    #[serde(default = "one")]
    pub enablewebservices: i64,
    #[serde(default = "one")]
    pub enablemobilewebservice: i64,
}

fn one() -> i64 {
    1
}

impl PublicConfig {
    pub fn canonical_url(&self) -> Result<Url> {
        normalize_url(self.httpswwwroot.as_deref().unwrap_or(&self.wwwroot))
    }

    pub fn sso_only(&self) -> bool {
        self.typeoflogin != 1
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SiteInfo {
    pub userid: i64,
    pub fullname: String,
    pub sitename: String,
}

/// Bringt eine eingegebene Adresse in die Form `https://host/pfad/`.
pub fn normalize_url(input: &str) -> Result<Url> {
    let input = input.trim();
    let with_scheme = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    let invalid = || Error::InvalidUrl(input.to_owned());
    let mut url = Url::parse(&with_scheme).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(invalid());
    }
    url.set_query(None);
    url.set_fragment(None);
    let segments: Vec<String> = url
        .path_segments()
        .into_iter()
        .flatten()
        .take_while(|s| !PAGE_SEGMENTS.contains(s))
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let mut path = segments.join("/");
    path.push('/');
    url.set_path(&path);
    Ok(url)
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    /// Endet immer auf `/`, damit `join` relative Pfade anhängt.
    base: Url,
}

impl Client {
    pub fn new(base: Url) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("TSA-Sync/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("HTTP-Client mit Standardeinstellungen");
        Self { http, base }
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    fn url(&self, path: &str) -> Url {
        self.base.join(path).expect("relativer Pfad")
    }

    pub async fn public_config(&self) -> Result<PublicConfig> {
        let mut url = self.url("lib/ajax/service-nologin.php");
        url.set_query(Some("info=tool_mobile_get_public_config"));
        let body = json!([{
            "index": 0,
            "methodname": "tool_mobile_get_public_config",
            "args": {},
        }]);
        let response = self.http.post(url).json(&body).send().await?;
        // Alles, was keine passende Antwort liefert, ist für uns kein Moodle.
        if !response.status().is_success() {
            return Err(Error::NotMoodle);
        }
        let value: Value = response.json().await.map_err(|_| Error::NotMoodle)?;
        let data = value
            .get(0)
            .filter(|r| r["error"] == false)
            .map(|r| r["data"].clone())
            .ok_or(Error::NotMoodle)?;
        serde_json::from_value(data).map_err(|_| Error::NotMoodle)
    }

    /// Meldet an und liefert das Token.
    pub async fn login(&self, username: &str, password: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct TokenResponse {
            token: Option<String>,
            errorcode: Option<String>,
            error: Option<String>,
        }
        let response: TokenResponse = self
            .http
            .post(self.url("login/token.php"))
            .form(&[
                ("username", username),
                ("password", password),
                ("service", SERVICE),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        match response {
            TokenResponse {
                token: Some(token), ..
            } => Ok(token),
            TokenResponse {
                errorcode: Some(code),
                ..
            } if code == "invalidlogin" => Err(Error::InvalidLogin),
            TokenResponse {
                errorcode, error, ..
            } => Err(Error::Moodle {
                code: errorcode.unwrap_or_default(),
                message: error.unwrap_or_default(),
            }),
        }
    }

    /// Ruft eine Webservice-Funktion auf. `params` in der von Moodle
    /// erwarteten flachen Form, z. B. `("courseids[0]", "5")`.
    pub async fn call<T: DeserializeOwned>(
        &self,
        token: &str,
        function: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let mut url = self.url("webservice/rest/server.php");
        url.query_pairs_mut()
            .append_pair("moodlewsrestformat", "json")
            .append_pair("wsfunction", function);
        let mut form: Vec<(&str, &str)> = vec![("wstoken", token)];
        form.extend(params.iter().map(|(k, v)| (*k, v.as_str())));
        let value: Value = self
            .http
            .post(url)
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(code) = value.get("exception").and(value.get("errorcode")) {
            let code = code.as_str().unwrap_or_default();
            return Err(match code {
                "invalidtoken" => Error::InvalidToken,
                _ => Error::Moodle {
                    code: code.to_owned(),
                    message: value["message"].as_str().unwrap_or_default().to_owned(),
                },
            });
        }
        Ok(serde_json::from_value(value)?)
    }

    /// Lädt eine Datei; liefert Inhalt und Content-Type.
    pub async fn download(&self, url: Url) -> Result<(Vec<u8>, Option<String>)> {
        let response = self.http.get(url).send().await?.error_for_status()?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        Ok((response.bytes().await?.to_vec(), content_type))
    }

    /// Macht eine Datei-Adresse der Seite mit dem Token abrufbar:
    /// `pluginfile.php` braucht dafür den Webservice-Pfad. Fremde Adressen
    /// bleiben unverändert und bekommen kein Token.
    pub fn authorize(&self, url: &Url, token: &str) -> Url {
        let same_site =
            url.origin() == self.base.origin() && url.path().starts_with(self.base.path());
        if !same_site {
            return url.clone();
        }
        let mut url = url.clone();
        let relative = url.path()[self.base.path().len()..].to_owned();
        if let Some(rest) = relative.strip_prefix("pluginfile.php/") {
            url.set_path(&format!(
                "{}webservice/pluginfile.php/{rest}",
                self.base.path()
            ));
        }
        if relative.contains("pluginfile.php/") {
            url.query_pairs_mut().append_pair("token", token);
        }
        url
    }

    pub async fn site_info(&self, token: &str) -> Result<SiteInfo> {
        self.call(token, "core_webservice_get_site_info", &[]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize() {
        let cases = [
            ("moodle.example", "https://moodle.example/"),
            ("  https://moodle.example/  ", "https://moodle.example/"),
            ("http://moodle.example", "http://moodle.example/"),
            ("https://example.org/moodle", "https://example.org/moodle/"),
            (
                "https://example.org/moodle/login/index.php?x=1#y",
                "https://example.org/moodle/",
            ),
            ("https://moodle.example/my/", "https://moodle.example/"),
            (
                "https://moodle.example/course/view.php?id=4",
                "https://moodle.example/",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(normalize_url(input).unwrap().as_str(), expected, "{input}");
        }
    }

    #[test]
    fn authorize() {
        let client = Client::new(Url::parse("https://e.org/moodle/").unwrap());
        let cases = [
            (
                "https://e.org/moodle/pluginfile.php/1/a.png",
                "https://e.org/moodle/webservice/pluginfile.php/1/a.png?token=T",
            ),
            (
                "https://e.org/moodle/webservice/pluginfile.php/1/a.pdf?forcedownload=1",
                "https://e.org/moodle/webservice/pluginfile.php/1/a.pdf?forcedownload=1&token=T",
            ),
            (
                "https://e.org/moodle/theme/image.php/boost/f/pdf",
                "https://e.org/moodle/theme/image.php/boost/f/pdf",
            ),
            (
                "https://e.org/other/pluginfile.php/1/a.png",
                "https://e.org/other/pluginfile.php/1/a.png",
            ),
            (
                "https://evil.example/pluginfile.php/1/a",
                "https://evil.example/pluginfile.php/1/a",
            ),
        ];
        for (input, expected) in cases {
            let url = Url::parse(input).unwrap();
            assert_eq!(client.authorize(&url, "T").as_str(), expected, "{input}");
        }
    }

    #[test]
    fn normalize_rejects() {
        for input in ["", "ftp://moodle.example", "https://"] {
            assert!(normalize_url(input).is_err(), "{input}");
        }
    }
}
