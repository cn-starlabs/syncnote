use serde::{Deserialize, Serialize};

/// Shared (client + server) view of the authenticated user.
/// Never carries the password hash — that stays server-side only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub is_admin: bool,
}
