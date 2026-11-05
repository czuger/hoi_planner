use crate::wiki::focus_link::focus_links;
use crate::wiki::html_table::{TableRow, section_tree, table_rows, tables};
use crate::wiki::html_tree::{HtmlTree, NodeId};
use crate::wiki::politics_entry::{PoliticsEntry, assign_ids};
use crate::wiki::text::{clean, extend_unique, starts_with_phrase};

const START_HEADING: &str = r#"id="Country_leaders""#;
const END_HEADING: &str = r#"id="Country_names""#;

/// The possible country leaders of the saved German Reich page, in page order.
///
/// `section` is the leader's ideology, and `requires` one group of every focus the page gives
/// as a route to them: a starting leader, or one only an event brings in, requires nothing.
pub fn parse_leaders(markup: &str) -> Vec<PoliticsEntry> {
    let tree = section_tree(markup, START_HEADING, END_HEADING);
    let mut leaders: Vec<PoliticsEntry> = Vec::new();
    let mut previous_cell = None;
    for table in tables(&tree) {
        for row in table_rows(&tree, table) {
            let Some(&name_cell) = row.get("Name") else {
                continue;
            };
            let routes = routes(&tree, row.get("Possible").copied());
            if previous_cell == Some(name_cell)
                && let Some(leader) = leaders.last_mut()
            {
                add_routes(leader, routes);
                continue;
            }
            leaders.push(new_leader(&tree, &row, name_cell, routes, leaders.len()));
            previous_cell = Some(name_cell);
        }
    }
    assign_ids(&mut leaders);
    leaders
}

fn new_leader(
    tree: &HtmlTree,
    row: &TableRow,
    name_cell: NodeId,
    routes: Vec<String>,
    position: usize,
) -> PoliticsEntry {
    let section = row
        .get("Ideology")
        .and_then(|cell| tree.find(*cell, "span").last().copied())
        .map_or_else(String::new, |span| clean(&tree.text(span)));
    let mut leader = PoliticsEntry::new(
        tree,
        clean(&tree.text(name_cell)),
        row.get("Portrait").copied(),
        section,
        position as i64,
    );
    if !routes.is_empty() {
        leader.requires = vec![routes];
    }
    leader
}

/// The same leader again, with another trait and another way in.
fn add_routes(leader: &mut PoliticsEntry, routes: Vec<String>) {
    if routes.is_empty() {
        return;
    }
    let mut group = Vec::new();
    extend_unique(&mut group, leader.requires.concat());
    extend_unique(&mut group, routes);
    leader.requires = vec![group];
}

/// The focuses a leader's Possible cell names as a way in ("Via focus…").
fn routes(tree: &HtmlTree, cell: Option<NodeId>) -> Vec<String> {
    let mut routes = Vec::new();
    let Some(cell) = cell else {
        return routes;
    };
    for item in tree.find(cell, "li") {
        if starts_with_phrase(&tree.own_text(item), "via focus") {
            extend_unique(&mut routes, focus_links(tree, item, true));
        }
    }
    routes
}
