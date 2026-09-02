use leptos::prelude::*;

use crate::models::Note;

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

#[server(endpoint = "notes/list")]
pub async fn list_my_notes() -> Result<Vec<Note>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let notes = sqlx::query_as::<_, Note>(
        "SELECT id, title, body, updated_at FROM notes WHERE owner_id = ? ORDER BY updated_at DESC",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(notes)
}

#[server(endpoint = "notes/get")]
pub async fn get_note(id: i64) -> Result<Note, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let note = sqlx::query_as::<_, Note>(
        "SELECT id, title, body, updated_at FROM notes WHERE id = ? AND owner_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    note.ok_or_else(|| srv("note", "not found"))
}

#[server(endpoint = "notes/create")]
pub async fn create_note() -> Result<i64, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let id: (i64,) = sqlx::query_as("INSERT INTO notes (owner_id, title, body) VALUES (?, 'Untitled', '') RETURNING id")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(id.0)
}

#[server(endpoint = "notes/save")]
pub async fn save_note(id: i64, title: String, body: String) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let title = if title.trim().is_empty() { "Untitled".to_string() } else { title };

    let result = sqlx::query(
        "UPDATE notes SET title = ?, body = ?, updated_at = datetime('now') WHERE id = ? AND owner_id = ?",
    )
    .bind(title)
    .bind(body)
    .bind(id)
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError("note not found".into()));
    }
    Ok(())
}

#[server(endpoint = "notes/delete")]
pub async fn delete_note(id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    sqlx::query("DELETE FROM notes WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}

#[server(endpoint = "notes/email")]
pub async fn send_note_via_email(id: i64, recipient_email: String) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::components::markdown::render_markdown;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let recipient_email = recipient_email.trim().to_string();
    if recipient_email.is_empty() || !recipient_email.contains('@') {
        return Err(srv("input", "invalid recipient email address"));
    }

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let note = sqlx::query_as::<_, Note>(
        "SELECT id, title, body, updated_at FROM notes WHERE id = ? AND owner_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    let Some(note) = note else {
        return Err(srv("note", "note not found"));
    };

    let title = if note.title.trim().is_empty() {
        "Untitled Note".to_string()
    } else {
        note.title
    };

    let sender_name = user
        .display_name
        .clone()
        .unwrap_or_else(|| user.email.clone());
    let subject = format!("Note: {title}");
    let text = format!(
        "Note: {title}\nShared by: {sender_name}\nLast updated: {}\n\n---\n\n{}",
        note.updated_at, note.body
    );

    let rendered_html = render_markdown(&note.body);
    let html = format!(
        "<div style=\"font-family: sans-serif; line-height: 1.6; color: #1e293b; max-width: 680px; margin: 0 auto; padding: 20px;\">\
            <div style=\"border-bottom: 2px solid #e2e8f0; padding-bottom: 12px; margin-bottom: 20px;\">\
                <h1 style=\"font-size: 22px; font-weight: bold; color: #0f172a; margin: 0 0 6px 0;\">{title}</h1>\
                <p style=\"font-size: 12px; color: #64748b; margin: 0;\">Shared by {sender_name} &bull; {updated_at}</p>\
            </div>\
            <div style=\"font-size: 15px; color: #334155;\">\
                {rendered_html}\
            </div>\
            <hr style=\"border: none; border-top: 1px solid #e2e8f0; margin: 32px 0 16px 0;\"/>\
            <p style=\"font-size: 11px; color: #94a3b8;\">Sent via SyncNote</p>\
        </div>",
        title = title,
        sender_name = sender_name,
        updated_at = note.updated_at,
        rendered_html = rendered_html
    );

    crate::server::mailer::send_email(&recipient_email, &subject, &text, Some(&html))
        .await
        .map_err(|e| srv("email", e))?;

    Ok(())
}

