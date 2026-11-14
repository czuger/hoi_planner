use askama::Template;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde_json::json;

use crate::db::PlanRecord;
use crate::error::AppError;
use crate::service::load_draft;
use crate::web::api_error::ApiError;
use crate::web::app_state::AppState;
use crate::web::planner_page::{PlannerData, PlannerPage};

/// The one page: the three catalogs, the saved plans, and the plan that was being built.
///
/// It still draws when the database fails, catalogs included, saying so. The draft comes down
/// with the page rather than being fetched after it, or an empty plan would show for a moment.
pub async fn index(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = match load_page(&state).await {
        Ok(data) => PlannerPage::new(&state, data, Vec::new())?,
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
///
/// Relative, as the app does not know the sub-path a proxy serves it under.
pub async fn to_planner() -> Redirect {
    Redirect::permanent("./")
}

async fn load_page(state: &AppState) -> Result<PlannerData, AppError> {
    Ok(PlannerData {
        plans: PlanRecord::list_saved(&state.store).await?,
        draft: load_draft(&state.store).await?,
    })
}

/// Is the app up, and can it read the database? For the deploy check.
pub async fn health(State(state): State<AppState>) -> Response {
    let catalogs = &state.catalogs;
    match state.store.ping().await {
        Ok(()) => Json(json!({
            "status": "ok",
            "database": state.database_name,
            "focuses": catalogs.focuses.len(),
            "technologies": catalogs.technologies.len(),
            "staff": catalogs.staff.len(),
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
