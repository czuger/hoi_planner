//! Every read and write of the catalog and the plans, against an in-memory SQLite.

mod common;

use std::collections::BTreeMap;

use common::{FOCUS_ID, catalog, focus, staff, technologies};
use hoi4_planner::db::{self, FocusRecord, PlanRecord, StaffRecord, TechnologyRecord};
use hoi4_planner::domain::{DRAFT_PLAN_ID, PlanFields};
use hoi4_planner::error::AppError;
use hoi4_planner::service::{clear_draft, load_draft, save_draft, save_plan};
use sqlx::SqlitePool;

async fn pool() -> SqlitePool {
    db::connect("sqlite::memory:").await.unwrap()
}

fn fields(focus_ids: &[&str]) -> PlanFields {
    PlanFields {
        focus_ids: focus_ids.iter().map(ToString::to_string).collect(),
        start_date: Some("1936-01-01".to_string()),
        ..PlanFields::empty()
    }
}

fn marked(focus_ids: &[&str], marks: &[(&str, i64)]) -> PlanFields {
    PlanFields {
        focus_days: marks
            .iter()
            .map(|(id, days)| (id.to_string(), *days))
            .collect(),
        ..fields(focus_ids)
    }
}

async fn saved_ids(pool: &SqlitePool) -> Vec<String> {
    PlanRecord::list_saved(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|plan| plan.id)
        .collect()
}

#[tokio::test]
async fn replacing_the_catalog_stores_it_and_says_how_many() {
    let pool = pool().await;
    assert_eq!(
        FocusRecord::replace_all(&pool, &catalog()).await.unwrap(),
        3
    );
    assert_eq!(FocusRecord::count(&pool).await.unwrap(), 3);
}

#[tokio::test]
async fn a_re_import_loses_what_the_page_no_longer_has() {
    let pool = pool().await;
    FocusRecord::replace_all(&pool, &catalog()).await.unwrap();
    let only = [focus("Anschluss", "Anschluss", "S", 0)];
    FocusRecord::replace_all(&pool, &only).await.unwrap();

    assert_eq!(FocusRecord::list(&pool).await.unwrap()[0].id, "Anschluss");
    assert_eq!(FocusRecord::count(&pool).await.unwrap(), 1);
}

