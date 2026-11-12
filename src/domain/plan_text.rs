use chrono::{Datelike, NaiveDate};

use crate::domain::{EchoRow, PlanFile, parse_start};

/// The months as the page writes them (`MONTHS` in `static/app.js`).
const MONTHS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// A plan as plain text, to be copied and pasted: one numbered line per focus, its date and
/// name, then its technologies, staff and comments on indented lines.
///
/// It is written from the file's readable echo, so the dates are the server's and the ids the
/// catalog has lost are left out. Nothing reads it back.
pub fn plan_text(file: &PlanFile) -> String {
    let name = file
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Plan");
    let start = french_date(parse_start(file.start_date.as_deref()));
    let mut text = format!("{name} — début {start}\n\n");
    for row in &file.focuses {
        text.push_str(&focus_block(row, file.days_per_focus));
    }
    text
}

/// One focus's line, followed by the lines of what is attached to it.
fn focus_block(row: &EchoRow, days_per_focus: i64) -> String {
    let number = format!("{}. ", row.position);
    let date = NaiveDate::parse_from_str(&row.completes_on, "%Y-%m-%d")
        .map(french_date)
        .unwrap_or_else(|_| row.completes_on.clone());
    let mut block = format!("{number}{date} — {}", row.name);
    if row.days != days_per_focus {
        block.push_str(&format!(" ({} j)", row.days));
    }
    block.push('\n');

    let technologies: Vec<&str> = row.technologies.iter().map(|t| t.name.as_str()).collect();
    let staff: Vec<String> = row
        .staff
        .iter()
        .map(|member| format!("{} ({})", member.name, member.post))
        .collect();
    let comments: Vec<String> = row.comments.iter().map(|c| one_line(c)).collect();
    let indent = " ".repeat(number.chars().count());
    for (label, items) in [
        ("Technologies", technologies.join(", ")),
        ("État-major", staff.join(", ")),
        ("Commentaires", comments.join(" ; ")),
    ] {
        if !items.is_empty() {
            block.push_str(&format!("{indent}{label} : {items}\n"));
        }
    }
    block
}

