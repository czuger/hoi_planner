//! The app on an in-memory database, driven one request at a time.

use std::path::Path;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use hoi4_planner::db::{self, FocusRecord, TechnologyRecord};
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
    pub async fn new() -> TestApp {
        TestApp::under("").await
    }

    pub async fn under(url_prefix: &str) -> TestApp {
        let pool = db::connect("sqlite::memory:").await.unwrap();
        let state = AppState {
            pool: pool.clone(),
            url_prefix: url_prefix.to_string(),
            database_name: DATABASE_NAME.to_string(),
        };
        let router = build_router(state, Path::new(env!("CARGO_MANIFEST_DIR")));
        TestApp { pool, router }
    }

    pub async fn with_catalog() -> TestApp {
        let app = TestApp::new().await;
        FocusRecord::replace_all(&app.pool, &super::catalog())
            .await
            .unwrap();
        app
    }

    /// The focus catalog and the research catalog, both filled.
    pub async fn with_technologies() -> TestApp {
        let app = TestApp::with_catalog().await;
        TechnologyRecord::replace_all(&app.pool, &super::technologies())
            .await
            .unwrap();
        app
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
