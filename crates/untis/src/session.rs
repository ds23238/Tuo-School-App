use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::Mutex;
use tsa_core::settings::{delete_value, get_value, set_value};
use tsa_core::{Account, Database, SecretKind, SecretStore, Service};

use crate::client::{Client, Person, School};
use crate::{Error, Result, cache};

const SCHOOL: &str = "untis.school";
const PERSON: &str = "untis.person";

struct Live {
    session: String,
    jwt: Option<String>,
}

/// Laufende WebUntis-Sitzung; lebt nur im Speicher. Sitzungen laufen nach
/// wenigen Minuten ohne Nutzung ab und werden dann neu aufgebaut.
#[derive(Default)]
pub struct UntisState(Mutex<Option<Live>>);

impl UntisState {
    pub async fn reset(&self) {
        *self.0.lock().await = None;
    }
}

/// Zugriff mit dem gespeicherten Konto.
pub struct Session<'a> {
    db: &'a Database,
    secrets: &'a dyn SecretStore,
    state: &'a UntisState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub account: Account,
    /// `false`: Weder App-Schlüssel noch Passwort gespeichert, nach einem
    /// Neustart ist eine neue Anmeldung nötig.
    pub persistent: bool,
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

impl<'a> Session<'a> {
    pub fn new(db: &'a Database, secrets: &'a dyn SecretStore, state: &'a UntisState) -> Self {
        Self { db, secrets, state }
    }

    pub(crate) fn db(&self) -> &Database {
        self.db
    }

    pub async fn login(
        &self,
        school: School,
        username: &str,
        password: &str,
        remember: bool,
    ) -> Result<LoginResult> {
        let client = Client::for_school(&school)?;
        let (session, mut person) = client.authenticate(username, password).await?;
        // Der App-Schlüssel ersetzt das Passwort; manche Schulen erlauben ihn nicht.
        let secret = client.app_secret(username, password).await.ok();
        if let Ok(found) = client.person(&session).await {
            person.name = found.name;
            if person.klasse_id.is_none() {
                person.klasse_id = found.klasse_id;
            }
        }

        let previous = Account::get(self.db, Service::Untis)?;
        if previous.is_some() {
            cache::clear(self.db)?;
        }
        match &secret {
            Some(s) => self.secrets.set(Service::Untis, SecretKind::Token, s)?,
            None => self.secrets.delete(Service::Untis, SecretKind::Token)?,
        }
        if remember {
            self.secrets
                .set(Service::Untis, SecretKind::Password, password)?;
        } else {
            self.secrets.delete(Service::Untis, SecretKind::Password)?;
        }
        set_value(self.db, SCHOOL, &serde_json::to_string(&school)?)?;
        set_value(self.db, PERSON, &serde_json::to_string(&person)?)?;
        let mut account = Account::new(
            Service::Untis,
            client.base().to_string(),
            username.to_owned(),
        );
        account.display_name = person.name.clone();
        account.save(self.db)?;
        *self.state.0.lock().await = Some(Live { session, jwt: None });
        Ok(LoginResult {
            account,
            persistent: secret.is_some() || remember,
        })
    }

    pub async fn sign_out(&self) -> Result<()> {
        *self.state.0.lock().await = None;
        Account::remove(self.db, self.secrets, Service::Untis)?;
        delete_value(self.db, SCHOOL)?;
        delete_value(self.db, PERSON)?;
        cache::clear(self.db)
    }

    /// Cookies for an isolated native browser. Never expose these to frontend IPC.
    pub async fn browser_cookies(&self) -> Result<Vec<(String, String)>> {
        use base64::Engine;
        let school = self.school()?;
        self.with_session(false, |_, session, _| {
            let school = school.login_name.clone();
            async move {
                Ok(vec![
                    ("JSESSIONID".into(), session),
                    (
                        "schoolname".into(),
                        format!(
                            "\"_{}\"",
                            base64::engine::general_purpose::STANDARD.encode(school)
                        ),
                    ),
                ])
            }
        })
        .await
    }

