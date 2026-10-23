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
    /// Für Texte, deren Bilder nicht angezeigt werden.
    pub fn none() -> Self {
        Self {
            prefix: String::new(),
            scheme: "media",
        }
    }

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

/// Entfernt alles Ausführbare (Skripte, Event-Handler) und alle Styles
/// außer Layout-Angaben; leitet Bilder über [`MediaUrls`] um. iframes werden
/// zu Links. Links werden absolute Adressen.
pub fn sanitize(html: &str, base: &Url, media: &MediaUrls) -> String {
    let html = unescape_breaks(html);
    let html = iframes_to_links(&html);
    let (base_for_images, media_for_images) = (base.clone(), media.clone());
    let schemes: HashSet<&str> = ["http", "https", "mailto", "tel", media.scheme].into();
    Builder::default()
        .url_schemes(schemes)
        .url_relative(UrlRelative::RewriteWithBase(base.clone()))
        .link_rel(Some("noopener noreferrer"))
        .add_generic_attributes(["style"])
        // Ausrichtung aus dem Atto-Editor; TinyMCE setzt sie als Style.
        .add_allowed_classes(
            "img",
            [
                "icon",
                "iconlarge",
                "iconsmall",
                "atto_image_button_left",
                "atto_image_button_right",
                "atto_image_button_middle",
                "atto_image_button_text-top",
                "atto_image_button_text-bottom",
                "float-left",
                "float-right",
                "float-start",
                "float-end",
            ],
        )
        .add_allowed_classes("a", ["embed", "video", "interactive", "page"])
        .add_allowed_classes(
            "span",
            ["embed-tile", "embed-text", "embed-kind", "embed-name"],
        )
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

/// Ersetzt eingebettete Seiten (Videos, H5P, Formulare …) durch einen Link
/// im Stil eines Link-Elements: Fremde Inhalte laufen nicht im App-Fenster,
/// sind aber erreichbar. Die Art (Video, interaktiv, Seite) steht als Klasse
/// dran, die Beschriftung setzt die Oberfläche.
fn iframes_to_links(html: &str) -> std::borrow::Cow<'_, str> {
    use std::sync::LazyLock;
    static IFRAME: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?is)<iframe\b([^>]*)>(?:.*?</iframe\s*>)?").expect("gültiges Muster")
    });
    static ATTRIBUTE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?is)\b(src|title)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#)
            .expect("gültiges Muster")
    });
    if !html.contains("<iframe") && !html.contains("<IFRAME") {
        return html.into();
    }
    IFRAME.replace_all(html, |caps: &regex::Captures| {
        let (mut src, mut title) = (None, None);
        for attr in ATTRIBUTE.captures_iter(&caps[1]) {
            let value = attr
                .get(2)
                .or(attr.get(3))
                .or(attr.get(4))
                .map_or("", |m| m.as_str());
            let value = html_escape::decode_html_entities(value).trim().to_owned();
            match attr[1].to_ascii_lowercase().as_str() {
                "src" => src = Some(value),
                _ => title = Some(value).filter(|t| !t.is_empty()),
            }
        }
        let Some(src) = src.filter(|s| !s.is_empty()) else {
            return String::new();
        };
        let href = watch_url(&src);
        if Url::parse(&href).is_ok_and(|u| !matches!(u.scheme(), "http" | "https")) {
            return String::new();
        }
        let host = Url::parse(&href)
            .ok()
            .and_then(|u| {
                u.host_str()
                    .map(|h| h.trim_start_matches("www.").to_owned())
            })
            .unwrap_or_default();
        let kind = embed_kind(&href, &host);
        // Ohne Titel wird der Host zum Namen und fehlt in der Art-Zeile.
        let (label, host) = match title {
            Some(title) => (title, host),
            None if host.is_empty() => (href.clone(), host),
            None => (host, String::new()),
        };
        format!(
            r#"<p><a class="embed {kind}" href="{}"><span class="embed-tile"></span><span class="embed-text"><span class="embed-kind">{}</span><span class="embed-name">{}</span></span></a></p>"#,
            html_escape::encode_double_quoted_attribute(&href),
            html_escape::encode_text(&host),
            html_escape::encode_text(&label)
        )
    })
}

/// Art eines eingebetteten Inhalts für Symbol und Beschriftung.
fn embed_kind(href: &str, host: &str) -> &'static str {
    const VIDEO: &[&str] = &[
        "youtube.com",
        "youtu.be",
        "youtube-nocookie.com",
        "vimeo.com",
        "player.vimeo.com",
        "dailymotion.com",
        "ardmediathek.de",
        "zdf.de",
    ];
    let path = href.to_ascii_lowercase();
    if VIDEO.contains(&host) || path.contains("/mod/bigbluebuttonbn/") {
        "video"
    } else if path.contains("h5p") || path.contains("learningapps") || path.contains("/mod/hvp/") {
        "interactive"
    } else {
        "page"
    }
}

/// Eingebettete YouTube- und Vimeo-Player als normale Seite öffnen.
fn watch_url(src: &str) -> String {
    let absolute = if src.starts_with("//") {
        format!("https:{src}")
    } else {
        src.to_owned()
    };
    let Ok(url) = Url::parse(&absolute) else {
        return src.to_owned();
    };
    let host = url
        .host_str()
        .unwrap_or_default()
        .trim_start_matches("www.");
    let id = url.path_segments().and_then(|mut s| match s.next() {
        Some("embed") => s.next().map(str::to_owned),
        Some("video") if host == "player.vimeo.com" => s.next().map(str::to_owned),
        _ => None,
    });
    match (host, id) {
        ("youtube.com" | "youtube-nocookie.com", Some(id)) => {
            format!("https://www.youtube.com/watch?v={id}")
        }
        ("player.vimeo.com", Some(id)) => format!("https://vimeo.com/{id}"),
        _ => absolute,
    }
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
            r#"<p onclick="x()">Hi<script>alert(1)</script></p><iframe srcdoc="<script>x()</script>"></iframe>"#,
        );
        assert_eq!(out, "<p>Hi</p>");
    }

    #[test]
    fn iframes_become_links() {
        let out = clean(
            r#"<iframe width="560" src="https://www.youtube.com/embed/abc123?si=x" title="Mein &amp; Video"></iframe><IFRAME SRC='//h5p.example/x'>"#,
        );
        assert_eq!(
            out,
            concat!(
                r#"<p><a class="embed video" href="https://www.youtube.com/watch?v=abc123" rel="noopener noreferrer"><span class="embed-tile"></span><span class="embed-text"><span class="embed-kind">youtube.com</span><span class="embed-name">Mein &amp; Video</span></span></a></p>"#,
                r#"<p><a class="embed interactive" href="https://h5p.example/x" rel="noopener noreferrer"><span class="embed-tile"></span><span class="embed-text"><span class="embed-kind"></span><span class="embed-name">h5p.example</span></span></a></p>"#
            )
        );
        assert_eq!(clean(r#"<iframe src="javascript:alert(1)"></iframe>"#), "");
        assert!(
            clean(r#"<iframe src="/mod/hvp/embed.php?id=3"></iframe>"#)
                .contains(r#"href="https://moodle.example/mod/hvp/embed.php?id=3""#)
        );
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
