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
}
