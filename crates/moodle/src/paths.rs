//! Datei- und Ordnernamen, die unter Windows und Linux gültig sind.

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Länge eines Namens in Zeichen; Windows erlaubt 255, wir lassen Luft für
/// Zusätze wie „ (lokal 2026-10-21)“.
const MAX_CHARS: usize = 120;

/// Macht aus einem beliebigen Text einen einzelnen gültigen Pfadteil.
pub fn component(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.chars().count() > MAX_CHARS {
        out = shorten(&out);
    }
    // Windows entfernt Punkte und Leerzeichen am Ende stillschweigend.
    let trimmed = out.trim_end_matches(['.', ' ']).to_owned();
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    match trimmed.as_str() {
        "" => "_".into(),
        _ if RESERVED.contains(&stem.as_str()) => format!("_{trimmed}"),
        _ => trimmed,
    }
}

/// Kürzt den Namen, behält aber die Endung.
fn shorten(name: &str) -> String {
    let (stem, ext) = split_ext(name);
    let keep = MAX_CHARS.saturating_sub(ext.chars().count());
    let stem: String = stem.chars().take(keep).collect();
    format!("{}{ext}", stem.trim_end())
}

/// `("Bericht", ".pdf")`; ohne Endung ist der zweite Teil leer.
pub fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && name.len() - i <= 10 => name.split_at(i),
        _ => (name, ""),
    }
}

/// `Bericht.pdf` + `lokal 2026-10-21` → `Bericht (lokal 2026-10-21).pdf`.
pub fn with_suffix(name: &str, suffix: &str) -> String {
    let (stem, ext) = split_ext(name);
    format!("{stem} ({suffix}){ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components() {
        let cases = [
            ("Bio 10b: Fotosynthese", "Bio 10b_ Fotosynthese"),
            ("a/b\\c", "a_b_c"),
            ("  viel   Platz  ", "viel Platz"),
            ("Ende.", "Ende"),
            ("con", "_con"),
            ("NUL.txt", "_NUL.txt"),
            ("", "_"),
            ("...", "_"),
        ];
        for (input, expected) in cases {
            assert_eq!(component(input), expected, "{input}");
        }
    }

    #[test]
    fn long_names_keep_extension() {
        let long = format!("{}.pdf", "x".repeat(300));
        let out = component(&long);
        assert_eq!(out.chars().count(), MAX_CHARS);
        assert!(out.ends_with(".pdf"));
    }

    #[test]
    fn suffix() {
        assert_eq!(
            with_suffix("Bericht.pdf", "lokal 1"),
            "Bericht (lokal 1).pdf"
        );
        assert_eq!(with_suffix("README", "x"), "README (x)");
        assert_eq!(with_suffix(".bashrc", "x"), ".bashrc (x)");
    }
}
