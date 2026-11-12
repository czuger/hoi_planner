use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::domain::StaffMember;
use crate::domain::rules::WIKI_BASE;
use crate::wiki::html_tree::{HtmlTree, NodeId};

// safe: the pattern is a compile-time constant.
static THUMBNAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/images/thumb(/.+?)/[^/]+$").expect("valid regex"));

/// A leader or a member of staff, in the shape of a focus.
///
/// What they need is said the way a focus says it, but the focus ids are not resolved against
/// the catalog: most routes run through branches the saved focus page does not carry, and
/// dropping them would lose the one thing these entries say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PoliticsEntry {
    /// Made from the name, unique across the page.
    pub id: String,
    /// The name as the page writes it.
    pub name: String,
    /// The wiki's copy of the portrait or logo; not everyone has one.
    pub icon: Option<String>,
    /// The file's page on the wiki.
    pub icon_page: Option<String>,
    /// The leader's ideology, or the staff's table.
    pub section: String,
    /// Document order on the page.
    pub position: i64,
    /// Groups of focus ids: every group needs one of its focuses taken.
    pub requires: Vec<Vec<String>>,
    /// The focuses that must not be taken.
    pub excludes: Vec<String>,
}

impl PoliticsEntry {
    /// An entry whose id `assign_ids` will fill in, its picture read from `picture_cell`.
    pub fn new(
        tree: &HtmlTree,
        name: String,
        picture_cell: Option<NodeId>,
        section: String,
        position: i64,
    ) -> PoliticsEntry {
        let (icon, icon_page) = picture_links(tree, picture_cell);
        PoliticsEntry {
            id: String::new(),
            name,
            icon,
            icon_page,
            section,
            position,
            requires: Vec::new(),
            excludes: Vec::new(),
        }
    }

    /// A row of a staff table as the catalog keeps it, a missing picture as empty links.
    pub fn into_staff_member(self) -> StaffMember {
        StaffMember {
            id: self.id,
            name: self.name,
            icon: self.icon.unwrap_or_default(),
            icon_page: self.icon_page.unwrap_or_default(),
            section: self.section,
            position: self.position,
            requires: self.requires,
            excludes: self.excludes,
        }
    }
}

/// A portrait or logo as a focus keeps its icon: links to the wiki's full-size copy.
pub(crate) fn picture_links(
    tree: &HtmlTree,
    cell: Option<NodeId>,
) -> (Option<String>, Option<String>) {
    let Some(cell) = cell else {
        return (None, None);
    };
    let Some(image) = tree.find(cell, "img").first().copied() else {
        return (None, None);
    };
    let src = tree.attr(image, "src").unwrap_or("");
    let src = THUMBNAIL.replace(src, "/images$1");
    let page = tree
        .find(cell, "a")
        .into_iter()
        .filter_map(|link| tree.attr(link, "href"))
        .find(|href| href.starts_with("/File:"));
    (
        Some(format!("{WIKI_BASE}{src}")),
        page.map(|href| format!("{WIKI_BASE}{href}")),
    )
}

/// Gives every entry an id from its name, underscored as the wiki anchors a focus.
///
/// A person in several tables takes the table after the name, and one listed twice in the
/// same table a number.
pub fn assign_ids(entries: &mut [PoliticsEntry]) {
    let mut sections: HashMap<String, HashSet<String>> = HashMap::new();
    for entry in entries.iter() {
        sections
            .entry(entry.name.clone())
            .or_default()
            .insert(entry.section.clone());
    }
    let mut taken: HashMap<String, usize> = HashMap::new();
    for entry in entries.iter_mut() {
        let mut id = entry.name.replace(' ', "_");
        if sections.get(&entry.name).is_some_and(|s| s.len() > 1) {
            id = format!("{id}__{}", entry.section.replace(' ', "_"));
        }
        let count = taken.entry(id.clone()).or_insert(0);
        *count += 1;
        entry.id = if *count == 1 {
            id
        } else {
            format!("{id}__{count}")
        };
    }
}
