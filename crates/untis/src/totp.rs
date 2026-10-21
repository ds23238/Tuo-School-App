//! Einmalcodes (TOTP, RFC 6238) aus dem App-Schlüssel, wie in der Untis-App.

use hmac::{Hmac, Mac};
use sha1::Sha1;

fn base32(input: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = Vec::new();
    let (mut buffer, mut bits) = (0u64, 0u32);
    for c in input.bytes().filter(|c| !matches!(c, b'=' | b' ' | b'-')) {
        let value = ALPHABET.iter().position(|&a| a == c.to_ascii_uppercase())? as u64;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Sechsstelliger Code für den Zeitpunkt `unix_seconds` (30-s-Schritte).
pub fn code(secret: &str, unix_seconds: u64) -> Option<String> {
    let key = base32(secret)?;
    let mut mac = Hmac::<Sha1>::new_from_slice(&key).ok()?;
    mac.update(&(unix_seconds / 30).to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = (hash[19] & 0x0f) as usize;
    let value = u32::from_be_bytes([
        hash[offset],
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]) & 0x7fff_ffff;
    Some(format!("{:06}", value % 1_000_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_vector() {
        // Schlüssel "12345678901234567890" aus RFC 6238, als Base32.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(code(secret, 59).as_deref(), Some("287082"));
        assert_eq!(code(secret, 1_111_111_109).as_deref(), Some("081804"));
        assert_eq!(code("not base32!", 0), None);
    }
}
