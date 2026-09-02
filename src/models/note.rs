use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub updated_at: String,
}

#[cfg(feature = "ssr")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Note {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Note {
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            body: row.try_get("body")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}
