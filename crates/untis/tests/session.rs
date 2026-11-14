use chrono::NaiveDate;
use serde_json::json;
use tsa_core::{Account, Database, MemoryStore, SecretKind, SecretStore, Service};
use tsa_untis::client::{Client, School};
use tsa_untis::{Error, Session, UntisState};
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn db() -> Database {
    let db = Database::open_in_memory().unwrap();
    tsa_untis::init(&db).unwrap();
    db
}

/// Der Mock-Server läuft über HTTP; die App spricht sonst immer HTTPS.
fn client(server: &MockServer) -> Client {
    Client::new(
        url::Url::parse(&format!("{}/", server.uri())).unwrap(),
        "test-school",
    )
}

async fn rpc(server: &MockServer, rpc_method: &str, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/WebUntis/jsonrpc.do"))
        .and(body_string_contains(format!("\"method\":\"{rpc_method}\"")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"jsonrpc": "2.0", "id": "x", "result": body})),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn authenticate_and_app_secret() {
    let server = MockServer::start().await;
    rpc(
        &server,
        "authenticate",
        json!({"sessionId": "S1", "personType": 5, "personId": 77, "klasseId": 12}),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/WebUntis/jsonrpc_intern.do"))
        .and(query_param("m", "getAppSharedSecret"))
        .and(query_param("school", "test-school"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"result": "GEZDGNBVGY3TQOJQ"})),
        )
        .mount(&server)
        .await;
    let c = client(&server);
    let (session, person) = c.authenticate("anna", "geheim").await.unwrap();
    assert_eq!(
        (session.as_str(), person.kind, person.id, person.klasse_id),
        ("S1", 5, 77, Some(12))
    );
    assert_eq!(
        c.app_secret("anna", "geheim").await.unwrap(),
        "GEZDGNBVGY3TQOJQ"
    );
}

#[tokio::test]
async fn wrong_password() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/WebUntis/jsonrpc.do"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"jsonrpc": "2.0", "id": "x", "error": {"code": -8504, "message": "bad credentials"}}),
        ))
        .mount(&server)
        .await;
    let result = client(&server).authenticate("anna", "falsch").await;
    assert!(matches!(result, Err(Error::InvalidLogin)));
}

#[tokio::test]
async fn otp_login_reads_session_cookie() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/WebUntis/jsonrpc_intern.do"))
        .and(query_param("m", "getUserData2017"))
        .and(body_string_contains("\"user\":\"anna\""))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "set-cookie",
                    "JSESSIONID=OTPSESSION; Path=/WebUntis; HttpOnly",
                )
                .set_body_json(json!({"result": {"userData": {}}})),
        )
        .mount(&server)
        .await;
    let session = client(&server)
        .otp_login("anna", "GEZDGNBVGY3TQOJQ", 1_700_000_000_000)
        .await
        .unwrap();
    assert_eq!(session, "OTPSESSION");
}

/// Ein angemeldetes Konto mit gespeichertem Passwort, das auf den Mock zeigt.
fn signed_in(server: &MockServer, db: &Database, secrets: &MemoryStore) {
    let school = School {
        server: server.uri(),
        login_name: "test-school".into(),
        display_name: "Testschule".into(),
        address: String::new(),
    };
    tsa_core::settings::set_value(db, "untis.school", &serde_json::to_string(&school).unwrap())
        .unwrap();
    tsa_core::settings::set_value(
        db,
        "untis.person",
        r#"{"kind":5,"id":77,"klasseId":12,"name":null}"#,
    )
    .unwrap();
    Account::new(Service::Untis, server.uri(), "anna".into())
        .save(db)
        .unwrap();
    secrets
        .set(Service::Untis, SecretKind::Password, "geheim")
        .unwrap();
}

#[tokio::test]
async fn week_relogins_after_expired_session() {
    let server = MockServer::start().await;
    let (db, secrets, state) = (db(), MemoryStore::default(), UntisState::default());
    signed_in(&server, &db, &secrets);
    rpc(
        &server,
        "authenticate",
        json!({"sessionId": "S1", "personType": 5, "personId": 77}),
    )
    .await;
    // Die erste Abfrage findet eine abgelaufene Sitzung vor.
    Mock::given(method("POST"))
        .and(path("/WebUntis/jsonrpc.do"))
        .and(body_string_contains("getTimetable"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"jsonrpc": "2.0", "id": "x", "error": {"code": -8520, "message": "not authenticated"}}),
        ))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/WebUntis/jsonrpc.do"))
        .and(body_string_contains("getTimetable"))
        .and(body_string_contains("\"startDate\":\"20261019\""))
        .and(header("cookie", "JSESSIONID=S1; schoolname=\"_dGVzdC1zY2hvb2w=\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"jsonrpc": "2.0", "id": "x", "result": [
            {"id": 1, "date": 20261021, "startTime": 800, "endTime": 845,
             "su": [{"id": 3, "name": "M", "longname": "Mathematik"}], "te": [], "ro": [], "kl": []},
            {"id": 2, "date": 20261021, "startTime": 850, "endTime": 935, "code": "cancelled",
             "su": [{"id": 4, "name": "D", "longname": "Deutsch"}]},
        ]})))
        .mount(&server)
        .await;
    rpc(&server, "getTimegridUnits", json!([{"day": 2, "timeUnits": [
        {"name": "1", "startTime": 800, "endTime": 845}, {"name": "2", "startTime": 850, "endTime": 935}]}]))
    .await;
    rpc(
        &server,
        "getHolidays",
        json!([{"id": 1, "name": "HF", "longName": "Herbstferien",
        "startDate": 20261023, "endDate": 20261031}]),
    )
    .await;

    let session = Session::new(&db, &secrets, &state);
    let week = session
        .week(NaiveDate::from_ymd_opt(2026, 10, 21).unwrap())
        .await
        .unwrap();
    assert_eq!(week.monday, "2026-10-19");
    assert_eq!(week.days.len(), 5);
    assert_eq!(week.days[4].holiday.as_deref(), Some("Herbstferien"));
    assert_eq!(week.lessons.len(), 2);
    assert_eq!(week.lessons[1].status, "cancelled");
    assert_eq!(week.grid[1].name, "2");
    assert_eq!(session.subjects().unwrap().len(), 2);
    assert!(!week.offline);
}

#[tokio::test]
async fn browser_cookies_reconnect_and_are_cleared_on_sign_out() {
    let server = MockServer::start().await;
    let (db, secrets, state) = (db(), MemoryStore::default(), UntisState::default());
    signed_in(&server, &db, &secrets);
    rpc(
        &server,
        "authenticate",
        json!({"sessionId":"BROWSER", "personId":77, "personType":5}),
    )
    .await;
    let session = Session::new(&db, &secrets, &state);
    let cookies = session.browser_cookies().await.unwrap();
    assert!(cookies.contains(&("JSESSIONID".into(), "BROWSER".into())));
    assert!(cookies.contains(&("schoolname".into(), "\"_dGVzdC1zY2hvb2w=\"".into())));
    session.sign_out().await.unwrap();
    assert!(matches!(
        session.browser_cookies().await,
        Err(Error::NotSignedIn)
    ));
}
