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
/// Automatically sends a notification email to the target user. If no
/// account exists for that email, falls back to creating (or reusing) a
/// public share link for the note and emailing that instead.
#[server(endpoint = "notes/share-with-user")]
pub async fn share_note_with_user(
    note_id: i64,
    email: String,
) -> Result<crate::models::ShareOutcome, ServerFnError> {
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
        // No SyncNote account for that email — email them a public link instead.
        email_share_link_to_non_user(&pool, note_id, &user, &email).await?;
        return Ok(crate::models::ShareOutcome::LinkEmailed);
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

    // Fire-and-forget notification email to the newly-shared-with user
    {
        let note_title: String = sqlx::query_as::<_, (String,)>(
            "SELECT COALESCE(NULLIF(TRIM(title), ''), 'Untitled Note') FROM notes WHERE id = ?",
        )
        .bind(note_id)
        .fetch_optional(&pool).await.ok().flatten()
        .map(|(t,)| t).unwrap_or_else(|| "a note".to_string());

        let sender_name = user.display_name.clone().unwrap_or_else(|| user.email.clone());

        let host_header = leptos_axum::extract::<axum::http::HeaderMap>()
            .await
            .ok()
            .and_then(|headers| headers.get(axum::http::header::HOST).and_then(|v| v.to_str().ok()).map(str::to_string));

        if let Some(h) = host_header {
            let scheme = if h.starts_with("localhost") || h.starts_with("127.") { "http" } else { "https" };
            let base_url = format!("{scheme}://{h}");
            let note_url = format!("{base_url}/app/note/shared-user/{note_id}");
            let subject = format!("{sender_name} shared \"{note_title}\" with you on SyncNote");
            let text_body = format!(
                "{sender_name} has shared the note \"{note_title}\" with you on SyncNote.\n\nSign in and view it here:\n{note_url}"
            );
            let html_body = format!(
                r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;line-height:1.6;color:#1e293b;max-width:600px;margin:0 auto;padding:24px;">
  <div style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:12px;padding:28px;">
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:28px;">
      <div style="width:36px;height:36px;background:#4f46e5;border-radius:8px;text-align:center;line-height:36px;font-size:20px;">
        <span style="color:#fff;">✎</span>
      </div>
      <span style="font-size:20px;font-weight:700;color:#0f172a;letter-spacing:-0.5px;">SyncNote</span>
    </div>
    <h1 style="font-size:22px;font-weight:700;color:#0f172a;margin:0 0 8px 0;">{note_title}</h1>
    <p style="font-size:14px;color:#64748b;margin:0 0 24px 0;">
      <strong style="color:#334155;">{sender_name}</strong> has shared this note with you.
    </p>
    <a href="{note_url}" style="display:inline-block;padding:12px 28px;background:#4f46e5;color:#ffffff;text-decoration:none;border-radius:8px;font-weight:600;font-size:15px;">
      View Note →
    </a>
    <p style="font-size:12px;color:#94a3b8;margin-top:24px;border-top:1px solid #e2e8f0;padding-top:16px;">
      Or paste this link:<br/>
      <a href="{note_url}" style="color:#6366f1;word-break:break-all;">{note_url}</a>
    </p>
  </div>
  <p style="font-size:11px;color:#94a3b8;text-align:center;margin-top:16px;">Sent via SyncNote · Sign in to view this note.</p>
</div>"#,
                note_title = note_title,
                sender_name = sender_name,
                note_url = note_url,
            );
            let _ = crate::server::mailer::send_email(&email, &subject, &text_body, Some(&html_body)).await;
        }
    }

    Ok(crate::models::ShareOutcome::SharedWithUser(crate::models::NoteShareInfo {
        user_id: target_id,
        email,
    }))
}

