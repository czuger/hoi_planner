use crate::wiki::html_tree::{HtmlTree, NodeId};
use crate::wiki::text::clean;

/// The focuses an element links, once each, in order; `own` skips nested lists.
pub fn focus_links(tree: &HtmlTree, node: NodeId, own: bool) -> Vec<String> {
    let mut ids = Vec::new();
    for child in tree.elements(node) {
        let tag = tree.tag(child);
        if own && matches!(tag, "ul" | "ol") {
            continue;
        }
        let href = tree.attr(child, "href").unwrap_or("");
        let found = if tag == "a" && href.contains("national_focus_tree") && href.contains('#') {
            vec![focus_id(tree, child, href)]
        } else {
            focus_links(tree, child, own)
        };
        crate::wiki::text::extend_unique(&mut ids, found);
    }
    ids
}

/// The focus a link names: its anchor on the focus tree's page.
///
/// The wiki sometimes links a focus to the anchor of its whole branch; the link's own words,
/// or its icon's, still name the focus.
fn focus_id(tree: &HtmlTree, link: NodeId, href: &str) -> String {
    let anchor = href.split_once('#').map_or("", |(_, anchor)| anchor);
    if !anchor.ends_with("_Branch") {
        return anchor.to_string();
    }
    let words = clean(&tree.text(link));
    let name = if words.is_empty() {
        tree.find(link, "img")
            .first()
            .and_then(|image| tree.attr(*image, "alt"))
            .unwrap_or("")
            .to_string()
    } else {
        words
    };
    if name.is_empty() {
        anchor.to_string()
    } else {
        name.replace(' ', "_")
    }
}
