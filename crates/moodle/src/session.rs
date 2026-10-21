use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use tsa_core::{Account, Database, SecretKind, SecretStore, Service};
use url::Url;

use crate::client::normalize_url;
use crate::contents::{self, CourseView, RemoteSection};
use crate::courses::RemoteCourse;
use crate::html::MediaUrls;
use crate::sync::{self, Files};
use crate::{Client, Course, Error, Result};

type Download = std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Ergebnis der Prüfung einer Adresse vor dem Login.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCheck {
    pub url: String,
    pub name: String,
    /// Die Seite meldet Login nur über den Browser (SSO).
    pub sso_only: bool,
}

/// Moodle-Zugriff mit dem gespeicherten Konto.
pub struct Session<'a> {
    db: &'a Database,
    secrets: &'a dyn SecretStore,
}

impl<'a> Session<'a> {
    pub fn new(db: &'a Database, secrets: &'a dyn SecretStore) -> Self {
        Self { db, secrets }
    }

    pub async fn check_site(url: &str) -> Result<SiteCheck> {
        let config = Client::new(normalize_url(url)?).public_config().await?;
        if config.enablewebservices == 0 || config.enablemobilewebservice == 0 {
            return Err(Error::WebServicesDisabled);
        }
        Ok(SiteCheck {
            url: config.canonical_url()?.to_string(),
            sso_only: config.sso_only(),
            name: crate::plain(&config.sitename),
        })
    }

    /// Meldet an und ersetzt ein vorhandenes Moodle-Konto. Das Passwort wird
    /// nur mit `remember` gespeichert.
    pub async fn login(
        &self,
        url: &str,
        username: &str,
        password: &str,
        remember: bool,
    ) -> Result<Account> {
        let site = Self::check_site(url).await?;
        let client = Client::new(normalize_url(&site.url)?);
        let tokens = client.login(username, password).await?;
        let info = client.site_info(&tokens.token).await?;

        let previous = Account::get(self.db, Service::Moodle)?;
        if previous.is_some_and(|p| p.base_url != site.url || p.username != username) {
            self.forget_data()?;
        }
        self.store_tokens(&tokens)?;
        if remember {
            self.secrets
                .set(Service::Moodle, SecretKind::Password, password)?;
        } else {
            self.secrets.delete(Service::Moodle, SecretKind::Password)?;
        }
        let mut account = Account::new(Service::Moodle, site.url, username.to_owned());
        account.display_name = Some(crate::plain(&info.fullname));
        account.save(self.db)?;
        Ok(account)
    }

    fn store_tokens(&self, tokens: &crate::client::Tokens) -> Result<()> {
        self.secrets
            .set(Service::Moodle, SecretKind::Token, &tokens.token)?;
        match &tokens.private_token {
            Some(p) => self
                .secrets
                .set(Service::Moodle, SecretKind::PrivateToken, p)?,
            None => self
                .secrets
                .delete(Service::Moodle, SecretKind::PrivateToken)?,
        }
        Ok(())
    }

    /// Entfernt Kurse, Inhalte und Dateiliste; Dateien im Sync-Ordner bleiben.
    fn forget_data(&self) -> Result<()> {
        Course::clear(self.db)?;
        contents::clear(self.db)?;
        sync::clear(self.db)
    }

    pub fn sign_out(&self) -> Result<()> {
        Account::remove(self.db, self.secrets, Service::Moodle)?;
        self.forget_data()
    }

    fn client(&self) -> Result<(Client, Account)> {
        let account = Account::get(self.db, Service::Moodle)?.ok_or(Error::NotSignedIn)?;
        Ok((Client::new(normalize_url(&account.base_url)?), account))
    }

    /// Lädt die Kursliste neu und gibt den gespeicherten Stand zurück.
    pub async fn refresh_courses(&self) -> Result<Vec<Course>> {
        let info: crate::SiteInfo = self.call("core_webservice_get_site_info", &[]).await?;
        let remote: Vec<RemoteCourse> = self
            .call(
                "core_enrol_get_users_courses",
                &[("userid", info.userid.to_string())],
            )
            .await?;
        Course::store_enrolled(self.db, &remote)?;
        Course::list(self.db)
    }

