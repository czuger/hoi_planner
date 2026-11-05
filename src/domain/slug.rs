use unicode_normalization::UnicodeNormalization;

/// A plan's name as its id: lowercase, ascii, dashes.
///
/// The id travels through a URL and a file name. A name that slugs to nothing, punctuation
/// only, has no usable id, which is for the caller to refuse.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in name.nfkd().filter(char::is_ascii) {
        let character = character.to_ascii_lowercase();
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_lowercase_ascii_and_dashes() {
        for (name, expected) in [
            ("Barbarossa rush", "barbarossa-rush"),
            ("  Spaced  Out  ", "spaced-out"),
            ("Plan #1: Anschluss!", "plan-1-anschluss"),
            ("Café Züger", "cafe-zuger"),
            ("!!!", ""),
        ] {
            assert_eq!(slugify(name), expected);
        }
    }
}
