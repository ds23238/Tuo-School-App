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

/// Markiert Bilder, deren Größe Moodle sonst per CSS festlegt: Symbole aus
/// dem Theme (`#icon`) und SVG-Grafiken (`#svg`). SVGs ohne eigene Größe
/// würden sonst die ganze Breite füllen. Das Fragment erreicht den Server
/// nicht, die Oberfläche setzt die Größe per CSS.
fn size_hint(url: &Url) -> &'static str {
    let path = url.path();
    if path.contains("/theme/image.php") || path.contains("/pix/") {
        "#icon"
    } else if path.to_ascii_lowercase().ends_with(".svg") {
        "#svg"
    } else {
        ""
    }
}

/// Entfernt alles Ausführbare (Skripte, Event-Handler, iframes) und alle
/// Styles außer Layout-Angaben; leitet Bilder über [`MediaUrls`] um. Links
/// werden absolute Adressen.
pub fn sanitize(html: &str, base: &Url, media: &MediaUrls) -> String {
    let html = unescape_breaks(html);
    let (base_for_images, media_for_images) = (base.clone(), media.clone());
    let schemes: HashSet<&str> = ["http", "https", "mailto", "tel", media.scheme].into();
    Builder::default()
        .url_schemes(schemes)
        .url_relative(UrlRelative::RewriteWithBase(base.clone()))
        .link_rel(Some("noopener noreferrer"))
        .add_generic_attributes(["style"])
        .add_allowed_classes("img", ["icon", "iconlarge", "iconsmall"])
        .add_tags(["video", "audio", "source", "track"])
        .add_tag_attributes(
            "video",
            ["controls", "width", "height", "poster", "preload"],
        )
        .add_tag_attributes("audio", ["controls", "preload"])
        .add_tag_attributes("source", ["src", "type"])
        .add_tag_attributes("track", ["src", "kind", "srclang", "label"])
        .filter_style_properties(STYLE_PROPERTIES.iter().copied().collect())
        .attribute_filter(move |element, attribute, value| {
            let media_src = matches!(
                (element, attribute),
                ("img" | "source" | "video" | "audio" | "track", "src") | ("video", "poster")
            );
            if media_src {
                // Nicht ladbare Bildquellen fallen weg statt kaputt zu laden.
                return base_for_images
                    .join(value)
                    .ok()
                    .filter(|u| matches!(u.scheme(), "http" | "https"))
                    .map(|u| {
                        let mut out = media_for_images.url(&u);
                        out.push_str(size_hint(&u));
                        out.into()
                    });
            }
            Some(value.into())
        })
        .clean(&html)
        .to_string()
}

/// Macht als Text maskierte Zeilenumbrüche (`&lt;br /&gt;`) wieder zu echten.
/// Solche Texte entstehen, wenn HTML in den Editor kopiert wurde.
fn unescape_breaks(html: &str) -> std::borrow::Cow<'_, str> {
    if !html.contains("&lt;") {
        return html.into();
    }
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("&lt;") {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 4..];
        let end = tail.find("&gt;").filter(|&e| e <= 8);
        let tag = end.map(|e| {
            tail[..e]
                .trim()
                .trim_end_matches('/')
                .trim()
                .to_ascii_lowercase()
        });
        if let (Some(e), Some("br")) = (end, tag.as_deref()) {
            out.push_str("<br>");
            rest = &tail[e + 4..];
        } else {
            out.push_str("&lt;");
            rest = tail;
        }
    }
    out.push_str(rest);
    out.into()
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
    fn keeps_video() {
        let out = clean(
            r#"<video controls="true" data-setup="{}"><source src="/pluginfile.php/1/v.mp4" type="video/mp4">/pluginfile.php/1/v.mp4</video>"#,
        );
        assert!(
            out.starts_with(r#"<video controls="true"><source src="media://localhost/"#),
            "{out}"
        );
        assert!(out.contains(r#"type="video/mp4""#));
    }

    #[test]
    fn marks_icons_and_svgs() {
        let out = clean(
            r#"<img class="icon foo" src="/theme/image.php/boost/core/1/f/pdf"><img src="/pluginfile.php/1/a.SVG">"#,
        );
        assert!(
            out.starts_with(r#"<img class="icon" src="media://"#),
            "{out}"
        );
        assert!(out.contains(r##"#icon">"##), "{out}");
        assert!(out.contains(r##"#svg">"##), "{out}");
    }

    #[test]
    fn escaped_breaks_become_real() {
        assert_eq!(
            clean("<p>Eins&lt;br&gt;Zwei&lt;BR /&gt;Drei &lt;b&gt;fett&lt;/b&gt; a &lt; b</p>"),
            "<p>Eins<br>Zwei<br>Drei &lt;b&gt;fett&lt;/b&gt; a &lt; b</p>"
        );
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
