//! The commands reading the saved pages: page -> JSON -> database.

mod common;

use std::fs;

use common::politics_markup::{
    country_leaders, default_leader, named_staff, staff_section, staff_table,
};
use common::technology_markup::{heading, link_to, row, wikitable};
use common::wiki_markup::{plain_row, wiki_heading};
use hoi4_planner::config::DataFiles;
use hoi4_planner::db::{self, FocusRecord, StaffRecord, TechnologyRecord};
use hoi4_planner::error::AppError;
use hoi4_planner::service::{
    CatalogSource, ImportOptions, import_focuses, import_staff, import_technologies, parse_politics,
};
use serde_json::Value;
use tempfile::TempDir;

struct Workspace {
    dir: TempDir,
    files: DataFiles,
    database_url: String,
}

fn workspace() -> Workspace {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("data")).unwrap();
    let files = DataFiles::under(dir.path());
    let database_url = dir.path().join("test.db").display().to_string();
    Workspace {
        dir,
        files,
        database_url,
    }
}

impl Workspace {
    fn options(&self, source: CatalogSource) -> ImportOptions<'_> {
        ImportOptions {
            source,
            database_url: Some(&self.database_url),
        }
    }

    async fn stored_ids(&self) -> Vec<String> {
        let pool = db::connect(&self.database_url).await.unwrap();
        let focuses = FocusRecord::list(&pool).await.unwrap();
        pool.close().await;
        focuses.into_iter().map(|focus| focus.id).collect()
    }
}

fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[tokio::test]
async fn parses_the_page_and_seeds_the_database() {
    let work = workspace();
    let page = wiki_heading("Rhineland Branch") + &plain_row("A", "Alpha");
    fs::write(&work.files.wiki_page, page).unwrap();

    let outcome = import_focuses(&work.files, work.options(CatalogSource::WikiPage))
        .await
        .unwrap();

    assert_eq!((outcome.parsed, outcome.stored), (1, Some(1)));
    assert_eq!(work.stored_ids().await, ["A"]);
    // The JSON is written on the way through, for --from-json later.
    assert_eq!(read_json(&work.files.focuses_json)[0]["id"], "A");
}

#[tokio::test]
async fn json_only_leaves_the_database_alone() {
    let work = workspace();
    fs::write(&work.files.wiki_page, plain_row("A", "Alpha")).unwrap();
    let options = ImportOptions {
        source: CatalogSource::WikiPage,
        database_url: None,
    };

    let outcome = import_focuses(&work.files, options).await.unwrap();

    assert_eq!(outcome.stored, None);
    assert!(work.files.focuses_json.exists());
    assert!(!work.dir.path().join("test.db").exists());
}

#[tokio::test]
async fn from_json_seeds_without_reading_the_page() {
    let work = workspace();
    let catalog = r#"[{"id": "Anschluss", "name": "Anschluss", "icon": "https://x/i.png",
                       "icon_page": "https://x/F", "section": "S", "position": 0}]"#;
    fs::write(&work.files.focuses_json, catalog).unwrap();

    import_focuses(&work.files, work.options(CatalogSource::Json))
        .await
        .unwrap();

    assert_eq!(work.stored_ids().await, ["Anschluss"]);
    // The page was never there, and was never needed.
    assert!(!work.files.wiki_page.exists());
}

#[tokio::test]
async fn from_json_does_not_rewrite_the_json() {
    let work = workspace();
    let catalog = "[{\"id\":\"A\",\"name\":\"A\",\"icon\":\"i\",\"icon_page\":\"p\",\"section\":\"S\",\"position\":0}]";
    fs::write(&work.files.focuses_json, catalog).unwrap();

    import_focuses(&work.files, work.options(CatalogSource::Json))
        .await
        .unwrap();

    assert_eq!(
        fs::read_to_string(&work.files.focuses_json).unwrap(),
        catalog
    );
}

#[tokio::test]
async fn a_re_import_leaves_the_saved_plans_alone() {
    let work = workspace();
    fs::write(&work.files.wiki_page, plain_row("A", "Alpha")).unwrap();
    let pool = db::connect(&work.database_url).await.unwrap();
    let fields = hoi4_planner::domain::PlanFields::empty();
    hoi4_planner::service::save_plan(&pool, "Kept", &fields)
        .await
        .unwrap();
    pool.close().await;

    import_focuses(&work.files, work.options(CatalogSource::WikiPage))
        .await
        .unwrap();

    let pool = db::connect(&work.database_url).await.unwrap();
    let plans = hoi4_planner::db::PlanRecord::list_saved(&pool)
        .await
        .unwrap();
    pool.close().await;
    assert_eq!(plans[0].id, "kept");
}

