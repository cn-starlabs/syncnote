use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Editor,
    Viewer,
}

impl MemberRole {
    pub fn as_str(self) -> &'static str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Editor => "editor",
            MemberRole::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(MemberRole::Owner),
            "editor" => Some(MemberRole::Editor),
            "viewer" => Some(MemberRole::Viewer),
            _ => None,
        }
    }

    pub fn can_edit(self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Editor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedPage {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub version: i64,
    pub my_role: MemberRole,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedPageMember {
    pub user_id: i64,
    pub email: String,
    pub role: MemberRole,
}

/// WebSocket wire message, used both directions:
/// client -> server: "here's my edit, based on `version`"
/// server -> client: "the authoritative current state is `body`/`version`"
/// (on a version mismatch the server sends this back to just the sender so
/// their editor can reconcile; on a successful write it's broadcast to everyone)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageEdit {
    pub body: String,
    pub version: i64,
}
