use std::sync::Arc;

use crate::cli::listener::listen;
use crate::config::{AppConfig, database_label};
use crate::db::Store;
use crate::error::AppError;
use crate::service::bundled_catalogs;
use crate::web::{AppState, build_router};

/// Reads the catalogs, opens the database and serves the app until SIGTERM or Ctrl-C.
pub async fn serve(config: &AppConfig) -> Result<(), AppError> {
    let catalogs = bundled_catalogs()?;
    let store = Store::open(&config.database_path)?;
    let state = AppState {
        store,
        database_name: database_label(&config.database_path),
        catalogs: Arc::new(catalogs),
    };
    let router = build_router(state);
    listen(router, config).await
}