/// A comment's line breaks as ` / `, so that a focus stays a block of single lines.
fn one_line(comment: &str) -> String {
    comment
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

/// `5 févr. 1936`, as the page dates its lines.
fn french_date(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        MONTHS[date.month0() as usize],
        date.year()
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::{
        Catalog, Focus, PlanFields, StaffCatalog, StaffMember, Technology, TechnologyCatalog,
    };

    fn focus(id: &str, name: &str) -> Focus {
        Focus {
            id: id.to_string(),
            name: name.to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: "S".to_string(),
            position: 0,
            requires: Vec::new(),
            excludes: Vec::new(),
        }
    }

    fn catalog() -> Catalog {
        Catalog::new(vec![
            focus("Rhineland", "Remilitarize the Rhineland"),
            focus("Anschluss", "Anschluss"),
        ])
    }

    fn technologies() -> TechnologyCatalog {
        TechnologyCatalog::new(vec![Technology {
            id: "Construction_I".to_string(),
            name: "Construction I".to_string(),
            icon: String::new(),
            icon_page: String::new(),
            domain: "Industrie".to_string(),
            section: "Construction".to_string(),
            position: 0,
            year: Some(1936),
            base_days: Some(220),
            requires: Vec::new(),
            excludes: Vec::new(),
        }])
    }

    fn staff() -> StaffCatalog {
        StaffCatalog::new(vec![StaffMember {
            id: "Heinz_Guderian__Theorists".to_string(),
            name: "Heinz Guderian".to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: "Theorists".to_string(),
            position: 0,
            requires: Vec::new(),
            excludes: Vec::new(),
        }])
    }

    fn fields(focus_ids: &[&str]) -> PlanFields {
        PlanFields {
            focus_ids: focus_ids.iter().map(ToString::to_string).collect(),
            start_date: Some("1936-01-01".to_string()),
            days_per_focus: 35,
            focus_days: BTreeMap::from([("Anschluss".to_string(), 70)]),
            focus_technologies: BTreeMap::new(),
            focus_staff: BTreeMap::new(),
            focus_comments: BTreeMap::new(),
        }
    }

    fn text_of(name: Option<&str>, plan: PlanFields, catalog: &Catalog) -> String {
        let file = PlanFile::new(
            name.map(ToString::to_string),
            plan,
            catalog,
            &technologies(),
            &staff(),
        );
        plan_text(&file)
    }

    fn attach(plan: &mut PlanFields, focus_id: &str, comments: &[&str]) {
        let focus_id = focus_id.to_string();
        plan.focus_technologies
            .insert(focus_id.clone(), vec!["Construction_I".to_string()]);
        plan.focus_staff.insert(
            focus_id.clone(),
            vec!["Heinz_Guderian__Theorists".to_string()],
        );
        plan.focus_comments
            .insert(focus_id, comments.iter().map(ToString::to_string).collect());
    }

    #[test]
    fn a_plan_reads_as_a_numbered_list() {
        let mut plan = fields(&["Rhineland", "Anschluss"]);
        attach(&mut plan, "Rhineland", &["garder 2 PP"]);
        assert_eq!(
            text_of(Some("Barbarossa rush"), plan, &catalog()),
            "Barbarossa rush — début 1 janv. 1936\n\
             \n\
             1. 5 févr. 1936 — Remilitarize the Rhineland\n   \
             Technologies : Construction I\n   \
             État-major : Heinz Guderian (Theorists)\n   \
             Commentaires : garder 2 PP\n\
             2. 15 avr. 1936 — Anschluss (70 j)\n"
        );
    }

    #[test]
    fn a_plan_with_no_name_is_called_plan() {
        for name in [None, Some(""), Some("   ")] {
            let text = text_of(name, fields(&[]), &catalog());
            assert_eq!(text, "Plan — début 1 janv. 1936\n\n");
        }
    }

    #[test]
    fn the_term_is_only_written_when_it_differs_from_the_plans() {
        let mut plan = fields(&["Anschluss"]);
        plan.days_per_focus = 70;
        let text = text_of(None, plan, &catalog());
        assert!(text.contains("— Anschluss\n"));
    }

    #[test]
    fn comments_are_joined_and_kept_on_one_line() {
        let mut plan = fields(&["Rhineland"]);
        attach(&mut plan, "Rhineland", &["un\ndeux\n", "trois"]);
        let text = text_of(None, plan, &catalog());
        assert!(text.contains("   Commentaires : un / deux ; trois\n"));
    }

    #[test]
    fn attached_lines_align_past_the_ninth_focus() {
        let catalog = Catalog::new(
            (1..=10)
                .map(|n| focus(&format!("F{n}"), &format!("Focus {n}")))
                .collect(),
        );
        let ids: Vec<String> = (1..=10).map(|n| format!("F{n}")).collect();
        let mut plan = fields(&ids.iter().map(String::as_str).collect::<Vec<_>>());
        attach(&mut plan, "F10", &[]);
        let text = text_of(None, plan, &catalog);
        assert!(
            text.contains("\n10. 16 déc. 1936 — Focus 10\n    Technologies : Construction I\n")
        );
    }

    #[test]
    fn what_the_catalogs_lost_is_left_out() {
        let mut plan = fields(&["Gone", "Rhineland"]);
        plan.focus_technologies
            .insert("Rhineland".to_string(), vec!["Gone".to_string()]);
        let text = text_of(None, plan, &catalog());
        assert_eq!(
            text,
            "Plan — début 1 janv. 1936\n\n1. 5 févr. 1936 — Remilitarize the Rhineland\n"
        );
    }

    #[test]
    fn dates_are_written_in_french() {
        let date = NaiveDate::from_ymd_opt(1939, 8, 23).unwrap();
        assert_eq!(french_date(date), "23 août 1939");
    }
}