#[tokio::test]
async fn a_missing_page_says_so() {
    let work = workspace();
    let error = import_focuses(&work.files, work.options(CatalogSource::WikiPage))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("focuses.html"));
}

#[tokio::test]
async fn a_missing_json_says_so() {
    let work = workspace();
    let error = import_focuses(&work.files, work.options(CatalogSource::Json))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("focuses.json"));
}

#[tokio::test]
async fn an_unreadable_json_says_so() {
    let work = workspace();
    fs::write(&work.files.focuses_json, "{nope").unwrap();
    let error = import_focuses(&work.files, work.options(CatalogSource::Json))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::InvalidCatalogFile { .. }));
}

#[tokio::test]
async fn a_page_with_no_focus_is_an_error() {
    let work = workspace();
    fs::write(&work.files.wiki_page, "<p>nothing</p>").unwrap();
    let error = import_focuses(&work.files, work.options(CatalogSource::WikiPage))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!work.files.focuses_json.exists());
}

fn staff_page() -> String {
    staff_section(&[staff_table(
        "Generals",
        "Portrait",
        &[named_staff("Franz Halder")],
    )])
}

#[test]
fn the_politics_command_writes_both_files() {
    let work = workspace();
    let page = country_leaders(&[default_leader()]) + &staff_page();
    fs::write(&work.files.politics_page, page).unwrap();

    let counts = parse_politics(&work.files).unwrap();

    assert_eq!((counts.leaders, counts.staff), (1, 1));
    assert_eq!(read_json(&work.files.leaders_json)[0]["id"], "Adolf_Hitler");
    assert_eq!(read_json(&work.files.staff_json)[0]["section"], "Generals");
}

#[test]
fn a_missing_country_page_is_an_error() {
    let work = workspace();
    let error = parse_politics(&work.files).unwrap_err();
    assert!(error.to_string().contains("politics.html"));
}

#[test]
fn a_page_with_nothing_to_read_writes_nothing() {
    let work = workspace();
    fs::write(
        &work.files.politics_page,
        country_leaders(&[default_leader()]),
    )
    .unwrap();

    let error = parse_politics(&work.files).unwrap_err();

    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!work.files.staff_json.exists());
    assert!(!work.files.leaders_json.exists());
}

/// Every research page saved, the first holding `first` and the rest nothing to read.
fn save_research_pages(workspace: &Workspace, first: &str) {
    fs::create_dir(workspace.dir.path().join("data/technologies")).unwrap();
    for (index, page) in workspace.files.technology_pages.iter().enumerate() {
        fs::write(&page.path, if index == 0 { first } else { "" }).unwrap();
    }
}

fn a_research_page() -> String {
    heading(2, "Construction")
        + &wikitable(&[
            row("Construction I", 1936, "–"),
            row("Construction II", 1937, &link_to("Construction_I")),
        ])
}

async fn stored_technologies(workspace: &Workspace) -> Vec<String> {
    let pool = db::connect(&workspace.database_url).await.unwrap();
    let technologies = TechnologyRecord::list(&pool).await.unwrap();
    pool.close().await;
    technologies
        .into_iter()
        .map(|technology| technology.id)
        .collect()
}

#[tokio::test]
async fn the_technology_import_goes_from_the_pages_to_the_json_to_the_database() {
    let workspace = workspace();
    save_research_pages(&workspace, &a_research_page());

    let outcome = import_technologies(&workspace.files, workspace.options(CatalogSource::WikiPage))
        .await
        .unwrap();

    assert_eq!(outcome.parsed, 2);
    assert_eq!(outcome.stored, Some(2));
    assert_eq!(
        stored_technologies(&workspace).await,
        ["Construction_I", "Construction_II"]
    );
    let json: Value =
        serde_json::from_str(&fs::read_to_string(&workspace.files.technologies_json).unwrap())
            .unwrap();
    assert_eq!(json[0]["domain"], "Infanterie");
    assert_eq!(json[1]["requires"][0][0], "Construction_I");
}

