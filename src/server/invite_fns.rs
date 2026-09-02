use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

#[cfg(feature = "ssr")]
fn gen_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvitePreview {
    pub page_title: String,
    pub role: String,
}

/// `role` must be "editor" or "viewer"; `expires_in_hours` of `None` means no expiry.
#[server(endpoint = "invites/create")]
pub async fn create_invite(page_id: i64, role: String, expires_in_hours: Option<i64>) -> Result<String, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    if role != "editor" && role != "viewer" {
        return Err(ServerFnError::ServerError("role must be editor or viewer".into()));
    }

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let is_owner: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM shared_pages WHERE id = ? AND owner_id = ?")
        .bind(page_id)
        .bind(user.id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    if is_owner.is_none() {
        return Err(ServerFnError::ServerError("only the owner can create invites".into()));
    }

    let token = gen_token();
    sqlx::query(
        "INSERT INTO shared_page_invites (token, page_id, role, created_by, expires_at) \
         VALUES (?, ?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE datetime('now', ? || ' hours') END)",
    )
    .bind(&token)
    .bind(page_id)
    .bind(&role)
    .bind(user.id)
    .bind(expires_in_hours)
    .bind(expires_in_hours)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(token)
}

#[server(endpoint = "invites/preview")]
pub async fn get_invite_preview(token: String) -> Result<InvitePreview, ServerFnError> {
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;

    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT p.title, i.role, i.expires_at FROM shared_page_invites i \
         JOIN shared_pages p ON p.id = i.page_id WHERE i.token = ?",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    let (page_title, role, expires_at) = row.ok_or_else(|| srv("invite", "not found"))?;
    if let Some(exp) = expires_at {
        let now: (String,) = sqlx::query_as("SELECT datetime('now')")
            .fetch_one(&pool)
            .await
            .map_err(|e| srv("db", e))?;
        if exp.as_str() < now.0.as_str() {
            return Err(ServerFnError::ServerError("invite has expired".into()));
        }
    }

    Ok(InvitePreview { page_title, role })
}

#[server(endpoint = "invites/join")]
pub async fn join_invite(token: String) -> Result<i64, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let row: Option<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT page_id, role, expires_at FROM shared_page_invites WHERE token = ?")
            .bind(&token)
            .fetch_optional(&pool)
            .await
            .map_err(|e| srv("db", e))?;

    let (page_id, role, expires_at) = row.ok_or_else(|| srv("invite", "not found"))?;
    if let Some(exp) = expires_at {
        let now: (String,) = sqlx::query_as("SELECT datetime('now')")
            .fetch_one(&pool)
            .await
            .map_err(|e| srv("db", e))?;
        if exp.as_str() < now.0.as_str() {
            return Err(ServerFnError::ServerError("invite has expired".into()));
        }
    }

    sqlx::query(
        "INSERT INTO shared_page_members (page_id, user_id, role) VALUES (?, ?, ?) \
         ON CONFLICT (page_id, user_id) DO NOTHING",
    )
    .bind(page_id)
    .bind(user.id)
    .bind(&role)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(page_id)
}