/// Ensures a public share link exists for `note_id` and emails it to `email`.
/// Used as the fallback when `share_note_with_user` is given an address with
/// no SyncNote account.
#[cfg(feature = "ssr")]
async fn email_share_link_to_non_user(
    pool: &sqlx::SqlitePool,
    note_id: i64,
    owner: &crate::auth::AuthUser,
    email: &str,
) -> Result<(), ServerFnError> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT token FROM note_share_links WHERE note_id = ? LIMIT 1")
            .bind(note_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| srv("db", e))?;
    let token = if let Some((t,)) = existing {
        t
    } else {
        let token = gen_token();
        sqlx::query(
            "INSERT INTO note_share_links (token, note_id, created_by, password_hash) VALUES (?, ?, ?, NULL)",
        )
        .bind(&token)
        .bind(note_id)
        .bind(owner.id)
        .execute(pool)
        .await
        .map_err(|e| srv("db", e))?;
        token
    };

    let note_title: String = sqlx::query_as::<_, (String,)>(
        "SELECT COALESCE(NULLIF(TRIM(title), ''), 'Untitled Note') FROM notes WHERE id = ?",
    )
    .bind(note_id)
    .fetch_optional(pool).await.ok().flatten()
    .map(|(t,)| t).unwrap_or_else(|| "a note".to_string());

    let sender_name = owner.display_name.clone().unwrap_or_else(|| owner.email.clone());

    let host_header = leptos_axum::extract::<axum::http::HeaderMap>()
        .await
        .ok()
        .and_then(|headers| headers.get(axum::http::header::HOST).and_then(|v| v.to_str().ok()).map(str::to_string));

    let Some(h) = host_header else { return Ok(()) };
    let scheme = if h.starts_with("localhost") || h.starts_with("127.") { "http" } else { "https" };
    let share_url = format!("{scheme}://{h}/note/shared/{token}");
    let subject = format!("{sender_name} shared a note with you: {note_title}");
    let text_body = format!(
        "{sender_name} has shared \"{note_title}\" with you via SyncNote.\n\nYou don't have a SyncNote account yet, so here's a read-only link (no account required):\n{share_url}"
    );
    let html_body = format!(
        r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;line-height:1.6;color:#1e293b;max-width:600px;margin:0 auto;padding:24px;">
  <div style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:12px;padding:28px;">
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:28px;">
      <div style="width:36px;height:36px;background:#4f46e5;border-radius:8px;text-align:center;line-height:36px;font-size:20px;">
        <span style="color:#fff;">✎</span>
      </div>
      <span style="font-size:20px;font-weight:700;color:#0f172a;letter-spacing:-0.5px;">SyncNote</span>
    </div>
    <h1 style="font-size:22px;font-weight:700;color:#0f172a;margin:0 0 8px 0;">{note_title}</h1>
    <p style="font-size:14px;color:#64748b;margin:0 0 24px 0;">
      <strong style="color:#334155;">{sender_name}</strong> has shared this note with you. You don't have a SyncNote account yet, so here's a read-only link — no account required.
    </p>
    <a href="{share_url}" style="display:inline-block;padding:12px 28px;background:#4f46e5;color:#ffffff;text-decoration:none;border-radius:8px;font-weight:600;font-size:15px;">
      View Note →
    </a>
    <p style="font-size:12px;color:#94a3b8;margin-top:24px;border-top:1px solid #e2e8f0;padding-top:16px;">
      Or paste this link in your browser:<br/>
      <a href="{share_url}" style="color:#6366f1;word-break:break-all;">{share_url}</a>
    </p>
  </div>
  <p style="font-size:11px;color:#94a3b8;text-align:center;margin-top:16px;">Sent via SyncNote</p>
</div>"#,
        note_title = note_title,
        sender_name = sender_name,
        share_url = share_url,
    );

    let _ = crate::server::mailer::send_email(email, &subject, &text_body, Some(&html_body)).await;
    Ok(())
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

