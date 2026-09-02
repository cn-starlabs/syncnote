use serde::{Deserialize, Serialize};

/// Response returned to the client after a successful upload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UploadResult {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    pub url: String,
}
