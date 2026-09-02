use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminUserInfo {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub is_admin: bool,
    pub locked: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}
