//! Every read and write of the plans, against a database in a temporary folder.

mod common;

use std::collections::BTreeMap;

use hoi4_planner::db::{PlanRecord, Store};
use hoi4_planner::domain::{DRAFT_PLAN_ID, PlanFields};
use hoi4_planner::error::AppError;
use hoi4_planner::service::{clear_draft, load_draft, save_draft, save_plan};

use common::temp_store::TempStore;

fn store() -> TempStore {
    TempStore::new()
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

async fn saved_ids(store: &Store) -> Vec<String> {
    PlanRecord::list_saved(store)
        .await
        .unwrap()
        .into_iter()
        .map(|plan| plan.id)
        .collect()
}

#[tokio::test]
async fn saving_stores_the_order_under_a_slug() {
    let store = store();
    let plan = save_plan(
        &store,
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
    let plan = save_plan(&store(), "  Rush  ", &fields(&[])).await.unwrap();
    assert_eq!(plan.name, "Rush");
}

#[tokio::test]
async fn a_plan_with_no_start_date_starts_when_the_game_does() {
    let plan = save_plan(&store(), "Rush", &PlanFields::empty())
        .await
        .unwrap();
    assert_eq!(plan.start_date, "1936-01-01");
}

#[tokio::test]
async fn saving_keeps_the_terms_of_the_focuses_it_holds() {
    let plan = marked(&["Rhineland", "Anschluss"], &[("Rhineland", 70)]);
    let stored = save_plan(&store(), "Marked", &plan).await.unwrap();
    assert_eq!(
        stored.focus_days,
        BTreeMap::from([("Rhineland".to_string(), 70)])
    );
}

#[tokio::test]
async fn saving_drops_a_term_for_a_focus_not_in_the_order() {
    let plan = marked(&["Rhineland"], &[("Anschluss", 14)]);
    let stored = save_plan(&store(), "Marked", &plan).await.unwrap();
    assert!(stored.focus_days.is_empty());
}

#[tokio::test]
async fn saving_the_same_name_revises_one_plan() {
    let store = store();
    let first = save_plan(&store, "My plan", &fields(&["Rhineland"]))
        .await
        .unwrap();
    let mut revised = fields(&["Anschluss", "Rhineland"]);
    revised.start_date = Some("1937-01-01".to_string());
    let second = save_plan(&store, "My plan", &revised).await.unwrap();

    assert_eq!(saved_ids(&store).await.len(), 1);
    assert_eq!(second.focus_ids, ["Anschluss", "Rhineland"]);
    assert_eq!(second.start_date, "1937-01-01");
    // An overwrite is the same plan revised, so it keeps its birthday.
    assert_eq!(second.created_at, first.created_at);
    assert!(second.updated_at > first.updated_at);
}

#[tokio::test]
async fn a_name_with_nothing_to_slug_is_refused() {
    let store = store();
    let refused = save_plan(&store, "!!!", &fields(&["Rhineland"])).await;
    assert!(matches!(refused, Err(AppError::PlanNameUnusable)));
    assert!(saved_ids(&store).await.is_empty());
}

#[tokio::test]
async fn plans_come_back_most_recently_touched_first() {
    let store = store();
    save_plan(&store, "First", &fields(&[])).await.unwrap();
    save_plan(&store, "Second", &fields(&[])).await.unwrap();
    assert_eq!(saved_ids(&store).await, ["second", "first"]);

    save_plan(&store, "First", &fields(&[])).await.unwrap();
    assert_eq!(saved_ids(&store).await, ["first", "second"]);
}

#[tokio::test]
async fn find_gets_a_plan_by_its_slug_or_nothing() {
    let store = store();
    save_plan(&store, "Barbarossa rush", &fields(&["Anschluss"]))
        .await
        .unwrap();

    let found = PlanRecord::find(&store, "barbarossa-rush")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.focus_ids, ["Anschluss"]);
    assert!(PlanRecord::find(&store, "nope").await.unwrap().is_none());
}

#[tokio::test]
async fn delete_says_whether_there_was_one() {
    let store = store();
    save_plan(&store, "Doomed", &fields(&[])).await.unwrap();

    assert!(PlanRecord::delete(&store, "doomed").await.unwrap());
    assert!(!PlanRecord::delete(&store, "doomed").await.unwrap());
    assert!(saved_ids(&store).await.is_empty());
}

#[tokio::test]
async fn a_plan_may_hold_a_focus_the_catalog_lost() {
    // Nothing joins the two tables: a re-import must not fail a save.
    let plan = save_plan(&store(), "Stale", &fields(&["gone_focus"]))
        .await
        .unwrap();
    assert_eq!(plan.focus_ids, ["gone_focus"]);
}

#[tokio::test]
async fn the_draft_is_stored_without_a_name_to_slug() {
    let store = store();
    let draft = save_draft(&store, "", &fields(&["Rhineland"]))
        .await
        .unwrap();

    assert_eq!(draft.id, DRAFT_PLAN_ID);
    assert_eq!(
        load_draft(&store).await.unwrap().unwrap().focus_ids,
        ["Rhineland"]
    );
}

#[tokio::test]
async fn the_draft_is_one_plan_however_often_it_is_written() {
    let store = store();
    let first = save_draft(&store, "", &fields(&["Rhineland"]))
        .await
        .unwrap();
    let second = save_draft(&store, "Half named", &fields(&["Rhineland", "Anschluss"]))
        .await
        .unwrap();

    assert_eq!(second.focus_ids, ["Rhineland", "Anschluss"]);
    assert_eq!(second.name, "Half named");
    assert_eq!(second.created_at, first.created_at);
    assert_eq!(load_draft(&store).await.unwrap(), Some(second));
    assert!(saved_ids(&store).await.is_empty());
}

#[tokio::test]
async fn the_draft_keeps_the_terms_of_the_focuses_it_holds() {
    let plan = marked(&["Rhineland"], &[("Rhineland", 70), ("Gone", 14)]);
    let draft = save_draft(&store(), "", &plan).await.unwrap();
    assert_eq!(
        draft.focus_days,
        BTreeMap::from([("Rhineland".to_string(), 70)])
    );
}

#[tokio::test]
async fn the_draft_is_not_offered_among_the_saved_plans() {
    let store = store();
    save_plan(&store, "Barbarossa rush", &fields(&["Anschluss"]))
        .await
        .unwrap();
    save_draft(&store, "", &fields(&["Rhineland"]))
        .await
        .unwrap();
    assert_eq!(saved_ids(&store).await, ["barbarossa-rush"]);
}

#[tokio::test]
async fn a_plan_named_draft_is_not_the_draft() {
    let store = store();
    save_draft(&store, "", &fields(&["Rhineland"]))
        .await
        .unwrap();
    save_plan(&store, "Draft", &fields(&["Anschluss"]))
        .await
        .unwrap();

    assert_eq!(
        load_draft(&store).await.unwrap().unwrap().focus_ids,
        ["Rhineland"]
    );
    let plan = PlanRecord::find(&store, "draft").await.unwrap().unwrap();
    assert_eq!(plan.focus_ids, ["Anschluss"]);
}

#[tokio::test]
async fn there_is_no_draft_before_anything_is_built() {
    assert!(load_draft(&store()).await.unwrap().is_none());
}

#[tokio::test]
async fn clearing_the_draft_says_whether_there_was_one() {
    let store = store();
    save_draft(&store, "", &fields(&["Rhineland"]))
        .await
        .unwrap();

    assert!(clear_draft(&store).await.unwrap());
    assert!(!clear_draft(&store).await.unwrap());
    assert!(load_draft(&store).await.unwrap().is_none());
}

#[tokio::test]
async fn clearing_the_draft_leaves_the_saved_plans_alone() {
    let store = store();
    save_plan(&store, "Barbarossa rush", &fields(&["Anschluss"]))
        .await
        .unwrap();
    save_draft(&store, "", &fields(&["Rhineland"]))
        .await
        .unwrap();

    clear_draft(&store).await.unwrap();
    assert_eq!(saved_ids(&store).await, ["barbarossa-rush"]);
}

#[tokio::test]
async fn a_database_folder_keeps_what_it_holds_and_gets_its_parents_made() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deep/down/plans.polodb");

    let store = Store::open(&path).unwrap();
    let kept = save_plan(
        &store,
        "Kept",
        &marked(&["Anschluss"], &[("Anschluss", 70)]),
    )
    .await
    .unwrap();
    store.close();

    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        PlanRecord::find(&reopened, "kept").await.unwrap(),
        Some(kept)
    );
    reopened.close();
}

