use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::error::AppError;
use crate::web::app_state::AppState;

/// A failure answered as `{"error": "..."}` and a status, which the browser shows next to the
/// save button rather than a 500 in the console.
#[derive(Debug)]
pub struct ApiError {
    error: AppError,
    database_name: String,
}

impl ApiError {
    /// An error, reported against the database of this state.
    pub fn new(state: &AppState, error: AppError) -> ApiError {
        ApiError {
            error,
            database_name: state.database_name.clone(),
        }
    }

    /// The sentence shown when the database did not answer.
    pub fn database_message(database_name: &str) -> String {
        format!(
            "Impossible d'accéder à la base de données ({database_name}). Le fichier est-il accessible, et DATABASE_URL est-il correct ?"
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = if self.error.is_database() {
            tracing::error!(error = ?self.error, "database failure");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                ApiError::database_message(&self.database_name),
            )
        } else {
            let status = match self.error {
                AppError::PlanNotFound => StatusCode::NOT_FOUND,
                AppError::PlanFields(_)
                | AppError::PlanFile(_)
                | AppError::PlanNameMissing
                | AppError::PlanNameUnusable => StatusCode::BAD_REQUEST,
                _ => {
                    tracing::error!(error = ?self.error, "request failed");
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            };
            (status, self.error.to_string())
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
