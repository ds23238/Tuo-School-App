use serde_json::json;
use tsa_core::{Database, MemoryStore};
use tsa_moodle::Session;
use tsa_moodle::html::MediaUrls;
use tsa_moodle::mail::{Folder, MailState};
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn media() -> MediaUrls {
    MediaUrls {
        prefix: "media://localhost/".into(),
        scheme: "media",
    }
}

async fn ws(server: &MockServer, function: &str, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/webservice/rest/server.php"))
        .and(query_param("wsfunction", function))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

/// Angemeldet mit gespeichertem Passwort; `functions` erscheinen in den
/// Seiteninfos als für das Token freigegeben.
async fn signed_in(functions: &[&str]) -> (MockServer, Database, MemoryStore) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/lib/ajax/service-nologin.php"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([{"error": false, "data": {
                "wwwroot": server.uri(), "sitename": "Testschule", "typeoflogin": 1,
            }}])),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/token.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": "t1"})))
        .mount(&server)
        .await;
    let functions: Vec<_> = functions.iter().map(|f| json!({"name": f})).collect();
    ws(
        &server,
        "core_webservice_get_site_info",
        json!({"userid": 7, "fullname": "Anna", "sitename": "Testschule", "functions": functions}),
    )
    .await;
    let (db, secrets) = (Database::open_in_memory().unwrap(), MemoryStore::default());
    tsa_moodle::init(&db).unwrap();
    Session::new(&db, &secrets)
        .login(&server.uri(), "anna", "geheim", true)
        .await
        .unwrap();
    (server, db, secrets)
}

const COURSES: &str = r#"[{"id": 3, "shortname": "BIO", "fullname": "Biologie", "visible": true,
    "groupmode": 0, "unread": 2, "drafts": 1}]"#;

#[tokio::test]
async fn mail_over_browser_session_with_password() {
    let (server, db, secrets) = signed_in(&[]).await;
    let session_cookie = "MoodleSession=abc";
    Mock::given(method("GET"))
        .and(path("/local/mail/view.php"))
        .and(header("cookie", session_cookie))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"<script>M.cfg = {"sesskey":"KEY1","x":1};</script>"#),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/local/mail/view.php"))
        .respond_with(ResponseTemplate::new(303).insert_header("location", "/login/index.php"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/login/index.php"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                r#"<form><input type="hidden" name="logintoken" value="LT"></form>"#,
            ),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/index.php"))
        .and(body_string_contains("logintoken=LT"))
        .and(body_string_contains("password=geheim"))
        .respond_with(
            ResponseTemplate::new(303)
                .insert_header("set-cookie", "MoodleSession=abc; path=/")
                .insert_header("location", "/my/"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/my/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/lib/ajax/service.php"))
        .and(query_param("sesskey", "KEY1"))
        .and(query_param("info", "local_mail_get_courses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(format!(r#"[{{"error": false, "data": {COURSES}}}]"#)),
        )
        .mount(&server)
        .await;

    let state = MailState::default();
    let status = Session::new(&db, &secrets)
        .mail_status(&state)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.unread, 2);
    assert_eq!(status.courses[0].full_name, "Biologie");

    // Die Sitzung wird wiederverwendet, ohne neue Anmeldung.
    Session::new(&db, &secrets)
        .mail_status(&state)
        .await
        .unwrap();
    let logins = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "POST" && r.url.path() == "/login/index.php")
        .count();
    assert_eq!(logins, 1);
}

#[tokio::test]
async fn mail_over_token_when_allowed() {
    let (server, db, secrets) = signed_in(&["local_mail_search_messages"]).await;
    ws(&server, "local_mail_count_messages", json!(1)).await;
    Mock::given(method("POST"))
        .and(path("/webservice/rest/server.php"))
        .and(query_param("wsfunction", "local_mail_search_messages"))
        .and(body_string_contains("query%5Broles%5D%5B0%5D=to"))
        .and(body_string_contains("query%5Bdeleted%5D=0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "id": 11, "subject": "Exkursion", "numattachments": 1, "draft": false,
            "time": 1700000000, "unread": true, "starred": false, "deleted": false,
            "course": {"id": 3, "shortname": "BIO", "fullname": "Biologie"},
            "sender": {"id": 5, "fullname": "Herr Lehrer", "pictureurl": format!("{}/pluginfile.php/1/user/icon/f1", server.uri())},
            "recipients": [{"type": "to", "id": 7, "fullname": "Anna"}],
            "labels": [],
        }])))
        .mount(&server)
        .await;

    let page = Session::new(&db, &secrets)
        .mail_list(&MailState::default(), Folder::Inbox, None, "", 0, &media())
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    let m = &page.messages[0];
    assert_eq!(
        (m.subject.as_str(), m.course.as_str()),
        ("Exkursion", "BIO")
    );
    assert!(
        m.sender
            .image
            .as_deref()
            .unwrap()
            .starts_with("media://localhost/")
    );
    assert_eq!(m.recipients[0].kind, "to");
}

#[tokio::test]
async fn no_mail_plugin() {
    let (_server, db, secrets) = signed_in(&[]).await;
    // Ohne Seite /local/mail/ antwortet der Server mit 404.
    let status = Session::new(&db, &secrets)
        .mail_status(&MailState::default())
        .await
        .unwrap();
    assert!(status.is_none());
}

#[tokio::test]
async fn reports_only_new_messages() {
    let (server, db, secrets) = signed_in(&[]).await;
    ws(
        &server,
        "core_message_get_unread_conversations_count",
        json!(1),
    )
    .await;
    ws(
        &server,
        "message_popup_get_unread_popup_notification_count",
        json!(1),
    )
    .await;
    ws(
        &server,
        "message_popup_get_popup_notifications",
        json!({"notifications": [{"id": 4, "subject": "Neue Bewertung", "timecreated": 1, "read": false}], "unreadcount": 1}),
    )
    .await;
    let conversations = |message_id: i64| {
        json!({"conversations": [{
            "id": 9, "type": 1, "unreadcount": 1,
            "members": [{"id": 5, "fullname": "Ben"}],
            "messages": [{"id": message_id, "useridfrom": 5, "text": "<p>Hallo!</p>", "timecreated": 2}],
        }]})
    };
    ws(&server, "core_message_get_conversations", conversations(20)).await;

    let state = MailState::default();
    let session = Session::new(&db, &secrets);
    let (counts, news) = session.check_messages(&state).await.unwrap();
    assert_eq!(
        (counts.chats, counts.notifications, counts.mail),
        (1, 1, None)
    );
    assert!(news.is_empty(), "beim ersten Mal ist nichts neu");

    server.reset().await;
    ws(
        &server,
        "core_message_get_unread_conversations_count",
        json!(1),
    )
    .await;
    ws(
        &server,
        "message_popup_get_unread_popup_notification_count",
        json!(1),
    )
    .await;
    ws(
        &server,
        "message_popup_get_popup_notifications",
        json!({"notifications": [{"id": 4, "subject": "Neue Bewertung", "timecreated": 1, "read": false}], "unreadcount": 1}),
    )
    .await;
    ws(&server, "core_message_get_conversations", conversations(21)).await;
    let (_, news) = session.check_messages(&state).await.unwrap();
    assert_eq!(news.len(), 1);
    assert_eq!(
        (news[0].kind, news[0].title.as_str(), news[0].body.as_str()),
        ("chat", "Ben", "Hallo!")
    );
}
