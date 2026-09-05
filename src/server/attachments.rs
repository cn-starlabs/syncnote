use axum::body::Bytes;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::auth::session as sess;
use crate::models::UploadResult;
use crate::server_ctx::AppState;

const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

fn gen_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn can_write(pool: &SqlitePool, user_id: i64, scope: &str, scope_id: i64) -> bool {
    let found: Option<(i64,)> = match scope {
        "note" => sqlx::query_as("SELECT 1 FROM notes WHERE id = ? AND owner_id = ?")
            .bind(scope_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten(),
        "shared_page" => sqlx::query_as(
            "SELECT 1 FROM shared_page_members WHERE page_id = ? AND user_id = ? AND role IN ('owner','editor')",
        )
        .bind(scope_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten(),
        _ => None,
    };
    found.is_some()
}

async fn can_read(pool: &SqlitePool, user_id: i64, scope: &str, scope_id: i64) -> bool {
    let found: Option<(i64,)> = match scope {
        "note" => sqlx::query_as("SELECT 1 FROM notes WHERE id = ? AND owner_id = ?")
            .bind(scope_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten(),
        "shared_page" => sqlx::query_as("SELECT 1 FROM shared_page_members WHERE page_id = ? AND user_id = ?")
            .bind(scope_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten(),
        _ => None,
    };
    found.is_some()
}

/// `scope` is "note", "shared_page", or "library". `scope_id` is the id of
/// that note/page — required for "note"/"shared_page", absent for "library"
/// (a personal file not tied to any specific note/page at upload time).
/// Caller must already have edit access to the scope (owner of the note,
/// owner/editor of the shared page, or just any signed-in user for their own
/// library) — checked here, not trusted from the client.
pub async fn upload_handler(State(state): State<AppState>, session: Session, mut multipart: Multipart) -> impl IntoResponse {
    let user = match sess::current_user(&session, &state.pool.0).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::UNAUTHORIZED, "not signed in").into_response(),
    };

    let mut scope: Option<String> = None;
    let mut scope_id: Option<i64> = None;
    let mut filename = "file".to_string();
    let mut content_type = "application/octet-stream".to_string();
    let mut data: Option<Bytes> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return (StatusCode::BAD_REQUEST, "bad multipart body").into_response(),
        };
        match field.name().unwrap_or("") {
            "scope" => scope = field.text().await.ok(),
            "scope_id" => scope_id = field.text().await.ok().and_then(|s| s.parse().ok()),
            "file" => {
                filename = field.file_name().unwrap_or("file").to_string();
                content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
                data = field.bytes().await.ok();
            }
            _ => {}
        }
    }

    let (Some(scope), Some(data)) = (scope, data) else {
        return (StatusCode::BAD_REQUEST, "missing scope or file").into_response();
    };
    if data.len() > MAX_UPLOAD_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE, "file too large (max 10MB)").into_response();
    }

    match scope.as_str() {
        "library" => {} // any signed-in user may upload to their own library
        _ => {
            let Some(sid) = scope_id else {
                return (StatusCode::BAD_REQUEST, "missing scope_id").into_response();
            };
            if !can_write(&state.pool.0, user.id, &scope, sid).await {
                return (StatusCode::FORBIDDEN, "not permitted").into_response();
            }
        }
    }

    let stored_name = gen_token();
    let dest = state.uploads_dir.join(&stored_name);
    if tokio::fs::write(&dest, &data).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to store file").into_response();
    }

    let inserted: Result<(i64,), _> = sqlx::query_as(
        "INSERT INTO attachments (owner_id, scope, scope_id, filename, content_type, byte_size, stored_name) \
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(user.id)
    .bind(&scope)
    .bind(scope_id)
    .bind(&filename)
    .bind(&content_type)
    .bind(data.len() as i64)
    .bind(&stored_name)
    .fetch_one(&state.pool.0)
    .await;

    let id = match inserted {
        Ok((id,)) => id,
        Err(_) => {
            let _ = tokio::fs::remove_file(&dest).await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    Json(UploadResult {
        id,
        filename,
        content_type,
        url: format!("/attachments/{id}"),
    })
    .into_response()
}

pub async fn serve_attachment(Path(id): Path<i64>, State(state): State<AppState>, session: Session) -> impl IntoResponse {
    let user = match sess::current_user(&session, &state.pool.0).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let row: Option<(i64, String, Option<i64>, String, String, String)> = sqlx::query_as(
        "SELECT owner_id, scope, scope_id, filename, content_type, stored_name FROM attachments WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool.0)
    .await
    .unwrap_or(None);

    let Some((owner_id, scope, scope_id, filename, content_type, stored_name)) = row else {
        tracing::warn!("Attachment id={id} not found in database");
        return StatusCode::NOT_FOUND.into_response();
    };

    // A library file has no scope_id to check permissions against — it's a
    // personal file, so only its owner can view it (even once its URL has
    // been pasted into a note's Markdown).
    let allowed = if scope == "library" {
        owner_id == user.id
    } else {
        can_read(&state.pool.0, user.id, &scope, scope_id.unwrap_or(0)).await
    };
    if !allowed {
        tracing::warn!("User id={} forbidden from reading attachment id={id}", user.id);
        return StatusCode::FORBIDDEN.into_response();
    }

    let path = state.uploads_dir.join(&stored_name);
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to read attachment file at path {:?}: {e}", path);
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, format!("inline; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response()
}
