use leptos::prelude::*;

use crate::models::SignupInvite;

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

#[cfg(feature = "ssr")]
fn gen_code() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Admin-only. `note` is a free-text label ("for the design team", etc).
#[server(endpoint = "admin/invites/create")]
pub async fn create_signup_invite(
    uses_left: i64,
    expires_in_hours: Option<i64>,
    note: Option<String>,
) -> Result<String, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let admin = sess::require_admin(&session, &pool).await?;

    if uses_left < 1 {
        return Err(ServerFnError::ServerError("uses must be at least 1".into()));
    }

    let code = gen_code();
    sqlx::query(
        "INSERT INTO invite_codes (code, created_by, uses_left, expires_at, note) \
         VALUES (?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE datetime('now', ? || ' hours') END, ?)",
    )
    .bind(&code)
    .bind(admin.id)
    .bind(uses_left)
    .bind(expires_in_hours)
    .bind(expires_in_hours)
    .bind(note)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(code)
}

#[server(endpoint = "admin/invites/list")]
pub async fn list_signup_invites() -> Result<Vec<SignupInvite>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    sess::require_admin(&session, &pool).await?;

    let rows: Vec<(String, i64, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT code, uses_left, expires_at, note, created_at FROM invite_codes ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows
        .into_iter()
        .map(|(code, uses_left, expires_at, note, created_at)| SignupInvite {
            code,
            uses_left,
            expires_at,
            note,
            created_at,
        })
        .collect())
}

#[server(endpoint = "admin/invites/delete")]
pub async fn delete_signup_invite(code: String) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    sess::require_admin(&session, &pool).await?;

    sqlx::query("DELETE FROM invite_codes WHERE code = ?")
        .bind(code)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}
