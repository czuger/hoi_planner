use std::sync::LazyLock;

use regex::Regex;

use crate::wiki::condition::Condition;
use crate::wiki::html_table::{TableRow, section_tree, table_rows, tables};
use crate::wiki::html_tree::{HtmlTree, NodeId};
use crate::wiki::politics_entry::{PoliticsEntry, assign_ids};
use crate::wiki::text::{clean, extend_unique};

// safe: the pattern is a compile-time constant.
static COLLAPSE_TOGGLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*(afficher|masquer)$").expect("valid regex"));

const START_HEADING: &str = r#"id="Staff_and_designers""#;
// The organizations after the designers are their trait trees, not something to hire.
const END_HEADING: &str = r#"id="Organizations""#;

/// Everyone and everything the Staff and designers section offers to hire.
///
/// One entry per row of each table, `section` naming the table. A person whose row spans two
/// is one entry, the second row adding its routes in.
pub fn parse_staff(markup: &str) -> Vec<PoliticsEntry> {
    let tree = section_tree(markup, START_HEADING, END_HEADING);
    let mut staff = Vec::new();
    for table in tables(&tree) {
        let Some(&caption) = tree.elements_named(table, "caption").first() else {
            continue;
        };
        let section = COLLAPSE_TOGGLE
            .replace(&clean(&tree.text(caption)), "")
            .to_string();
        read_table(&tree, table, &section, &mut staff);
    }
    assign_ids(&mut staff);
    staff
}

fn read_table(tree: &HtmlTree, table: NodeId, section: &str, staff: &mut Vec<PoliticsEntry>) {
    let mut previous_cell = None;
    for row in table_rows(tree, table) {
        let Some(&name_cell) = row.get("Name") else {
            continue;
        };
        let name = clean(&tree.text(name_cell));
        if name.is_empty() {
            continue;
        }
        let (requires, excludes) = dependencies(tree, row.get("Requirements").copied());
        if previous_cell == Some(name_cell)
            && let Some(entry) = staff.last_mut()
        {
            add_groups(entry, requires);
            continue;
        }
        staff.push(new_entry(
            tree,
            &row,
            name,
            section,
            staff.len(),
            requires,
            excludes,
        ));
        previous_cell = Some(name_cell);
    }
}

fn new_entry(
    tree: &HtmlTree,
    row: &TableRow,
    name: String,
    section: &str,
    position: usize,
    requires: Vec<Vec<String>>,
    excludes: Vec<String>,
) -> PoliticsEntry {
    let picture = row.get("Portrait").or_else(|| row.get("Logo")).copied();
    let mut entry = PoliticsEntry::new(tree, name, picture, section.to_string(), position as i64);
    entry.requires = requires;
    entry.excludes = excludes;
    entry
}

fn add_groups(entry: &mut PoliticsEntry, groups: Vec<Vec<String>>) {
    for group in groups {
        if !entry.requires.contains(&group) {
            entry.requires.push(group);
        }
    }
}

/// A Requirements cell as `requires` and `excludes`.
fn dependencies(tree: &HtmlTree, cell: Option<NodeId>) -> (Vec<Vec<String>>, Vec<String>) {
    let Some(cell) = cell else {
        return (Vec::new(), Vec::new());
    };
    let condition = Condition::of_cell(tree, cell);
    let mut requires: Vec<Vec<String>> = Vec::new();
    for mut group in condition.groups().unwrap_or_default() {
        group.sort();
        if !requires.contains(&group) {
            requires.push(group);
        }
    }
    let mut excludes = Vec::new();
    extend_unique(&mut excludes, condition.negations());
    (requires, excludes)
}
