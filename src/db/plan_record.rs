use std::collections::BTreeMap;

use chrono::Utc;
use serde::de::DeserializeOwned;
use sqlx::{FromRow, SqlitePool};

use crate::domain::rules::DEFAULT_START_TEXT;
use crate::domain::{DRAFT_PLAN_ID, Plan, PlanFields};
use crate::error::AppError;

#[derive(FromRow)]
struct PlanRow {
    id: String,
    name: String,
    focus_ids: String,
    start_date: String,
    days_per_focus: i64,
    focus_days: String,
    focus_technologies: String,
    focus_staff: String,
    focus_comments: String,
    created_at: String,
    updated_at: String,
}

/// A plan about to be stored, under an id chosen by the caller.
#[derive(Debug, Clone)]
pub struct NewPlan<'a> {
    /// A slug of the name, or the draft's fixed id.
    pub id: &'a str,
    /// What the person typed.
    pub name: &'a str,
    /// The plan itself.
    pub fields: &'a PlanFields,
}

/// The `plans` table: the saved plans, and the draft under its fixed id.
#[derive(Debug, Clone)]
pub struct PlanRecord;

impl PlanRecord {
    /// Every saved plan, most recently touched first; the draft is left out.
    ///
    /// # Errors
    /// Fails when the database does or a stored value cannot be read.
    pub async fn list_saved(pool: &SqlitePool) -> Result<Vec<Plan>, AppError> {
        let rows: Vec<PlanRow> = sqlx::query_as(
            "SELECT id, name, focus_ids, start_date, days_per_focus, focus_days, focus_technologies, focus_staff, focus_comments, created_at, updated_at
             FROM plans WHERE id <> ? ORDER BY updated_at DESC, id ASC",
        )
            .bind(DRAFT_PLAN_ID)
            .fetch_all(pool)
            .await?;
        rows.into_iter().map(into_plan).collect()
    }

    /// One plan, or `None` when there is no such id.
    ///
    /// # Errors
    /// Fails when the database does or a stored value cannot be read.
    pub async fn find(pool: &SqlitePool, plan_id: &str) -> Result<Option<Plan>, AppError> {
        let row: Option<PlanRow> = sqlx::query_as(
            "SELECT id, name, focus_ids, start_date, days_per_focus, focus_days, focus_technologies, focus_staff, focus_comments, created_at, updated_at
             FROM plans WHERE id = ?",
        )
            .bind(plan_id)
            .fetch_optional(pool)
            .await?;
        row.map(into_plan).transpose()
    }

    /// Writes one plan and hands back what was stored.
    ///
    /// `created_at` is kept from an existing row, as an overwrite is the same plan revised.
    /// `focus_days` is trimmed to the order being stored: a focus dragged out of the plan
    /// takes its term with it, rather than leaving a mark nothing on the page can see or clear.
    /// `focus_technologies`, `focus_staff` and `focus_comments` are trimmed the same way, as a
    /// technology or a member has no date without its focus, and a comment no line to sit on.
    ///
    /// # Errors
    /// Fails when the database does or a stored value cannot be read.
    pub async fn store(pool: &SqlitePool, plan: NewPlan<'_>) -> Result<Plan, AppError> {
        let fields = plan.fields;
        let marks: BTreeMap<&String, i64> = fields
            .focus_days
            .iter()
            .filter(|(focus_id, _)| fields.focus_ids.contains(focus_id))
            .map(|(focus_id, days)| (focus_id, *days))
            .collect();
        let technologies = attached_to_order(&fields.focus_technologies, &fields.focus_ids);
        let staff = attached_to_order(&fields.focus_staff, &fields.focus_ids);
        let comments = attached_to_order(&fields.focus_comments, &fields.focus_ids);
        let stamp = now();
        let row: PlanRow = sqlx::query_as(
            "INSERT INTO plans
                 (id, name, focus_ids, start_date, days_per_focus, focus_days, focus_technologies, focus_staff, focus_comments, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 focus_ids = excluded.focus_ids,
                 start_date = excluded.start_date,
                 days_per_focus = excluded.days_per_focus,
                 focus_days = excluded.focus_days,
                 focus_technologies = excluded.focus_technologies,
                 focus_staff = excluded.focus_staff,
                 focus_comments = excluded.focus_comments,
                 updated_at = excluded.updated_at
             RETURNING id, name, focus_ids, start_date, days_per_focus, focus_days, focus_technologies, focus_staff, focus_comments, created_at, updated_at",
        )
            .bind(plan.id)
            .bind(plan.name)
            .bind(serde_json::to_string(&fields.focus_ids)?)
            .bind(fields.start_date.as_deref().unwrap_or(DEFAULT_START_TEXT))
            .bind(fields.days_per_focus)
            .bind(serde_json::to_string(&marks)?)
            .bind(serde_json::to_string(&technologies)?)
            .bind(serde_json::to_string(&staff)?)
            .bind(serde_json::to_string(&comments)?)
            .bind(&stamp)
            .bind(&stamp)
            .fetch_one(pool)
            .await?;
        into_plan(row)
    }

    /// Throws one plan away, and says whether there was one to throw.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn delete(pool: &SqlitePool, plan_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM plans WHERE id = ?")
            .bind(plan_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// What the focuses of `focus_ids` carry (ids, or comments), leaving out the focuses no longer planned and the
/// empty lists.
fn attached_to_order<'a>(
    attached: &'a BTreeMap<String, Vec<String>>,
    focus_ids: &[String],
) -> BTreeMap<&'a String, &'a Vec<String>> {
    attached
        .iter()
        .filter(|(focus_id, ids)| !ids.is_empty() && focus_ids.contains(focus_id))
        .collect()
}

/// UTC to the microsecond, as text that sorts chronologically.
fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

fn into_plan(row: PlanRow) -> Result<Plan, AppError> {
    Ok(Plan {
        focus_ids: read_json("plans.focus_ids", &row.focus_ids)?,
        focus_days: read_json("plans.focus_days", &row.focus_days)?,
        focus_technologies: read_json("plans.focus_technologies", &row.focus_technologies)?,
        focus_staff: read_json("plans.focus_staff", &row.focus_staff)?,
        focus_comments: read_json("plans.focus_comments", &row.focus_comments)?,
        id: row.id,
        name: row.name,
        start_date: row.start_date,
        days_per_focus: row.days_per_focus,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// Reads a JSON column back.
fn read_json<T: DeserializeOwned>(column: &'static str, text: &str) -> Result<T, AppError> {
    serde_json::from_str(text).map_err(|source| AppError::CorruptData { column, source })
}