#[tokio::test]
async fn a_closed_database_fails_as_a_database_error() {
    let store = store();
    store.close();

    let error = save_plan(&store, "Rush", &fields(&[])).await.unwrap_err();
    assert!(error.is_database());
    assert!(store.ping().await.is_err());
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
    let store = store();
    let plan = commented(&["A"], &[("A", &["puis l'Autriche", "garder 2 PP"])]);
    save_plan(&store, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&store, "rush").await.unwrap().unwrap();
    assert_eq!(
        stored.focus_comments["A"],
        ["puis l'Autriche", "garder 2 PP"]
    );
}

#[tokio::test]
async fn comments_of_a_focus_not_in_the_order_are_not_stored() {
    let store = store();
    let plan = commented(&["A"], &[("A", &["note"]), ("Gone", &["lost"]), ("B", &[])]);
    let stored = save_draft(&store, "", &plan).await.unwrap();

    assert_eq!(stored.focus_comments.len(), 1);
    assert_eq!(stored.focus_comments["A"], ["note"]);
}

#[tokio::test]
async fn a_plan_keeps_its_technologies_in_their_order() {
    let store = store();
    let plan = attached(&["A"], &[("A", &["T2", "T1"])]);
    save_plan(&store, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&store, "rush").await.unwrap().unwrap();
    assert_eq!(stored.focus_technologies["A"], ["T2", "T1"]);
}

