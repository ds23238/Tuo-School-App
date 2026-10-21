//! `media://`: liefert eingebettete Bilder aus Moodle-Inhalten aus. Das Token
//! bleibt im Backend, geladene Dateien landen im Cache und gehen offline.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, UriSchemeResponder};
use tsa_moodle::Session;
use tsa_moodle::html::MediaUrls;

use crate::AppState;

pub const SCHEME: &str = "media";

/// Unter Windows bildet WebView2 eigene Schemata auf `http://<schema>.localhost` ab.
pub fn urls() -> MediaUrls {
    if cfg!(windows) {
        MediaUrls {
            prefix: format!("http://{SCHEME}.localhost/"),
            scheme: "http",
        }
    } else {
        MediaUrls {
            prefix: format!("{SCHEME}://localhost/"),
            scheme: SCHEME,
        }
    }
}

pub fn handle(app: &AppHandle, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let response = match serve(&app, request.uri().path()).await {
            Ok((body, content_type)) => Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "max-age=86400")
                .body(body),
            Err(_) => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new()),
        };
        responder.respond(response.expect("gültige Antwort"));
    });
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

async fn serve(app: &AppHandle, path: &str) -> Result<(Vec<u8>, String), BoxError> {
    let url = MediaUrls::decode(path).ok_or("ungültige Medien-Adresse")?;
    let file = cache_file(app, url.as_str())?;
    let type_file = file.with_extension("type");
    if let (Ok(body), Ok(content_type)) = (
        tokio::fs::read(&file).await,
        tokio::fs::read_to_string(&type_file).await,
    ) {
        return Ok((body, content_type));
    }
    let state = app.state::<AppState>();
    let (body, content_type) = Session::new(&state.db, state.secrets.as_ref())
        .fetch_media(&url)
        .await?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".into());
    // Ein fehlgeschlagenes Schreiben kostet nur den Cache, nicht die Anzeige.
    if tokio::fs::write(&file, &body).await.is_ok() {
        let _ = tokio::fs::write(&type_file, &content_type).await;
    }
    Ok((body, content_type))
}

fn cache_file(app: &AppHandle, url: &str) -> Result<PathBuf, BoxError> {
    let dir = app.path().app_cache_dir()?.join("media");
    std::fs::create_dir_all(&dir)?;
    let hash = Sha256::digest(url.as_bytes());
    let name: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    Ok(dir.join(name))
}
