//! The page, its assets, the old pages leading to it, and the health check.

mod common;

use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, LOCATION};
use common::app::{DATABASE_NAME, TestApp};
use common::{catalog, technologies};
use hoi4_planner::db::{FocusRecord, TechnologyRecord};
use hoi4_planner::domain::PlanFields;
use hoi4_planner::service::{save_draft, save_plan};
use std::collections::BTreeMap;

fn fields(focus_ids: &[&str], start_date: &str) -> PlanFields {
    PlanFields {
        focus_ids: focus_ids.iter().map(ToString::to_string).collect(),
        start_date: Some(start_date.to_string()),
        ..PlanFields::empty()
    }
}

#[tokio::test]
async fn the_page_lists_every_focus() {
    let body = TestApp::with_catalog().await.get("/").await.body;
    assert!(body.contains("Remilitarize the Rhineland"));
    assert!(body.contains("Fortify the Vaterland"));
}

#[tokio::test]
async fn the_page_groups_focuses_under_their_branch() {
    let body = TestApp::with_catalog().await.get("/").await.body;
    assert!(body.contains("data-section=\"Rhineland Branch\""));
    assert!(body.contains("data-section=\"Anschluss Sub-branch\""));
    assert_eq!(body.matches("class=\"branch\"").count(), 2);
}

#[tokio::test]
async fn the_page_links_icons_rather_than_serving_them() {
    let body = TestApp::with_catalog().await.get("/").await.body;
    assert!(body.contains("https://hoi4.paradoxwikis.com/images/"));
}

#[tokio::test]
async fn the_page_carries_what_a_focus_depends_on() {
    let app = TestApp::new().await;
    let mut focuses = catalog();
    focuses[1].requires = vec![vec!["Anschluss".to_string()]];
    FocusRecord::replace_all(&app.pool, &focuses).await.unwrap();

    let body = app.get("/").await.body;
    assert!(body.contains("data-requires=\"[[&#34;Anschluss&#34;]]\""));
}

#[tokio::test]
async fn the_page_offers_the_saved_plans() {
    let app = TestApp::with_catalog().await;
    save_plan(
        &app.pool,
        "Barbarossa rush",
        &fields(&["Anschluss"], "1936-01-01"),
    )
    .await
    .unwrap();
    let body = app.get("/").await.body;
    assert!(body.contains("<option value=\"barbarossa-rush\">Barbarossa rush (1)</option>"));
}

#[tokio::test]
async fn the_page_carries_the_plan_that_was_being_built() {
    let app = TestApp::with_catalog().await;
    save_draft(
        &app.pool,
        "Half named",
        &fields(&["Anschluss"], "1937-01-01"),
    )
    .await
    .unwrap();

    let body = app.get("/").await.body;
    assert!(body.contains("data-draft="));
    assert!(body.contains("Half named"));
    assert!(body.contains("1937-01-01"));
}

#[tokio::test]
async fn the_page_draws_with_no_draft_to_pick_up() {
    let body = TestApp::with_catalog().await.get("/").await.body;
    assert!(body.contains("data-draft=\"{}\""));
}

#[tokio::test]
async fn the_page_carries_the_urls_a_plan_travels_through() {
    let body = TestApp::with_catalog().await.get("/").await.body;
    assert!(body.contains("data-export-url=\"/api/plans/export\""));
    assert!(body.contains("data-import-url=\"/api/plans/import\""));
}

#[tokio::test]
async fn every_link_is_built_under_the_prefix() {
    let app = TestApp::under("/hoi4").await;
    let body = app.get("/").await.body;
    assert!(body.contains("href=\"/hoi4/static/app.css\""));
    assert!(body.contains("src=\"/hoi4/static/app.js\""));
    assert!(body.contains("data-plans-url=\"/hoi4/api/plans\""));
    assert!(body.contains("data-draft-url=\"/hoi4/api/draft\""));
    // The proxy strips the prefix, so routing itself does not change.
    assert_eq!(app.get("/health").await.status, StatusCode::OK);
}

#[tokio::test]
async fn the_draft_is_not_offered_among_the_saved_plans() {
    let app = TestApp::with_catalog().await;
    save_draft(
        &app.pool,
        "Half named",
        &fields(&["Anschluss"], "1936-01-01"),
    )
    .await
    .unwrap();
    assert!(
        !app.get("/")
            .await
            .body
            .contains("<option value=\"~draft\">")
    );
}

#[tokio::test]
async fn an_empty_catalog_says_how_to_seed_it() {
    let body = TestApp::new().await.get("/").await.body;
    assert!(body.contains("import-focuses"));
}

#[tokio::test]
async fn the_page_still_draws_when_the_database_fails() {
    let app = TestApp::new().await;
    app.lose_database().await;

    let answer = app.get("/").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert!(
        answer
            .body
            .contains("Impossible d&#39;accéder à la base de données")
    );
}

#[tokio::test]
async fn the_page_speaks_french() {
    let body = TestApp::with_catalog().await.get("/").await.body;
    assert!(body.contains("<html lang=\"fr\">"));
    assert!(body.contains("Enregistrer"));
    assert!(body.contains("Planificateur"));
}

#[tokio::test]
async fn assets_are_served_and_revalidated() {
    let app = TestApp::new().await;
    for path in ["/static/app.js", "/static/app.css"] {
        let answer = app.get(path).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.headers[CACHE_CONTROL], "no-cache");
    }
}

#[tokio::test]
async fn health_reports_the_catalog_size() {
    let body = TestApp::with_catalog().await.get("/health").await.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["focuses"], 3);
    assert_eq!(body["database"], DATABASE_NAME);
}

