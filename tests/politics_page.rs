//! The country page's parser: leaders, and the staff and designers.

mod common;

use common::WIKI;
use common::politics_markup::{
    all_of, completed, conditions, country_leaders, default_leader, halder, leader_row,
    leader_trait, named_staff, not_completed, one_of, other, portrait, staff_row, staff_section,
    staff_table, staff_trait, via_focus,
};
use common::wiki_markup::wiki_link;
use hoi4_planner::wiki::{PoliticsEntry, parse_leaders, parse_staff};

type Groups = Vec<Vec<String>>;

fn staff(rows: &[String]) -> Vec<PoliticsEntry> {
    parse_staff(&staff_section(&[staff_table(
        "Political Advisors",
        "Portrait",
        rows,
    )]))
}

fn requirements_of(items: &[String], visible: &[String]) -> (Groups, Vec<String>) {
    let cell = conditions(items, visible);
    let entries = staff(&[staff_row("Franz Halder", &cell, &halder(), 1)]);
    (entries[0].requires.clone(), entries[0].excludes.clone())
}

fn groups(groups: &[&[&str]]) -> Groups {
    groups
        .iter()
        .map(|group| group.iter().map(ToString::to_string).collect())
        .collect()
}

fn leader_with(possible: &str) -> PoliticsEntry {
    let markup = country_leaders(&[leader_row("Adolf Hitler", "Nazism", possible, 1)]);
    parse_leaders(&markup).remove(0)
}

#[test]
fn parses_a_leader_into_the_shape_of_a_focus() {
    let leaders = parse_leaders(&country_leaders(&[default_leader()]));

    assert_eq!(
        leaders,
        [PoliticsEntry {
            id: "Adolf_Hitler".to_string(),
            name: "Adolf Hitler".to_string(),
            // The full-size image, not the resized copy the table shows: a link only.
            icon: Some(format!("{WIKI}/images/a/ab/Portrait_GER_adolf_hitler.png")),
            icon_page: Some(format!("{WIKI}/File:Portrait_GER_adolf_hitler.png")),
            section: "Nazism".to_string(),
            position: 0,
            requires: Vec::new(),
            excludes: Vec::new(),
        }]
    );
}

#[test]
fn every_route_to_a_leader_is_one_group() {
    let possible = format!("<ul>{}{}</ul>", via_focus(&["A"]), via_focus(&["B", "C"]));
    assert_eq!(leader_with(&possible).requires, [["A", "B", "C"]]);
}

#[test]
fn a_route_nested_under_a_condition_still_counts_but_a_note_does_not() {
    let possible = format!(
        "<ul><li>If Thälmann is <i>not</i> freed<ul>{}<li>Event <b>Something</b></li></ul></li></ul>\
         <p><small>Trait is replaced after completing focus {}</small></p>",
        via_focus(&["The_People's_Victory"]),
        wiki_link("Later")
    );
    assert_eq!(leader_with(&possible).requires, [["The_People's_Victory"]]);
}

#[test]
fn a_route_is_only_the_focuses_of_its_own_line() {
    // What the route leads on to is listed under it, and is not another way in.
    let possible = format!(
        "<ul><li>Via focus {}<ul><li>Then {}</li></ul></li></ul>",
        wiki_link("A"),
        wiki_link("B")
    );
    assert_eq!(leader_with(&possible).requires, [["A"]]);
}

#[test]
fn a_leader_spanning_two_rows_is_one_leader_with_both_routes() {
    let route = |ids: &[&[&str]]| {
        let routes: String = ids.iter().map(|ids| via_focus(ids)).collect();
        format!("<ul>{routes}</ul>")
    };
    let markup = country_leaders(&[
        leader_row("Walther Ulbricht", "Stalinism", &route(&[&["A"]]), 2),
        leader_trait("Leninism", &route(&[&["B"], &["A"]])),
        leader_row("Ernst Thälmann", "Leninism", "Starting leader", 2),
        leader_trait("Leninism", &route(&[&["C"]])),
    ]);
    let leaders = parse_leaders(&markup);

    assert_eq!(leaders.len(), 2);
    assert_eq!(leaders[0].section, "Stalinism");
    assert_eq!(leaders[0].requires, [["A", "B"]]);
    assert_eq!(leaders[1].requires, [["C"]]);
    assert_eq!((leaders[0].position, leaders[1].position), (0, 1));
}

#[test]
fn a_page_without_the_table_has_no_leader() {
    assert!(parse_leaders("<p>Nothing here</p>").is_empty());
}

