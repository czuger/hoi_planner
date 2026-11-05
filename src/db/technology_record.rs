use sqlx::{FromRow, SqlitePool};

use crate::db::focus_record::read_json;
use crate::domain::Technology;
use crate::error::AppError;

#[derive(FromRow)]
struct TechnologyRow {
    id: String,
    name: String,
    icon: String,
    icon_page: String,
    domain: String,
    section: String,
    position: i64,
    year: Option<i64>,
    base_days: Option<i64>,
    requires: String,
    excludes: String,
}

/// The `technologies` table: the research tree, a copy of what the wiki pages say.
#[derive(Debug, Clone)]
pub struct TechnologyRecord;

impl TechnologyRecord {
    /// Rewrites the catalog wholesale, and says how many rows it now holds.
    ///
    /// Plans are untouched: they store ids, and what the catalog no longer knows is not drawn.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn replace_all(
        pool: &SqlitePool,
        technologies: &[Technology],
    ) -> Result<i64, AppError> {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM technologies")
            .execute(&mut *transaction)
            .await?;
        for technology in technologies {
            sqlx::query(
                "INSERT INTO technologies
                     (id, name, icon, icon_page, domain, section, position, year, base_days, requires, excludes)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&technology.id)
            .bind(&technology.name)
            .bind(&technology.icon)
            .bind(&technology.icon_page)
            .bind(&technology.domain)
            .bind(&technology.section)
            .bind(technology.position)
            .bind(technology.year)
            .bind(technology.base_days)
            .bind(serde_json::to_string(&technology.requires)?)
            .bind(serde_json::to_string(&technology.excludes)?)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        TechnologyRecord::count(pool).await
    }

    /// The whole catalog, in the pages' own order, which groups it by area and section.
    ///
    /// # Errors
    /// Fails when the database does or a stored list cannot be read.
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Technology>, AppError> {
        let rows: Vec<TechnologyRow> = sqlx::query_as(
            "SELECT id, name, icon, icon_page, domain, section, position, year, base_days, requires, excludes
             FROM technologies ORDER BY position",
        )
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(into_technology).collect()
    }

    /// How many technologies are stored.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn count(pool: &SqlitePool) -> Result<i64, AppError> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM technologies")
            .fetch_one(pool)
            .await?)
    }
}

fn into_technology(row: TechnologyRow) -> Result<Technology, AppError> {
    Ok(Technology {
        requires: read_json("technologies.requires", &row.requires)?,
        excludes: read_json("technologies.excludes", &row.excludes)?,
        id: row.id,
        name: row.name,
        icon: row.icon,
        icon_page: row.icon_page,
        domain: row.domain,
        section: row.section,
        position: row.position,
        year: row.year,
        base_days: row.base_days,
    })
}
