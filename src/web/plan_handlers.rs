use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::db::PlanRecord;
use crate::domain::{Plan, PlanFields, posted_name};
use crate::error::AppError;
use crate::service::{clear_draft, save_draft, save_plan};
use crate::web::api_error::ApiError;
use crate::web::app_state::AppState;
use crate::web::json_body::posted_object;

/// Every saved plan, most recently touched first.
pub async fn list(State(state): State<AppState>) -> Result<Response, ApiError> {
    let plans = PlanRecord::list_saved(&state.pool)
        .await
        .map_err(|error| ApiError::new(&state, error))?;
    Ok(Json(json!({ "plans": plans })).into_response())
}

/// One plan, so that the page can load it into the panel.
pub async fn show(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Response, ApiError> {
    show_plan(&state, &plan_id).await
}

/// The plan whose name slugs to `export`.
pub async fn show_export(State(state): State<AppState>) -> Result<Response, ApiError> {
    show_plan(&state, "export").await
}

/// The plan whose name slugs to `import`.
pub async fn show_import(State(state): State<AppState>) -> Result<Response, ApiError> {
    show_plan(&state, "import").await
}

async fn show_plan(state: &AppState, plan_id: &str) -> Result<Response, ApiError> {
    let plan = found_plan(state, plan_id).await?;
    Ok(Json(json!({ "plan": plan })).into_response())
}

/// A stored plan, or the error saying there is none.
pub(crate) async fn found_plan(state: &AppState, plan_id: &str) -> Result<Plan, ApiError> {
    PlanRecord::find(&state.pool, plan_id)
        .await
        .and_then(|plan| plan.ok_or(AppError::PlanNotFound))
        .map_err(|error| ApiError::new(state, error))
}

/// Creates the plan of that name, or overwrites the one already called that.
///
/// The name is the identity, so this is one route rather than a create and an update.
pub async fn save(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let payload = posted_object(&headers, &body);
    let saved = async {
        let name = posted_name(&payload).unwrap_or_default();
        if name.trim().is_empty() {
            return Err(AppError::PlanNameMissing);
        }
        let fields = PlanFields::from_json(&payload)?;
        save_plan(&state.pool, &name, &fields).await
    };
    let plan = saved.await.map_err(|error| ApiError::new(&state, error))?;
    Ok((StatusCode::CREATED, Json(json!({ "plan": plan }))).into_response())
}

/// Throws one plan away. There is no undo.
pub async fn destroy(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Response, ApiError> {
    destroy_plan(&state, &plan_id).await
}

/// Throws away the plan whose name slugs to `export`.
pub async fn destroy_export(State(state): State<AppState>) -> Result<Response, ApiError> {
    destroy_plan(&state, "export").await
}

/// Throws away the plan whose name slugs to `import`.
pub async fn destroy_import(State(state): State<AppState>) -> Result<Response, ApiError> {
    destroy_plan(&state, "import").await
}

async fn destroy_plan(state: &AppState, plan_id: &str) -> Result<Response, ApiError> {
    let found = PlanRecord::delete(&state.pool, plan_id)
        .await
        .map_err(|error| ApiError::new(state, error))?;
    if !found {
        return Err(ApiError::new(state, AppError::PlanNotFound));
    }
    Ok(Json(json!({ "deleted": plan_id })).into_response())
}

/// Keeps the plan on screen, whatever state it is in.
///
/// The browser calls this after every edit, and no name is required: the draft is what is
/// being built rather than something someone has decided to keep.
pub async fn write_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let payload = posted_object(&headers, &body);
    let written = async {
        let fields = PlanFields::from_json(&payload)?;
        let name = posted_name(&payload).unwrap_or_default();
        save_draft(&state.pool, &name, &fields).await
    };
    let plan = written
        .await
        .map_err(|error| ApiError::new(&state, error))?;
    Ok(Json(json!({ "plan": plan })).into_response())
}

/// Throws the draft away: the Clear button.
///
/// Clearing a panel that was never saved is not an error, so this answers the same either way.
pub async fn discard_draft(State(state): State<AppState>) -> Result<Response, ApiError> {
    let cleared = clear_draft(&state.pool)
        .await
        .map_err(|error| ApiError::new(&state, error))?;
    Ok(Json(json!({ "cleared": cleared })).into_response())
}
