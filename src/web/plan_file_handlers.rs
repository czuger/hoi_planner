use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::db::{FocusRecord, StaffRecord, TechnologyRecord};
use crate::domain::{
    Catalog, PlanFields, StaffCatalog, TechnologyCatalog, posted_name, read_plan_file,
};
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
    let answer = async {
        let fields = PlanFields::from_json(&payload)?;
        let catalogs = load_catalogs(&state).await?;
        download(posted_name(&payload), fields, &catalogs)
    };
    answer.await.map_err(|error| ApiError::new(&state, error))
}

/// A saved plan as a file, by id: the same file, without a browser.
pub async fn export_saved(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Response, ApiError> {
    let plan = found_plan(&state, &plan_id).await?;
    let answer = async {
        let catalogs = load_catalogs(&state).await?;
        let fields = PlanFields {
            focus_ids: plan.focus_ids,
            start_date: Some(plan.start_date),
            days_per_focus: plan.days_per_focus,
            focus_days: plan.focus_days,
            focus_technologies: plan.focus_technologies,
            focus_staff: plan.focus_staff,
        };
        download(Some(plan.name), fields, &catalogs)
    };
    answer.await.map_err(|error| ApiError::new(&state, error))
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
    let answer = async {
        let plan = read_plan_file(&data)?;
        let fields = PlanFields::from_json(&plan)?;
        let catalogs = load_catalogs(&state).await?;
        Ok::<Value, AppError>(imported(&plan, fields, &catalogs))
    };
    let imported = answer.await.map_err(|error| ApiError::new(&state, error))?;
    Ok(Json(imported).into_response())
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
        },
        "unknown": unknown,
        "unknown_technologies": unknown_technologies,
        "unknown_staff": unknown_staff,
    })
}

/// What names the ids of a plan.
struct Catalogs {
    focuses: Catalog,
    technologies: TechnologyCatalog,
    staff: StaffCatalog,
}

async fn load_catalogs(state: &AppState) -> Result<Catalogs, AppError> {
    Ok(Catalogs {
        focuses: Catalog::new(FocusRecord::list(&state.pool).await?),
        technologies: TechnologyCatalog::new(TechnologyRecord::list(&state.pool).await?),
        staff: StaffCatalog::new(StaffRecord::list(&state.pool).await?),
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
