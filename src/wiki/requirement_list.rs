use crate::wiki::html_token::{HtmlToken, tokenize};

/// One `<li>` of a table cell: its own words, the focuses it links, and the `<li>`s inside it.
///
/// A real walk over the tags rather than a pattern, because the requirement lists nest and
/// only the tags can tell which `</ul>` closes which. Text and links land on the innermost
/// open `<li>`, so a child's words never bleed into its parent's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListItem {
    /// The words of the `<li>` itself.
    pub text: String,
    /// The anchors it links, which is how the wiki names a focus, once each.
    pub focuses: Vec<String>,
    /// The `<li>`s of the `<ul>` inside it.
    pub children: Vec<ListItem>,
}

/// The top-level `<li>`s of one table cell.
pub fn parse_list_items(cell: &str) -> Vec<ListItem> {
    let mut open = vec![ListItem::default()];
    for token in tokenize(cell) {
        match token {
            HtmlToken::Start { tag, attrs, closed } => {
                if tag == "li" {
                    open.push(ListItem::default());
                    if closed {
                        close_innermost(&mut open);
                    }
                } else if tag == "a" {
                    add_link(&mut open, attrs.get("href").map_or("", String::as_str));
                }
            }
            HtmlToken::End(tag) if tag == "li" => close_innermost(&mut open),
            HtmlToken::Text(text) => {
                if let Some(item) = open.last_mut() {
                    item.text.push_str(&text);
                }
            }
            HtmlToken::End(_) => {}
        }
    }
    while open.len() > 1 {
        close_innermost(&mut open);
    }
    open.pop().map_or_else(Vec::new, |root| root.children)
}

/// The wiki links a focus twice, as its icon and as its name.
fn add_link(open: &mut [ListItem], href: &str) {
    let Some((_, anchor)) = href.split_once('#') else {
        return;
    };
    let Some(item) = open.last_mut() else {
        return;
    };
    if !anchor.is_empty() && !item.focuses.iter().any(|focus| focus == anchor) {
        item.focuses.push(anchor.to_string());
    }
}

fn close_innermost(open: &mut Vec<ListItem>) {
    if open.len() > 1
        && let Some(item) = open.pop()
        && let Some(parent) = open.last_mut()
    {
        parent.children.push(item);
    }
}
