use askama::Template;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde_json::json;

use crate::db::{FocusRecord, PlanRecord, StaffRecord, TechnologyRecord};
use crate::error::AppError;
use crate::service::load_draft;
use crate::web::api_error::ApiError;
use crate::web::app_state::AppState;
use crate::web::planner_page::{PlannerData, PlannerPage};

const CATALOG_EMPTY: &str =
    "Aucun focus enregistré. Remplissez le catalogue avec : hoi4_planner import-focuses";
const TECHNOLOGIES_EMPTY: &str = "Aucune technologie enregistrée. Remplissez le catalogue avec : hoi4_planner import-technologies";
const STAFF_EMPTY: &str = "Aucun membre de l'état-major enregistré. Remplissez le catalogue avec : hoi4_planner import-staff";

/// The one page: the three catalogs, the saved plans, and the plan that was being built.
///
/// It still draws when the database fails, saying so. The draft comes down with the page
/// rather than being fetched after it, or an empty plan would show for a moment.
pub async fn index(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = match load_page(&state).await {
        Ok(data) => {
            let errors = empty_catalogs(&data);
            PlannerPage::new(&state, data, errors)?
        }
        Err(error) if error.is_database() => {
            tracing::error!(?error, "database failure");
            let message = ApiError::database_message(&state.database_name);
            PlannerPage::new(&state, PlannerData::default(), vec![message])?
        }
        Err(error) => return Err(error),
    };
    Ok(Html(page.render()?).into_response())
}

/// Where the technology and staff pages used to be: they are tabs of the one page now.
pub async fn to_planner(State(state): State<AppState>) -> Redirect {
    Redirect::permanent(&format!("{}/", state.url_prefix))
}

async fn load_page(state: &AppState) -> Result<PlannerData, AppError> {
    Ok(PlannerData {
        focuses: FocusRecord::list(&state.pool).await?,
        technologies: TechnologyRecord::list(&state.pool).await?,
        staff: StaffRecord::list(&state.pool).await?,
        plans: PlanRecord::list_saved(&state.pool).await?,
        draft: load_draft(&state.pool).await?,
    })
}

/// How to fill each catalog that is still empty.
fn empty_catalogs(data: &PlannerData) -> Vec<String> {
    [
        (data.focuses.is_empty(), CATALOG_EMPTY),
        (data.technologies.is_empty(), TECHNOLOGIES_EMPTY),
        (data.staff.is_empty(), STAFF_EMPTY),
    ]
    .into_iter()
    .filter(|(empty, _)| *empty)
    .map(|(_, message)| message.to_string())
    .collect()
}

/// Is the app up, and can it read the database? For the deploy check.
pub async fn health(State(state): State<AppState>) -> Response {
    match catalog_sizes(&state).await {
        Ok((focuses, technologies, staff)) => Json(json!({
            "status": "ok",
            "database": state.database_name,
            "focuses": focuses,
            "technologies": technologies,
            "staff": staff,
        }))
        .into_response(),
        Err(error) => {
            tracing::error!(?error, "database failure");
            let body = json!({
                "status": "degraded",
                "database": state.database_name,
                "error": ApiError::database_message(&state.database_name),
            });
            (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
        }
    }
}

async fn catalog_sizes(state: &AppState) -> Result<(i64, i64, i64), AppError> {
    Ok((
        FocusRecord::count(&state.pool).await?,
        TechnologyRecord::count(&state.pool).await?,
        StaffRecord::count(&state.pool).await?,
    ))
}
