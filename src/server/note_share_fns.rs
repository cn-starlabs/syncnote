use leptos::prelude::*;

use crate::models::Note;

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
fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| srv("hash", e))
}

#[cfg(feature = "ssr")]
fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Create (or return existing) public share link. Optionally password-protect it.
/// If a link already exists for the note, returns its token and ignores the new password.
#[server(endpoint = "notes/create-share-link")]
pub async fn create_note_share_link(
    note_id: i64,
    password: Option<String>,
) -> Result<String, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    // Verify ownership
    let owned: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM notes WHERE id = ? AND owner_id = ?")
        .bind(note_id)
        .bind(user.id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    if owned.is_none() {
        return Err(srv("note", "not found or not owned by you"));
    }

    // Return existing token if one already exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT token FROM note_share_links WHERE note_id = ? LIMIT 1")
            .bind(note_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| srv("db", e))?;
    if let Some((token,)) = existing {
        return Ok(token);
    }

    let token = gen_token();
    let password_hash = password
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .map(hash_password)
        .transpose()?;

    sqlx::query(
        "INSERT INTO note_share_links (token, note_id, created_by, password_hash) VALUES (?, ?, ?, ?)",
    )
    .bind(&token)
    .bind(note_id)
    .bind(user.id)
    .bind(password_hash)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(token)
}

/// Revoke all share links for a note.
#[server(endpoint = "notes/revoke-share-link")]
pub async fn revoke_note_share_link(note_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    sqlx::query(
        "DELETE FROM note_share_links WHERE note_id = ? AND note_id IN \
         (SELECT id FROM notes WHERE owner_id = ?)",
    )
    .bind(note_id)
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(())
}

/// Get share token for a note (returns None if no link exists).
#[server(endpoint = "notes/get-share-token")]
pub async fn get_note_share_token(note_id: i64) -> Result<Option<String>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let row: Option<(String,)> = sqlx::query_as(
        "SELECT nsl.token FROM note_share_links nsl \
         JOIN notes n ON n.id = nsl.note_id \
         WHERE nsl.note_id = ? AND n.owner_id = ? LIMIT 1",
    )
    .bind(note_id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(row.map(|(t,)| t))
}

/// Check whether a share link requires a password.
/// Returns Ok(true) if protected, Ok(false) if open, Err if token invalid.
#[server(endpoint = "notes/check-share-protected")]
pub async fn check_share_link_protected(token: String) -> Result<bool, ServerFnError> {
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;

    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT password_hash FROM note_share_links WHERE token = ?")
            .bind(&token)
            .fetch_optional(&pool)
            .await
            .map_err(|e| srv("db", e))?;

    match row {
        None => Err(srv("link", "not found or invalid")),
        Some((hash,)) => Ok(hash.is_some()),
    }
}

/// Public endpoint: fetch a note by share token. Pass password if the link is protected.
#[server(endpoint = "notes/shared")]
pub async fn get_shared_note(
    token: String,
    password: Option<String>,
) -> Result<Note, ServerFnError> {
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;

    let row: Option<(i64, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT n.id, n.title, n.body, n.updated_at, nsl.password_hash \
         FROM notes n \
         JOIN note_share_links nsl ON nsl.note_id = n.id \
         WHERE nsl.token = ?",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    let Some((id, title, body, updated_at, password_hash)) = row else {
        return Err(srv("note", "not found or link invalid"));
    };

    // Verify password if required
    if let Some(hash) = password_hash {
        let provided = password.as_deref().unwrap_or("").trim().to_string();
        if provided.is_empty() {
            return Err(ServerFnError::ServerError("password_required".into()));
        }
        if !verify_password(&provided, &hash) {
            return Err(ServerFnError::ServerError("wrong_password".into()));
        }
    }

    Ok(Note { id, title, body, updated_at })
}

// ── User-specific note sharing ────────────────────────────────────────────────