#[tokio::test]
async fn re_importing_nothing_empties_the_catalog() {
    let pool = pool().await;
    FocusRecord::replace_all(&pool, &catalog()).await.unwrap();
    assert_eq!(FocusRecord::replace_all(&pool, &[]).await.unwrap(), 0);
    assert!(FocusRecord::list(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn focuses_come_back_in_page_order() {
    let pool = pool().await;
    let mut reversed = catalog();
    reversed.reverse();
    FocusRecord::replace_all(&pool, &reversed).await.unwrap();

    let stored = FocusRecord::list(&pool).await.unwrap();
    let positions: Vec<i64> = stored.iter().map(|focus| focus.position).collect();
    assert_eq!(positions, [0, 1, 2]);
    assert_eq!(stored[0].id, FOCUS_ID);
}

#[tokio::test]
async fn a_focus_comes_back_as_it_went_in() {
    let pool = pool().await;
    let mut gamma = focus("Gamma", "Gamma", "S", 0);
    gamma.requires = vec![
        vec!["Alpha".to_string()],
        vec!["Beta".to_string(), "Delta".to_string()],
    ];
    gamma.excludes = vec!["Epsilon".to_string()];
    FocusRecord::replace_all(&pool, std::slice::from_ref(&gamma))
        .await
        .unwrap();

    assert_eq!(FocusRecord::list(&pool).await.unwrap(), [gamma]);
}

#[tokio::test]
async fn saving_stores_the_order_under_a_slug() {
    let pool = pool().await;
    let plan = save_plan(
        &pool,
        "Barbarossa rush",
        &fields(&["Rhineland", "Anschluss"]),
    )
    .await
    .unwrap();

    assert_eq!(plan.id, "barbarossa-rush");
    assert_eq!(plan.name, "Barbarossa rush");
    assert_eq!(plan.focus_ids, ["Rhineland", "Anschluss"]);
    assert_eq!(plan.days_per_focus, 35);
}

#[tokio::test]
async fn a_name_is_stored_without_the_spaces_around_it() {
    let plan = save_plan(&pool().await, "  Rush  ", &fields(&[]))
        .await
        .unwrap();
    assert_eq!(plan.name, "Rush");
}

#[tokio::test]
async fn a_plan_with_no_start_date_starts_when_the_game_does() {
    let plan = save_plan(&pool().await, "Rush", &PlanFields::empty())
        .await
        .unwrap();
    assert_eq!(plan.start_date, "1936-01-01");
}

#[tokio::test]
async fn saving_keeps_the_terms_of_the_focuses_it_holds() {
    let plan = marked(&["Rhineland", "Anschluss"], &[("Rhineland", 70)]);
    let stored = save_plan(&pool().await, "Marked", &plan).await.unwrap();
    assert_eq!(
        stored.focus_days,
        BTreeMap::from([("Rhineland".to_string(), 70)])
    );
}

#[tokio::test]
async fn saving_drops_a_term_for_a_focus_not_in_the_order() {
    let plan = marked(&["Rhineland"], &[("Anschluss", 14)]);
    let stored = save_plan(&pool().await, "Marked", &plan).await.unwrap();
    assert!(stored.focus_days.is_empty());
}

#[tokio::test]
async fn saving_the_same_name_revises_one_plan() {
    let pool = pool().await;
    let first = save_plan(&pool, "My plan", &fields(&["Rhineland"]))
        .await
        .unwrap();
    let mut revised = fields(&["Anschluss", "Rhineland"]);
    revised.start_date = Some("1937-01-01".to_string());
    let second = save_plan(&pool, "My plan", &revised).await.unwrap();

    assert_eq!(saved_ids(&pool).await.len(), 1);
    assert_eq!(second.focus_ids, ["Anschluss", "Rhineland"]);
    assert_eq!(second.start_date, "1937-01-01");
    // An overwrite is the same plan revised, so it keeps its birthday.
    assert_eq!(second.created_at, first.created_at);
    assert!(second.updated_at > first.updated_at);
}

#[tokio::test]
async fn a_name_with_nothing_to_slug_is_refused() {
    let pool = pool().await;
    let refused = save_plan(&pool, "!!!", &fields(&["Rhineland"])).await;
    assert!(matches!(refused, Err(AppError::PlanNameUnusable)));
    assert!(saved_ids(&pool).await.is_empty());
}

#[tokio::test]
async fn plans_come_back_most_recently_touched_first() {
    let pool = pool().await;
    save_plan(&pool, "First", &fields(&[])).await.unwrap();
    save_plan(&pool, "Second", &fields(&[])).await.unwrap();
    assert_eq!(saved_ids(&pool).await, ["second", "first"]);

    save_plan(&pool, "First", &fields(&[])).await.unwrap();
    assert_eq!(saved_ids(&pool).await, ["first", "second"]);
}

#[tokio::test]
async fn find_gets_a_plan_by_its_slug_or_nothing() {
    let pool = pool().await;
    save_plan(&pool, "Barbarossa rush", &fields(&["Anschluss"]))
        .await
        .unwrap();

    let found = PlanRecord::find(&pool, "barbarossa-rush")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.focus_ids, ["Anschluss"]);
    assert!(PlanRecord::find(&pool, "nope").await.unwrap().is_none());
}

#[tokio::test]
async fn delete_says_whether_there_was_one() {
    let pool = pool().await;
    save_plan(&pool, "Doomed", &fields(&[])).await.unwrap();

    assert!(PlanRecord::delete(&pool, "doomed").await.unwrap());
    assert!(!PlanRecord::delete(&pool, "doomed").await.unwrap());
    assert!(saved_ids(&pool).await.is_empty());
}

#[tokio::test]
async fn a_plan_may_hold_a_focus_the_catalog_lost() {
    // Nothing joins the two tables: a re-import must not fail a save.
    let plan = save_plan(&pool().await, "Stale", &fields(&["gone_focus"]))
        .await
        .unwrap();
    assert_eq!(plan.focus_ids, ["gone_focus"]);
}

#[tokio::test]
async fn the_draft_is_stored_without_a_name_to_slug() {
    let pool = pool().await;
    let draft = save_draft(&pool, "", &fields(&["Rhineland"]))
        .await
        .unwrap();

    assert_eq!(draft.id, DRAFT_PLAN_ID);
    assert_eq!(
        load_draft(&pool).await.unwrap().unwrap().focus_ids,
        ["Rhineland"]
    );
}

#[tokio::test]
async fn the_draft_is_one_plan_however_often_it_is_written() {
    let pool = pool().await;
    let first = save_draft(&pool, "", &fields(&["Rhineland"]))
        .await
        .unwrap();
    let second = save_draft(&pool, "Half named", &fields(&["Rhineland", "Anschluss"]))
        .await
        .unwrap();

    assert_eq!(second.focus_ids, ["Rhineland", "Anschluss"]);
    assert_eq!(second.name, "Half named");
    assert_eq!(second.created_at, first.created_at);
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plans")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn the_draft_keeps_the_terms_of_the_focuses_it_holds() {
    let plan = marked(&["Rhineland"], &[("Rhineland", 70), ("Gone", 14)]);
    let draft = save_draft(&pool().await, "", &plan).await.unwrap();
    assert_eq!(
        draft.focus_days,
        BTreeMap::from([("Rhineland".to_string(), 70)])
    );
}

#[tokio::test]
async fn the_draft_is_not_offered_among_the_saved_plans() {
    let pool = pool().await;
    save_plan(&pool, "Barbarossa rush", &fields(&["Anschluss"]))
        .await
        .unwrap();
    save_draft(&pool, "", &fields(&["Rhineland"]))
        .await
        .unwrap();
    assert_eq!(saved_ids(&pool).await, ["barbarossa-rush"]);
}

#[tokio::test]
async fn a_plan_named_draft_is_not_the_draft() {
    let pool = pool().await;
    save_draft(&pool, "", &fields(&["Rhineland"]))
        .await
        .unwrap();
    save_plan(&pool, "Draft", &fields(&["Anschluss"]))
        .await
        .unwrap();

    assert_eq!(
        load_draft(&pool).await.unwrap().unwrap().focus_ids,
        ["Rhineland"]
    );
    let plan = PlanRecord::find(&pool, "draft").await.unwrap().unwrap();
    assert_eq!(plan.focus_ids, ["Anschluss"]);
}

#[tokio::test]
async fn there_is_no_draft_before_anything_is_built() {
    assert!(load_draft(&pool().await).await.unwrap().is_none());
}

#[tokio::test]
async fn clearing_the_draft_says_whether_there_was_one() {
    let pool = pool().await;
    save_draft(&pool, "", &fields(&["Rhineland"]))
        .await
        .unwrap();

    assert!(clear_draft(&pool).await.unwrap());
    assert!(!clear_draft(&pool).await.unwrap());
    assert!(load_draft(&pool).await.unwrap().is_none());
}

#[tokio::test]
async fn clearing_the_draft_leaves_the_saved_plans_alone() {
    let pool = pool().await;
    save_plan(&pool, "Barbarossa rush", &fields(&["Anschluss"]))
        .await
        .unwrap();
    save_draft(&pool, "", &fields(&["Rhineland"]))
        .await
        .unwrap();

    clear_draft(&pool).await.unwrap();
    assert_eq!(saved_ids(&pool).await, ["barbarossa-rush"]);
}

#[tokio::test]
async fn a_database_file_keeps_what_it_holds_and_gets_its_folder_made() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deep/down/plans.db");
    let url = path.display().to_string();

    let pool = db::connect(&url).await.unwrap();
    save_plan(&pool, "Kept", &fields(&["Anschluss"]))
        .await
        .unwrap();
    pool.close().await;

    let reopened = db::connect(&format!("sqlite://{url}")).await.unwrap();
    assert_eq!(saved_ids(&reopened).await, ["kept"]);
    reopened.close().await;
}

fn by_focus(attached: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    attached
        .iter()
        .map(|(focus_id, ids)| {
            let ids = ids.iter().map(ToString::to_string).collect();
            (focus_id.to_string(), ids)
        })
        .collect()
}

fn attached(focus_ids: &[&str], attached: &[(&str, &[&str])]) -> PlanFields {
    PlanFields {
        focus_technologies: by_focus(attached),
        ..fields(focus_ids)
    }
}

fn hired(focus_ids: &[&str], hired: &[(&str, &[&str])]) -> PlanFields {
    PlanFields {
        focus_staff: by_focus(hired),
        ..fields(focus_ids)
    }
}

fn commented(focus_ids: &[&str], comments: &[(&str, &[&str])]) -> PlanFields {
    PlanFields {
        focus_comments: by_focus(comments),
        ..fields(focus_ids)
    }
}

#[tokio::test]
async fn a_plan_keeps_its_comments_by_focus_in_their_order() {
    let pool = pool().await;
    let plan = commented(&["A"], &[("A", &["puis l'Autriche", "garder 2 PP"])]);
    save_plan(&pool, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&pool, "rush").await.unwrap().unwrap();
    assert_eq!(
        stored.focus_comments["A"],
        ["puis l'Autriche", "garder 2 PP"]
    );
}

#[tokio::test]
async fn comments_of_a_focus_not_in_the_order_are_not_stored() {
    let pool = pool().await;
    let plan = commented(&["A"], &[("A", &["note"]), ("Gone", &["lost"]), ("B", &[])]);
    let stored = save_draft(&pool, "", &plan).await.unwrap();

    assert_eq!(stored.focus_comments.len(), 1);
    assert_eq!(stored.focus_comments["A"], ["note"]);
}

#[tokio::test]
async fn a_plan_keeps_its_technologies_in_their_order() {
    let pool = pool().await;
    let plan = attached(&["A"], &[("A", &["T2", "T1"])]);
    save_plan(&pool, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&pool, "rush").await.unwrap().unwrap();
    assert_eq!(stored.focus_technologies["A"], ["T2", "T1"]);
}

#[tokio::test]
async fn technologies_of_a_focus_not_in_the_order_are_not_stored() {
    let pool = pool().await;
    let plan = attached(&["A"], &[("A", &["T1"]), ("Gone", &["T2"]), ("B", &[])]);
    let stored = save_draft(&pool, "", &plan).await.unwrap();

    assert_eq!(stored.focus_technologies.len(), 1);
    assert_eq!(stored.focus_technologies["A"], ["T1"]);
}

#[tokio::test]
async fn a_plan_written_before_technologies_reads_as_having_none() {
    let pool = pool().await;
    sqlx::query(
        "INSERT INTO plans (id, name, focus_ids, start_date, days_per_focus, focus_days, created_at, updated_at)
         VALUES ('old', 'Old', '[\"A\"]', '1936-01-01', 35, '{}', 'x', 'x')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let stored = PlanRecord::find(&pool, "old").await.unwrap().unwrap();
    assert!(stored.focus_technologies.is_empty());
    assert!(stored.focus_staff.is_empty());
    assert!(stored.focus_comments.is_empty());
}

#[tokio::test]
async fn replacing_the_technologies_stores_them_in_order_and_says_how_many() {
    let pool = pool().await;
    assert_eq!(
        TechnologyRecord::replace_all(&pool, &technologies())
            .await
            .unwrap(),
        3
    );

    let stored = TechnologyRecord::list(&pool).await.unwrap();
    assert_eq!(stored, technologies());
    assert_eq!(TechnologyRecord::count(&pool).await.unwrap(), 3);
}

#[tokio::test]
async fn replacing_the_technologies_loses_what_the_pages_no_longer_have() {
    let pool = pool().await;
    TechnologyRecord::replace_all(&pool, &technologies())
        .await
        .unwrap();
    TechnologyRecord::replace_all(&pool, &technologies()[..1])
        .await
        .unwrap();
    assert_eq!(TechnologyRecord::count(&pool).await.unwrap(), 1);
}

#[tokio::test]
async fn a_plan_keeps_its_staff_by_focus_in_their_order() {
    let pool = pool().await;
    let plan = hired(&["A"], &[("A", &["Krupp", "Heinz_Guderian__Theorists"])]);
    save_plan(&pool, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&pool, "rush").await.unwrap().unwrap();
    assert_eq!(
        stored.focus_staff["A"],
        ["Krupp", "Heinz_Guderian__Theorists"]
    );
}

#[tokio::test]
async fn staff_of_a_focus_not_in_the_order_are_not_stored() {
    let pool = pool().await;
    let plan = hired(
        &["A"],
        &[("A", &["Krupp"]), ("Gone", &["Franz_Halder"]), ("B", &[])],
    );
    let stored = save_draft(&pool, "", &plan).await.unwrap();

    assert_eq!(stored.focus_staff.len(), 1);
    assert_eq!(stored.focus_staff["A"], ["Krupp"]);
}

#[tokio::test]
async fn replacing_the_staff_stores_them_in_order_and_says_how_many() {
    let pool = pool().await;
    assert_eq!(StaffRecord::replace_all(&pool, &staff()).await.unwrap(), 3);

    let mut expected = staff();
    expected.sort_by_key(|member| member.position);
    assert_eq!(StaffRecord::list(&pool).await.unwrap(), expected);
}

#[tokio::test]
async fn replacing_the_staff_loses_what_the_page_no_longer_has() {
    let pool = pool().await;
    StaffRecord::replace_all(&pool, &staff()).await.unwrap();
    StaffRecord::replace_all(&pool, &staff()[..1])
        .await
        .unwrap();
    assert_eq!(StaffRecord::count(&pool).await.unwrap(), 1);
}
