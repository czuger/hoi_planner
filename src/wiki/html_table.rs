use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::wiki::html_tree::{HtmlTree, NodeId, ROOT};
use crate::wiki::text::{clean, leading_digits};

// safe: the pattern is a compile-time constant.
static HEADER_NOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" \(.*$").expect("valid regex"));

/// One row of a table: `{column heading: cell}`.
pub type TableRow = HashMap<String, NodeId>;

/// The tree of the part of the page between two headings, empty when the first is missing.
pub fn section_tree(markup: &str, start: &str, end: &str) -> HtmlTree {
    let Some(found) = markup.find(start) else {
        return HtmlTree::parse("");
    };
    let begin = markup[..found].rfind("<h").unwrap_or(0);
    let stop = markup[begin..]
        .find(end)
        .and_then(|at| markup[..begin + at].rfind("<h"))
        .unwrap_or(markup.len());
    HtmlTree::parse(&markup[begin..stop])
}

/// The tables of a tree, nested ones included.
pub fn tables(tree: &HtmlTree) -> Vec<NodeId> {
    tree.find(ROOT, "table")
}

/// A table as rows, a spanning cell repeated on each row it covers.
///
/// A person with two traits is one row whose portrait, name and party span two, and the
/// second `<tr>` only carries the cells that differ: the grid is rebuilt here, and the same
/// cell shows up on both rows, which is how a caller recognizes them as one person.
pub fn table_rows(tree: &HtmlTree, table: NodeId) -> Vec<TableRow> {
    let trs = own_rows(tree, table);
    let Some((head, body)) = trs.split_first() else {
        return Vec::new();
    };
    let header: Vec<String> = cells_of(tree, *head)
        .into_iter()
        .map(|cell| {
            HEADER_NOTE
                .replace(&clean(&tree.text(cell)), "")
                .to_string()
        })
        .collect();

    let mut spanning: HashMap<usize, (NodeId, usize)> = HashMap::new();
    body.iter()
        .map(|tr| grid_row(tree, *tr, &header, &mut spanning))
        .collect()
}

fn own_rows(tree: &HtmlTree, table: NodeId) -> Vec<NodeId> {
    tree.find(table, "tr")
        .into_iter()
        .filter(|tr| {
            let parent = tree.parent(*tr);
            parent == Some(table) || parent.and_then(|p| tree.parent(p)) == Some(table)
        })
        .collect()
}

fn cells_of(tree: &HtmlTree, tr: NodeId) -> Vec<NodeId> {
    tree.elements(tr)
        .into_iter()
        .filter(|cell| matches!(tree.tag(*cell), "td" | "th"))
        .collect()
}

fn grid_row(
    tree: &HtmlTree,
    tr: NodeId,
    header: &[String],
    spanning: &mut HashMap<usize, (NodeId, usize)>,
) -> TableRow {
    let mut cells = cells_of(tree, tr).into_iter();
    let mut row = TableRow::new();
    for (column, name) in header.iter().enumerate() {
        if let Some((cell, left)) = spanning.get_mut(&column).filter(|(_, left)| *left > 0) {
            row.insert(name.clone(), *cell);
            *left -= 1;
            continue;
        }
        let Some(cell) = cells.next() else {
            break;
        };
        row.insert(name.clone(), cell);
        let rowspan = tree
            .attr(cell, "rowspan")
            .and_then(|span| leading_digits(span.trim()).parse::<usize>().ok())
            .unwrap_or(1);
        if rowspan > 1 {
            spanning.insert(column, (cell, rowspan - 1));
        }
    }
    row
}
