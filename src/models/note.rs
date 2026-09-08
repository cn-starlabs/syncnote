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

/// A user a note has been directly shared with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteShareInfo {
    pub user_id: i64,
    pub email: String,
}

/// Result of `share_note_with_user`: the recipient either already has a
/// SyncNote account (granted direct access) or didn't (a public share link
/// was emailed to them instead).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShareOutcome {
    SharedWithUser(NoteShareInfo),
    LinkEmailed,
}
