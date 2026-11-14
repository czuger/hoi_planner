use std::collections::BTreeMap;

use serde::Serialize;

/// The id of the plan being built, saved on every edit.
///
/// A slug only ever holds `[a-z0-9-]`, so the tilde keeps a plan someone calls "Draft" from
/// landing on top of it.
pub const DRAFT_PLAN_ID: &str = "~draft";

/// A stored plan: an order of focuses, and what to date it from.
///
/// Everything drawn on screen is derived from these fields by `schedule`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    /// A slug of the name, or `DRAFT_PLAN_ID`.
    pub id: String,
    /// What the person typed.
    pub name: String,
    /// Focus ids, in the order they are to be taken.
    pub focus_ids: Vec<String>,
    /// `YYYY-MM-DD`, free text in the browser, read by `parse_start`.
    pub start_date: String,
    /// What an unmarked focus costs.
    pub days_per_focus: i64,
    /// `{focus id: days}`, only for the focuses that differ.
    pub focus_days: BTreeMap<String, i64>,
    /// `{focus id: [technology ids]}`: each technology takes the date of its focus.
    pub focus_technologies: BTreeMap<String, Vec<String>>,
    /// `{focus id: [staff ids]}`: each member takes the date of its focus and fills the post of
    /// its table.
    pub focus_staff: BTreeMap<String, Vec<String>>,
    /// `{focus id: [text]}`: free comments written on the focus's line, in their order.
    pub focus_comments: BTreeMap<String, Vec<String>>,
    /// `{technology id: staff id}`: the designer a planned technology is meant for, a note of
    /// intent that changes no date.
    pub technology_designers: BTreeMap<String, String>,
    /// When the plan was first stored.
    pub created_at: String,
    /// When it was last stored, which is what the plan list is ordered by.
    pub updated_at: String,
}
