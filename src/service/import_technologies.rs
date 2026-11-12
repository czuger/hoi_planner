use sqlx::SqlitePool;

use crate::config::DataFiles;
use crate::db::{self, TechnologyRecord};
use crate::domain::Technology;
use crate::error::AppError;
use crate::service::bundled_catalog::BUNDLED_TECHNOLOGIES;
use crate::service::catalog_import::{CatalogSource, ImportOptions, ImportOutcome};
use crate::service::data_file::{read_file, write_json};
use crate::wiki::{TechnologyPageSource, parse_technology_pages};

/// Reads the research tree and fills a database with it.
///
/// # Errors
/// Fails when a file is missing or unreadable, the pages hold no technology, or the database
/// fails.
pub async fn import_technologies(
    files: &DataFiles,
    options: ImportOptions<'_>,
) -> Result<ImportOutcome, AppError> {
    let technologies = read_technologies(files, options.source)?;
    if technologies.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucune technologie",
        });
    }
    if options.source == CatalogSource::WikiPage {
        write_json(&files.technologies_json, &technologies)?;
    }
    let stored = match options.database_url {
        Some(url) => Some(store(&db::connect(url).await?, &technologies).await?),
        None => None,
    };
    Ok(ImportOutcome {
        parsed: technologies.len(),
        stored,
    })
}

async fn store(pool: &SqlitePool, technologies: &[Technology]) -> Result<i64, AppError> {
    let stored = TechnologyRecord::replace_all(pool, technologies).await;
    pool.close().await;
    stored
}

fn read_technologies(
    files: &DataFiles,
    source: CatalogSource,
) -> Result<Vec<Technology>, AppError> {
    match source {
        CatalogSource::Bundled => BUNDLED_TECHNOLOGIES.entries(),
        CatalogSource::Json => {
            let text = read_file(&files.technologies_json)?;
            serde_json::from_str(&text).map_err(|source| AppError::InvalidCatalogFile {
                path: files.technologies_json.display().to_string(),
                source,
            })
        }
        CatalogSource::WikiPage => read_pages(files),
    }
}

fn read_pages(files: &DataFiles) -> Result<Vec<Technology>, AppError> {
    let mut pages = Vec::with_capacity(files.technology_pages.len());
    for page in &files.technology_pages {
        pages.push((page.domain, read_file(&page.path)?));
    }
    let sources: Vec<TechnologyPageSource<'_>> = pages
        .iter()
        .map(|(domain, markup)| TechnologyPageSource { domain, markup })
        .collect();
    Ok(parse_technology_pages(&sources))
}
