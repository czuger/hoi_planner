//! The research pages to a catalog. No real page is read: `common::technology_markup` builds the
//! few rows each test needs.

mod common;

use common::technology_markup::{
    community_notice, dlc_off, dlc_on, excludes, heading, link_on_page, link_to, one_of, row,
    row_with_anchors, secret_row, secret_table, table, wikitable,
};
use hoi4_planner::domain::Technology;
use hoi4_planner::wiki::{TechnologyPageSource, parse_technology_pages};

fn parse(markup: &str) -> Vec<Technology> {
    parse_technology_pages(&[TechnologyPageSource {
        domain: "Industrie",
        markup,
    }])
}

fn find<'a>(technologies: &'a [Technology], id: &str) -> &'a Technology {
    technologies
        .iter()
        .find(|technology| technology.id == id)
        .unwrap_or_else(|| panic!("{id} is not in the catalog"))
}

fn ids(technologies: &[Technology]) -> Vec<&str> {
    technologies
        .iter()
        .map(|technology| technology.id.as_str())
        .collect()
}

#[test]
fn a_row_gives_the_name_the_links_the_year_and_the_research_time() {
    let page = heading(2, "Construction") + &wikitable(&[row("Construction I", 1936, "–")]);
    let technologies = parse(&page);

    let technology = &technologies[0];
    assert_eq!(technology.id, "Construction_I");
    assert_eq!(technology.name, "Construction I");
    assert_eq!(technology.domain, "Industrie");
    assert_eq!(technology.section, "Construction");
    assert_eq!(technology.year, Some(1936));
    assert_eq!(technology.base_days, Some(220));
    assert!(technology.requires.is_empty());
    assert_eq!(
        technology.icon,
        "https://hoi4.paradoxwikis.com/images/9/94/Construction_I.png"
    );
    assert_eq!(
        technology.icon_page,
        "https://hoi4.paradoxwikis.com/File:Construction_I.png"
    );
}

#[test]
fn a_table_is_recognized_by_its_columns_whatever_its_class() {
    let page = table("mildtable", &[row("Light Chassis", 1936, "–")])
        + "<table class=\"wikitable\"><tbody><tr><th>Modifier</th></tr>\
           <tr><td>Not a technology</td></tr></tbody></table>";
    assert_eq!(ids(&parse(&page)), ["Light_Chassis"]);
}

#[test]
fn a_table_nested_in_a_row_is_not_read() {
    let nested = table("mildtable", &[row("Stat block", 1936, "–")]);
    let page = wikitable(&[row("Hull", 1936, &nested)]);
    assert_eq!(ids(&parse(&page)), ["Hull"]);
}

#[test]
fn the_nearest_heading_names_the_section() {
    let page = heading(2, "Chassis")
        + &wikitable(&[row("A", 1936, "–")])
        + &heading(3, "Light Chassis")
        + &wikitable(&[row("B", 1936, "–")])
        + &heading(2, "Upgrades")
        + &wikitable(&[row("C", 1936, "–")]);
    let technologies = parse(&page);
    assert_eq!(find(&technologies, "A").section, "Chassis");
    assert_eq!(find(&technologies, "B").section, "Light Chassis");
    assert_eq!(find(&technologies, "C").section, "Upgrades");
}

#[test]
fn a_section_for_when_a_dlc_is_off_is_skipped() {
    let page = heading(3, "Old way")
        + &community_notice()
        + &dlc_off("La Résistance")
        + &community_notice()
        + &wikitable(&[row("Skipped", 1936, "–")])
        + &heading(3, "Next")
        + &wikitable(&[row("Kept", 1936, "–")]);
    assert_eq!(ids(&parse(&page)), ["Kept"]);
}

#[test]
fn a_section_that_needs_a_dlc_is_read() {
    let page = heading(3, "Armored Cars")
        + &dlc_on("La Résistance")
        + &wikitable(&[row("Armored Car", 1936, "–")]);
    assert_eq!(ids(&parse(&page)), ["Armored_Car"]);
}

#[test]
fn a_notice_only_covers_the_tables_after_it() {
    let page = heading(3, "Computing")
        + &wikitable(&[row("Before", 1936, "–")])
        + &dlc_off("La Résistance")
        + &wikitable(&[row("After", 1936, "–")]);
    assert_eq!(ids(&parse(&page)), ["Before"]);
}

#[test]
fn each_prerequisite_is_a_group_of_its_own() {
    let cell = format!("{}<br>{}", link_to("A"), link_to("B"));
    let page = wikitable(&[
        row("A", 1936, "–"),
        row("B", 1936, "–"),
        row("C", 1937, &cell),
    ]);
    assert_eq!(find(&parse(&page), "C").requires, [["A"], ["B"]]);
}

#[test]
fn one_of_is_a_single_group() {
    let page = wikitable(&[
        row("A", 1936, "–"),
        row("B", 1936, "–"),
        row("C", 1937, &one_of(&["A", "B"])),
    ]);
    assert_eq!(find(&parse(&page), "C").requires, [["A", "B"]]);
}

#[test]
fn a_required_link_beside_a_one_of_list_stays_its_own_group() {
    let cell = format!("{}<br>{}", link_to("A"), one_of(&["B", "D"]));
    let page = wikitable(&[
        row("A", 1936, "–"),
        row("B", 1936, "–"),
        row("D", 1936, "–"),
        row("C", 1937, &cell),
    ]);
    assert_eq!(
        find(&parse(&page), "C").requires,
        [vec!["A"], vec!["B", "D"]]
    );
}