#[test]
fn each_table_is_a_section_and_the_organizations_are_left_out() {
    let markup = staff_section(&[
        staff_table(
            "Political Advisors",
            "Portrait",
            &[staff_row("Otto Strasser", "", "", 1)],
        ),
        staff_table(
            "Tank designers",
            "Logo",
            &[staff_row("Henschel", "", &portrait("Henschel.png"), 1)],
        ),
    ]);
    let entries = parse_staff(&markup);

    assert_eq!(entries.len(), 2);
    assert_eq!(
        (entries[0].section.as_str(), entries[0].position),
        ("Political Advisors", 0)
    );
    // The wiki has no portrait for everyone; nothing is made up.
    assert_eq!((&entries[0].icon, &entries[0].icon_page), (&None, &None));
    assert_eq!(
        (entries[1].section.as_str(), entries[1].position),
        ("Tank designers", 1)
    );
    assert_eq!(
        entries[1].icon.as_deref(),
        Some(format!("{WIKI}/images/a/ab/Henschel.png").as_str())
    );
}

#[test]
fn a_row_with_no_name_is_skipped() {
    assert!(staff(&[named_staff(" ")]).is_empty());
}

#[test]
fn a_focus_to_take_is_a_group_and_one_to_avoid_an_exclusion() {
    let (requires, excludes) = requirements_of(&[completed("A"), not_completed("B")], &[]);
    assert_eq!(requires, [["A"]]);
    assert_eq!(excludes, ["B"]);
}

#[test]
fn one_of_is_a_single_group() {
    let (requires, excludes) = requirements_of(&[one_of(&[completed("B"), completed("A")])], &[]);
    assert_eq!(requires, [["A", "B"]]);
    assert!(excludes.is_empty());
}

#[test]
fn one_of_several_conjunctions_picks_one_focus_from_each() {
    let both = all_of(&[completed("A"), completed("B")]);
    let (requires, _) = requirements_of(&[one_of(&[both, completed("C")])], &[]);
    assert_eq!(requires, groups(&[&["A", "C"], &["B", "C"]]));
}

#[test]
fn an_alternative_with_no_focus_lifts_the_whole_choice() {
    // "Reorganize the Wehrmacht and fascist, or Reinstated Nazi Leadership": the second needs
    // no focus at all, so no focus is required.
    let fascist = all_of(&[completed("A"), other("Current ruling party is Fascist")]);
    let (requires, _) = requirements_of(&[one_of(&[fascist, other("Reinstated")])], &[]);
    assert!(requires.is_empty());
}

#[test]
fn what_is_not_a_focus_is_dropped() {
    let party = || other("Current ruling party is Fascist");
    let (requires, excludes) = requirements_of(&[party(), all_of(&[party(), completed("A")])], &[]);
    assert_eq!(requires, [["A"]]);
    assert!(excludes.is_empty());
}

#[test]
fn an_exclusion_under_a_choice_is_not_an_exclusion() {
    let (requires, excludes) =
        requirements_of(&[one_of(&[not_completed("A"), completed("B")])], &[]);
    assert!(requires.is_empty());
    assert!(excludes.is_empty());
}

#[test]
fn visible_when_is_part_of_the_conditions() {
    let (requires, excludes) =
        requirements_of(&[completed("A")], &[not_completed("B"), completed("A")]);
    assert_eq!(requires, [["A"]]);
    assert_eq!(excludes, ["B"]);
}

#[test]
fn a_focus_linked_to_its_branch_is_named_by_the_link() {
    let href = "/German_national_focus_tree:_industrial#Remilitarize_the_Rhineland_Branch";
    let link = format!(
        "<a href=\"{href}\" title=\"German national focus tree\">Integrate Czech Manufacturers</a>"
    );
    let icon =
        format!("<a href=\"{href}\"><img alt=\"Integrate Czech Manufacturers\" src=\"x.png\"></a>");
    let item = format!("<li>Has completed focus {icon}{link}</li>");
    let (requires, _) = requirements_of(&[item], &[]);
    assert_eq!(requires, [["Integrate_Czech_Manufacturers"]]);
}

#[test]
fn a_person_spanning_two_rows_is_one_entry() {
    let first = conditions(&[completed("A")], &[]);
    let second = conditions(&[completed("B")], &[]);
    let entries = staff(&[
        staff_row("Franz Halder", &first, &halder(), 2),
        staff_trait(&second),
    ]);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].requires, [["A"], ["B"]]);
}

#[test]
fn ids_tell_apart_one_person_in_two_tables_and_twice_in_one() {
    let markup = staff_section(&[
        staff_table(
            "Political Advisors",
            "Portrait",
            &[
                named_staff("Hermann Göring"),
                named_staff("Reinhard Heydrich"),
                named_staff("Reinhard Heydrich"),
            ],
        ),
        staff_table(
            "Chief of Airforce",
            "Portrait",
            &[named_staff("Hermann Göring")],
        ),
    ]);
    let ids: Vec<String> = parse_staff(&markup).into_iter().map(|e| e.id).collect();
    assert_eq!(
        ids,
        [
            "Hermann_Göring__Political_Advisors",
            "Reinhard_Heydrich",
            "Reinhard_Heydrich__2",
            "Hermann_Göring__Chief_of_Airforce",
        ]
    );
}
