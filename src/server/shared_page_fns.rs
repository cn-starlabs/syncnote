use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::models::MemberRole;
use crate::models::{SharedPage, SharedPageMember};

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

#[cfg(feature = "ssr")]
fn parse_role(s: &str) -> Result<MemberRole, ServerFnError> {
    MemberRole::parse(s).ok_or_else(|| ServerFnError::ServerError("bad role in db".into()))
}

#[server(endpoint = "pages/list")]
pub async fn list_my_shared_pages() -> Result<Vec<SharedPage>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let rows: Vec<(i64, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT p.id, p.title, p.body, p.version, m.role, p.updated_at \
         FROM shared_pages p JOIN shared_page_members m ON m.page_id = p.id \
         WHERE m.user_id = ? ORDER BY p.updated_at DESC",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    rows.into_iter()
        .map(|(id, title, body, version, role, updated_at)| {
            Ok(SharedPage {
                id,
                title,
                body,
                version,
                my_role: parse_role(&role)?,
                updated_at,
            })
        })
        .collect()
}

#[server(endpoint = "pages/get")]
pub async fn get_shared_page(id: i64) -> Result<SharedPage, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let row: Option<(i64, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT p.id, p.title, p.body, p.version, m.role, p.updated_at \
         FROM shared_pages p JOIN shared_page_members m ON m.page_id = p.id \
         WHERE p.id = ? AND m.user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    let (id, title, body, version, role, updated_at) =
        row.ok_or_else(|| srv("page", "not found"))?;

    Ok(SharedPage {
        id,
        title,
        body,
        version,
        my_role: parse_role(&role)?,
        updated_at,
    })
}

#[server(endpoint = "pages/create")]
pub async fn create_shared_page() -> Result<i64, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let mut tx = pool.begin().await.map_err(|e| srv("db", e))?;

    let id: (i64,) = sqlx::query_as("INSERT INTO shared_pages (owner_id, title, body) VALUES (?, 'Untitled', '') RETURNING id")
        .bind(user.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| srv("db", e))?;

    sqlx::query("INSERT INTO shared_page_members (page_id, user_id, role) VALUES (?, ?, 'owner')")
        .bind(id.0)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| srv("db", e))?;

    tx.commit().await.map_err(|e| srv("db", e))?;
    Ok(id.0)
}

#[server(endpoint = "pages/rename")]
pub async fn rename_shared_page(id: i64, title: String) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let title = if title.trim().is_empty() { "Untitled".to_string() } else { title };

    let result = sqlx::query(
        "UPDATE shared_pages SET title = ?, updated_at = datetime('now') \
         WHERE id = ? AND id IN (SELECT page_id FROM shared_page_members WHERE user_id = ? AND role IN ('owner','editor'))",
    )
    .bind(title)
    .bind(id)
    .bind(user.id)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError("page not found or not permitted".into()));
    }
    Ok(())
}

#[server(endpoint = "pages/delete")]
pub async fn delete_shared_page(id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let result = sqlx::query("DELETE FROM shared_pages WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError("page not found or not owner".into()));
    }
    Ok(())
}

#[server(endpoint = "pages/members")]
pub async fn list_members(page_id: i64) -> Result<Vec<SharedPageMember>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    // Must be a member yourself to see the roster.
    let member: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM shared_page_members WHERE page_id = ? AND user_id = ?")
        .bind(page_id)
        .bind(user.id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    if member.is_none() {
        return Err(ServerFnError::ServerError("page not found".into()));
    }

    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT u.id, u.email, m.role FROM shared_page_members m \
         JOIN users u ON u.id = m.user_id WHERE m.page_id = ? ORDER BY m.joined_at ASC",
    )
    .bind(page_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    rows.into_iter()
        .map(|(user_id, email, role)| {
            Ok(SharedPageMember {
                user_id,
                email,
                role: parse_role(&role)?,
            })
        })
        .collect()
}

#[server(endpoint = "pages/remove-member")]
pub async fn remove_member(page_id: i64, user_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

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
        return Err(ServerFnError::ServerError("only the owner can remove members".into()));
    }
    if user_id == user.id {
        return Err(ServerFnError::ServerError("owner cannot remove themself".into()));
    }

    sqlx::query("DELETE FROM shared_page_members WHERE page_id = ? AND user_id = ?")
        .bind(page_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}
