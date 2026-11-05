use std::collections::{HashMap, HashSet};

use crate::domain::Technology;
use crate::wiki::html_table::{TableRow, table_rows};
use crate::wiki::html_tree::{HtmlTree, NodeId, ROOT};
use crate::wiki::mutual_exclusion::make_exclusions_mutual;
use crate::wiki::politics_entry::picture_links;
use crate::wiki::text::{clean, extend_unique, leading_digits, phrase};

const DLC_NOTICE: &str = "available only with the";
const DLC_OFF: &str = "dlc disabled";
const ONE_OF: &str = "one of";
const EXCLUDES: &str = "excludes";
/// These cells name who can get a secret tech, and their links go to other countries' focuses.
const NOT_PREREQUISITES: [&str; 2] = ["can unlock this tech", "starts with this tech"];
/// A prerequisite links within its own page, or to one of these.
const TECHNOLOGY_PAGE_SUFFIX: &str = "_technology";

/// One saved page of the research tree.
#[derive(Debug, Clone, Copy)]
pub struct TechnologyPageSource<'a> {
    /// The research area the page covers, as the catalog names it.
    pub domain: &'a str,
    /// The page's markup.
    pub markup: &'a str,
}

/// The technologies of the saved pages, in page order, with every DLC taken to be on.
///
/// Read together because prerequisites cross pages, and resolved once all are read.
pub fn parse_technology_pages(pages: &[TechnologyPageSource<'_>]) -> Vec<Technology> {
    let mut reading = Reading::default();
    for page in pages {
        reading.read_page(page);
    }
    reading.resolve()
}

/// What the pages have given so far: `aliases` holds every anchor a technology answers to.
#[derive(Default)]
struct Reading {
    technologies: Vec<Technology>,
    seen: HashSet<String>,
    aliases: HashMap<String, String>,
}

impl Reading {
    /// Walks a page in document order, as a table belongs to the heading and notice before it.
    fn read_page(&mut self, page: &TechnologyPageSource<'_>) {
        let tree = HtmlTree::parse(page.markup);
        let mut main = String::new();
        let mut sub = String::new();
        let mut needs_dlc_off = false;
        for node in tree.descendants(ROOT) {
            match tree.tag(node) {
                "h2" | "h3" => {
                    let Some(title) = headline(&tree, node) else {
                        continue;
                    };
                    if tree.tag(node) == "h2" {
                        main = title;
                        sub.clear();
                    } else {
                        sub = title;
                    }
                    needs_dlc_off = false;
                }
                "table" if !is_nested(&tree, node) => {
                    if has_class(&tree, node, "ambox") {
                        needs_dlc_off = dlc_notice(&tree, node).unwrap_or(needs_dlc_off);
                    } else if !needs_dlc_off {
                        let section = if sub.is_empty() { &main } else { &sub };
                        self.read_table(&tree, node, page.domain, section);
                    }
                }
                _ => {}
            }
        }
    }

    fn read_table(&mut self, tree: &HtmlTree, table: NodeId, domain: &str, section: &str) {
        for row in table_rows(tree, table) {
            let Some(&cell) = row.get("Technology") else {
                continue;
            };
            let anchors = anchors_of(tree, cell);
            let Some(id) = anchors.first().cloned() else {
                continue;
            };
            for anchor in anchors {
                self.aliases.entry(anchor).or_insert_with(|| id.clone());
            }
            if self.seen.insert(id.clone()) {
                let position = self.technologies.len() as i64;
                self.technologies.push(new_technology(
                    tree, &row, cell, id, domain, section, position,
                ));
            }
        }
    }

    /// Turns the links into ids: what the catalog cannot name is dropped, and a group left with
    /// nothing stops being a requirement rather than stranding its technology forever.
    fn resolve(self) -> Vec<Technology> {
        let by_case: HashMap<String, String> = self
            .aliases
            .iter()
            .map(|(anchor, id)| (anchor.to_lowercase(), id.clone()))
            .collect();
        let named = |anchor: &String| {
            self.aliases
                .get(anchor)
                .or_else(|| by_case.get(&anchor.to_lowercase()))
                .cloned()
        };
        let mut technologies = self.technologies.clone();
        for technology in &mut technologies {
            let own_id = technology.id.clone();
            for group in &mut technology.requires {
                let ids = group.iter().filter_map(&named).filter(|id| *id != own_id);
                let mut resolved = Vec::new();
                extend_unique(&mut resolved, ids);
                *group = resolved;
            }
            technology.requires.retain(|group| !group.is_empty());
            let ids = technology.excludes.iter().filter_map(&named);
            let mut excludes = Vec::new();
            extend_unique(&mut excludes, ids.filter(|id| *id != own_id));
            technology.excludes = excludes;
        }
        make_exclusions_mutual(
            &mut technologies,
            |technology| &technology.id,
            |technology| &mut technology.excludes,
        );
        technologies
    }
}

fn headline(tree: &HtmlTree, heading: NodeId) -> Option<String> {
    tree.find(heading, "span")
        .into_iter()
        .find(|span| has_class(tree, *span, "mw-headline"))
        .map(|span| clean(&tree.text(span)))
}

fn has_class(tree: &HtmlTree, node: NodeId, class: &str) -> bool {
    tree.attr(node, "class")
        .is_some_and(|classes| classes.split_whitespace().any(|name| name == class))
}

fn is_nested(tree: &HtmlTree, table: NodeId) -> bool {
    let mut node = tree.parent(table);
    while let Some(id) = node {
        if tree.tag(id) == "table" {
            return true;
        }
        node = tree.parent(id);
    }
    false
}

/// Whether a notice wants a DLC off; `None` when it says nothing about DLCs at all.
fn dlc_notice(tree: &HtmlTree, notice: NodeId) -> Option<bool> {
    let words = phrase(&tree.text(notice));
    words.contains(DLC_NOTICE).then(|| words.contains(DLC_OFF))
}

/// Every anchor in the name cell, the one beside the name first: that is the technology's id,
/// as some rows open with an older anchor kept for old links.
fn anchors_of(tree: &HtmlTree, cell: NodeId) -> Vec<String> {
    let mut anchors = Vec::new();
    if let Some(name) = name_line(tree, cell) {
        extend_unique(&mut anchors, span_ids(tree, name));
    }
    extend_unique(&mut anchors, span_ids(tree, cell));
    anchors
}

fn span_ids(tree: &HtmlTree, node: NodeId) -> Vec<String> {
    tree.find(node, "span")
        .into_iter()
        .filter_map(|span| tree.attr(span, "id"))
        .map(str::to_string)
        .collect()
}

/// The bold line of a name cell.
fn name_line(tree: &HtmlTree, cell: NodeId) -> Option<NodeId> {
    tree.find(cell, "div").into_iter().find(|div| {
        tree.attr(*div, "style")
            .is_some_and(|style| style.contains("font-weight: bold"))
    })
}

fn new_technology(
    tree: &HtmlTree,
    row: &TableRow,
    cell: NodeId,
    id: String,
    domain: &str,
    section: &str,
    position: i64,
) -> Technology {
    let (icon, icon_page) = picture_links(tree, Some(cell));
    let (requires, excludes) = prerequisites(tree, row.get("Prerequisites").copied());
    Technology {
        name: name_of(tree, cell, &id),
        id,
        icon: icon.unwrap_or_default(),
        icon_page: icon_page.unwrap_or_default(),
        domain: domain.to_string(),
        section: section.to_string(),
        position,
        year: number_in(tree, row.get("Year").copied()),
        base_days: number_in(tree, row.get("Base time").copied()),
        requires,
        excludes,
    }
}

/// The bold line of the cell; failing that the icon's title, then the anchor.
fn name_of(tree: &HtmlTree, cell: NodeId, id: &str) -> String {
    let bold = name_line(tree, cell).map(|line| clean(&tree.text(line)));
    let titled = || {
        tree.find(cell, "a")
            .into_iter()
            .filter_map(|link| tree.attr(link, "title"))
            .find_map(|title| title.strip_suffix(" icon"))
            .map(str::to_string)
    };
    bold.filter(|name| !name.is_empty())
        .or_else(titled)
        .unwrap_or_else(|| id.replace('_', " "))
}

fn number_in(tree: &HtmlTree, cell: Option<NodeId>) -> Option<i64> {
    let text = clean(&tree.text(cell?));
    leading_digits(&text).parse().ok()
}

/// A Prerequisites cell as `requires` and `excludes`, still in anchors.
///
/// Links after the red `Excludes` are exclusions: node ids grow in document order. An `One of:`
/// list is a single group; any other link is a group of its own.
fn prerequisites(tree: &HtmlTree, cell: Option<NodeId>) -> (Vec<Vec<String>>, Vec<String>) {
    let mut requires = Vec::new();
    let mut excludes = Vec::new();
    let Some(cell) = cell else {
        return (requires, excludes);
    };
    let words = phrase(&tree.text(cell));
    if NOT_PREREQUISITES.iter().any(|form| words.contains(form)) {
        return (requires, excludes);
    }
    let excludes_from = tree
        .find(cell, "b")
        .into_iter()
        .find(|bold| phrase(&tree.text(*bold)) == EXCLUDES);
    let one_of = words.contains(ONE_OF);
    let mut choice = Vec::new();
    for link in tree.find(cell, "a") {
        let Some(anchor) = tree.attr(link, "href").and_then(technology_anchor) else {
            continue;
        };
        if excludes_from.is_some_and(|marker| link > marker) {
            extend_unique(&mut excludes, [anchor]);
        } else if one_of && is_in_list(tree, link, cell) {
            extend_unique(&mut choice, [anchor]);
        } else if !requires.contains(&vec![anchor.clone()]) {
            requires.push(vec![anchor]);
        }
    }
    if !choice.is_empty() {
        requires.push(choice);
    }
    (requires, excludes)
}

/// The anchor a link points at, when it can be a technology.
fn technology_anchor(href: &str) -> Option<String> {
    let (page, anchor) = href.split_once('#')?;
    let on_technology_page = page.is_empty() || page.ends_with(TECHNOLOGY_PAGE_SUFFIX);
    (on_technology_page && !anchor.is_empty()).then(|| anchor.to_string())
}

fn is_in_list(tree: &HtmlTree, link: NodeId, cell: NodeId) -> bool {
    let mut node = tree.parent(link);
    while let Some(id) = node.filter(|id| *id != cell) {
        if matches!(tree.tag(id), "ul" | "ol") {
            return true;
        }
        node = tree.parent(id);
    }
    false
}
