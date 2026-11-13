//! The commands reading the saved pages into the JSON the binary compiles in.

mod common;

use std::fs;

use common::politics_markup::{
    country_leaders, default_leader, named_staff, staff_section, staff_table,
};
use common::technology_markup::{heading, link_to, row, wikitable};
use common::wiki_markup::{plain_row, wiki_heading};
use hoi4_planner::config::DataFiles;
use hoi4_planner::error::AppError;
use hoi4_planner::service::{parse_focuses, parse_politics, parse_technologies};
use serde_json::Value;
use tempfile::TempDir;

struct Workspace {
    dir: TempDir,
    files: DataFiles,
}

fn workspace() -> Workspace {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("data")).unwrap();
    let files = DataFiles::under(dir.path());
    Workspace { dir, files }
}

fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_focus_page_becomes_the_json() {
    let work = workspace();
    let page = wiki_heading("Rhineland Branch") + &plain_row("A", "Alpha");
    fs::write(&work.files.wiki_page, page).unwrap();

    assert_eq!(parse_focuses(&work.files).unwrap(), 1);
    assert_eq!(read_json(&work.files.focuses_json)[0]["id"], "A");
    assert_eq!(
        read_json(&work.files.focuses_json)[0]["section"],
        "Rhineland Branch"
    );
}

#[test]
fn a_missing_page_says_so() {
    let work = workspace();
    let error = parse_focuses(&work.files).unwrap_err();
    assert!(error.to_string().contains("focuses.html"));
}

#[test]
fn a_page_with_no_focus_is_an_error() {
    let work = workspace();
    fs::write(&work.files.wiki_page, "<p>nothing</p>").unwrap();
    let error = parse_focuses(&work.files).unwrap_err();
    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!work.files.focuses_json.exists());
}

fn staff_page() -> String {
    staff_section(&[staff_table(
        "Generals",
        "Portrait",
        &[named_staff("Franz Halder")],
    )])
}

#[test]
fn the_politics_command_writes_both_files() {
    let work = workspace();
    let page = country_leaders(&[default_leader()]) + &staff_page();
    fs::write(&work.files.politics_page, page).unwrap();

    let counts = parse_politics(&work.files).unwrap();

    assert_eq!((counts.leaders, counts.staff), (1, 1));
    assert_eq!(read_json(&work.files.leaders_json)[0]["id"], "Adolf_Hitler");
    assert_eq!(read_json(&work.files.staff_json)[0]["section"], "Generals");
}

#[test]
fn a_missing_country_page_is_an_error() {
    let work = workspace();
    let error = parse_politics(&work.files).unwrap_err();
    assert!(error.to_string().contains("politics.html"));
}

#[test]
fn a_page_with_nothing_to_read_writes_nothing() {
    let work = workspace();
    fs::write(
        &work.files.politics_page,
        country_leaders(&[default_leader()]),
    )
    .unwrap();

    let error = parse_politics(&work.files).unwrap_err();

    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!work.files.staff_json.exists());
    assert!(!work.files.leaders_json.exists());
}

/// Every research page saved, the first holding `first` and the rest nothing to read.
fn save_research_pages(workspace: &Workspace, first: &str) {
    fs::create_dir(workspace.dir.path().join("data/technologies")).unwrap();
    for (index, page) in workspace.files.technology_pages.iter().enumerate() {
        fs::write(&page.path, if index == 0 { first } else { "" }).unwrap();
    }
}

fn a_research_page() -> String {
    heading(2, "Construction")
        + &wikitable(&[
            row("Construction I", 1936, "–"),
            row("Construction II", 1937, &link_to("Construction_I")),
        ])
}

#[test]
fn the_research_pages_become_the_json() {
    let workspace = workspace();
    save_research_pages(&workspace, &a_research_page());

    assert_eq!(parse_technologies(&workspace.files).unwrap(), 2);
    let json = read_json(&workspace.files.technologies_json);
    assert_eq!(json[0]["id"], "Construction_I");
    assert_eq!(json[0]["domain"], "Infanterie");
    assert_eq!(json[1]["requires"][0][0], "Construction_I");
}

#[test]
fn pages_with_no_technology_change_nothing() {
    let workspace = workspace();
    save_research_pages(&workspace, "<p>no table here</p>");
    let error = parse_technologies(&workspace.files).unwrap_err();
    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!workspace.files.technologies_json.exists());
}

#[test]
fn a_missing_research_page_is_an_error() {
    let workspace = workspace();
    let error = parse_technologies(&workspace.files).unwrap_err();
    assert!(matches!(error, AppError::FileRead { .. }));
}
