use askama::Template;

use crate::domain::rules::{DEFAULT_DAYS_PER_FOCUS, DEFAULT_START_TEXT, LONG_DAYS, SHORT_DAYS};
use crate::domain::{Focus, Plan, StaffMember};
use crate::error::AppError;
use crate::web::app_state::AppState;
use crate::web::staff_row::{StaffRow, staff_rows, unique_posts};
use crate::web::technology_row::{TechnologyRow, technology_rows, unique_domains};

/// One row of the catalog, as the template draws it.
#[derive(Debug, Clone)]
pub struct FocusRow {
    /// The focus.
    pub focus: Focus,
    /// Whether a branch heading goes above this row.
    pub opens_section: bool,
    /// The name, lowercased, which is what the search box filters on.
    pub search_name: String,
    /// `requires` as JSON, read once by `app.js`.
    pub requires_json: String,
    /// `excludes` as JSON, read once by `app.js`.
    pub excludes_json: String,
}

/// One saved plan, as the dropdown offers it.
#[derive(Debug, Clone)]
pub struct SavedPlanOption {
    /// The plan's id.
    pub id: String,
    /// The plan's name.
    pub name: String,
    /// How many focuses it holds.
    pub focus_count: usize,
}

/// What the page is drawn from besides the catalogs: the saved plans and the draft.
///
/// Empty, it is the page drawn when the database could not be read.
#[derive(Debug, Clone, Default)]
pub struct PlannerData {
    /// The saved plans.
    pub plans: Vec<Plan>,
    /// The plan being built, if any.
    pub draft: Option<Plan>,
}

/// The one page: the three catalogs on the left, in tabs, and on the right the plan being
/// built, one line per focus with its technologies and its staff.
///
/// The catalogs are rendered here (rows that never change between requests, which the browser
/// filters); the plan is edited in the browser and posted as JSON.
#[derive(Template)]
#[template(path = "planner.html")]
pub struct PlannerPage {
    /// The focus catalog.
    pub rows: Vec<FocusRow>,
    /// The branches, in page order.
    pub sections: Vec<String>,
    /// The research catalog.
    pub tech_rows: Vec<TechnologyRow>,
    /// The research areas, in page order.
    pub domains: Vec<String>,
    /// The staff catalog, designers left out.
    pub staff_rows: Vec<StaffRow>,
    /// The staff's posts, in page order.
    pub posts: Vec<String>,
    /// The designers, which a technology is tagged with rather than a line hiring them.
    pub designer_rows: Vec<StaffRow>,
    /// The saved plans.
    pub plans: Vec<SavedPlanOption>,
    /// The plan being built as JSON, `{}` when there is none.
    pub draft_json: String,
    /// What went wrong, if anything: one sentence each.
    pub errors: Vec<String>,
    /// What the database is called on screen.
    pub database_name: String,
    /// The date the start field shows at first.
    pub default_start: &'static str,
    /// What an unmarked focus costs.
    pub default_days: i64,
    /// The short term.
    pub short_days: i64,
    /// The long term.
    pub long_days: i64,
}

impl PlannerPage {
    /// Prepares the page from the state's catalogs and what was read from the database.
    ///
    /// # Errors
    /// Fails if a value cannot be written as JSON.
    pub fn new(
        state: &AppState,
        data: PlannerData,
        errors: Vec<String>,
    ) -> Result<PlannerPage, AppError> {
        let draft_json = match &data.draft {
            Some(plan) => serde_json::to_string(plan)?,
            None => "{}".to_string(),
        };
        let catalogs = &state.catalogs;
        let focuses = catalogs.focuses.entries();
        let technologies = catalogs.technologies.entries();
        let (designers, staff): (Vec<StaffMember>, Vec<StaffMember>) = catalogs
            .staff
            .entries()
            .iter()
            .cloned()
            .partition(StaffMember::is_designer);
        Ok(PlannerPage {
            sections: unique_sections(focuses),
            rows: focus_rows(focuses.to_vec())?,
            domains: unique_domains(technologies),
            tech_rows: technology_rows(technologies.to_vec())?,
            posts: unique_posts(&staff),
            staff_rows: staff_rows(staff)?,
            designer_rows: staff_rows(designers)?,
            plans: data.plans.into_iter().map(saved_option).collect(),
            draft_json,
            errors,
            database_name: state.database_name.clone(),
            default_start: DEFAULT_START_TEXT,
            default_days: DEFAULT_DAYS_PER_FOCUS,
            short_days: SHORT_DAYS,
            long_days: LONG_DAYS,
        })
    }
}

fn unique_sections(focuses: &[Focus]) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    for focus in focuses {
        if !sections.contains(&focus.section) {
            sections.push(focus.section.clone());
        }
    }
    sections
}

fn focus_rows(focuses: Vec<Focus>) -> Result<Vec<FocusRow>, AppError> {
    let mut rows: Vec<FocusRow> = Vec::with_capacity(focuses.len());
    for focus in focuses {
        let opens_section = rows
            .last()
            .is_none_or(|previous| previous.focus.section != focus.section);
        rows.push(FocusRow {
            opens_section,
            search_name: focus.name.to_lowercase(),
            requires_json: serde_json::to_string(&focus.requires)?,
            excludes_json: serde_json::to_string(&focus.excludes)?,
            focus,
        });
    }
    Ok(rows)
}

fn saved_option(plan: Plan) -> SavedPlanOption {
    SavedPlanOption {
        focus_count: plan.focus_ids.len(),
        id: plan.id,
        name: plan.name,
    }
}
