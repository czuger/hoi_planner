use std::collections::BTreeMap;

use chrono::Utc;
use polodb_core::bson::doc;
use polodb_core::{CollectionT, Database};
use serde::{Deserialize, Serialize};

use crate::db::store::{PLANS, Store};
use crate::domain::rules::DEFAULT_START_TEXT;
use crate::domain::{DRAFT_PLAN_ID, Plan, PlanFields};
use crate::error::AppError;

/// A plan as stored.
///
/// The maps are kept as lists of pairs: their keys are ids, which may hold the dots and dollars
/// a document's field names cannot.
#[derive(Serialize, Deserialize)]
struct PlanDocument {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    focus_ids: Vec<String>,
    start_date: String,
    days_per_focus: i64,
    focus_days: Vec<(String, i64)>,
    focus_technologies: Vec<(String, Vec<String>)>,
    focus_staff: Vec<(String, Vec<String>)>,
    focus_comments: Vec<(String, Vec<String>)>,
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

/// The `plans` collection: the saved plans, and the draft under its fixed id.
#[derive(Debug, Clone)]
pub struct PlanRecord;

impl PlanRecord {
    /// Every saved plan, most recently touched first; the draft is left out.
    ///
    /// # Errors
    /// Fails when the database does or a stored plan cannot be read.
    pub async fn list_saved(store: &Store) -> Result<Vec<Plan>, AppError> {
        // Filtered here: PoloDB 5.3 answers `$ne` on `_id` with the very documents it excludes.
        let mut plans: Vec<Plan> = store
            .run(|database| {
                database
                    .collection::<PlanDocument>(PLANS)
                    .find(doc! {})
                    .run()?
                    .map(|document| Ok(into_plan(document?)))
                    .filter(|plan| !matches!(plan, Ok(plan) if plan.id == DRAFT_PLAN_ID))
                    .collect()
            })
            .await?;
        plans.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(plans)
    }

    /// One plan, or `None` when there is no such id.
    ///
    /// # Errors
    /// Fails when the database does or the stored plan cannot be read.
    pub async fn find(store: &Store, plan_id: &str) -> Result<Option<Plan>, AppError> {
        let plan_id = plan_id.to_string();
        let document = store
            .run(move |database| {
                Ok(database
                    .collection::<PlanDocument>(PLANS)
                    .find_one(doc! { "_id": plan_id })?)
            })
            .await?;
        Ok(document.map(into_plan))
    }

    /// Writes one plan and hands back what was stored.
    ///
    /// `created_at` is kept from an existing plan, as an overwrite is the same plan revised.
    /// `focus_days` is trimmed to the order being stored: a focus dragged out of the plan
    /// takes its term with it, rather than leaving a mark nothing on the page can see or clear.
    /// `focus_technologies`, `focus_staff` and `focus_comments` are trimmed the same way, as a
    /// technology or a member has no date without its focus, and a comment no line to sit on.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn store(store: &Store, plan: NewPlan<'_>) -> Result<Plan, AppError> {
        let fields = plan.fields;
        let stamp = now();
        let document = PlanDocument {
            id: plan.id.to_string(),
            name: plan.name.to_string(),
            focus_ids: fields.focus_ids.clone(),
            start_date: fields
                .start_date
                .clone()
                .unwrap_or_else(|| DEFAULT_START_TEXT.to_string()),
            days_per_focus: fields.days_per_focus,
            focus_days: fields
                .focus_days
                .iter()
                .filter(|(focus_id, _)| fields.focus_ids.contains(focus_id))
                .map(|(focus_id, days)| (focus_id.clone(), *days))
                .collect(),
            focus_technologies: attached_to_order(&fields.focus_technologies, &fields.focus_ids),
            focus_staff: attached_to_order(&fields.focus_staff, &fields.focus_ids),
            focus_comments: attached_to_order(&fields.focus_comments, &fields.focus_ids),
            created_at: stamp.clone(),
            updated_at: stamp,
        };
        let stored = store
            .run(move |database| replace(database, document))
            .await?;
        Ok(into_plan(stored))
    }

    /// Throws one plan away, and says whether there was one to throw.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn delete(store: &Store, plan_id: &str) -> Result<bool, AppError> {
        let plan_id = plan_id.to_string();
        store
            .run(move |database| {
                let result = database
                    .collection::<PlanDocument>(PLANS)
                    .delete_one(doc! { "_id": plan_id })?;
                Ok(result.deleted_count > 0)
            })
            .await
    }
}

/// Puts `document` in place of the plan with its id, keeping that plan's `created_at`.
fn replace(database: &Database, mut document: PlanDocument) -> Result<PlanDocument, AppError> {
    let transaction = database.start_transaction()?;
    let plans = transaction.collection::<PlanDocument>(PLANS);
    if let Some(existing) = plans.find_one(doc! { "_id": &document.id })? {
        document.created_at = existing.created_at;
        plans.delete_one(doc! { "_id": &document.id })?;
    }
    plans.insert_one(&document)?;
    transaction.commit()?;
    Ok(document)
}

/// What the focuses of `focus_ids` carry (ids, or comments), leaving out the focuses no longer
/// planned and the empty lists.
fn attached_to_order(
    attached: &BTreeMap<String, Vec<String>>,
    focus_ids: &[String],
) -> Vec<(String, Vec<String>)> {
    attached
        .iter()
        .filter(|(focus_id, ids)| !ids.is_empty() && focus_ids.contains(focus_id))
        .map(|(focus_id, ids)| (focus_id.clone(), ids.clone()))
        .collect()
}

/// UTC to the microsecond, as text that sorts chronologically.
fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

fn into_plan(document: PlanDocument) -> Plan {
    Plan {
        id: document.id,
        name: document.name,
        focus_ids: document.focus_ids,
        start_date: document.start_date,
        days_per_focus: document.days_per_focus,
        focus_days: document.focus_days.into_iter().collect(),
        focus_technologies: document.focus_technologies.into_iter().collect(),
        focus_staff: document.focus_staff.into_iter().collect(),
        focus_comments: document.focus_comments.into_iter().collect(),
        created_at: document.created_at,
        updated_at: document.updated_at,
    }
}
