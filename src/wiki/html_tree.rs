use std::collections::HashMap;

use crate::wiki::html_token::{HtmlToken, tokenize};
use crate::wiki::text::clean;

/// Elements with no closing tag.
const VOID_TAGS: [&str; 10] = [
    "img", "br", "hr", "input", "meta", "link", "wbr", "source", "col", "area",
];
/// Elements an unclosed item stops looking inside.
const CONTAINER_TAGS: [&str; 5] = ["ul", "ol", "table", "tbody", "thead"];

/// Index of an element in its `HtmlTree`.
pub type NodeId = usize;

/// The element at the top of every tree.
pub const ROOT: NodeId = 0;

#[derive(Debug, Clone)]
enum Child {
    Text(String),
    Element(NodeId),
}

#[derive(Debug, Clone)]
struct Node {
    tag: String,
    attrs: HashMap<String, String>,
    parent: Option<NodeId>,
    children: Vec<Child>,
}

/// Markup as a tree of elements.
///
/// The country page's tables nest and their cells span rows, which only a tree can line up.
/// Rows and items the markup leaves unclosed are closed by the next of their kind.
#[derive(Debug, Clone)]
pub struct HtmlTree {
    nodes: Vec<Node>,
}

impl HtmlTree {
    /// Builds the tree of `markup`.
    pub fn parse(markup: &str) -> HtmlTree {
        let mut tree = HtmlTree {
            nodes: vec![Node {
                tag: "#root".to_string(),
                attrs: HashMap::new(),
                parent: None,
                children: Vec::new(),
            }],
        };
        let mut current = ROOT;
        for token in tokenize(markup) {
            current = tree.add_token(current, token);
        }
        tree
    }

    fn add_token(&mut self, current: NodeId, token: HtmlToken) -> NodeId {
        match token {
            HtmlToken::Text(text) => {
                self.nodes[current].children.push(Child::Text(text));
                current
            }
            HtmlToken::End(tag) => self.close(current, &tag),
            HtmlToken::Start { tag, attrs, closed } => {
                if closed {
                    self.add_element(current, tag, attrs);
                    return current;
                }
                let parent = self.close_implied(current, &tag);
                let is_void = VOID_TAGS.contains(&tag.as_str());
                let node = self.add_element(parent, tag, attrs);
                if is_void { parent } else { node }
            }
        }
    }

    fn add_element(
        &mut self,
        parent: NodeId,
        tag: String,
        attrs: HashMap<String, String>,
    ) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            tag,
            attrs,
            parent: Some(parent),
            children: Vec::new(),
        });
        self.nodes[parent].children.push(Child::Element(id));
        id
    }

    /// The element a new `tag` opens in, once what it implicitly closes is closed.
    fn close_implied(&self, current: NodeId, tag: &str) -> NodeId {
        let Some(closes) = implicitly_closed_by(tag) else {
            return current;
        };
        let mut node = current;
        while let Some(parent) = self.nodes[node].parent {
            let name = self.tag(node);
            if CONTAINER_TAGS.contains(&name) || closes.contains(&name) {
                break;
            }
            node = parent;
        }
        if closes.contains(&self.tag(node)) {
            self.nodes[node].parent.unwrap_or(ROOT)
        } else {
            current
        }
    }

    /// A closing tag ends the nearest open element of its name; a stray one changes nothing.
    fn close(&self, current: NodeId, tag: &str) -> NodeId {
        let mut node = Some(current);
        while let Some(id) = node {
            if self.tag(id) == tag {
                return self.nodes[id].parent.unwrap_or(current);
            }
            node = self.nodes[id].parent;
        }
        current
    }

    /// The tag of an element.
    pub fn tag(&self, id: NodeId) -> &str {
        &self.nodes[id].tag
    }

    /// The value of an attribute, if the element has it.
    pub fn attr(&self, id: NodeId, name: &str) -> Option<&str> {
        self.nodes[id].attrs.get(name).map(String::as_str)
    }

    /// The parent of an element, `None` for the root.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id].parent
    }

    /// The element children of an element.
    pub fn elements(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes[id]
            .children
            .iter()
            .filter_map(|child| match child {
                Child::Element(element) => Some(*element),
                Child::Text(_) => None,
            })
            .collect()
    }

    /// The element children of one tag.
    pub fn elements_named(&self, id: NodeId, tag: &str) -> Vec<NodeId> {
        self.elements(id)
            .into_iter()
            .filter(|element| self.tag(*element) == tag)
            .collect()
    }

    /// Every element of one tag under this one, itself included, in document order.
    pub fn find(&self, id: NodeId, tag: &str) -> Vec<NodeId> {
        let mut found = Vec::new();
        self.collect_named(id, tag, &mut found);
        found
    }

    /// Every element under this one, in document order.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        for element in self.elements(id) {
            found.push(element);
            found.extend(self.descendants(element));
        }
        found
    }

    fn collect_named(&self, id: NodeId, tag: &str, found: &mut Vec<NodeId>) {
        if self.tag(id) == tag {
            found.push(id);
        }
        for element in self.elements(id) {
            self.collect_named(element, tag, found);
        }
    }

    /// All the text under an element, as written.
    pub fn text(&self, id: NodeId) -> String {
        self.nodes[id]
            .children
            .iter()
            .map(|child| match child {
                Child::Text(text) => text.clone(),
                Child::Element(element) => self.text(*element),
            })
            .collect()
    }

    /// The words of an element minus any list inside it: a `<li>`'s own.
    pub fn own_text(&self, id: NodeId) -> String {
        let text: String = self.nodes[id]
            .children
            .iter()
            .map(|child| match child {
                Child::Text(text) => text.clone(),
                Child::Element(element) if matches!(self.tag(*element), "ul" | "ol") => {
                    String::new()
                }
                Child::Element(element) => self.text(*element),
            })
            .collect();
        clean(&text)
    }
}

fn implicitly_closed_by(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "li" => Some(&["li"]),
        "tr" => Some(&["tr", "td", "th"]),
        "td" | "th" => Some(&["td", "th"]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unclosed_item_is_closed_by_the_next_and_stray_tags_are_ignored() {
        let tree = HtmlTree::parse("<ul><li>a<li>b<br/></span></ul>");
        let lists = tree.find(ROOT, "ul");
        let items: Vec<String> = tree
            .elements_named(lists[0], "li")
            .into_iter()
            .map(|item| tree.own_text(item))
            .collect();
        assert_eq!(items, ["a", "b"]);
    }

    #[test]
    fn a_nested_list_is_not_closed_by_its_own_items() {
        let tree = HtmlTree::parse("<ul><li>a<ul><li>b</li></ul></li><li>c</li></ul>");
        let lists = tree.find(ROOT, "ul");
        assert_eq!(tree.elements_named(lists[0], "li").len(), 2);
        assert_eq!(tree.elements_named(lists[1], "li").len(), 1);
    }

    #[test]
    fn descendants_come_in_document_order() {
        let tree = HtmlTree::parse("<div><h2>a</h2><p><b>b</b></p></div><table></table>");
        let tags: Vec<&str> = tree
            .descendants(ROOT)
            .into_iter()
            .map(|node| tree.tag(node))
            .collect();
        assert_eq!(tags, ["div", "h2", "p", "b", "table"]);
    }

    #[test]
    fn a_void_element_holds_nothing() {
        let tree = HtmlTree::parse("<p><img src=x>after</p>");
        let image = tree.find(ROOT, "img")[0];
        assert_eq!(tree.text(image), "");
        assert_eq!(tree.text(ROOT), "after");
    }
}
