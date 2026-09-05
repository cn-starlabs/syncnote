use leptos::prelude::*;

use crate::models::{FileShareInfo, FileShareLink};

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

#[cfg(feature = "ssr")]
async fn require_owner(pool: &sqlx::SqlitePool, user_id: i64, attachment_id: i64) -> Result<(), ServerFnError> {
    let owned: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM attachments WHERE id = ? AND owner_id = ?")
        .bind(attachment_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| srv("db", e))?;
    if owned.is_none() {
        return Err(srv("file", "not found or not owned by you"));
    }
    Ok(())
}

#[server(endpoint = "files/share-with-user")]
pub async fn share_file_with_user(attachment_id: i64, email: String) -> Result<FileShareInfo, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;
    require_owner(&pool, user.id, attachment_id).await?;

    let email = email.trim().to_lowercase();
    let target: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    let Some((target_id,)) = target else {
        return Err(srv("user", "no account with that email"));
    };

    if target_id == user.id {
        return Err(srv("user", "you already own this file"));
    }

    let existing: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM file_shares WHERE attachment_id = ? AND shared_with_user_id = ?")
        .bind(attachment_id)
        .bind(target_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    if existing.is_some() {
        return Err(srv("user", "already shared with that user"));
    }

    sqlx::query("INSERT INTO file_shares (attachment_id, shared_with_user_id, shared_by) VALUES (?, ?, ?)")
        .bind(attachment_id)
        .bind(target_id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(FileShareInfo { user_id: target_id, email })
}

#[server(endpoint = "files/list-shares")]
pub async fn list_file_shares(attachment_id: i64) -> Result<Vec<FileShareInfo>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;
    require_owner(&pool, user.id, attachment_id).await?;

    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT u.id, u.email FROM file_shares fs JOIN users u ON u.id = fs.shared_with_user_id \
         WHERE fs.attachment_id = ? ORDER BY fs.created_at ASC",
    )
    .bind(attachment_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows.into_iter().map(|(user_id, email)| FileShareInfo { user_id, email }).collect())
}

#[server(endpoint = "files/unshare")]
pub async fn unshare_file(attachment_id: i64, user_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let owner = sess::require_user(&session, &pool).await?;
    require_owner(&pool, owner.id, attachment_id).await?;

    sqlx::query("DELETE FROM file_shares WHERE attachment_id = ? AND shared_with_user_id = ?")
        .bind(attachment_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}

/// `expires_in_hours`/`max_downloads` of `None` mean "never expires"/"unlimited".
#[server(endpoint = "files/create-share-link")]
pub async fn create_file_share_link(
    attachment_id: i64,
    expires_in_hours: Option<i64>,
    max_downloads: Option<i64>,
) -> Result<String, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;
    require_owner(&pool, user.id, attachment_id).await?;

    if let Some(n) = max_downloads {
        if n < 1 {
            return Err(srv("input", "max downloads must be at least 1"));
        }
    }

    let token = gen_token();
    sqlx::query(
        "INSERT INTO file_share_links (token, attachment_id, created_by, expires_at, max_downloads) \
         VALUES (?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE datetime('now', ? || ' hours') END, ?)",
    )
    .bind(&token)
    .bind(attachment_id)
    .bind(user.id)
    .bind(expires_in_hours)
    .bind(expires_in_hours)
    .bind(max_downloads)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(token)
}

#[server(endpoint = "files/list-share-links")]
pub async fn list_file_share_links(attachment_id: i64) -> Result<Vec<FileShareLink>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;
    require_owner(&pool, user.id, attachment_id).await?;

    let rows: Vec<(String, Option<String>, Option<i64>, i64, String)> = sqlx::query_as(
        "SELECT token, expires_at, max_downloads, download_count, created_at \
         FROM file_share_links WHERE attachment_id = ? ORDER BY created_at DESC",
    )
    .bind(attachment_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows
        .into_iter()
        .map(|(token, expires_at, max_downloads, download_count, created_at)| FileShareLink {
            url: format!("/share/{token}"),
            token,
            expires_at,
            max_downloads,
            download_count,
            created_at,
        })
        .collect())
}

#[server(endpoint = "files/revoke-share-link")]
pub async fn revoke_file_share_link(token: String) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let result = sqlx::query(
        "DELETE FROM file_share_links WHERE token = ? AND attachment_id IN \
         (SELECT id FROM attachments WHERE owner_id = ?)",
    )
    .bind(&token)
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    if result.rows_affected() == 0 {
        return Err(srv("link", "not found or not owned by you"));
    }

    Ok(())
}
