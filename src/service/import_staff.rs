use sqlx::SqlitePool;

use crate::config::DataFiles;
use crate::db::{self, StaffRecord};
use crate::domain::StaffMember;
use crate::error::AppError;
use crate::service::bundled_catalog::BUNDLED_STAFF;
use crate::service::catalog_import::{CatalogSource, ImportOptions, ImportOutcome};
use crate::service::data_file::{read_file, write_json};
use crate::wiki::{PoliticsEntry, parse_staff};

/// Reads the staff and designers of the country page and fills a database with them.
///
/// From the page, `staff.json` is rewritten on the way through, as `parse-politics` writes it.
///
/// # Errors
/// Fails when a file is missing or unreadable, the page holds no staff, or the database fails.
pub async fn import_staff(
    files: &DataFiles,
    options: ImportOptions<'_>,
) -> Result<ImportOutcome, AppError> {
    let members = read_staff(files, options.source)?;
    if members.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucun membre de l'état-major",
        });
    }
    let stored = match options.database_url {
        Some(url) => Some(store(&db::connect(url).await?, &members).await?),
        None => None,
    };
    Ok(ImportOutcome {
        parsed: members.len(),
        stored,
    })
}

async fn store(pool: &SqlitePool, members: &[StaffMember]) -> Result<i64, AppError> {
    let stored = StaffRecord::replace_all(pool, members).await;
    pool.close().await;
    stored
}

fn read_staff(files: &DataFiles, source: CatalogSource) -> Result<Vec<StaffMember>, AppError> {
    match source {
        CatalogSource::Bundled => BUNDLED_STAFF.entries(),
        CatalogSource::Json => {
            let text = read_file(&files.staff_json)?;
            serde_json::from_str(&text).map_err(|source| AppError::InvalidCatalogFile {
                path: files.staff_json.display().to_string(),
                source,
            })
        }
        CatalogSource::WikiPage => {
            let entries = parse_staff(&read_file(&files.politics_page)?);
            if !entries.is_empty() {
                write_json(&files.staff_json, &entries)?;
            }
            Ok(entries
                .into_iter()
                .map(PoliticsEntry::into_staff_member)
                .collect())
        }
    }
}
