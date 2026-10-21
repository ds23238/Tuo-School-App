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

async fn courses(server: &MockServer, token: &str) {
    ws(
        server,
        "core_enrol_get_users_courses",
        token,
        json!([{"id": 3, "fullname": "Bio 10b", "shortname": "B", "visible": 1}]),
    )
    .await;
}

#[tokio::test]
async fn sync_downloads_with_token() {
    let server = site(json!({})).await;
    let (db, secrets) = signed_in(&server, false).await;
    site_info(&server, "old").await;
    courses(&server, "old").await;
    let session = Session::new(&db, &secrets);
    session.refresh_courses().await.unwrap();
    tsa_moodle::Course::set_sync(&db, 3, true).unwrap();
    let file_url = format!(
        "{}/webservice/pluginfile.php/1/mod_resource/content/1/Blatt.pdf",
        server.uri()
    );
    ws(
        &server,
        "core_course_get_contents",
        "old",
        json!([{"id": 1, "name": "Thema 1", "section": 1, "modules": [
            {"id": 10, "name": "Blatt", "modname": "resource", "contents": [
                {"type": "file", "filename": "Blatt.pdf", "filepath": "/", "filesize": 4,
                 "fileurl": file_url, "timemodified": 5}]}]}]),
    )
    .await;
    Mock::given(method("GET"))
        .and(path(
            "/webservice/pluginfile.php/1/mod_resource/content/1/Blatt.pdf",
        ))
        .and(query_param("token", "old"))
        .respond_with(ResponseTemplate::new(200).set_body_string("%PDF"))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let result = session.sync_all(dir.path()).await.unwrap();
    assert_eq!(result[0].downloaded, 1, "{result:?}");
    let file = dir.path().join("Bio 10b/Thema 1/Blatt.pdf");
    assert_eq!(std::fs::read_to_string(file).unwrap(), "%PDF");
    // Zweiter Lauf lädt nichts erneut (expect(1) oben).
    assert_eq!(session.sync_all(dir.path()).await.unwrap()[0].downloaded, 0);
}

#[tokio::test]
async fn save_submission_reuploads_kept_files() {
    let server = site(json!({})).await;
    let (db, secrets) = signed_in(&server, false).await;
    let media = tsa_moodle::html::MediaUrls {
        prefix: "media://localhost/".into(),
        scheme: "media",
    };
    ws(
        &server,
        "mod_assign_get_assignments",
        "old",
        json!({"courses": [{"assignments": [{"id": 5, "cmid": 50, "name": "Protokoll",
        "configs": [
            {"plugin": "file", "subtype": "assignsubmission", "name": "enabled", "value": "1"},
            {"plugin": "onlinetext", "subtype": "assignsubmission", "name": "enabled", "value": "1"}
        ]}]}]}),
    )
    .await;
    let old_file = format!(
        "{}/webservice/pluginfile.php/9/assignsubmission_file/submission_files/1/alt.pdf",
        server.uri()
    );
    ws(
        &server,
        "mod_assign_get_submission_status",
        "old",
        json!({"lastattempt": {"submission": {"status": "draft", "plugins": [
            {"type": "file", "fileareas": [{"files": [
                {"filename": "alt.pdf", "filesize": 3, "fileurl": old_file},
                {"filename": "weg.pdf", "filesize": 3, "fileurl": old_file}]}]}
        ]}, "canedit": true, "cansubmit": true}}),
    )
    .await;
    Mock::given(method("GET"))
        .and(path(
            "/webservice/pluginfile.php/9/assignsubmission_file/submission_files/1/alt.pdf",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string("alt"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/webservice/upload.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"itemid": 77}])))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/webservice/rest/server.php"))
        .and(query_param("wsfunction", "mod_assign_save_submission"))
        .and(body_string_contains("files_filemanager%5D=77"))
        .and(body_string_contains("%3Cp%3EHallo%3C%2Fp%3E"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let neu = dir.path().join("neu.txt");
    std::fs::write(&neu, "neu").unwrap();
    let a = Session::new(&db, &secrets)
        .assignment(3, 50, &media)
        .await
        .unwrap();
    assert_eq!(a.files.len(), 2);
    assert!(a.can_edit && a.text_submission && a.file_submission.is_some());
    Session::new(&db, &secrets)
        .save_submission(
            3,
            50,
            tsa_moodle::assign::Draft {
                keep: vec!["alt.pdf".into()],
                add: vec![neu],
                text: Some("Hallo".into()),
            },
            &media,
        )
        .await
        .unwrap();
}
