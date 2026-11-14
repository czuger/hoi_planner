use crate::db::{NewPlan, PlanRecord, Store};
use crate::domain::{Plan, PlanFields, slugify};
use crate::error::AppError;

/// Creates or overwrites the plan of that name, and hands back what was stored.
///
/// Keyed by the slug of the name, so saving "Barbarossa rush" twice updates one plan.
///
/// # Errors
/// Refuses a name with nothing to slug, and fails when the database does.
pub async fn save_plan(store: &Store, name: &str, fields: &PlanFields) -> Result<Plan, AppError> {
    let name = name.trim();
    let id = slugify(name);
    if id.is_empty() {
        return Err(AppError::PlanNameUnusable);
    }
    PlanRecord::store(
        store,
        NewPlan {
            id: &id,
            name,
            fields,
        },
    )
    .await
}
