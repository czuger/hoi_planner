//! Small text helpers shared by the page parsers.

use std::sync::LazyLock;

use regex::Regex;

// safe: the pattern is a compile-time constant.
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("valid regex"));

/// Words flattened to single spaces, without any around them.
pub fn clean(text: &str) -> String {
    WHITESPACE.replace_all(text, " ").trim().to_string()
}

/// A `<li>`'s own words flattened for comparing: lowercase, single-spaced.
pub fn phrase(text: &str) -> String {
    clean(text).to_lowercase()
}

/// Whether `text` opens with `prefix`, whatever its case.
pub fn starts_with_phrase(text: &str, prefix: &str) -> bool {
    phrase(text).starts_with(prefix)
}

/// The digits a text opens with, which is what `220 days` and a sloppy `5;` are worth.
pub fn leading_digits(text: &str) -> &str {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    &text[..end]
}

/// Appends the values not already there, keeping the order they came in.
pub fn extend_unique(list: &mut Vec<String>, values: impl IntoIterator<Item = String>) {
    for value in values {
        if !list.contains(&value) {
            list.push(value);
        }
    }
}

/// Decodes the character references the saved pages use.
///
/// The wiki writes everything else as UTF-8, so the named references are limited to these
/// and any other is left as written.
pub fn decode_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        decoded.push_str(&rest[..at]);
        rest = &rest[at..];
        match decode_reference(rest) {
            Some((character, length)) => {
                decoded.push(character);
                rest = &rest[length..];
            }
            None => {
                decoded.push('&');
                rest = &rest[1..];
            }
        }
    }
    decoded.push_str(rest);
    decoded
}

/// The character a reference at the start of `text` stands for, and how long it is.
fn decode_reference(text: &str) -> Option<(char, usize)> {
    let body = &text[1..];
    if let Some(number) = body.strip_prefix('#') {
        return decode_numeric(number).map(|(character, length)| (character, length + 2));
    }
    let name_length = body
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(body.len());
    let character = named_reference(&body[..name_length])?;
    let closed = body[name_length..].starts_with(';');
    Some((character, 1 + name_length + usize::from(closed)))
}

fn decode_numeric(text: &str) -> Option<(char, usize)> {
    let (digits, radix, marker) = match text.strip_prefix(['x', 'X']) {
        Some(hex) => (hex, 16, 1),
        None => (text, 10, 0),
    };
    let length = digits
        .find(|c: char| !c.is_digit(radix))
        .unwrap_or(digits.len());
    let code = u32::from_str_radix(&digits[..length], radix).ok()?;
    let closed = digits[length..].starts_with(';');
    let character = char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER);
    Some((character, marker + length + usize::from(closed)))
}

fn named_reference(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_are_decoded() {
        assert_eq!(
            decode_entities("a &amp; b&nbsp;c &#39;d&#x27; &quot;e&quot;"),
            "a & b\u{a0}c 'd' \"e\""
        );
    }

    #[test]
    fn an_unknown_reference_is_left_as_written() {
        assert_eq!(decode_entities("x&y=1 &bogus; &"), "x&y=1 &bogus; &");
    }

    #[test]
    fn leading_digits_stop_at_the_first_other_character() {
        assert_eq!(leading_digits("220 days"), "220");
        assert_eq!(leading_digits("5;"), "5");
        assert_eq!(leading_digits("–"), "");
    }

    #[test]
    fn clean_flattens_every_kind_of_space() {
        assert_eq!(clean("  a \n\t b\u{a0}c  "), "a b c");
    }
}
