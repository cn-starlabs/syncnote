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