    async fn view(
        &self,
        course_id: i64,
        sections: Vec<RemoteSection>,
        fetched_at: i64,
        media: &MediaUrls,
        root: &Path,
    ) -> Result<CourseView> {
        let (client, _) = self.client()?;
        let states = Files { db: self.db, root }.states(course_id).await?;
        Ok(contents::view(
            course_id,
            sections,
            fetched_at,
            client.base(),
            media,
            &states,
        ))
    }

    /// Zuletzt geladener Stand eines Kurses, auch offline.
    pub async fn cached_course(
        &self,
        course_id: i64,
        media: &MediaUrls,
        root: &Path,
    ) -> Result<Option<CourseView>> {
        let Some((sections, fetched_at)) = contents::load(self.db, course_id)? else {
            return Ok(None);
        };
        Ok(Some(
            self.view(course_id, sections, fetched_at, media, root)
                .await?,
        ))
    }

    /// Lädt die Inhalte neu und trägt die Dateien in die Dateiliste ein.
    async fn fetch_contents(&self, course_id: i64) -> Result<(Vec<RemoteSection>, i64)> {
        let raw: serde_json::Value = self
            .call(
                "core_course_get_contents",
                &[("courseid", course_id.to_string())],
            )
            .await?;
        let sections: Vec<RemoteSection> = serde_json::from_value(raw.clone())?;
        let fetched_at = contents::store(self.db, course_id, &raw)?;
        let name = Course::list(self.db)?
            .into_iter()
            .find(|c| c.id == course_id)
            .map_or_else(|| format!("Kurs {course_id}"), |c| c.full_name);
        sync::record(self.db, course_id, &sync::collect(&name, &sections))?;
        Ok((sections, fetched_at))
    }

    pub async fn refresh_course(
        &self,
        course_id: i64,
        media: &MediaUrls,
        root: &Path,
    ) -> Result<CourseView> {
        let (sections, fetched_at) = self.fetch_contents(course_id).await?;
        self.view(course_id, sections, fetched_at, media, root)
            .await
    }

    /// Download-Funktion mit dem aktuellen Token für [`Files`].
    fn downloader(&self) -> Result<impl Fn(String, PathBuf) -> Download> {
        let (client, _) = self.client()?;
        let token = self
            .secrets
            .get(Service::Moodle, SecretKind::Token)?
            .ok_or(Error::InvalidToken)?;
        Ok(move |url: String, path: PathBuf| {
            let (client, token) = (client.clone(), token.clone());
            Box::pin(async move {
                let url = Url::parse(&url).map_err(|_| Error::InvalidUrl(url))?;
                client
                    .download_to(client.authorize(&url, &token), &path)
                    .await
            }) as Download
        })
    }

    /// Synchronisiert alle ausgewählten Kurse; liefert die Zahl neu
    /// geladener Dateien je Kurs. Ein Fehler in einem Kurs bricht nicht ab.
    pub async fn sync_all(&self, root: &Path) -> Result<Vec<CourseSync>> {
        let courses: Vec<Course> = Course::list(self.db)?
            .into_iter()
            .filter(|c| c.sync)
            .collect();
        let mut results = Vec::new();
        for course in courses {
            let result = self.sync_course(course.id, root).await;
            results.push(CourseSync {
                course_id: course.id,
                downloaded: *result.as_ref().unwrap_or(&0),
                error: result.err().map(|e| e.to_string()),
            });
        }
        Ok(results)
    }

    pub async fn sync_course(&self, course_id: i64, root: &Path) -> Result<usize> {
        self.fetch_contents(course_id).await?;
        let files = Files { db: self.db, root };
        files.sync(course_id, self.downloader()?).await
    }

    /// Pfad der lokalen Kopie zum Öffnen; lädt sie bei Bedarf.
    pub async fn local_file(&self, file_id: i64, root: &Path) -> Result<PathBuf> {
        Files { db: self.db, root }
            .open(file_id, self.downloader()?)
            .await
    }

    /// Holt die neue Version einer Datei, die alte bleibt umbenannt erhalten.
    pub async fn update_file(&self, file_id: i64, root: &Path) -> Result<PathBuf> {
        Files { db: self.db, root }
            .update(file_id, self.downloader()?)
            .await
    }

    pub fn file_stats(&self) -> Result<HashMap<i64, sync::CourseFiles>> {
        sync::course_stats(self.db)
    }

