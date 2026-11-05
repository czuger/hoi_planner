//! The focus page's parser.

mod common;

use std::collections::HashMap;

use common::wiki_markup::{
    ICON_IMAGE, excludes, named_link, plain_row, requirements, requires, requires_any,
    wiki_heading, wiki_row,
};
use hoi4_planner::domain::Focus;
use hoi4_planner::wiki::parse_wiki_page;

#[test]
fn parses_a_row_into_a_focus() {
    let markup = wiki_heading("Rhineland Branch")
        + &plain_row("Fortify_the_Vaterland", "Fortify the Vaterland");
    let focuses = parse_wiki_page(&markup);
    let focus = &focuses[0];

    assert_eq!(focuses.len(), 1);
    assert_eq!(focus.id, "Fortify_the_Vaterland");
    assert_eq!(focus.name, "Fortify the Vaterland");
    assert_eq!(focus.section, "Rhineland Branch");
    assert_eq!(focus.position, 0);
    // The icon is a link to the wiki, never a downloaded file.
    assert!(
        focus
            .icon
            .starts_with("https://hoi4.paradoxwikis.com/images/")
    );
    assert!(
        focus
            .icon_page
            .starts_with("https://hoi4.paradoxwikis.com/File:")
    );
}

#[test]
fn focuses_keep_the_page_order_and_their_branch() {
    let markup = wiki_heading("Rhineland Branch")
        + &plain_row("A", "Alpha")
        + &wiki_heading("Anschluss Sub-branch")
        + &plain_row("B", "Beta");
    let focuses = parse_wiki_page(&markup);

    assert_eq!(focuses[0].name, "Alpha");
    assert_eq!(focuses[1].name, "Beta");
    assert_eq!((focuses[0].position, focuses[1].position), (0, 1));
    assert_eq!(focuses[0].section, "Rhineland Branch");
    assert_eq!(focuses[1].section, "Anschluss Sub-branch");
}

#[test]
fn a_focus_outside_any_heading_is_uncategorized() {
    let focuses = parse_wiki_page(&plain_row("A", "Alpha"));
    assert_eq!(focuses[0].section, "Uncategorized");
}

#[test]
fn entities_in_a_name_are_decoded() {
    let focuses = parse_wiki_page(&plain_row("A", "Blood &amp; Iron"));
    assert_eq!(focuses[0].name, "Blood & Iron");
}

#[test]
fn thumbnail_rows_are_not_a_second_copy_of_a_focus() {
    // A focus reappears in the prerequisite list of every focus that follows it, where the
    // wiki links a resized copy.
    let thumbnail = "/thumb.php?f=Goal_generic.png&width=36";
    let markup = plain_row("A", "Alpha") + &wiki_row("A", "Alpha", thumbnail, "");
    assert_eq!(parse_wiki_page(&markup).len(), 1);
}

#[test]
fn the_same_row_twice_is_stored_once() {
    let markup = plain_row("A", "Alpha") + &plain_row("A", "Alpha");
    assert_eq!(parse_wiki_page(&markup).len(), 1);
}

#[test]
fn a_page_with_no_focus_row_gives_nothing() {
    assert!(parse_wiki_page("<p>nothing here</p>").is_empty());
}

/// Alpha and Beta to depend on, and Gamma carrying the markup under test.
fn dependencies(cells: &str) -> HashMap<String, Focus> {
    let markup = plain_row("Alpha", "Alpha")
        + &plain_row("Beta", "Beta")
        + &wiki_row("Gamma", "Gamma", ICON_IMAGE, cells);
    parse_wiki_page(&markup)
        .into_iter()
        .map(|focus| (focus.id.clone(), focus))
        .collect()
}

#[test]
fn a_focus_with_no_requirement_cell_depends_on_nothing() {
    let focuses = parse_wiki_page(&plain_row("Alpha", "Alpha"));
    assert!(focuses[0].requires.is_empty());
    assert!(focuses[0].excludes.is_empty());
}

#[test]
fn requires_the_following_is_one_group_for_each_focus() {
    let cells = requirements(&[requires(&["Alpha", "Beta"])], "");
    assert_eq!(
        dependencies(&cells)["Gamma"].requires,
        [["Alpha"], ["Beta"]]
    );
}

#[test]
fn requires_one_of_the_following_is_a_single_group() {
    let cells = requirements(&[requires_any(&["Alpha", "Beta"])], "");
    assert_eq!(dependencies(&cells)["Gamma"].requires, [["Alpha", "Beta"]]);
}

#[test]
fn a_choice_nested_inside_a_list_is_a_group_of_its_own() {
    let nested = requires_any(&["Beta", "Gamma"]);
    let cells = requirements(&[requires(&["Alpha", &nested])], "");
    assert_eq!(
        dependencies(&cells)["Gamma"].requires,
        vec![vec!["Alpha"], vec!["Beta", "Gamma"]]
    );
}

#[test]
fn what_the_catalog_cannot_name_is_dropped() {
    let off_page = format!("<li>{}</li>", named_link("Elsewhere", "Off-page focus"));
    let cells = requirements(&[requires(&[&off_page, "Alpha"])], "");
    assert_eq!(dependencies(&cells)["Gamma"].requires, [["Alpha"]]);
}

#[test]
fn a_requirement_of_nothing_the_catalog_knows_is_no_requirement() {
    let off_page = format!("<li>{}</li>", named_link("Elsewhere", "Off-page focus"));
    let cells = requirements(&[requires(&[&off_page])], "");
    assert!(dependencies(&cells)["Gamma"].requires.is_empty());
}

#[test]
fn a_requirement_that_is_not_a_link_at_all_is_ignored() {
    let cells = requirements(&[requires(&["<li>Is fully independent</li>", "Alpha"])], "");
    assert_eq!(dependencies(&cells)["Gamma"].requires, [["Alpha"]]);
}

#[test]
fn mutual_exclusion_is_read_from_both_sides() {
    let found = dependencies(&requirements(&[excludes(&["Alpha"])], ""));
    assert_eq!(found["Gamma"].excludes, ["Alpha"]);
    assert_eq!(found["Alpha"].excludes, ["Gamma"]);
}

#[test]
fn a_focus_is_not_made_exclusive_with_itself() {
    let found = dependencies(&requirements(&[excludes(&["Gamma"])], ""));
    assert!(found["Gamma"].excludes.is_empty());
}

#[test]
fn requirements_and_exclusions_of_one_focus_are_read_together() {
    let cells = requirements(&[requires(&["Alpha"]), excludes(&["Beta"])], "");
    let found = dependencies(&cells);
    assert_eq!(found["Gamma"].requires, [["Alpha"]]);
    assert_eq!(found["Gamma"].excludes, ["Beta"]);
}

#[test]
fn only_the_requirement_cell_is_read() {
    // A real row's effects cell is full of wording that reads like a requirement.
    let effects = format!("<ul>{}</ul>", requires(&["Beta"]));
    let cells = requirements(&[requires(&["Alpha"])], &effects);
    assert_eq!(dependencies(&cells)["Gamma"].requires, [["Alpha"]]);
}
