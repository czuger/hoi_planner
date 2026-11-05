use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;

const INTERNAL_MESSAGE: &str = "Une erreur interne est survenue. Veuillez réessayer.";

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = ?self, "page failed");
        (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_MESSAGE).into_response()
    }
}