/// Email a public share link to a recipient.
/// Requires the caller to own the note. The share URL must be the full absolute URL.
#[server(endpoint = "notes/email-share-link")]
pub async fn send_share_link_email(
    note_id: i64,
    recipient_email: String,
    share_url: String,
    /// Plaintext of the link's password, if the owner supplied one. Only
    /// included in the email when it actually matches the link's stored
    /// hash — the client can't be trusted to know whether it's protected.
    password: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let recipient_email = recipient_email.trim().to_lowercase();
    if recipient_email.is_empty() || !recipient_email.contains('@') {
        return Err(srv("input", "invalid email address"));
    }
    let share_url = share_url.trim().to_string();
    if share_url.is_empty() {
        return Err(srv("input", "share URL is required"));
    }

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    // Verify ownership and fetch title
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT COALESCE(NULLIF(TRIM(title), ''), 'Untitled Note') FROM notes WHERE id = ? AND owner_id = ?",
    )
    .bind(note_id).bind(user.id)
    .fetch_optional(&pool).await.map_err(|e| srv("db", e))?;

    let Some((title,)) = row else {
        return Err(srv("note", "not found or not owned by you"));
    };

    // Only fold the password into the email if the link is actually
    // password-protected and the supplied plaintext matches the stored hash.
    let verified_password: Option<String> = {
        let hash: Option<(Option<String>,)> =
            sqlx::query_as("SELECT password_hash FROM note_share_links WHERE note_id = ? LIMIT 1")
                .bind(note_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| srv("db", e))?;
        match (hash.and_then(|(h,)| h), password) {
            (Some(stored_hash), Some(candidate)) if verify_password(&candidate, &stored_hash) => {
                Some(candidate)
            }
            _ => None,
        }
    };

    let sender_name = user.display_name.clone().unwrap_or_else(|| user.email.clone());
    let subject = format!("{sender_name} shared a note with you: {title}");
    let password_text_line = verified_password
        .as_ref()
        .map(|p| format!("\n\nThis link is password protected. Password: {p}"))
        .unwrap_or_default();
    let text_body = format!(
        "{sender_name} has shared \"{title}\" with you via SyncNote.\n\nView it here (no account required):\n{share_url}{password_text_line}"
    );
    let password_html_block = verified_password
        .as_ref()
        .map(|p| {
            let escaped = p.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            format!(
                r#"<p style="font-size:12px;color:#334155;margin-top:12px;background:#f1f5f9;border-radius:6px;padding:8px 12px;">🔒 This link is password protected. Password: <strong>{escaped}</strong></p>"#
            )
        })
        .unwrap_or_default();
    let html_body = format!(
        r#"<div style="font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;line-height:1.6;color:#1e293b;max-width:600px;margin:0 auto;padding:24px;">
  <div style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:12px;padding:28px;">
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:28px;">
      <div style="width:36px;height:36px;background:#4f46e5;border-radius:8px;text-align:center;line-height:36px;font-size:20px;">
        <span style="color:#fff;">✎</span>
      </div>
      <span style="font-size:20px;font-weight:700;color:#0f172a;letter-spacing:-0.5px;">SyncNote</span>
    </div>
    <h1 style="font-size:22px;font-weight:700;color:#0f172a;margin:0 0 8px 0;">{title}</h1>
    <p style="font-size:14px;color:#64748b;margin:0 0 24px 0;">
      <strong style="color:#334155;">{sender_name}</strong> has shared this note with you.
    </p>
    <a href="{share_url}" style="display:inline-block;padding:12px 28px;background:#4f46e5;color:#ffffff;text-decoration:none;border-radius:8px;font-weight:600;font-size:15px;">
      View Note →
    </a>
    <p style="font-size:12px;color:#94a3b8;margin-top:24px;border-top:1px solid #e2e8f0;padding-top:16px;">
      Or paste this link in your browser:<br/>
      <a href="{share_url}" style="color:#6366f1;word-break:break-all;">{share_url}</a>
    </p>
    <p style="font-size:12px;color:#94a3b8;margin-top:8px;">🔒 Read-only · No account required to view</p>
    {password_html_block}
  </div>
  <p style="font-size:11px;color:#94a3b8;text-align:center;margin-top:16px;">Sent via SyncNote</p>
</div>"#,
        title = title,
        sender_name = sender_name,
        share_url = share_url,
        password_html_block = password_html_block,
    );

    crate::server::mailer::send_email(&recipient_email, &subject, &text_body, Some(&html_body))
        .await
        .map_err(|e| srv("email", e))?;

    Ok(())
}
