//! The JSON the planner saves through, and the draft it writes after every edit.

mod common;

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Method, Request, StatusCode};
use common::app::TestApp;
use hoi4_planner::db::PlanRecord;
use hoi4_planner::domain::PlanFields;
use hoi4_planner::service::{load_draft, save_draft, save_plan};
use serde_json::{Value, json};

async fn post_plan(app: &TestApp, payload: Value) -> common::app::Answer {
    let mut body = json!({"name": "My plan", "focus_ids": []});
    merge(&mut body, payload);
    app.post("/api/plans", body).await
}

async fn put_draft(app: &TestApp, payload: Value) -> common::app::Answer {
    let mut body = json!({"focus_ids": []});
    merge(&mut body, payload);
    app.put("/api/draft", body).await
}

fn merge(body: &mut Value, extra: Value) {
    if let Value::Object(extra) = extra {
        for (key, value) in extra {
            body[key] = value;
        }
    }
}

fn fields(focus_ids: &[&str], start_date: &str) -> PlanFields {
    PlanFields {
        focus_ids: focus_ids.iter().map(ToString::to_string).collect(),
        start_date: Some(start_date.to_string()),
        ..PlanFields::empty()
    }
}

async fn saved(app: &TestApp) -> Vec<hoi4_planner::domain::Plan> {
    PlanRecord::list_saved(&app.pool).await.unwrap()
}

#[tokio::test]
async fn saving_a_plan_stores_it() {
    let app = TestApp::new().await;
    let answer = post_plan(
        &app,
        json!({"name": "Barbarossa rush", "focus_ids": ["Anschluss", "Rhineland"]}),
    )
    .await;

    assert_eq!(answer.status, StatusCode::CREATED);
    assert_eq!(answer.json()["plan"]["id"], "barbarossa-rush");
    assert_eq!(saved(&app).await[0].focus_ids, ["Anschluss", "Rhineland"]);
}

#[tokio::test]
async fn the_stored_order_is_the_order_posted() {
    let app = TestApp::new().await;
    post_plan(&app, json!({"focus_ids": ["c", "a", "b"]})).await;
    assert_eq!(saved(&app).await[0].focus_ids, ["c", "a", "b"]);
}

#[tokio::test]
async fn a_plan_needs_a_name() {
    let app = TestApp::new().await;
    for body in [
        json!({"focus_ids": []}),
        json!({"name": "  ", "focus_ids": []}),
        json!({"name": null, "focus_ids": []}),
    ] {
        let answer = app.post("/api/plans", body).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
        assert!(answer.json()["error"].as_str().unwrap().contains("nom"));
    }
}

#[tokio::test]
async fn a_name_with_nothing_to_slug_is_refused() {
    let app = TestApp::new().await;
    let answer = post_plan(&app, json!({"name": "!!!"})).await;
    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    assert!(saved(&app).await.is_empty());
}

#[tokio::test]
async fn a_body_that_is_not_a_plan_is_refused_rather_than_saved() {
    let app = TestApp::new().await;
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/plans")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    assert_eq!(app.send(request).await.status, StatusCode::BAD_REQUEST);

    let array = app.post("/api/plans", json!([1, 2])).await;
    assert_eq!(array.status, StatusCode::BAD_REQUEST);
    assert!(saved(&app).await.is_empty());
}

#[tokio::test]
async fn focus_ids_must_be_a_list_of_ids() {
    let app = TestApp::new().await;
    let not_a_list = post_plan(&app, json!({"focus_ids": "Anschluss"})).await;
    assert_eq!(not_a_list.status, StatusCode::BAD_REQUEST);
    let not_ids = post_plan(&app, json!({"focus_ids": ["Anschluss", 7]})).await;
    assert_eq!(not_ids.status, StatusCode::BAD_REQUEST);
    assert!(saved(&app).await.is_empty());
}