#[tokio::test]
async fn technologies_of_a_focus_not_in_the_order_are_not_stored() {
    let store = store();
    let plan = attached(&["A"], &[("A", &["T1"]), ("Gone", &["T2"]), ("B", &[])]);
    let stored = save_draft(&store, "", &plan).await.unwrap();

    assert_eq!(stored.focus_technologies.len(), 1);
    assert_eq!(stored.focus_technologies["A"], ["T1"]);
}

#[tokio::test]
async fn a_plan_keeps_its_staff_by_focus_in_their_order() {
    let store = store();
    let plan = hired(&["A"], &[("A", &["Krupp", "Heinz_Guderian__Theorists"])]);
    save_plan(&store, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&store, "rush").await.unwrap().unwrap();
    assert_eq!(
        stored.focus_staff["A"],
        ["Krupp", "Heinz_Guderian__Theorists"]
    );
}

#[tokio::test]
async fn staff_of_a_focus_not_in_the_order_are_not_stored() {
    let store = store();
    let plan = hired(
        &["A"],
        &[("A", &["Krupp"]), ("Gone", &["Franz_Halder"]), ("B", &[])],
    );
    let stored = save_draft(&store, "", &plan).await.unwrap();

    assert_eq!(stored.focus_staff.len(), 1);
    assert_eq!(stored.focus_staff["A"], ["Krupp"]);
}

#[tokio::test]
async fn ids_holding_dots_and_dollars_are_stored_as_they_are() {
    let store = store();
    let plan = PlanFields {
        focus_staff: by_focus(&[("W.A", &["W._von_Richthofen"])]),
        focus_comments: by_focus(&[("$B", &["a.b $c"])]),
        ..marked(&["W.A", "$B"], &[("W.A", 14)])
    };
    save_plan(&store, "Rush", &plan).await.unwrap();

    let stored = PlanRecord::find(&store, "rush").await.unwrap().unwrap();
    assert_eq!(stored.focus_days["W.A"], 14);
    assert_eq!(stored.focus_staff["W.A"], ["W._von_Richthofen"]);
    assert_eq!(stored.focus_comments["$B"], ["a.b $c"]);
}
