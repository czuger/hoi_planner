use sqlx::SqlitePool;

use crate::config::DataFiles;
use crate::db::{self, FocusRecord};
use crate::domain::Focus;
use crate::error::AppError;
use crate::service::catalog_import::{CatalogSource, ImportOptions, ImportOutcome};
use crate::service::data_file::{read_file, write_json};
use crate::wiki::parse_wiki_page;

/// Reads the catalog and fills a database with it.
///
/// # Errors
/// Fails when a file is missing or unreadable, the page holds no focus, or the database fails.
pub async fn import_focuses(
    files: &DataFiles,
    options: ImportOptions<'_>,
) -> Result<ImportOutcome, AppError> {
    let focuses = read_focuses(files, options.source)?;
    if focuses.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucun focus",
        });
    }
    if options.source == CatalogSource::WikiPage {
        write_json(&files.focuses_json, &focuses)?;
    }
    let stored = match options.database_url {
        Some(url) => Some(store(&db::connect(url).await?, &focuses).await?),
        None => None,
    };
    Ok(ImportOutcome {
        parsed: focuses.len(),
        stored,
    })
}

async fn store(pool: &SqlitePool, focuses: &[Focus]) -> Result<i64, AppError> {
    let stored = FocusRecord::replace_all(pool, focuses).await;
    pool.close().await;
    stored
}

fn read_focuses(files: &DataFiles, source: CatalogSource) -> Result<Vec<Focus>, AppError> {
    match source {
        CatalogSource::Json => {
            let text = read_file(&files.focuses_json)?;
            serde_json::from_str(&text).map_err(|source| AppError::InvalidCatalogFile {
                path: files.focuses_json.display().to_string(),
                source,
            })
        }
        CatalogSource::WikiPage => Ok(parse_wiki_page(&read_file(&files.wiki_page)?)),
    }
}
