use crate::cli::listener::listen;
use crate::config::{AppConfig, database_label};
use crate::db;
use crate::error::AppError;
use crate::web::{AppState, build_router};

/// Opens the database and serves the app until SIGTERM or Ctrl-C.
pub async fn serve(config: &AppConfig) -> Result<(), AppError> {
    let pool = db::connect(&config.database_url).await?;
    let state = AppState {
        pool,
        database_name: database_label(&config.database_url),
    };
    let router = build_router(state);
    listen(router, config).await
}