    /// Adresse, die `target` im Browser bereits angemeldet öffnet. Klappt
    /// das nicht (Moodle erlaubt nur alle paar Minuten einen Schlüssel),
    /// kommt `target` unverändert zurück.
    pub async fn autologin_url(&self, target: &str) -> Result<String> {
        let (client, _) = self.client()?;
        let Ok(target_url) = Url::parse(target) else {
            return Ok(target.to_owned());
        };
        if target_url.origin() != client.base().origin() {
            return Ok(target.to_owned());
        }
        let Some(private) = self
            .secrets
            .get(Service::Moodle, SecretKind::PrivateToken)?
        else {
            return Ok(target.to_owned());
        };
        #[derive(serde::Deserialize)]
        struct Key {
            key: String,
            autologinurl: String,
        }
        let key: Result<Key> = self
            .call(
                "tool_mobile_get_autologin_key",
                &[("privatetoken", private)],
            )
            .await;
        let info: Result<crate::SiteInfo> = self.call("core_webservice_get_site_info", &[]).await;
        match (key, info) {
            (Ok(key), Ok(info)) => {
                let mut url = Url::parse(&key.autologinurl)
                    .map_err(|_| Error::InvalidUrl(key.autologinurl.clone()))?;
                url.query_pairs_mut()
                    .append_pair("userid", &info.userid.to_string())
                    .append_pair("key", &key.key)
                    .append_pair("urltogo", target);
                Ok(url.to_string())
            }
            _ => Ok(target.to_owned()),
        }
    }

    /// Setzt den manuellen Abschluss einer Aktivität.
    pub async fn set_completion(&self, module_id: i64, completed: bool) -> Result<()> {
        let _: serde_json::Value = self
            .call(
                "core_completion_update_activity_completion_status_manually",
                &[
                    ("cmid", module_id.to_string()),
                    ("completed", u8::from(completed).to_string()),
                ],
            )
            .await?;
        Ok(())
    }

    /// Lädt ein eingebettetes Bild o. Ä.; Dateien der Seite mit Token.
    pub async fn fetch_media(&self, url: &Url) -> Result<(Vec<u8>, Option<String>)> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::InvalidUrl(url.to_string()));
        }
        let (client, _) = self.client()?;
        let url = match self.secrets.get(Service::Moodle, SecretKind::Token)? {
            Some(token) => client.authorize(url, &token),
            None => url.clone(),
        };
        client.download(url).await
    }

    /// Lädt eine Datei der Seite (mit Token) als Text.
    pub(crate) async fn fetch_text(&self, url: &Url) -> Result<String> {
        let (bytes, _) = self.fetch_media(url).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Ruft eine Funktion mit dem gespeicherten Token auf. Ist das Token
    /// abgelaufen und ein Passwort gespeichert, wird einmal neu angemeldet.
    pub(crate) async fn call<T: DeserializeOwned>(
        &self,
        function: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let (client, account) = self.client()?;
        if let Some(token) = self.secrets.get(Service::Moodle, SecretKind::Token)? {
            match client.call(&token, function, params).await {
                Err(Error::InvalidToken) => {}
                result => return result,
            }
        }
        let token = self.relogin(&client, &account).await?;
        client.call(&token, function, params).await
    }

    pub(crate) fn db(&self) -> &Database {
        self.db
    }

    pub(crate) fn token(&self) -> Result<String> {
        self.secrets
            .get(Service::Moodle, SecretKind::Token)?
            .ok_or(Error::InvalidToken)
    }

    pub(crate) fn base(&self) -> Result<Url> {
        Ok(self.client()?.0.base().clone())
    }

    async fn relogin(&self, client: &Client, account: &Account) -> Result<String> {
        let password = self
            .secrets
            .get(Service::Moodle, SecretKind::Password)?
            .ok_or(Error::InvalidToken)?;
        match client.login(&account.username, &password).await {
            Ok(tokens) => {
                self.store_tokens(&tokens)?;
                Ok(tokens.token)
            }
            // Passwort wurde geändert: nicht weiter mit dem alten versuchen.
            Err(Error::InvalidLogin) => {
                self.secrets.delete(Service::Moodle, SecretKind::Password)?;
                Err(Error::InvalidToken)
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSync {
    pub course_id: i64,
    pub downloaded: usize,
    pub error: Option<String>,
}
