use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignupInvite {
    pub code: String,
    pub uses_left: i64,
    pub expires_at: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}
