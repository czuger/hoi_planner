use std::collections::HashMap;

use crate::wiki::text::decode_entities;

/// Elements whose content is not markup.
const RAW_TEXT_TAGS: [&str; 2] = ["script", "style"];

/// One piece of a saved page, tolerant of the markup's looseness: nothing here checks that
/// tags balance, which is the business of whoever reads the tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    /// `<tag attr="value">`, lowercased, or `<tag/>` when `closed`.
    Start {
        /// The tag name.
        tag: String,
        /// The attributes, values decoded; the last of a repeated one wins.
        attrs: HashMap<String, String>,
        /// Written `<tag/>`.
        closed: bool,
    },
    /// `</tag>`, lowercased.
    End(String),
    /// The words between tags, decoded.
    Text(String),
}

/// Splits markup into tokens, skipping comments and declarations.
pub fn tokenize(markup: &str) -> Vec<HtmlToken> {
    let mut tokens = Vec::new();
    let mut rest = markup;
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            push_text(&mut tokens, rest);
            break;
        };
        push_text(&mut tokens, &rest[..at]);
        rest = &rest[at..];
        rest = read_markup(rest, &mut tokens);
    }
    tokens
}

/// Reads the construct `rest` opens with, and returns what follows it.
fn read_markup<'a>(rest: &'a str, tokens: &mut Vec<HtmlToken>) -> &'a str {
    let second = rest[1..].chars().next();
    if rest.starts_with("<!--") {
        return after(rest, "-->");
    }
    match second {
        Some('!' | '?') => after(rest, ">"),
        Some('/') => read_end_tag(rest, tokens),
        Some(c) if c.is_ascii_alphabetic() => read_start_tag(rest, tokens),
        _ => {
            push_text(tokens, "<");
            &rest[1..]
        }
    }
}

fn after<'a>(rest: &'a str, closer: &str) -> &'a str {
    rest.find(closer)
        .map_or("", |at| &rest[at + closer.len()..])
}

fn push_text(tokens: &mut Vec<HtmlToken>, text: &str) {
    if !text.is_empty() {
        tokens.push(HtmlToken::Text(decode_entities(text)));
    }
}

fn read_end_tag<'a>(rest: &'a str, tokens: &mut Vec<HtmlToken>) -> &'a str {
    let body = &rest[2..];
    if body.starts_with(|c: char| c.is_ascii_alphabetic()) {
        let (name, _) = split_tag_name(body);
        tokens.push(HtmlToken::End(name.to_ascii_lowercase()));
    }
    after(rest, ">")
}

fn split_tag_name(body: &str) -> (&str, &str) {
    let end = body
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .unwrap_or(body.len());
    body.split_at(end)
}

fn read_start_tag<'a>(rest: &'a str, tokens: &mut Vec<HtmlToken>) -> &'a str {
    let (name, mut cursor) = split_tag_name(&rest[1..]);
    let tag = name.to_ascii_lowercase();
    let mut attrs = HashMap::new();
    let mut closed = false;
    loop {
        cursor = cursor.trim_start();
        if let Some(following) = cursor.strip_prefix("/>") {
            closed = true;
            cursor = following;
            break;
        }
        if let Some(following) = cursor.strip_prefix('>') {
            cursor = following;
            break;
        }
        if cursor.is_empty() {
            break;
        }
        cursor = read_attribute(cursor, &mut attrs);
    }
    let is_raw = !closed && RAW_TEXT_TAGS.contains(&tag.as_str());
    tokens.push(HtmlToken::Start {
        tag: tag.clone(),
        attrs,
        closed,
    });
    if is_raw {
        return read_raw_text(cursor, &tag, tokens);
    }
    cursor
}

