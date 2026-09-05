use leptos::prelude::*;

use crate::models::AttachmentInfo;

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

/// Everything the current user has ever uploaded, across both their personal
/// notes and any shared pages. `scope_title`/`scope_link` come back `None`
/// when the note/page it was attached to has since been deleted — `scope_id`
/// isn't a real foreign key (it's polymorphic across two tables), so those
/// rows are never cleaned up automatically; this page is where a user notices
/// and clears them out.
#[server(endpoint = "attachments/list")]
pub async fn list_my_attachments() -> Result<Vec<AttachmentInfo>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let rows: Vec<AttachmentRow> = sqlx::query_as(
        "SELECT a.id, a.filename, a.content_type, a.byte_size, a.created_at AS created_at, a.scope, a.scope_id, \
                COALESCE(n.title, sp.title) AS scope_title, 1 AS is_owner, NULL AS shared_by_email \
         FROM attachments a \
         LEFT JOIN notes n ON a.scope = 'note' AND n.id = a.scope_id \
         LEFT JOIN shared_pages sp ON a.scope = 'shared_page' AND sp.id = a.scope_id \
         WHERE a.owner_id = ? \
         \
         UNION ALL \
         \
         SELECT a.id, a.filename, a.content_type, a.byte_size, a.created_at AS created_at, a.scope, a.scope_id, \
                NULL AS scope_title, 0 AS is_owner, u.email AS shared_by_email \
         FROM attachments a \
         JOIN file_shares fs ON fs.attachment_id = a.id \
         JOIN users u ON u.id = fs.shared_by \
         WHERE fs.shared_with_user_id = ? \
         \
         ORDER BY created_at DESC",
    )
    .bind(user.id)
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows.into_iter().map(row_to_attachment_info).collect())
}

/// Just the personal-library files (uploaded from `/app/files`, not tied to a
/// specific note/page) — used by the note editor's "Insert from library" picker.
#[server(endpoint = "attachments/list-library")]
pub async fn list_library_attachments() -> Result<Vec<AttachmentInfo>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let rows: Vec<AttachmentRow> = sqlx::query_as(
        "SELECT id, filename, content_type, byte_size, created_at, scope, scope_id, NULL, 1, NULL \
         FROM attachments WHERE owner_id = ? AND scope = 'library' ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows.into_iter().map(row_to_attachment_info).collect())
}

#[cfg(feature = "ssr")]
type AttachmentRow = (
    i64,
    String,
    String,
    i64,
    String,
    String,
    Option<i64>,
    Option<String>,
    bool,
    Option<String>,
);

#[cfg(feature = "ssr")]
fn row_to_attachment_info(
    (id, filename, content_type, byte_size, created_at, scope, scope_id, scope_title, is_owner, shared_by_email): AttachmentRow,
) -> AttachmentInfo {
    let scope_link = match (scope.as_str(), scope_id, scope_title.is_some()) {
        ("note", Some(sid), true) => Some(format!("/app/note/{sid}")),
        ("shared_page", Some(sid), true) => Some(format!("/app/shared/{sid}")),
        _ => None,
    };
    AttachmentInfo {
        id,
        filename,
        content_type,
        byte_size,
        created_at,
        scope,
        scope_title,
        scope_link,
        url: format!("/attachments/{id}"),
        is_owner,
        shared_by_email,
    }
}

#[server(endpoint = "attachments/delete")]
pub async fn delete_attachment(id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::{AppPool, UploadsDir};
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let Extension(UploadsDir(uploads_dir)) = leptos_axum::extract::<Extension<UploadsDir>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT stored_name FROM attachments WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(user.id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    let Some((stored_name,)) = row else {
        return Err(srv("attachment", "not found"));
    };

    sqlx::query("DELETE FROM attachments WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    let _ = tokio::fs::remove_file(uploads_dir.join(&stored_name)).await;

    Ok(())
}
