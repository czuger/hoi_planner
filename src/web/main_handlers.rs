use askama::Template;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde_json::json;

use crate::db::{FocusRecord, PlanRecord, TechnologyRecord};
use crate::domain::{Focus, Plan, Technology};
use crate::error::AppError;
use crate::service::load_draft;
use crate::web::api_error::ApiError;
use crate::web::app_state::AppState;
use crate::web::planner_page::PlannerPage;
use crate::web::technologies_page::TechnologiesPage;

const CATALOG_EMPTY: &str =
    "Aucun focus enregistré. Remplissez le catalogue avec : hoi4_planner import-focuses";
const TECHNOLOGIES_EMPTY: &str = "Aucune technologie enregistrée. Remplissez le catalogue avec : hoi4_planner import-technologies";

/// The one page: the catalog, the saved plans, and the plan that was being built.
///
/// It still draws when the database fails, saying so. The draft comes down with the page
/// rather than being fetched after it, or an empty plan would show for a moment.
pub async fn index(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = match load_page(&state).await {
        Ok((focuses, plans, draft)) => {
            let error = focuses.is_empty().then(|| CATALOG_EMPTY.to_string());
            PlannerPage::new(&state, focuses, plans, draft, error)?
        }
        Err(error) if error.is_database() => {
            tracing::error!(?error, "database failure");
            let message = ApiError::database_message(&state.database_name);
            PlannerPage::new(&state, Vec::new(), Vec::new(), None, Some(message))?
        }
        Err(error) => return Err(error),
    };
    Ok(Html(page.render()?).into_response())
}

async fn load_page(state: &AppState) -> Result<(Vec<Focus>, Vec<Plan>, Option<Plan>), AppError> {
    let focuses = FocusRecord::list(&state.pool).await?;
    let plans = PlanRecord::list_saved(&state.pool).await?;
    let draft = load_draft(&state.pool).await?;
    Ok((focuses, plans, draft))
}

/// The second page: the research catalog, and the plan's focuses with their technologies.
///
/// It still draws when the database fails, saying so, like the focus page.
pub async fn technologies(State(state): State<AppState>) -> Result<Response, AppError> {
    let page = match load_technologies_page(&state).await {
        Ok((technologies, focuses, draft)) => {
            let error = technologies
                .is_empty()
                .then(|| TECHNOLOGIES_EMPTY.to_string());
            TechnologiesPage::new(&state, technologies, focuses, draft, error)?
        }
        Err(error) if error.is_database() => {
            tracing::error!(?error, "database failure");
            let message = ApiError::database_message(&state.database_name);
            TechnologiesPage::new(&state, Vec::new(), Vec::new(), None, Some(message))?
        }
        Err(error) => return Err(error),
    };
    Ok(Html(page.render()?).into_response())
}

async fn load_technologies_page(
    state: &AppState,
) -> Result<(Vec<Technology>, Vec<Focus>, Option<Plan>), AppError> {
    let technologies = TechnologyRecord::list(&state.pool).await?;
    let focuses = FocusRecord::list(&state.pool).await?;
    let draft = load_draft(&state.pool).await?;
    Ok((technologies, focuses, draft))
}

/// Is the app up, and can it read the database? For the deploy check.
pub async fn health(State(state): State<AppState>) -> Response {
    match catalog_sizes(&state).await {
        Ok((focuses, technologies)) => Json(json!({
            "status": "ok",
            "database": state.database_name,
            "focuses": focuses,
            "technologies": technologies,
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

async fn catalog_sizes(state: &AppState) -> Result<(i64, i64), AppError> {
    Ok((
        FocusRecord::count(&state.pool).await?,
        TechnologyRecord::count(&state.pool).await?,
    ))
}
