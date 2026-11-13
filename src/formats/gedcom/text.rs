//! Turning a GEDCOM file's bytes into text, including the ANSEL character set.

use unicode_normalization::UnicodeNormalization;

fn ansel_spacing(b: u8) -> Option<char> {
    Some(match b {
        0xA1 => 'Ł', 0xA2 => 'Ø', 0xA3 => 'Đ', 0xA4 => 'Þ', 0xA5 => 'Æ', 0xA6 => 'Œ', 0xA7 => 'ʹ',
        0xA8 => '·', 0xA9 => '♭', 0xAA => '®', 0xAB => '±', 0xAC => 'Ơ', 0xAD => 'Ư', 0xAE => 'ʼ',
        0xB0 => 'ʻ', 0xB1 => 'ł', 0xB2 => 'ø', 0xB3 => 'đ', 0xB4 => 'þ', 0xB5 => 'æ', 0xB6 => 'œ',
        0xB7 => 'ʺ', 0xB8 => 'ı', 0xB9 => '£', 0xBA => 'ð', 0xBC => 'ơ', 0xBD => 'ư', 0xC0 => '°',
        0xC1 => 'ℓ', 0xC2 => '℗', 0xC3 => '©', 0xC4 => '♯', 0xC5 => '¿', 0xC6 => '¡', 0xCF => 'ß',
        _ => return None,
    })
}

fn ansel_combining(b: u8) -> Option<char> {
    Some(match b {
        0xE0 => '\u{0309}', 0xE1 => '\u{0300}', 0xE2 => '\u{0301}', 0xE3 => '\u{0302}',
        0xE4 => '\u{0303}', 0xE5 => '\u{0304}', 0xE6 => '\u{0306}', 0xE7 => '\u{0307}',
        0xE8 => '\u{0308}', 0xE9 => '\u{030C}', 0xEA => '\u{030A}', 0xEB => '\u{FE20}',
        0xEC => '\u{FE21}', 0xED => '\u{0315}', 0xEE => '\u{030B}', 0xEF => '\u{0310}',
        0xF0 => '\u{0327}', 0xF1 => '\u{0328}', 0xF2 => '\u{0323}', 0xF3 => '\u{0324}',
        0xF4 => '\u{0325}', 0xF5 => '\u{0333}', 0xF6 => '\u{0332}', 0xF7 => '\u{0326}',
        0xF8 => '\u{031C}', 0xF9 => '\u{032E}', 0xFA => '\u{FE22}', 0xFB => '\u{FE23}',
        0xFE => '\u{0313}',
        _ => return None,
    })
}

/// ANSEL puts accents before their letter; Unicode puts them after.
fn decode_ansel(bytes: &[u8]) -> (String, usize) {
    let mut out = String::with_capacity(bytes.len());
    let mut pending: Vec<char> = vec![];
    let mut unknown = 0;
    for &b in bytes {
        if let Some(mark) = ansel_combining(b) {
            pending.push(mark);
            continue;
        }
        let c = if b < 0x80 {
            b as char
        } else if let Some(c) = ansel_spacing(b) {
            c
        } else {
            unknown += 1;
            '\u{FFFD}'
        };
        out.push(c);
        // A line break can't carry an accent; don't move it onto the next line's first letter.
        if c == '\n' || c == '\r' {
            pending.clear();
        } else {
            out.extend(pending.drain(..).rev());
        }
    }
    (out.nfc().collect(), unknown)
}

fn declared_charset(bytes: &[u8]) -> Option<String> {
    // The header is plain ASCII, so it can be read before the rest is decoded.
    let head = &bytes[..bytes.len().min(8192)];
    let text: String = head.iter().map(|b| *b as char).collect();
    text.split(['\r', '\n']).find_map(|line| {
        let mut parts = line.split_whitespace();
        (parts.next() == Some("1") && parts.next() == Some("CHAR"))
            .then(|| parts.next().unwrap_or("").to_ascii_uppercase())
    })
}

pub(crate) fn decode(bytes: &[u8], warnings: &mut Vec<String>) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let utf16 = if bytes.starts_with(&[0xFF, 0xFE]) || (bytes.len() > 1 && bytes[0] == b'0' && bytes[1] == 0) {
        Some(encoding_rs::UTF_16LE)
    } else if bytes.starts_with(&[0xFE, 0xFF]) || (bytes.len() > 1 && bytes[0] == 0 && bytes[1] == b'0') {
        Some(encoding_rs::UTF_16BE)
    } else {
        None
    };
    if let Some(enc) = utf16 {
        let (text, _, bad) = enc.decode(bytes);
        if bad {
            warnings.push("Some characters couldn't be read and show as \u{FFFD}.".into());
        }
        return text.trim_start_matches('\u{FEFF}').to_string();
    }
    let charset = declared_charset(bytes).unwrap_or_default();
    match charset.as_str() {
        "ANSEL" => {
            let (text, unknown) = decode_ansel(bytes);
            if unknown > 0 {
                warnings.push(format!("{unknown} characters couldn't be read and show as \u{FFFD}."));
            }
            text
        }
        "MACINTOSH" | "MACROMAN" => encoding_rs::MACINTOSH.decode(bytes).0.into_owned(),
        _ => match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                if !matches!(charset.as_str(), "ANSI" | "ASCII" | "WINDOWS-1252" | "CP1252" | "ISO-8859-1" | "LATIN1") {
                    warnings.push(format!(
                        "The file's character set ({}) wasn't recognised, so it was read as Windows Latin. Check accented letters.",
                        if charset.is_empty() { "not stated" } else { &charset }
                    ));
                }
                encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
            }
        },
    }
}