#[tokio::test]
async fn days_per_focus_must_be_a_positive_whole_number() {
    let app = TestApp::new().await;
    for days in [json!("soon"), json!(0)] {
        let answer = post_plan(&app, json!({ "days_per_focus": days })).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn days_per_focus_defaults_to_thirty_five() {
    let app = TestApp::new().await;
    post_plan(&app, json!({})).await;
    assert_eq!(saved(&app).await[0].days_per_focus, 35);
}

#[tokio::test]
async fn focus_days_are_stored_against_their_focus() {
    let app = TestApp::new().await;
    post_plan(
        &app,
        json!({"focus_ids": ["Anschluss", "Rhineland"], "focus_days": {"Anschluss": 70}}),
    )
    .await;
    assert_eq!(saved(&app).await[0].focus_days["Anschluss"], 70);
}

#[tokio::test]
async fn focus_days_default_to_none_marked() {
    let app = TestApp::new().await;
    post_plan(&app, json!({"focus_ids": ["Anschluss"]})).await;
    assert!(saved(&app).await[0].focus_days.is_empty());
}

#[tokio::test]
async fn focus_days_must_be_an_object_of_positive_whole_numbers() {
    let app = TestApp::new().await;
    for marks in [
        json!([70, 35]),
        json!({"Anschluss": "soon"}),
        json!({"Anschluss": 0}),
    ] {
        let answer = post_plan(&app, json!({ "focus_days": marks })).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn listing_gives_every_plan() {
    let app = TestApp::new().await;
    save_plan(&app.pool, "First", &fields(&[], "1936-01-01"))
        .await
        .unwrap();
    save_plan(&app.pool, "Second", &fields(&[], "1936-01-01"))
        .await
        .unwrap();

    let body = app.get("/api/plans").await.json();
    let names: Vec<&str> = body["plans"]
        .as_array()
        .unwrap()
        .iter()
        .map(|plan| plan["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Second", "First"]);
}

#[tokio::test]
async fn one_plan_can_be_fetched_back() {
    let app = TestApp::new().await;
    save_plan(
        &app.pool,
        "Barbarossa rush",
        &fields(&["Anschluss"], "1937-05-02"),
    )
    .await
    .unwrap();

    let body = app.get("/api/plans/barbarossa-rush").await.json();
    assert_eq!(body["plan"]["focus_ids"], json!(["Anschluss"]));
    assert_eq!(body["plan"]["start_date"], "1937-05-02");
}

#[tokio::test]
async fn fetching_a_plan_that_is_not_there() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/api/plans/nope").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn deleting_a_plan() {
    let app = TestApp::new().await;
    save_plan(&app.pool, "Doomed", &fields(&[], "1936-01-01"))
        .await
        .unwrap();

    let answer = app.delete("/api/plans/doomed").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.json()["deleted"], "doomed");
    assert!(saved(&app).await.is_empty());
}

#[tokio::test]
async fn deleting_a_plan_that_is_not_there() {
    let app = TestApp::new().await;
    assert_eq!(
        app.delete("/api/plans/nope").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_plan_called_like_a_file_route_is_still_a_plan() {
    let app = TestApp::new().await;
    for (name, id) in [("Export", "export"), ("Import", "import")] {
        save_plan(&app.pool, name, &fields(&["Anschluss"], "1936-01-01"))
            .await
            .unwrap();
        let path = format!("/api/plans/{id}");

        let shown = app.get(&path).await;
        assert_eq!(shown.status, StatusCode::OK);
        assert_eq!(shown.json()["plan"]["name"], name);
        assert_eq!(app.delete(&path).await.status, StatusCode::OK);
        assert_eq!(app.get(&path).await.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn every_route_reports_a_failing_database() {
    let app = TestApp::new().await;
    app.lose_database().await;

    let file = json!({"format": "hoi4-planner-plan", "version": 1});
    let answers = [
        app.get("/api/plans").await,
        app.get("/api/plans/any").await,
        post_plan(&app, json!({})).await,
        app.delete("/api/plans/any").await,
        put_draft(&app, json!({})).await,
        app.delete("/api/draft").await,
        app.post("/api/plans/export", json!({"focus_ids": []}))
            .await,
        app.get("/api/plans/any/export").await,
        app.post("/api/plans/import", file).await,
    ];
    for answer in answers {
        assert_eq!(answer.status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            answer.json()["error"]
                .as_str()
                .unwrap()
                .contains("base de données")
        );
    }
}

#[tokio::test]
async fn the_draft_is_kept_without_a_name() {
    let app = TestApp::new().await;
    let answer = put_draft(&app, json!({"focus_ids": ["Anschluss", "Rhineland"]})).await;

    assert_eq!(answer.status, StatusCode::OK);
    let draft = load_draft(&app.pool).await.unwrap().unwrap();
    assert_eq!(draft.focus_ids, ["Anschluss", "Rhineland"]);
    assert!(saved(&app).await.is_empty());
}

#[tokio::test]
async fn the_draft_keeps_the_name_being_typed() {
    let app = TestApp::new().await;
    put_draft(&app, json!({"name": "Barbarossa ru"})).await;
    assert_eq!(
        load_draft(&app.pool).await.unwrap().unwrap().name,
        "Barbarossa ru"
    );
}

#[tokio::test]
async fn writing_the_draft_again_replaces_it() {
    let app = TestApp::new().await;
    put_draft(&app, json!({"focus_ids": ["Anschluss"]})).await;
    put_draft(
        &app,
        json!({"focus_ids": ["Anschluss", "Rhineland"], "focus_days": {"Rhineland": 14}}),
    )
    .await;

    let draft = load_draft(&app.pool).await.unwrap().unwrap();
    assert_eq!(draft.focus_ids, ["Anschluss", "Rhineland"]);
    assert_eq!(draft.focus_days["Rhineland"], 14);
}

#[tokio::test]
async fn the_draft_is_checked_like_a_save_is() {
    let app = TestApp::new().await;
    for body in [
        json!({"focus_ids": "Anschluss"}),
        json!({"days_per_focus": 0}),
        json!({"focus_days": ["Anschluss"]}),
        json!({"focus_days": {"Anschluss": -1}}),
    ] {
        assert_eq!(put_draft(&app, body).await.status, StatusCode::BAD_REQUEST);
    }
    assert!(load_draft(&app.pool).await.unwrap().is_none());
}

#[tokio::test]
async fn clearing_throws_the_draft_away() {
    let app = TestApp::new().await;
    save_draft(&app.pool, "", &fields(&["Anschluss"], "1936-01-01"))
        .await
        .unwrap();

    let answer = app.delete("/api/draft").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.json()["cleared"], true);
    assert!(load_draft(&app.pool).await.unwrap().is_none());
}

#[tokio::test]
async fn clearing_a_plan_that_was_never_kept_is_not_an_error() {
    let app = TestApp::new().await;
    let answer = app.delete("/api/draft").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.json()["cleared"], false);
}

#[tokio::test]
async fn clearing_the_draft_leaves_the_saved_plans_alone() {
    let app = TestApp::new().await;
    save_plan(
        &app.pool,
        "Barbarossa rush",
        &fields(&["Anschluss"], "1936-01-01"),
    )
    .await
    .unwrap();
    save_draft(&app.pool, "", &fields(&["Rhineland"], "1936-01-01"))
        .await
        .unwrap();

    app.delete("/api/draft").await;
    assert_eq!(saved(&app).await[0].id, "barbarossa-rush");
}

#[tokio::test]
async fn technologies_are_saved_with_the_plan_and_kept_in_the_draft() {
    let app = TestApp::with_catalog().await;
    let body = json!({
        "name": "Rush",
        "focus_ids": ["Rhineland"],
        "focus_technologies": {"Rhineland": ["Construction_I"]},
    });

    let saved = app.post("/api/plans", body.clone()).await;
    assert_eq!(
        saved.json()["plan"]["focus_technologies"],
        json!({"Rhineland": ["Construction_I"]})
    );
    let draft = app.put("/api/draft", body).await;
    assert_eq!(
        draft.json()["plan"]["focus_technologies"],
        json!({"Rhineland": ["Construction_I"]})
    );
}

#[tokio::test]
async fn technologies_that_are_not_lists_of_ids_are_refused() {
    let app = TestApp::with_catalog().await;
    for attached in [json!(["T"]), json!({"A": "T"}), json!({"A": [7]})] {
        let body = json!({"name": "Rush", "focus_technologies": attached});
        let answer = app.post("/api/plans", body).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
        assert!(
            answer.json()["error"]
                .as_str()
                .unwrap()
                .contains("focus_technologies")
        );
    }
}
