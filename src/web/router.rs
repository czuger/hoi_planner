use std::path::Path as FilePath;

use axum::Router;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::{delete, get, post};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::web::app_state::AppState;
use crate::web::{main_handlers, plan_file_handlers, plan_handlers};

/// Builds every route of the application.
///
/// A plan named "Export" or "Import" slugs to the same path as the two routes for files, so
/// those paths also answer for a plan of that id, under the methods the files do not use.
///
/// # Arguments
/// * `asset_root` - directory holding the `static/` folder.
pub fn build_router(state: AppState, asset_root: &FilePath) -> Router {
    Router::new()
        .route("/", get(main_handlers::index))
        .route("/technologies", get(main_handlers::technologies))
        .route("/health", get(main_handlers::health))
        .route(
            "/api/plans",
            get(plan_handlers::list).post(plan_handlers::save),
        )
        .route(
            "/api/plans/export",
            post(plan_file_handlers::export_panel)
                .get(plan_handlers::show_export)
                .delete(plan_handlers::destroy_export),
        )
        .route(
            "/api/plans/import",
            post(plan_file_handlers::import_file)
                .get(plan_handlers::show_import)
                .delete(plan_handlers::destroy_import),
        )
        .route(
            "/api/plans/{plan_id}",
            get(plan_handlers::show).delete(plan_handlers::destroy),
        )
        .route(
            "/api/plans/{plan_id}/export",
            get(plan_file_handlers::export_saved),
        )
        .route(
            "/api/draft",
            delete(plan_handlers::discard_draft).put(plan_handlers::write_draft),
        )
        .nest_service("/static", ServeDir::new(asset_root.join("static")))
        // Assets keep stable URLs, so browsers must revalidate them (a 304 when unchanged) to see edits.
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .with_state(state)
}
