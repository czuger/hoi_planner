//! The app on an in-memory database, driven one request at a time.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use std::sync::Arc;

use hoi4_planner::db;
use hoi4_planner::domain::{Catalog, Catalogs, StaffCatalog, TechnologyCatalog};
use hoi4_planner::web::{AppState, build_router};
use serde_json::Value;
use sqlx::SqlitePool;
use tower::ServiceExt;

pub const DATABASE_NAME: &str = "test.db";

pub struct TestApp {
    pub pool: SqlitePool,
    pub router: Router,
}

pub struct Answer {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Answer {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }
}

impl TestApp {
    /// No catalog at all.
    pub async fn new() -> TestApp {
        TestApp::serving(Catalogs::default()).await
    }

    /// The app serving these catalogs.
    pub async fn serving(catalogs: Catalogs) -> TestApp {
        let pool = db::connect("sqlite::memory:").await.unwrap();
        let state = AppState {
            pool: pool.clone(),
            database_name: DATABASE_NAME.to_string(),
            catalogs: Arc::new(catalogs),
        };
        let router = build_router(state);
        TestApp { pool, router }
    }

    /// The focus catalog.
    pub async fn with_catalog() -> TestApp {
        TestApp::serving(Catalogs {
            focuses: Catalog::new(super::catalog()),
            ..Catalogs::default()
        })
        .await
    }

    /// The focus catalog and the research catalog.
    pub async fn with_technologies() -> TestApp {
        TestApp::serving(Catalogs {
            focuses: Catalog::new(super::catalog()),
            technologies: TechnologyCatalog::new(super::technologies()),
            ..Catalogs::default()
        })
        .await
    }

    /// The focus catalog and the staff catalog.
    pub async fn with_staff() -> TestApp {
        TestApp::serving(Catalogs {
            focuses: Catalog::new(super::catalog()),
            staff: StaffCatalog::new(super::staff()),
            ..Catalogs::default()
        })
        .await
    }

    /// Every call fails from now on, the way a database that cannot be read does.
    pub async fn lose_database(&self) {
        self.pool.close().await;
    }

    pub async fn request(&self, method: Method, path: &str, json: Option<Value>) -> Answer {
        let builder = Request::builder().method(method).uri(path);
        let request = match json {
            Some(json) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json.to_string())),
            None => builder.body(Body::empty()),
        };
        self.send(request.unwrap()).await
    }

    pub async fn send(&self, request: Request<Body>) -> Answer {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        Answer {
            status,
            headers,
            body: String::from_utf8(bytes.to_vec()).unwrap(),
        }
    }

    pub async fn get(&self, path: &str) -> Answer {
        self.request(Method::GET, path, None).await
    }

    pub async fn delete(&self, path: &str) -> Answer {
        self.request(Method::DELETE, path, None).await
    }

    pub async fn post(&self, path: &str, json: Value) -> Answer {
        self.request(Method::POST, path, Some(json)).await
    }

    pub async fn put(&self, path: &str, json: Value) -> Answer {
        self.request(Method::PUT, path, Some(json)).await
    }
}
