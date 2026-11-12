use sqlx::{FromRow, SqlitePool};

use crate::db::focus_record::read_json;
use crate::domain::StaffMember;
use crate::error::AppError;

#[derive(FromRow)]
struct StaffRow {
    id: String,
    name: String,
    icon: String,
    icon_page: String,
    section: String,
    position: i64,
    requires: String,
    excludes: String,
}

/// The `staff` table: the staff and designers, a copy of what the country page says.
#[derive(Debug, Clone)]
pub struct StaffRecord;

impl StaffRecord {
    /// Rewrites the catalog wholesale, and says how many rows it now holds.
    ///
    /// Plans are untouched: they store ids, and what the catalog no longer knows is not drawn.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn replace_all(pool: &SqlitePool, members: &[StaffMember]) -> Result<i64, AppError> {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("DELETE FROM staff")
            .execute(&mut *transaction)
            .await?;
        for member in members {
            sqlx::query(
                "INSERT INTO staff (id, name, icon, icon_page, section, position, requires, excludes)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&member.id)
            .bind(&member.name)
            .bind(&member.icon)
            .bind(&member.icon_page)
            .bind(&member.section)
            .bind(member.position)
            .bind(serde_json::to_string(&member.requires)?)
            .bind(serde_json::to_string(&member.excludes)?)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        StaffRecord::count(pool).await
    }

    /// The whole catalog, in the page's own order, which groups it by table.
    ///
    /// # Errors
    /// Fails when the database does or a stored list cannot be read.
    pub async fn list(pool: &SqlitePool) -> Result<Vec<StaffMember>, AppError> {
        let rows: Vec<StaffRow> = sqlx::query_as(
            "SELECT id, name, icon, icon_page, section, position, requires, excludes
             FROM staff ORDER BY position",
        )
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(into_member).collect()
    }

    /// How many members are stored.
    ///
    /// # Errors
    /// Fails when the database does.
    pub async fn count(pool: &SqlitePool) -> Result<i64, AppError> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM staff")
            .fetch_one(pool)
            .await?)
    }
}

fn into_member(row: StaffRow) -> Result<StaffMember, AppError> {
    Ok(StaffMember {
        requires: read_json("staff.requires", &row.requires)?,
        excludes: read_json("staff.excludes", &row.excludes)?,
        id: row.id,
        name: row.name,
        icon: row.icon,
        icon_page: row.icon_page,
        section: row.section,
        position: row.position,
    })
}