#[tokio::test]
async fn health_is_degraded_when_the_database_fails() {
    let app = TestApp::new().await;
    app.lose_database().await;

    let answer = app.get("/health").await;
    assert_eq!(answer.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(answer.json()["status"], "degraded");
}

#[tokio::test]
async fn health_reports_the_technologies_too() {
    let body = TestApp::with_technologies()
        .await
        .get("/health")
        .await
        .json();
    assert_eq!(body["technologies"], 3);
}

#[tokio::test]
async fn the_old_pages_lead_to_the_one_page_under_the_prefix() {
    let app = TestApp::under("/hoi4").await;
    for path in ["/technologies", "/staff"] {
        let answer = app.get(path).await;
        assert_eq!(answer.status, StatusCode::PERMANENT_REDIRECT);
        assert_eq!(answer.headers[LOCATION], "/hoi4/");
    }
}

#[tokio::test]
async fn the_page_loads_the_scripts_of_every_tab_under_the_prefix() {
    let body = TestApp::under("/hoi4").await.get("/").await.body;
    for script in ["common.js", "technologies.js", "staff.js", "app.js"] {
        assert!(body.contains(&format!("src=\"/hoi4/static/{script}\"")));
    }
    assert!(!body.contains("href=\"/hoi4/technologies\""));
}

#[tokio::test]
async fn the_page_draws_one_tab_per_catalog() {
    let body = TestApp::new().await.get("/").await.body;
    for pane in ["focus", "tech", "staff"] {
        assert!(body.contains(&format!("data-pane=\"{pane}\"")));
        assert!(body.contains(&format!("id=\"{pane}-list\"")));
    }
}

#[tokio::test]
async fn the_page_lists_the_research_catalog_by_domain_with_what_each_needs() {
    let app = TestApp::new().await;
    let mut listed = technologies();
    listed[1].requires = vec![vec!["Construction_I".to_string()]];
    TechnologyRecord::replace_all(&app.pool, &listed)
        .await
        .unwrap();

    let body = app.get("/").await.body;
    assert!(body.contains("Construction II"));
    assert_eq!(body.matches("class=\"domain\"").count(), 1);
    assert!(body.contains("data-requires=\"[[&#34;Construction_I&#34;]]\""));
    assert!(body.contains("data-year=\"1937\""));
    assert!(body.contains("220 j"));
}

#[tokio::test]
async fn a_secret_tech_is_drawn_without_a_year() {
    let body = TestApp::with_technologies().await.get("/").await.body;
    assert!(body.contains("Jungle Training"));
    assert!(body.contains("data-year=\"\""));
}

#[tokio::test]
async fn the_page_carries_the_attached_technologies_and_staff_in_the_draft() {
    let app = TestApp::with_technologies().await;
    let mut plan = fields(&["Rhineland"], "1937-06-01");
    plan.focus_technologies =
        BTreeMap::from([("Rhineland".to_string(), vec!["Construction_I".to_string()])]);
    plan.focus_staff = BTreeMap::from([("Rhineland".to_string(), vec!["Krupp".to_string()])]);
    save_draft(&app.pool, "Rush", &plan).await.unwrap();

    let body = app.get("/").await.body;
    assert!(
        body.contains(
            "&#34;focus_technologies&#34;:{&#34;Rhineland&#34;:[&#34;Construction_I&#34;]}"
        )
    );
    assert!(body.contains("&#34;focus_staff&#34;:{&#34;Rhineland&#34;:[&#34;Krupp&#34;]}"));
}

#[tokio::test]
async fn each_empty_catalog_says_how_to_fill_it() {
    let body = TestApp::new().await.get("/").await.body;
    assert_eq!(body.matches("class=\"err\"").count(), 3);
    for command in ["import-focuses", "import-technologies", "import-staff"] {
        assert!(body.contains(&format!("hoi4_planner {command}")));
    }

    let body = TestApp::with_technologies().await.get("/").await.body;
    assert_eq!(body.matches("class=\"err\"").count(), 1);
    assert!(body.contains("hoi4_planner import-staff"));
}

#[tokio::test]
async fn the_page_draws_with_every_catalog_when_the_database_fails() {
    let app = TestApp::with_staff().await;
    app.lose_database().await;
    let answer = app.get("/").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body.matches("class=\"err\"").count(), 1);
    assert!(answer.body.contains("id=\"staff-list\""));
}

#[tokio::test]
async fn the_tab_scripts_are_served() {
    let app = TestApp::new().await;
    for path in [
        "/static/common.js",
        "/static/technologies.js",
        "/static/staff.js",
    ] {
        assert_eq!(app.get(path).await.status, StatusCode::OK);
    }
}

#[tokio::test]
async fn health_reports_the_staff_too() {
    let body = TestApp::with_staff().await.get("/health").await.json();
    assert_eq!(body["staff"], 3);
}

#[tokio::test]
async fn the_page_lists_the_staff_under_their_posts_with_what_each_needs() {
    let body = TestApp::with_staff().await.get("/").await.body;
    assert!(body.contains("Heinz Guderian"));
    assert_eq!(body.matches("class=\"post\"").count(), 3);
    assert_eq!(
        body.matches("<option value=\"Industrial concerns\">")
            .count(),
        1
    );
    assert!(body.contains("data-post=\"Industrial concerns\""));
    assert!(body.contains("data-requires=\"[[&#34;Rhineland&#34;]]\""));
}

#[tokio::test]
async fn a_member_with_no_portrait_is_drawn_without_one() {
    let body = TestApp::with_staff().await.get("/").await.body;
    assert!(body.contains("<span class=\"noicon\"></span>"));
}
