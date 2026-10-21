use serde_json::json;
use tsa_core::{Account, Database, MemoryStore, SecretKind, SecretStore, Service};
use tsa_moodle::{Error, Session};
use wiremock::matchers::{body_string_contains, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn db() -> Database {
    let db = Database::open_in_memory().unwrap();
    tsa_moodle::init(&db).unwrap();
    db
}

async fn site(config: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    let mut data = json!({
        "wwwroot": server.uri(),
        "sitename": "Testschule",
        "typeoflogin": 1,
        "enablewebservices": 1,
        "enablemobilewebservice": 1,
    });
    data.as_object_mut()
        .unwrap()
        .extend(config.as_object().unwrap().clone());
    Mock::given(method("POST"))
        .and(path("/lib/ajax/service-nologin.php"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([{"error": false, "data": data}])),
        )
        .mount(&server)
        .await;
    server
}

async fn token_ok(server: &MockServer, password: &str, token: &str) {
    Mock::given(method("POST"))
        .and(path("/login/token.php"))
        .and(body_string_contains(format!("password={password}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": token})))
        .mount(server)
        .await;
}

async fn ws(server: &MockServer, function: &str, token: &str, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/webservice/rest/server.php"))
        .and(query_param("wsfunction", function))
        .and(body_string_contains(format!("wstoken={token}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn site_info(server: &MockServer, token: &str) {
    ws(
        server,
        "core_webservice_get_site_info",
        token,
        json!({"userid": 7, "fullname": "Anna Beispiel", "sitename": "Testschule"}),
    )
    .await;
}

#[tokio::test]
async fn login_stores_account_and_token() {
    let server = site(json!({})).await;
    token_ok(&server, "geheim", "t1").await;
    site_info(&server, "t1").await;
    let (db, secrets) = (db(), MemoryStore::default());

    let account = Session::new(&db, &secrets)
        .login(&server.uri(), "anna", "geheim", false)
        .await
        .unwrap();

    assert_eq!(account.base_url, format!("{}/", server.uri()));
    assert_eq!(account.display_name.as_deref(), Some("Anna Beispiel"));
    assert_eq!(Account::get(&db, Service::Moodle).unwrap(), Some(account));
    assert_eq!(
        secrets.get(Service::Moodle, SecretKind::Token).unwrap(),
        Some("t1".into())
    );
    assert_eq!(
        secrets.get(Service::Moodle, SecretKind::Password).unwrap(),
        None
    );
}

#[tokio::test]
async fn invalid_login() {
    let server = site(json!({})).await;
    Mock::given(method("POST"))
        .and(path("/login/token.php"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"error": "Invalid login", "errorcode": "invalidlogin"})),
        )
        .mount(&server)
        .await;
    let (db, secrets) = (db(), MemoryStore::default());
    let result = Session::new(&db, &secrets)
        .login(&server.uri(), "anna", "falsch", true)
        .await;
    assert!(matches!(result, Err(Error::InvalidLogin)));
    assert_eq!(Account::get(&db, Service::Moodle).unwrap(), None);
}

#[tokio::test]
async fn web_services_disabled() {
    let server = site(json!({"enablemobilewebservice": 0})).await;
    let result = Session::check_site(&server.uri()).await;
    assert!(matches!(result, Err(Error::WebServicesDisabled)));
}

#[tokio::test]
async fn not_moodle() {
    let server = MockServer::start().await;
    let result = Session::check_site(&server.uri()).await;
    assert!(matches!(result, Err(Error::NotMoodle)));
}

#[tokio::test]
async fn sso_only_is_reported() {
    let server = site(json!({"typeoflogin": 2})).await;
    assert!(Session::check_site(&server.uri()).await.unwrap().sso_only);
}

async fn signed_in(server: &MockServer, remember: bool) -> (Database, MemoryStore) {
    token_ok(server, "geheim", "old").await;
    site_info(server, "old").await;
    let (db, secrets) = (db(), MemoryStore::default());
    Session::new(&db, &secrets)
        .login(&server.uri(), "anna", "geheim", remember)
        .await
        .unwrap();
    server.reset().await;
    (db, secrets)
}

async fn expire(server: &MockServer, token: &str) {
    for function in [
        "core_webservice_get_site_info",
        "core_enrol_get_users_courses",
    ] {
        ws(
            server,
            function,
            token,
            json!({"exception": "moodle_exception", "errorcode": "invalidtoken", "message": "Invalid token"}),
        )
        .await;
    }
}

#[tokio::test]
async fn refresh_relogins_with_stored_password() {
    let server = site(json!({})).await;
    let (db, secrets) = signed_in(&server, true).await;
    expire(&server, "old").await;
    token_ok(&server, "geheim", "new").await;
    site_info(&server, "new").await;
    ws(
        &server,
        "core_enrol_get_users_courses",
        "new",
        json!([{"id": 3, "fullname": "Mathe 10b", "shortname": "M10b", "visible": 1,
                "startdate": 1700000000, "enddate": 0, "lastaccess": null}]),
    )
    .await;

    let courses = Session::new(&db, &secrets).refresh_courses().await.unwrap();

    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].full_name, "Mathe 10b");
    assert_eq!(courses[0].end_date, None);
    assert_eq!(
        secrets.get(Service::Moodle, SecretKind::Token).unwrap(),
        Some("new".into())
    );
}

#[tokio::test]
async fn expired_token_without_password() {
    let server = site(json!({})).await;
    let (db, secrets) = signed_in(&server, false).await;
    expire(&server, "old").await;
    let result = Session::new(&db, &secrets).refresh_courses().await;
    assert!(matches!(result, Err(Error::InvalidToken)));
}

#[tokio::test]
async fn sign_out_clears_everything() {
    let server = site(json!({})).await;
    let (db, secrets) = signed_in(&server, true).await;
    Session::new(&db, &secrets).sign_out().unwrap();
    assert_eq!(Account::get(&db, Service::Moodle).unwrap(), None);
    assert_eq!(
        secrets.get(Service::Moodle, SecretKind::Token).unwrap(),
        None
    );
    assert_eq!(
        secrets.get(Service::Moodle, SecretKind::Password).unwrap(),
        None
    );
}

/// Gegen eine echte Instanz, z. B.
/// `MOODLE_URL=school.moodledemo.net MOODLE_USER=student MOODLE_PASSWORD=… cargo test -p tsa-moodle -- --ignored`
#[tokio::test]
#[ignore]
async fn live() {
    let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("{k} fehlt"));
    let (db, secrets) = (db(), MemoryStore::default());
    let session = Session::new(&db, &secrets);
    let account = session
        .login(
            &var("MOODLE_URL"),
            &var("MOODLE_USER"),
            &var("MOODLE_PASSWORD"),
            false,
        )
        .await
        .unwrap();
    let courses = session.refresh_courses().await.unwrap();
    println!("{account:?}");
    for c in &courses {
        println!("{} {} {:?}", c.id, c.full_name, c.end_date);
    }
    assert!(!courses.is_empty());
}
