use serde::Serialize;
use serde::de::DeserializeOwned;
use tsa_core::{Account, Database, SecretKind, SecretStore, Service};

use crate::client::normalize_url;
use crate::courses::RemoteCourse;
use crate::{Client, Course, Error, Result};

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
        let token = client.login(username, password).await?;
        let info = client.site_info(&token).await?;

        let previous = Account::get(self.db, Service::Moodle)?;
        if previous.is_some_and(|p| p.base_url != site.url || p.username != username) {
            Course::clear(self.db)?;
        }
        self.secrets
            .set(Service::Moodle, SecretKind::Token, &token)?;
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

    pub fn sign_out(&self) -> Result<()> {
        Account::remove(self.db, self.secrets, Service::Moodle)?;
        Course::clear(self.db)
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

    /// Ruft eine Funktion mit dem gespeicherten Token auf. Ist das Token
    /// abgelaufen und ein Passwort gespeichert, wird einmal neu angemeldet.
    async fn call<T: DeserializeOwned>(
        &self,
        function: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let account = Account::get(self.db, Service::Moodle)?.ok_or(Error::NotSignedIn)?;
        let client = Client::new(normalize_url(&account.base_url)?);
        if let Some(token) = self.secrets.get(Service::Moodle, SecretKind::Token)? {
            match client.call(&token, function, params).await {
                Err(Error::InvalidToken) => {}
                result => return result,
            }
        }
        let token = self.relogin(&client, &account).await?;
        client.call(&token, function, params).await
    }

    async fn relogin(&self, client: &Client, account: &Account) -> Result<String> {
        let password = self
            .secrets
            .get(Service::Moodle, SecretKind::Password)?
            .ok_or(Error::InvalidToken)?;
        match client.login(&account.username, &password).await {
            Ok(token) => {
                self.secrets
                    .set(Service::Moodle, SecretKind::Token, &token)?;
                Ok(token)
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
