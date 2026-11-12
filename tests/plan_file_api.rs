//! A plan as a file: the download, and what an import hands back.

mod common;

use axum::http::StatusCode;
use axum::http::header::CONTENT_DISPOSITION;
use common::app::{Answer, TestApp};
use hoi4_planner::db::PlanRecord;
use hoi4_planner::domain::{FILE_FORMAT, PlanFields, VERSION};
use hoi4_planner::service::{load_draft, save_plan};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn a_file() -> Value {
    json!({
        "format": FILE_FORMAT,
        "version": VERSION,
        "name": "Barbarossa rush",
        "focus_ids": ["Rhineland", "Anschluss"],
        "start_date": "1936-01-01",
        "days_per_focus": 35,
        "focus_days": {"Anschluss": 70},
    })
}

/// The panel, exported: what the Export button posts.
async fn export(app: &TestApp, extra: Value) -> Answer {
    let mut body = json!({"name": "Barbarossa rush", "focus_ids": ["Rhineland"]});
    if let Value::Object(extra) = extra {
        for (key, value) in extra {
            body[key] = value;
        }
    }
    app.post("/api/plans/export", body).await
}

fn disposition(answer: &Answer) -> String {
    answer.headers[CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string()
}

async fn save(app: &TestApp, fields: PlanFields) {
    save_plan(&app.pool, "Barbarossa rush", &fields)
        .await
        .unwrap();
}

#[tokio::test]
async fn exporting_the_panel_gives_a_plan_file() {
    let app = TestApp::with_catalog().await;
    let answer = export(&app, json!({"focus_ids": ["Rhineland", "Anschluss"]})).await;

    assert_eq!(answer.json()["format"], FILE_FORMAT);
    assert_eq!(
        answer.json()["focus_ids"],
        json!(["Rhineland", "Anschluss"])
    );
    assert!(disposition(&answer).contains("attachment"));
}

#[tokio::test]
async fn the_file_is_indented_keeps_its_accents_and_ends_with_a_new_line() {
    let app = TestApp::with_catalog().await;
    let answer = export(&app, json!({"name": "Plan à Zürich"})).await;

    assert!(answer.body.starts_with(
        "{\n  \"format\": \"hoi4-planner-plan\",\n  \"version\": 4,\n  \"name\": \"Plan à Zürich\","
    ));
    assert!(answer.body.ends_with("}\n"));
}

#[tokio::test]
async fn exporting_twice_gives_the_same_bytes() {
    let app = TestApp::with_catalog().await;
    let first = export(
        &app,
        json!({"focus_days": {"Rhineland": 70, "Anschluss": 14}}),
    )
    .await;
    let second = export(
        &app,
        json!({"focus_days": {"Anschluss": 14, "Rhineland": 70}}),
    )
    .await;
    assert_eq!(first.body, second.body);
}

#[tokio::test]
async fn the_download_is_named_after_the_plan() {
    let app = TestApp::new().await;
    let answer = export(&app, json!({"name": "Barbarossa rush"})).await;
    assert!(disposition(&answer).contains("filename=\"barbarossa-rush.hoi4plan.json\""));
}

#[tokio::test]
async fn a_plan_with_no_usable_name_still_downloads() {
    let app = TestApp::new().await;
    let answer = export(&app, json!({"name": ""})).await;
    assert!(disposition(&answer).contains("filename=\"plan.hoi4plan.json\""));
}

#[tokio::test]
async fn the_exported_file_names_and_dates_the_focuses() {
    let app = TestApp::with_catalog().await;
    let answer = export(&app, json!({"focus_ids": ["Rhineland"]})).await;
    assert_eq!(
        answer.json()["focuses"],
        json!([{
            "position": 1,
            "id": "Rhineland",
            "name": "Remilitarize the Rhineland",
            "days": 35,
            "completes_on": "1936-02-05",
        }])
    );
}

#[tokio::test]
async fn the_panel_is_checked_the_way_a_save_is() {
    let app = TestApp::new().await;
    let days = export(&app, json!({"days_per_focus": 0})).await;
    assert_eq!(days.status, StatusCode::BAD_REQUEST);
    let ids = export(&app, json!({"focus_ids": "Rhineland"})).await;
    assert_eq!(ids.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_saved_plan_exports_by_id() {
    let app = TestApp::with_catalog().await;
    save(
        &app,
        PlanFields {
            focus_ids: vec!["Rhineland".to_string()],
            start_date: Some("1936-01-01".to_string()),
            focus_days: BTreeMap::from([("Rhineland".to_string(), 14)]),
            ..PlanFields::empty()
        },
    )
    .await;

    let answer = app.get("/api/plans/barbarossa-rush/export").await;
    assert_eq!(answer.json()["name"], "Barbarossa rush");
    assert_eq!(answer.json()["focus_days"], json!({"Rhineland": 14}));
    assert!(disposition(&answer).contains("filename=\"barbarossa-rush.hoi4plan.json\""));
}

#[tokio::test]
async fn exporting_a_plan_that_is_not_there() {
    let app = TestApp::new().await;
    let answer = app.get("/api/plans/nope/export").await;
    assert_eq!(answer.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_plan_survives_the_round_trip() {
    let app = TestApp::with_catalog().await;
    save(
        &app,
        PlanFields {
            focus_ids: vec!["Rhineland".to_string(), "Anschluss".to_string()],
            start_date: Some("1937-06-01".to_string()),
            days_per_focus: 20,
            focus_days: BTreeMap::from([("Anschluss".to_string(), 70)]),
            focus_technologies: BTreeMap::from([(
                "Anschluss".to_string(),
                vec!["Construction_I".to_string()],
            )]),
            focus_staff: BTreeMap::from([("Rhineland".to_string(), vec!["Krupp".to_string()])]),
            focus_comments: BTreeMap::from([(
                "Anschluss".to_string(),
                vec!["avant la Tchécoslovaquie".to_string()],
            )]),
        },
    )
    .await;
    let file = app.get("/api/plans/barbarossa-rush/export").await.json();

    let answer = app.post("/api/plans/import", file).await;
    assert_eq!(
        answer.json()["plan"],
        json!({
            "name": "Barbarossa rush",
            "focus_ids": ["Rhineland", "Anschluss"],
            "start_date": "1937-06-01",
            "days_per_focus": 20,
            "focus_days": {"Anschluss": 70},
            "focus_technologies": {"Anschluss": ["Construction_I"]},
            "focus_staff": {"Rhineland": ["Krupp"]},
            "focus_comments": {"Anschluss": ["avant la Tchécoslovaquie"]},
        })
    );
}

#[tokio::test]
async fn the_file_carries_the_comments_on_their_lines() {
    let app = TestApp::with_catalog().await;
    let panel = json!({
        "name": "Rush",
        "focus_ids": ["Rhineland"],
        "focus_comments": {"Rhineland": ["garder 2 PP", "puis l'Autriche"]},
    });
    let file = app.post("/api/plans/export", panel).await.json();

    assert_eq!(file["version"], 4);
    assert_eq!(
        file["focus_comments"],
        json!({"Rhineland": ["garder 2 PP", "puis l'Autriche"]})
    );
    assert_eq!(
        file["focuses"][0]["comments"],
        json!(["garder 2 PP", "puis l'Autriche"])
    );
}

#[tokio::test]
async fn importing_stores_nothing() {
    let app = TestApp::with_catalog().await;
    app.post("/api/plans/import", a_file()).await;

    assert!(PlanRecord::list_saved(&app.pool).await.unwrap().is_empty());
    assert!(load_draft(&app.pool).await.unwrap().is_none());
}

#[tokio::test]
async fn importing_says_which_focuses_this_catalog_has_lost() {
    let app = TestApp::with_catalog().await;
    let mut file = a_file();
    file["focus_ids"] = json!(["Rhineland", "Sea_Lion"]);

    let body = app.post("/api/plans/import", file).await.json();
    assert_eq!(body["unknown"], json!(["Sea_Lion"]));
    // Kept all the same: the plan is the order, and it is not ours to shorten.
    assert_eq!(body["plan"]["focus_ids"], json!(["Rhineland", "Sea_Lion"]));
}

#[tokio::test]
async fn a_file_that_is_not_one_of_ours_is_refused() {
    let app = TestApp::with_catalog().await;
    let answer = app
        .post("/api/plans/import", json!({"focus_ids": ["Rhineland"]}))
        .await;

    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    assert!(
        answer.json()["error"]
            .as_str()
            .unwrap()
            .contains(FILE_FORMAT)
    );
}

#[tokio::test]
async fn an_imported_file_is_checked_the_way_a_save_is() {
    let app = TestApp::with_catalog().await;
    let mut negative = a_file();
    negative["focus_days"] = json!({"Rhineland": -1});
    let answer = app.post("/api/plans/import", negative).await;
    assert_eq!(answer.status, StatusCode::BAD_REQUEST);

    let mut not_ids = a_file();
    not_ids["focus_ids"] = json!([7]);
    let answer = app.post("/api/plans/import", not_ids).await;
    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_file_exported_by_the_python_planner_still_imports() {
    let app = TestApp::with_catalog().await;
    let file = json!({
        "format": "hoi4-planner-plan",
        "version": 1,
        "name": "Ancien plan",
        "focus_ids": ["Rhineland"],
        "start_date": "1936-01-01",
        "days_per_focus": 35,
        "focus_days": {},
        "focuses": [{"position": 1, "id": "Rhineland", "name": "x", "days": 35, "completes_on": "1936-02-05"}],
    });
    let answer = app.post("/api/plans/import", file).await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.json()["plan"]["name"], "Ancien plan");
    assert_eq!(answer.json()["unknown"], json!([]));
}

#[tokio::test]
async fn the_file_carries_the_technologies_and_names_them_beside_their_focus() {
    let app = TestApp::with_technologies().await;
    let answer = export(
        &app,
        json!({"focus_technologies": {"Rhineland": ["Construction_I", "Gone"]}}),
    )
    .await;

    let file = answer.json();
    assert_eq!(
        file["focus_technologies"],
        json!({"Rhineland": ["Construction_I", "Gone"]})
    );
    assert_eq!(
        file["focuses"][0]["technologies"],
        json!([{"id": "Construction_I", "name": "Construction I"}])
    );
}

#[tokio::test]
async fn a_focus_with_no_technology_has_no_empty_list_in_the_echo() {
    let app = TestApp::with_technologies().await;
    let file = export(&app, json!({})).await.json();
    assert!(file["focuses"][0].get("technologies").is_none());
}

#[tokio::test]
async fn exporting_technologies_twice_gives_the_same_bytes() {
    let app = TestApp::with_technologies().await;
    let extra = json!({"focus_technologies": {"Rhineland": ["Construction_II", "Construction_I"]}});
    let first = export(&app, extra.clone()).await;
    let second = export(&app, extra).await;
    assert_eq!(first.body, second.body);
}

#[tokio::test]
async fn a_saved_plan_exports_with_its_technologies() {
    let app = TestApp::with_technologies().await;
    let mut fields = PlanFields::empty();
    fields.focus_ids = vec!["Rhineland".to_string()];
    fields.focus_technologies =
        BTreeMap::from([("Rhineland".to_string(), vec!["Construction_I".to_string()])]);
    save(&app, fields).await;

    let file = app.get("/api/plans/barbarossa-rush/export").await.json();
    assert_eq!(
        file["focus_technologies"],
        json!({"Rhineland": ["Construction_I"]})
    );
}

#[tokio::test]
async fn an_import_hands_the_technologies_back_and_says_which_are_unknown() {
    let app = TestApp::with_technologies().await;
    let mut file = a_file();
    file["focus_technologies"] = json!({"Rhineland": ["Construction_I", "Gone"]});

    let answer = app.post("/api/plans/import", file).await.json();
    assert_eq!(
        answer["plan"]["focus_technologies"],
        json!({"Rhineland": ["Construction_I", "Gone"]})
    );
    assert_eq!(answer["unknown_technologies"], json!(["Gone"]));
}

#[tokio::test]
async fn a_file_from_before_technologies_imports_with_none() {
    let app = TestApp::with_technologies().await;
    let mut file = a_file();
    file["version"] = json!(1);

    let answer = app.post("/api/plans/import", file).await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.json()["plan"]["focus_technologies"], json!({}));
    assert_eq!(answer.json()["unknown_technologies"], json!([]));
}

#[tokio::test]
async fn the_file_carries_the_staff_and_names_their_posts() {
    let app = TestApp::with_staff().await;
    let file = export(
        &app,
        json!({
            "focus_ids": ["Rhineland"],
            "focus_staff": {"Rhineland": ["Gone", "Heinz_Guderian__Theorists"]},
        }),
    )
    .await
    .json();

    assert_eq!(
        file["focus_staff"],
        json!({"Rhineland": ["Gone", "Heinz_Guderian__Theorists"]})
    );
    assert_eq!(
        file["focuses"][0]["staff"],
        json!([{"id": "Heinz_Guderian__Theorists", "name": "Heinz Guderian", "post": "Theorists"}])
    );
}

#[tokio::test]
async fn importing_says_which_staff_this_catalog_has_lost() {
    let app = TestApp::with_staff().await;
    let mut file = a_file();
    file["focus_staff"] = json!({"Rhineland": ["Krupp", "Gone"]});

    let body = app.post("/api/plans/import", file).await.json();
    assert_eq!(body["unknown_staff"], json!(["Gone"]));
    assert_eq!(
        body["plan"]["focus_staff"],
        json!({"Rhineland": ["Krupp", "Gone"]})
    );
}
