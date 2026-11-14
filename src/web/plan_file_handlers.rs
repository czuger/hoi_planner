use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::domain::{Catalogs, Plan, PlanFields, PlanFile, plan_text, posted_name, read_plan_file};
use crate::error::AppError;
use crate::service::PlanDownload;
use crate::web::api_error::ApiError;
use crate::web::app_state::AppState;
use crate::web::json_body::{posted_json, posted_object};
use crate::web::plan_handlers::found_plan;

/// The plan on screen, as a file: the Export button.
///
/// A POST because what is exported is the panel: the draft is written on a timer, so asking
/// for the stored one could hand back the plan as it was an edit ago.
pub async fn export_panel(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let payload = posted_object(&headers, &body);
    PlanFields::from_json(&payload)
        .map_err(AppError::from)
        .and_then(|fields| download(posted_name(&payload), fields, &state.catalogs))
        .map_err(|error| ApiError::new(&state, error))
}

/// A saved plan as a file, by id: the same file, without a browser.
pub async fn export_saved(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Response, ApiError> {
    let plan = found_plan(&state, &plan_id).await?;
    let (name, fields) = saved_fields(plan);
    download(Some(name), fields, &state.catalogs).map_err(|error| ApiError::new(&state, error))
}

/// The plan on screen as plain text, to be copied and pasted: the Text button.
pub async fn export_panel_text(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let payload = posted_object(&headers, &body);
    let fields = PlanFields::from_json(&payload)
        .map_err(|error| ApiError::new(&state, AppError::from(error)))?;
    Ok(text(posted_name(&payload), fields, &state.catalogs))
}

/// A saved plan as plain text, by id.
pub async fn export_saved_text(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Response, ApiError> {
    let plan = found_plan(&state, &plan_id).await?;
    let (name, fields) = saved_fields(plan);
    Ok(text(Some(name), fields, &state.catalogs))
}

/// A stored plan's name, and the fields a file is written from.
fn saved_fields(plan: Plan) -> (String, PlanFields) {
    let fields = PlanFields {
        focus_ids: plan.focus_ids,
        start_date: Some(plan.start_date),
        days_per_focus: plan.days_per_focus,
        focus_days: plan.focus_days,
        focus_technologies: plan.focus_technologies,
        focus_staff: plan.focus_staff,
        focus_comments: plan.focus_comments,
        technology_designers: plan.technology_designers,
    };
    (plan.name, fields)
}

/// A file's contents, checked and handed back for the panel to draw.
///
/// It stores nothing: an import is a plan arriving, not a plan kept. `unknown` is the ids this
/// catalog does not have, handed back with the rest rather than dropped, so that a plan from a
/// fuller catalog says what is missing instead of quietly coming up short.
/// `unknown_technologies` and `unknown_staff` do the same for the other two catalogs.
pub async fn import_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let data = posted_json(&headers, &body);
    let answer =
        read_import(&data, &state.catalogs).map_err(|error| ApiError::new(&state, error))?;
    Ok(Json(answer).into_response())
}

fn read_import(data: &Value, catalogs: &Catalogs) -> Result<Value, AppError> {
    let plan = read_plan_file(data)?;
    let fields = PlanFields::from_json(&plan)?;
    Ok(imported(&plan, fields, catalogs))
}

fn imported(plan: &Value, fields: PlanFields, catalogs: &Catalogs) -> Value {
    let unknown: Vec<&String> = fields
        .focus_ids
        .iter()
        .filter(|focus_id| !catalogs.focuses.contains(focus_id))
        .collect();
    let unknown_technologies: Vec<&String> = fields
        .focus_technologies
        .values()
        .flatten()
        .filter(|technology_id| !catalogs.technologies.contains(technology_id))
        .collect();
    let unknown_staff: Vec<&String> = fields
        .focus_staff
        .values()
        .flatten()
        .filter(|staff_id| !catalogs.staff.contains(staff_id))
        .collect();
    json!({
        "plan": {
            "name": posted_name(plan).unwrap_or_default().trim(),
            "focus_ids": fields.focus_ids,
            "start_date": fields.start_date,
            "days_per_focus": fields.days_per_focus,
            "focus_days": fields.focus_days,
            "focus_technologies": fields.focus_technologies,
            "focus_staff": fields.focus_staff,
            "focus_comments": fields.focus_comments,
            "technology_designers": fields.technology_designers,
        },
        "unknown": unknown,
        "unknown_technologies": unknown_technologies,
        "unknown_staff": unknown_staff,
    })
}

fn download(
    name: Option<String>,
    fields: PlanFields,
    catalogs: &Catalogs,
) -> Result<Response, AppError> {
    let file = PlanDownload::new(
        name,
        fields,
        &catalogs.focuses,
        &catalogs.technologies,
        &catalogs.staff,
    )?;
    let disposition = format!("attachment; filename=\"{}\"", file.filename);
    Ok((
        [
            (CONTENT_TYPE, "application/json".to_string()),
            (CONTENT_DISPOSITION, disposition),
        ],
        file.body,
    )
        .into_response())
}

/// The plan as text, shown rather than downloaded: nothing to save, only to copy.
fn text(name: Option<String>, fields: PlanFields, catalogs: &Catalogs) -> Response {
    let file = PlanFile::new(
        name,
        fields,
        &catalogs.focuses,
        &catalogs.technologies,
        &catalogs.staff,
    );
    (
        [(CONTENT_TYPE, "text/plain; charset=utf-8")],
        plan_text(&file),
    )
        .into_response()
}
