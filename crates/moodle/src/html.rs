//! Bereinigung von Moodle-HTML für die Anzeige in der App.

use std::collections::HashSet;

use ammonia::{Builder, UrlRelative};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use url::Url;

/// Adressen, unter denen die App eingebettete Bilder ausliefert (mit
/// Anmeldung und Cache), z. B. `media://localhost/<kodierte URL>`.
#[derive(Debug, Clone)]
pub struct MediaUrls {
    /// Endet auf `/`.
    pub prefix: String,
    /// Schema der Präfix-Adresse, damit der Bereiniger es erlaubt.
    pub scheme: &'static str,
}

impl MediaUrls {
    pub fn url(&self, original: &Url) -> String {
        format!(
            "{}{}",
            self.prefix,
            URL_SAFE_NO_PAD.encode(original.as_str())
        )
    }

    /// Umkehrung von [`Self::url`] für den Pfad einer Anfrage.
    pub fn decode(path: &str) -> Option<Url> {
        let bytes = URL_SAFE_NO_PAD.decode(path.trim_start_matches('/')).ok()?;
        Url::parse(std::str::from_utf8(&bytes).ok()?).ok()
    }
}

/// Style-Eigenschaften, die Moodle-Inhalte für ihr Layout brauchen. Farben
/// fehlen absichtlich: Sie passen oft nicht zum dunklen Modus.
const STYLE_PROPERTIES: &[&str] = &[
    "width",
    "height",
    "max-width",
    "max-height",
    "min-width",
    "float",
    "clear",
    "text-align",
    "vertical-align",
    "margin",
    "margin-left",
    "margin-right",
    "margin-top",
    "margin-bottom",
    "padding",
    "padding-left",
    "padding-right",
    "font-weight",
    "font-style",
    "text-decoration",
];

/// Entfernt alles Ausführbare (Skripte, Event-Handler, iframes) und alle
/// Styles außer Layout-Angaben; leitet Bilder über [`MediaUrls`] um. Links
/// werden absolute Adressen.
pub fn sanitize(html: &str, base: &Url, media: &MediaUrls) -> String {
    let (base_for_images, media_for_images) = (base.clone(), media.clone());
    let schemes: HashSet<&str> = ["http", "https", "mailto", "tel", media.scheme].into();
    Builder::default()
        .url_schemes(schemes)
        .url_relative(UrlRelative::RewriteWithBase(base.clone()))
        .link_rel(Some("noopener noreferrer"))
        .add_generic_attributes(["style"])
        .filter_style_properties(STYLE_PROPERTIES.iter().copied().collect())
        .attribute_filter(move |element, attribute, value| {
            if element == "img" && attribute == "src" {
                // Nicht ladbare Bildquellen fallen weg statt kaputt zu laden.
                return base_for_images
                    .join(value)
                    .ok()
                    .filter(|u| matches!(u.scheme(), "http" | "https"))
                    .map(|u| media_for_images.url(&u).into());
            }
            Some(value.into())
        })
        .clean(html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media() -> MediaUrls {
        MediaUrls {
            prefix: "media://localhost/".into(),
            scheme: "media",
        }
    }

    fn clean(html: &str) -> String {
        sanitize(
            html,
            &Url::parse("https://moodle.example/").unwrap(),
            &media(),
        )
    }

    #[test]
    fn strips_active_content() {
        let out = clean(
            r#"<p onclick="x()">Hi<script>alert(1)</script></p><iframe src="https://evil"></iframe>"#,
        );
        assert_eq!(out, "<p>Hi</p>");
    }

    #[test]
    fn keeps_layout_styles() {
        let out = clean(r#"<p style="text-align: center; color: red; position: fixed">x</p>"#);
        assert_eq!(out, r#"<p style="text-align:center">x</p>"#);
    }

    #[test]
    fn rewrites_images() {
        let out =
            clean(r#"<img src="/pluginfile.php/1/a.png" alt="A" width="10" style="width: 24px">"#);
        let original = Url::parse("https://moodle.example/pluginfile.php/1/a.png").unwrap();
        assert_eq!(
            out,
            format!(
                r#"<img src="{}" alt="A" width="10" style="width:24px">"#,
                media().url(&original)
            )
        );
        let path = media().url(&original).replace("media://localhost", "");
        assert_eq!(MediaUrls::decode(&path), Some(original));
    }

    #[test]
    fn drops_unusable_images() {
        assert_eq!(clean(r#"<img src="javascript:alert(1)">"#), "<img>");
    }

    #[test]
    fn absolutizes_links() {
        let out = clean(r#"<a href="/mod/page/view.php?id=4">x</a><a href="javascript:x()">y</a>"#);
        assert_eq!(
            out,
            r#"<a href="https://moodle.example/mod/page/view.php?id=4" rel="noopener noreferrer">x</a><a rel="noopener noreferrer">y</a>"#
        );
    }
}
