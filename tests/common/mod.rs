//! What the integration tests share. Each test crate only uses part of it.
#![allow(dead_code)]

pub mod app;
pub mod politics_markup;
pub mod technology_markup;
pub mod wiki_markup;

use hoi4_planner::domain::{Focus, StaffMember, Technology};

pub const FOCUS_ID: &str = "Fortify_the_Vaterland";
pub const WIKI: &str = "https://hoi4.paradoxwikis.com";

/// One catalog row, shaped as the parser hands it over.
pub fn focus(id: &str, name: &str, section: &str, position: i64) -> Focus {
    Focus {
        id: id.to_string(),
        name: name.to_string(),
        icon: format!("{WIKI}/images/f/f5/Goal_generic_construct_military.png"),
        icon_page: format!("{WIKI}/File:Goal_generic_construct_military.png"),
        section: section.to_string(),
        position,
        requires: Vec::new(),
        excludes: Vec::new(),
    }
}

/// Three focuses across two branches: enough to test grouping and filtering.
pub fn catalog() -> Vec<Focus> {
    vec![
        focus(FOCUS_ID, "Fortify the Vaterland", "Rhineland Branch", 0),
        focus(
            "Rhineland",
            "Remilitarize the Rhineland",
            "Rhineland Branch",
            1,
        ),
        focus("Anschluss", "Anschluss", "Anschluss Sub-branch", 2),
    ]
}

/// One research catalog row, shaped as the parser hands it over.
pub fn technology(id: &str, name: &str, year: Option<i64>, position: i64) -> Technology {
    Technology {
        id: id.to_string(),
        name: name.to_string(),
        icon: format!("{WIKI}/images/9/94/{id}.png"),
        icon_page: format!("{WIKI}/File:{id}.png"),
        domain: "Industrie".to_string(),
        section: "Construction technologies".to_string(),
        position,
        year,
        base_days: year.map(|_| 220),
        requires: Vec::new(),
        excludes: Vec::new(),
    }
}

/// Two technologies a plan can attach, and a secret one.
pub fn technologies() -> Vec<Technology> {
    vec![
        technology("Construction_I", "Construction I", Some(1936), 0),
        technology("Construction_II", "Construction II", Some(1937), 1),
        technology("Jungle_Training", "Jungle Training", None, 2),
    ]
}

/// One staff catalog row, shaped as the import hands it over.
pub fn staff_member(id: &str, name: &str, section: &str, position: i64) -> StaffMember {
    StaffMember {
        id: id.to_string(),
        name: name.to_string(),
        icon: format!("{WIKI}/images/d/dd/{id}.png"),
        icon_page: format!("{WIKI}/File:{id}.png"),
        section: section.to_string(),
        position,
        requires: Vec::new(),
        excludes: Vec::new(),
    }
}

/// A theorist who needs a focus, a concern who needs nothing, and a general with no portrait.
pub fn staff() -> Vec<StaffMember> {
    let mut guderian = staff_member(
        "Heinz_Guderian__Theorists",
        "Heinz Guderian",
        "Theorists",
        0,
    );
    guderian.requires = vec![vec!["Rhineland".to_string()]];
    let mut halder = staff_member("Franz_Halder", "Franz Halder", "Generals", 2);
    halder.icon = String::new();
    halder.icon_page = String::new();
    vec![
        guderian,
        staff_member("Krupp", "Krupp", "Industrial concerns", 1),
        halder,
    ]
}
