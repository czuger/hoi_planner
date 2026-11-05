use serde::de::DeserializeOwned;
use sqlx::{FromRow, SqlitePool};

use crate::domain::Focus;
use crate::error::AppError;

#[derive(FromRow)]
struct FocusRow {
    id: String,
    name: String,
    icon: String,
    icon_page: String,
    section: String,
    position: i64,
    requires: String,
    excludes: String,
}

/// The `focuses` table: the catalog, a copy of what the wiki page says.
#[derive(Debug, Clone)]
pub struct FocusRecord;

impl FocusRecord {
    /// Rewrites the catalog wholesale, and says how many rows it now holds.
    ///
    /// Wholesale because the page is the truth: a re-import loses the focuses it no longer
    /// has. Plans are untouched: they store ids, and `schedule` drops the ones the catalog no
    /// longer knows.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn replace_all(pool: &SqlitePool, focuses: &[Focus]) -> Result<i64, AppError> {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM focuses")
            .execute(&mut *transaction)
            .await?;
        for focus in focuses {
            sqlx::query(
                "INSERT INTO focuses (id, name, icon, icon_page, section, position, requires, excludes)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&focus.id)
            .bind(&focus.name)
            .bind(&focus.icon)
            .bind(&focus.icon_page)
            .bind(&focus.section)
            .bind(focus.position)
            .bind(serde_json::to_string(&focus.requires)?)
            .bind(serde_json::to_string(&focus.excludes)?)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        FocusRecord::count(pool).await
    }

    /// The whole catalog, in the page's own order, which groups it by branch.
    ///
    /// # Errors
    /// Fails when the database does or a stored list cannot be read.
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Focus>, AppError> {
        let rows: Vec<FocusRow> = sqlx::query_as(
            "SELECT id, name, icon, icon_page, section, position, requires, excludes
             FROM focuses ORDER BY position",
        )
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(into_focus).collect()
    }

    /// How many focuses are stored.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn count(pool: &SqlitePool) -> Result<i64, AppError> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM focuses")
            .fetch_one(pool)
            .await?)
    }
}

fn into_focus(row: FocusRow) -> Result<Focus, AppError> {
    Ok(Focus {
        requires: read_json("focuses.requires", &row.requires)?,
        excludes: read_json("focuses.excludes", &row.excludes)?,
        id: row.id,
        name: row.name,
        icon: row.icon,
        icon_page: row.icon_page,
        section: row.section,
        position: row.position,
    })
}

/// Reads a JSON column back.
pub(crate) fn read_json<T: DeserializeOwned>(
    column: &'static str,
    text: &str,
) -> Result<T, AppError> {
    serde_json::from_str(text).map_err(|source| AppError::CorruptData { column, source })
}