fn read_attribute<'a>(cursor: &'a str, attrs: &mut HashMap<String, String>) -> &'a str {
    let name_end = cursor
        .find(|c: char| c.is_ascii_whitespace() || matches!(c, '=' | '>' | '/'))
        .unwrap_or(cursor.len());
    if name_end == 0 {
        // A stray `=` or `/` names nothing: step over it.
        return &cursor[1..];
    }
    let name = cursor[..name_end].to_ascii_lowercase();
    let after_name = cursor[name_end..].trim_start();
    let Some(value_part) = after_name.strip_prefix('=') else {
        attrs.insert(name, String::new());
        return after_name;
    };
    let (value, following) = split_attribute_value(value_part.trim_start());
    attrs.insert(name, decode_entities(value));
    following
}

fn split_attribute_value(text: &str) -> (&str, &str) {
    if let Some(quote) = text.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        let body = &text[1..];
        return match body.find(quote) {
            Some(end) => (&body[..end], &body[end + 1..]),
            None => (body, ""),
        };
    }
    let end = text
        .find(|c: char| c.is_ascii_whitespace() || c == '>')
        .unwrap_or(text.len());
    text.split_at(end)
}

fn read_raw_text<'a>(cursor: &'a str, tag: &str, tokens: &mut Vec<HtmlToken>) -> &'a str {
    let closer = format!("</{tag}");
    let lowered = cursor.to_ascii_lowercase();
    let end = lowered.find(&closer).unwrap_or(cursor.len());
    if end > 0 {
        tokens.push(HtmlToken::Text(cursor[..end].to_string()));
    }
    &cursor[end..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag_of(token: &HtmlToken) -> Option<&str> {
        match token {
            HtmlToken::Start { tag, .. } => Some(tag),
            _ => None,
        }
    }

    #[test]
    fn tags_and_attributes_are_lowercased_and_values_decoded() {
        let tokens = tokenize(r#"<A HREF="/a?b=1&amp;c=2" Title='x' data-y=z hidden>t</A>"#);
        let HtmlToken::Start { tag, attrs, closed } = &tokens[0] else {
            panic!("not a start tag: {tokens:?}");
        };
        assert_eq!(tag, "a");
        assert_eq!(attrs["href"], "/a?b=1&c=2");
        assert_eq!(attrs["title"], "x");
        assert_eq!(attrs["data-y"], "z");
        assert_eq!(attrs["hidden"], "");
        assert!(!closed);
        assert_eq!(tokens[1], HtmlToken::Text("t".to_string()));
        assert_eq!(tokens[2], HtmlToken::End("a".to_string()));
    }

    #[test]
    fn a_closed_tag_says_so() {
        let tokens = tokenize("<br/><br />");
        assert!(tokens.iter().all(
            |token| matches!(token, HtmlToken::Start { closed: true, tag, .. } if tag == "br")
        ));
    }

    #[test]
    fn comments_and_declarations_are_skipped() {
        let tokens = tokenize("<!doctype html><!-- <b>no</b> --><?x?><i>y</i>");
        assert_eq!(tag_of(&tokens[0]), Some("i"));
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn a_stray_angle_bracket_is_text() {
        assert_eq!(
            tokenize("1 < 2"),
            [
                HtmlToken::Text("1 ".to_string()),
                HtmlToken::Text("<".to_string()),
                HtmlToken::Text(" 2".to_string()),
            ]
        );
    }

    #[test]
    fn a_script_holds_no_markup() {
        let tokens = tokenize("<script>if (a<b) { c('</i>'); }</SCRIPT><i>");
        assert_eq!(
            tokens[1],
            HtmlToken::Text("if (a<b) { c('</i>'); }".to_string())
        );
        assert_eq!(tokens[2], HtmlToken::End("script".to_string()));
        assert_eq!(tag_of(&tokens[3]), Some("i"));
    }

    #[test]
    fn an_unfinished_tag_does_not_hang() {
        assert_eq!(tag_of(&tokenize(r#"<a href="x"#)[0]), Some("a"));
        assert_eq!(tag_of(&tokenize("<a b")[0]), Some("a"));
    }
}