/// Share a note directly with another registered user (read-only for them).
#[server(endpoint = "notes/share-with-user")]
pub async fn share_note_with_user(
    note_id: i64,
    email: String,
) -> Result<crate::models::NoteShareInfo, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let owned: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM notes WHERE id = ? AND owner_id = ?")
        .bind(note_id).bind(user.id).fetch_optional(&pool).await.map_err(|e| srv("db", e))?;
    if owned.is_none() {
        return Err(srv("note", "not found or not owned by you"));
    }

    let email = email.trim().to_lowercase();
    let target: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email).fetch_optional(&pool).await.map_err(|e| srv("db", e))?;
    let Some((target_id,)) = target else {
        return Err(srv("user", "no account with that email"));
    };
    if target_id == user.id {
        return Err(srv("user", "you already own this note"));
    }

    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM note_shares WHERE note_id = ? AND shared_with_user_id = ?",
    ).bind(note_id).bind(target_id).fetch_optional(&pool).await.map_err(|e| srv("db", e))?;
    if existing.is_some() {
        return Err(srv("user", "already shared with that user"));
    }

    sqlx::query("INSERT INTO note_shares (note_id, shared_with_user_id, shared_by) VALUES (?, ?, ?)")
        .bind(note_id).bind(target_id).bind(user.id)
        .execute(&pool).await.map_err(|e| srv("db", e))?;

    Ok(crate::models::NoteShareInfo { user_id: target_id, email })
}

/// List users a note has been directly shared with.
#[server(endpoint = "notes/list-user-shares")]
pub async fn list_note_user_shares(note_id: i64) -> Result<Vec<crate::models::NoteShareInfo>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let owned: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM notes WHERE id = ? AND owner_id = ?")
        .bind(note_id).bind(user.id).fetch_optional(&pool).await.map_err(|e| srv("db", e))?;
    if owned.is_none() {
        return Err(srv("note", "not found or not owned by you"));
    }

    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT u.id, u.email FROM note_shares ns JOIN users u ON u.id = ns.shared_with_user_id \
         WHERE ns.note_id = ? ORDER BY ns.created_at ASC",
    ).bind(note_id).fetch_all(&pool).await.map_err(|e| srv("db", e))?;

    Ok(rows.into_iter().map(|(user_id, email)| crate::models::NoteShareInfo { user_id, email }).collect())
}

/// Remove a direct user share for a note.
#[server(endpoint = "notes/unshare-user")]
pub async fn unshare_note_from_user(note_id: i64, user_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let owner = sess::require_user(&session, &pool).await?;

    let owned: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM notes WHERE id = ? AND owner_id = ?")
        .bind(note_id).bind(owner.id).fetch_optional(&pool).await.map_err(|e| srv("db", e))?;
    if owned.is_none() {
        return Err(srv("note", "not found or not owned by you"));
    }

    sqlx::query("DELETE FROM note_shares WHERE note_id = ? AND shared_with_user_id = ?")
        .bind(note_id).bind(user_id).execute(&pool).await.map_err(|e| srv("db", e))?;

    Ok(())
}

/// List notes that other users have shared with the current user.
#[server(endpoint = "notes/shared-with-me")]
pub async fn list_notes_shared_with_me() -> Result<Vec<Note>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let notes = sqlx::query_as::<_, Note>(
        "SELECT n.id, n.title, n.body, n.updated_at FROM notes n \
         JOIN note_shares ns ON ns.note_id = n.id \
         WHERE ns.shared_with_user_id = ? ORDER BY n.updated_at DESC",
    ).bind(user.id).fetch_all(&pool).await.map_err(|e| srv("db", e))?;

    Ok(notes)
}

/// Get a note that has been directly shared with the current logged-in user.
#[server(endpoint = "notes/shared-user-view")]
pub async fn get_note_shared_with_me(note_id: i64) -> Result<Note, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let note: Option<Note> = sqlx::query_as(
        "SELECT n.id, n.title, n.body, n.updated_at FROM notes n \
         JOIN note_shares ns ON ns.note_id = n.id \
         WHERE n.id = ? AND ns.shared_with_user_id = ?",
    )
    .bind(note_id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    note.ok_or_else(|| srv("note", "not found or not shared with you"))
}
