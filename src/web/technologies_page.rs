use askama::Template;
use chrono::{Datelike, NaiveDate};

use crate::domain::{Catalog, Focus, Plan, Technology, parse_start, schedule};
use crate::error::AppError;
use crate::web::app_state::AppState;

const MONTHS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// One row of the research catalog, as the template draws it.
#[derive(Debug, Clone)]
pub struct TechnologyRow {
    /// The technology.
    pub technology: Technology,
    /// Whether a research-area heading goes above this row.
    pub opens_domain: bool,
    /// Whether a section heading goes above this row.
    pub opens_section: bool,
    /// The name, lowercased, which is what the search box filters on.
    pub search_name: String,
    /// `requires` as JSON, read once by `technologies.js`.
    pub requires_json: String,
    /// `excludes` as JSON, read once by `technologies.js`.
    pub excludes_json: String,
    /// The year, empty for a secret tech.
    pub year_text: String,
}

/// One focus of the plan, as a line of the two aligned columns.
#[derive(Debug, Clone)]
pub struct PlanLine {
    /// The focus id, which the technologies are attached to.
    pub focus_id: String,
    /// Its place in the plan.
    pub position: i64,
    /// Its name.
    pub name: String,
    /// The wiki's copy of its icon.
    pub icon: String,
    /// When it completes, and so when its technologies do, as the focus page writes a date.
    pub completes_on: String,
}

/// The technology page: the catalog on the left, the plan's focuses and technologies on the right.
///
/// The focus lines are dated here rather than in the browser: nothing on this page can change
/// them, so `app.js`'s date arithmetic needs no third copy.
#[derive(Template)]
#[template(path = "technologies.html")]
pub struct TechnologiesPage {
    /// The catalog.
    pub rows: Vec<TechnologyRow>,
    /// The research areas, in page order.
    pub domains: Vec<String>,
    /// The plan's focuses, dated.
    pub lines: Vec<PlanLine>,
    /// The plan being built as JSON, `{}` when there is none.
    pub draft_json: String,
    /// The name it goes by so far.
    pub plan_name: String,
    /// The year it starts: a technology older than this counts as already researched.
    pub start_year: i32,
    /// What went wrong, if anything.
    pub error: Option<String>,
    /// What every link is built under.
    pub prefix: String,
    /// Which page the navigation marks as current.
    pub active: &'static str,
    /// What the database is called on screen.
    pub database_name: String,
}

impl TechnologiesPage {
    /// Prepares the page from what was read.
    ///
    /// # Errors
    /// Fails if a value cannot be written as JSON.
    pub fn new(
        state: &AppState,
        technologies: Vec<Technology>,
        focuses: Vec<Focus>,
        draft: Option<Plan>,
        error: Option<String>,
    ) -> Result<TechnologiesPage, AppError> {
        let draft_json = match &draft {
            Some(plan) => serde_json::to_string(plan)?,
            None => "{}".to_string(),
        };
        let start = parse_start(draft.as_ref().map(|plan| plan.start_date.as_str()));
        let lines = draft.as_ref().map_or_else(Vec::new, |plan| {
            plan_lines(plan, Catalog::new(focuses), start)
        });
        Ok(TechnologiesPage {
            domains: unique_domains(&technologies),
            rows: technology_rows(technologies)?,
            lines,
            draft_json,
            plan_name: draft.map(|plan| plan.name).unwrap_or_default(),
            start_year: start.year(),
            error,
            prefix: state.url_prefix.clone(),
            active: "technologies",
            database_name: state.database_name.clone(),
        })
    }
}

fn plan_lines(plan: &Plan, catalog: Catalog, start: NaiveDate) -> Vec<PlanLine> {
    schedule(
        &plan.focus_ids,
        &catalog,
        start,
        plan.days_per_focus,
        &plan.focus_days,
    )
    .into_iter()
    .map(|row| PlanLine {
        completes_on: french_date(&row.completes_on),
        position: row.plan_position,
        focus_id: row.focus.id,
        name: row.focus.name,
        icon: row.focus.icon,
    })
    .collect()
}

fn unique_domains(technologies: &[Technology]) -> Vec<String> {
    let mut domains: Vec<String> = Vec::new();
    for technology in technologies {
        if !domains.contains(&technology.domain) {
            domains.push(technology.domain.clone());
        }
    }
    domains
}

fn technology_rows(technologies: Vec<Technology>) -> Result<Vec<TechnologyRow>, AppError> {
    let mut rows: Vec<TechnologyRow> = Vec::with_capacity(technologies.len());
    for technology in technologies {
        let previous = rows.last().map(|row| &row.technology);
        let opens_domain = previous.is_none_or(|other| other.domain != technology.domain);
        let opens_section =
            opens_domain || previous.is_none_or(|other| other.section != technology.section);
        rows.push(TechnologyRow {
            opens_domain,
            opens_section,
            search_name: technology.name.to_lowercase(),
            requires_json: serde_json::to_string(&technology.requires)?,
            excludes_json: serde_json::to_string(&technology.excludes)?,
            year_text: technology
                .year
                .map(|year| year.to_string())
                .unwrap_or_default(),
            technology,
        });
    }
    Ok(rows)
}

/// A `YYYY-MM-DD` as `fmt()` writes it in `app.js`: `5 févr. 1936`.
fn french_date(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return date.to_string();
    };
    let month = month
        .parse::<usize>()
        .ok()
        .and_then(|month| MONTHS.get(month.checked_sub(1)?));
    match month {
        Some(month) => format!("{} {month} {year}", day.trim_start_matches('0')),
        None => date.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_reads_as_it_does_on_the_focus_page() {
        assert_eq!(french_date("1936-02-05"), "5 févr. 1936");
        assert_eq!(french_date("1941-12-31"), "31 déc. 1941");
        assert_eq!(french_date("1936-08-10"), "10 août 1936");
    }

    #[test]
    fn what_is_not_a_date_is_left_as_written() {
        assert_eq!(french_date("soon"), "soon");
        assert_eq!(french_date("1936-13-01"), "1936-13-01");
        assert_eq!(french_date("1936-00-01"), "1936-00-01");
    }
}