    pub fn school(&self) -> Result<School> {
        let raw = get_value(self.db, SCHOOL)?.ok_or(Error::NotSignedIn)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn person(&self) -> Result<Person> {
        let raw = get_value(self.db, PERSON)?.ok_or(Error::NotSignedIn)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub(crate) fn client(&self) -> Result<Client> {
        Client::for_school(&self.school()?)
    }

    fn account(&self) -> Result<Account> {
        Account::get(self.db, Service::Untis)?.ok_or(Error::NotSignedIn)
    }

    /// Neue Sitzung mit App-Schlüssel, sonst mit gespeichertem Passwort.
    async fn connect(&self, client: &Client) -> Result<String> {
        let account = self.account()?;
        if let Some(secret) = self.secrets.get(Service::Untis, SecretKind::Token)? {
            match client
                .otp_login(&account.username, &secret, now_millis())
                .await
            {
                Ok(session) => return Ok(session),
                Err(Error::Network(e)) => return Err(Error::Network(e)),
                Err(_) => {}
            }
        }
        let password = self
            .secrets
            .get(Service::Untis, SecretKind::Password)?
            .ok_or(Error::SessionExpired)?;
        match client.authenticate(&account.username, &password).await {
            Ok((session, _)) => Ok(session),
            Err(Error::InvalidLogin) => {
                // Passwort geändert: nicht weiter mit dem alten versuchen.
                self.secrets.delete(Service::Untis, SecretKind::Password)?;
                Err(Error::SessionExpired)
            }
            Err(e) => Err(e),
        }
    }

    /// Führt `f` mit gültiger Sitzung aus; ist sie abgelaufen, einmal neu.
    async fn with_session<T, F, Fut>(&self, bearer: bool, f: F) -> Result<T>
    where
        F: Fn(Client, String, Option<String>) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let client = self.client()?;
        let mut live = self.state.0.lock().await;
        for attempt in 0..2 {
            if live.is_none() {
                *live = Some(Live {
                    session: self.connect(&client).await?,
                    jwt: None,
                });
            }
            let current = live.as_mut().expect("gerade gesetzt");
            if bearer && current.jwt.is_none() {
                match client.jwt(&current.session).await {
                    Ok(token) => current.jwt = Some(token),
                    Err(Error::SessionExpired) if attempt == 0 => {
                        *live = None;
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            let (session, jwt) = (current.session.clone(), current.jwt.clone());
            match f(client.clone(), session, jwt).await {
                Err(Error::SessionExpired) if attempt == 0 => *live = None,
                result => return result,
            }
        }
        Err(Error::SessionExpired)
    }

    pub(crate) async fn rpc<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        self.with_session(false, |client, session, _| {
            let params = params.clone();
            async move { client.rpc(Some(&session), method, params).await }
        })
        .await
    }

    pub(crate) async fn rest<T: DeserializeOwned>(
        &self,
        bearer: bool,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        self.rest_with(bearer, path, query, &[]).await
    }

    pub(crate) async fn rest_with<T: DeserializeOwned>(
        &self,
        bearer: bool,
        path: &str,
        query: &[(&str, String)],
        extra: &[(&str, String)],
    ) -> Result<T> {
        self.with_session(bearer, |client, session, jwt| async move {
            client
                .rest_with(&session, jwt.as_deref(), path, query, extra)
                .await
        })
        .await
    }

    /// Formular mit Token senden; liefert Status und Antworttext.
    pub(crate) async fn post_form(
        &self,
        path: &str,
        extra: &[(&str, String)],
        request_json: &str,
        files: &[(String, Vec<u8>)],
    ) -> Result<(u16, String)> {
        self.with_session(true, |client, session, jwt| async move {
            client
                .post_form(
                    &session,
                    jwt.as_deref(),
                    path,
                    extra,
                    request_json.to_owned(),
                    files.to_vec(),
                )
                .await
        })
        .await
    }
}