#[test]
fn links_after_excludes_are_exclusions_and_both_sides_carry_them() {
    let flexible = format!("{}{}", link_to("Assembly"), excludes("Streamlined"));
    let page = wikitable(&[
        row("Assembly", 1936, "–"),
        row("Flexible", 1943, &flexible),
        row("Streamlined", 1943, &link_to("Assembly")),
    ]);
    let technologies = parse(&page);
    assert_eq!(find(&technologies, "Flexible").requires, [["Assembly"]]);
    assert_eq!(find(&technologies, "Flexible").excludes, ["Streamlined"]);
    assert_eq!(find(&technologies, "Streamlined").excludes, ["Flexible"]);
}

#[test]
fn who_can_unlock_a_secret_tech_is_not_a_prerequisite() {
    // The focus has the anchor of a technology, as `Jungle Training` really does.
    let unlocks = format!(
        "Can unlock this tech:<ul><li>{}</li></ul>",
        link_on_page("/British_Raj_national_focus_tree", "Jungle_Training")
    );
    let starts = "Starts with this tech: Belgium";
    let page = secret_table(&[
        secret_row("Jungle Training", &unlocks),
        secret_row("Chasseurs Ardennais", starts),
    ]);
    let technologies = parse(&page);
    assert!(find(&technologies, "Jungle_Training").requires.is_empty());
    assert!(
        find(&technologies, "Chasseurs_Ardennais")
            .requires
            .is_empty()
    );
}

#[test]
fn a_secret_tech_has_no_year_and_no_research_time() {
    let technologies = parse(&secret_table(&[secret_row("Hidden", "–")]));
    assert_eq!(technologies[0].year, None);
    assert_eq!(technologies[0].base_days, None);
}

#[test]
fn a_link_to_any_other_page_is_not_a_prerequisite() {
    let cell = link_on_page("/Special_project", "A");
    let page = wikitable(&[row("A", 1936, "–"), row("B", 1937, &cell)]);
    assert!(find(&parse(&page), "B").requires.is_empty());
}

#[test]
fn anchors_are_matched_whatever_their_case() {
    let page = wikitable(&[
        row("Computing Machine", 1938, "–"),
        row("Next", 1939, &link_to("Computing_machine")),
    ]);
    assert_eq!(
        find(&parse(&page), "Next").requires,
        [["Computing_Machine"]]
    );
}

#[test]
fn prerequisites_cross_pages() {
    let naval = wikitable(&[row(
        "Destroyer",
        1936,
        &link_on_page("/Naval_support_technology", "Torpedo"),
    )]);
    let support = wikitable(&[row("Torpedo", 1936, "–")]);
    let technologies = parse_technology_pages(&[
        TechnologyPageSource {
            domain: "Marine",
            markup: &naval,
        },
        TechnologyPageSource {
            domain: "Soutien naval",
            markup: &support,
        },
    ]);
    assert_eq!(find(&technologies, "Destroyer").requires, [["Torpedo"]]);
    assert_eq!(find(&technologies, "Destroyer").domain, "Marine");
    assert_eq!(find(&technologies, "Torpedo").position, 1);
}

#[test]
fn what_the_catalog_cannot_name_stops_being_a_requirement() {
    let page = wikitable(&[
        row("A", 1936, "–"),
        row("B", 1937, &link_to("Nowhere")),
        row("C", 1937, &one_of(&["A", "Nowhere"])),
    ]);
    let technologies = parse(&page);
    assert!(find(&technologies, "B").requires.is_empty());
    assert_eq!(find(&technologies, "C").requires, [["A"]]);
}

#[test]
fn the_anchor_beside_the_name_is_the_id_and_an_older_one_still_resolves() {
    let page = wikitable(&[
        row_with_anchors(
            "Rubber processing",
            &["Synthetic_rubber", "Rubber_processing"],
            1936,
            "–",
        ),
        row("Next", 1937, &link_to("Synthetic_rubber")),
    ]);
    let technologies = parse(&page);
    assert_eq!(technologies[0].id, "Rubber_processing");
    assert_eq!(
        find(&technologies, "Next").requires,
        [["Rubber_processing"]]
    );
}

#[test]
fn an_id_listed_twice_keeps_its_first_row() {
    let page = wikitable(&[row("Twice", 1936, "–"), row("Twice", 1940, "–")]);
    let technologies = parse(&page);
    assert_eq!(technologies.len(), 1);
    assert_eq!(technologies[0].year, Some(1936));
}

#[test]
fn a_sloppy_rowspan_does_not_shift_the_columns() {
    let spanning = "<tr>\n<td><div style=\"font-weight: bold;\">First\
                    <span id=\"First\"></span></div></td>\n<td>1936</td>\n<td>220 days</td>\n\
                    <td>–</td>\n<td rowspan=\"2;\">shared</td></tr>\n";
    let second = "<tr>\n<td><div style=\"font-weight: bold;\">Second\
                  <span id=\"Second\"></span></div></td>\n<td>1937</td>\n<td>110 days</td>\n\
                  <td>–</td></tr>\n";
    let technologies = parse(&table(
        "wikitable",
        &[spanning.to_string(), second.to_string()],
    ));
    assert_eq!(find(&technologies, "Second").year, Some(1937));
    assert_eq!(find(&technologies, "Second").base_days, Some(110));
}

#[test]
fn a_row_with_no_anchor_is_not_a_technology() {
    let page = wikitable(&[
        "<tr><td>Light SP Artillery I</td><td>1936</td><td>220 days</td>\
         <td>–</td><td>x</td></tr>"
            .to_string(),
    ]);
    assert!(parse(&page).is_empty());
}

#[test]
fn positions_follow_the_pages() {
    let page = wikitable(&[row("A", 1936, "–"), row("B", 1936, "–")]);
    let positions: Vec<i64> = parse(&page).iter().map(|t| t.position).collect();
    assert_eq!(positions, [0, 1]);
}