#[tokio::test]
async fn the_technology_import_can_seed_from_the_json_alone() {
    let workspace = workspace();
    save_research_pages(&workspace, &a_research_page());
    let json_only = ImportOptions {
        source: CatalogSource::WikiPage,
        database_url: None,
    };
    let outcome = import_technologies(&workspace.files, json_only)
        .await
        .unwrap();
    assert_eq!(outcome.stored, None);
    for page in &workspace.files.technology_pages {
        fs::remove_file(&page.path).unwrap();
    }

    let outcome = import_technologies(&workspace.files, workspace.options(CatalogSource::Json))
        .await
        .unwrap();
    assert_eq!(outcome.stored, Some(2));
}

#[tokio::test]
async fn pages_with_no_technology_change_nothing() {
    let workspace = workspace();
    save_research_pages(&workspace, "<p>no table here</p>");
    let error = import_technologies(&workspace.files, workspace.options(CatalogSource::WikiPage))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!workspace.files.technologies_json.exists());
}

#[tokio::test]
async fn a_missing_research_page_is_an_error() {
    let workspace = workspace();
    let error = import_technologies(&workspace.files, workspace.options(CatalogSource::WikiPage))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::FileRead { .. }));
}

#[tokio::test]
async fn an_unreadable_technology_json_is_an_error() {
    let workspace = workspace();
    fs::write(&workspace.files.technologies_json, "{nope").unwrap();
    let error = import_technologies(&workspace.files, workspace.options(CatalogSource::Json))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::InvalidCatalogFile { .. }));
}

async fn stored_staff(workspace: &Workspace) -> Vec<String> {
    let pool = db::connect(&workspace.database_url).await.unwrap();
    let staff = StaffRecord::list(&pool).await.unwrap();
    pool.close().await;
    staff.into_iter().map(|member| member.id).collect()
}

#[tokio::test]
async fn the_staff_import_goes_from_the_page_to_the_json_to_the_database() {
    let work = workspace();
    fs::write(&work.files.politics_page, staff_page()).unwrap();

    let outcome = import_staff(&work.files, work.options(CatalogSource::WikiPage))
        .await
        .unwrap();

    assert_eq!((outcome.parsed, outcome.stored), (1, Some(1)));
    assert_eq!(stored_staff(&work).await, ["Franz_Halder"]);
    // The same file parse-politics writes, so that either leaves `git diff data/` empty.
    assert_eq!(read_json(&work.files.staff_json)[0]["section"], "Generals");
}

#[tokio::test]
async fn the_staff_import_can_seed_from_the_json_alone() {
    let work = workspace();
    fs::write(&work.files.politics_page, staff_page()).unwrap();
    let json_only = ImportOptions {
        source: CatalogSource::WikiPage,
        database_url: None,
    };
    import_staff(&work.files, json_only).await.unwrap();
    fs::remove_file(&work.files.politics_page).unwrap();

    let outcome = import_staff(&work.files, work.options(CatalogSource::Json))
        .await
        .unwrap();
    assert_eq!(outcome.stored, Some(1));
    assert_eq!(stored_staff(&work).await, ["Franz_Halder"]);
}

#[tokio::test]
async fn a_page_with_no_staff_changes_nothing() {
    let work = workspace();
    fs::write(
        &work.files.politics_page,
        country_leaders(&[default_leader()]),
    )
    .unwrap();

    let error = import_staff(&work.files, work.options(CatalogSource::WikiPage))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NothingParsed { .. }));
    assert!(!work.files.staff_json.exists());
}

#[tokio::test]
async fn the_bundled_catalogs_seed_a_database_with_no_file_around() {
    // A deployed server holds the binary and nothing else: no page, no JSON.
    let work = workspace();
    let source = CatalogSource::Bundled;

    let focuses = import_focuses(&work.files, work.options(source))
        .await
        .unwrap();
    let technologies = import_technologies(&work.files, work.options(source))
        .await
        .unwrap();
    let staff = import_staff(&work.files, work.options(source))
        .await
        .unwrap();

    for outcome in [focuses, technologies, staff] {
        assert!(outcome.parsed > 0);
        assert_eq!(outcome.stored, Some(outcome.parsed as i64));
    }
    assert_eq!(
        fs::read_dir(work.dir.path().join("data")).unwrap().count(),
        0
    );
}
