//! Turning file bytes into text, and text into the plain form used for comparing.

/// Decodes a file: UTF-8 (with or without a mark), UTF-16 with a mark, otherwise Latin-1.
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(rest, u16::from_le_bytes);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(rest, u16::from_be_bytes);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => latin1(bytes),
    }
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| unit(*pair)).collect();
    String::from_utf16_lossy(&units)
}

pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if index <= chunk.len() {
                out.push(BASE64[(n >> shift & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Reads base64, passing over spaces and line breaks. `None` when a character does not belong.
pub fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut bits, mut have) = (0u32, 0);
    for byte in text.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=') {
        let value = BASE64.iter().position(|&c| c == byte)? as u32;
        bits = bits << 6 | value;
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have) as u8);
        }
    }
    Some(out)
}

/// Text for comparing: lower case, common accents removed, runs of spaces and punctuation
/// reduced to one space.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut gap = false;
    for ch in text.chars().flat_map(char::to_lowercase) {
        let ch = unaccent(ch);
        if ch.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.push(ch);
        } else {
            gap = true;
        }
    }
    out
}

fn unaccent(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ě' | 'ę' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ī' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' | 'ø' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'ū' | 'ů' => 'u',
        'ý' | 'ÿ' => 'y',
        'ñ' | 'ń' | 'ň' => 'n',
        'ç' | 'ć' | 'č' => 'c',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        'ř' => 'r',
        'ł' => 'l',
        'ď' => 'd',
        'ť' => 't',
        other => other,
    }
}

/// A phone number for comparing and for "Simplify numbers": digits, with a leading `+` kept.
pub fn phone_digits(number: &str) -> String {
    let trimmed = number.trim();
    let mut out = String::new();
    if trimmed.starts_with('+') {
        out.push('+');
    }
    out.extend(trimmed.chars().filter(char::is_ascii_digit));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ignores_case_accents_and_spacing() {
        assert_eq!(plain("  José   GARCÍA-López "), "jose garcia lopez");
    }

    #[test]
    fn phone_digits_keeps_the_plus() {
        assert_eq!(phone_digits(" +1 (555) 010-2030 "), "+15550102030");
        assert_eq!(phone_digits("555.010.2030"), "5550102030");
    }

    #[test]
    fn base64_goes_both_ways() {
        for bytes in [&b""[..], b"a", b"ab", b"abc", &[0, 255, 16, 200, 7]] {
            assert_eq!(unbase64(&base64(bytes)).unwrap(), bytes);
        }
        assert_eq!(base64(b"ana:app-password"), "YW5hOmFwcC1wYXNzd29yZA==");
        assert_eq!(unbase64("YW Jj\r\n ZA==").unwrap(), b"abcd");
        assert!(unbase64("a*b").is_none());
    }

    #[test]
    fn decode_handles_utf16_and_latin1() {
        assert_eq!(decode(&[0xFF, 0xFE, b'h', 0, b'i', 0]), "hi");
        assert_eq!(decode(&[b'J', b'o', b's', 0xE9]), "José");
    }
}
