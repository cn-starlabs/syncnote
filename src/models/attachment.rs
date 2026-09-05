use serde::{Deserialize, Serialize};

/// Response returned to the client after a successful upload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadResult {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    pub url: String,
}

/// A row in the "My Files" attachment manager. `scope_title`/`scope_link` are
/// `None` when the note or shared page this was attached to has since been
/// deleted — `attachments.scope_id` isn't a real foreign key (it's polymorphic
/// across two tables), so those rows are never cleaned up automatically and
/// show up here as orphaned instead of erroring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentInfo {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub created_at: String,
    pub scope: String,
    pub scope_title: Option<String>,
    pub scope_link: Option<String>,
    pub url: String,
    /// `false` when this row is here because someone else shared it with the
    /// current user, not because they own it — the Files page hides
    /// Delete/Share for those.
    pub is_owner: bool,
    pub shared_by_email: Option<String>,
}

/// One user a file has been directly shared with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileShareInfo {
    pub user_id: i64,
    pub email: String,
}

/// A public download link for a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileShareLink {
    pub token: String,
    pub url: String,
    pub expires_at: Option<String>,
    pub max_downloads: Option<i64>,
    pub download_count: i64,
    pub created_at: String,
}
