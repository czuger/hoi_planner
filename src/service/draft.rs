use crate::db::{NewPlan, PlanRecord, Store};
use crate::domain::{DRAFT_PLAN_ID, Plan, PlanFields};
use crate::error::AppError;

/// Writes the plan on screen over the draft, and hands back what was stored.
///
/// The browser posts here on every edit, so the panel survives a reload without being named
/// first: the name is only what is typed in the box so far, and may well be empty.
///
/// # Errors
/// Fails when the database does.
pub async fn save_draft(store: &Store, name: &str, fields: &PlanFields) -> Result<Plan, AppError> {
    PlanRecord::store(
        store,
        NewPlan {
            id: DRAFT_PLAN_ID,
            name: name.trim(),
            fields,
        },
    )
    .await
}

/// The plan being built, or `None` when there is no draft to pick back up.
///
/// # Errors
/// Fails when the database does.
pub async fn load_draft(store: &Store) -> Result<Option<Plan>, AppError> {
    PlanRecord::find(store, DRAFT_PLAN_ID).await
}

/// Throws the draft away, and says whether there was one: the Clear button.
///
/// # Errors
/// Fails when the database does.
pub async fn clear_draft(store: &Store) -> Result<bool, AppError> {
    PlanRecord::delete(store, DRAFT_PLAN_ID).await
}
