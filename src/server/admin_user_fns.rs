use leptos::prelude::*;

use crate::models::AdminUserInfo;

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

#[cfg(feature = "ssr")]
fn gen_temp_password() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    rand::thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect()
}

#[server(endpoint = "admin/users/list")]
pub async fn list_users() -> Result<Vec<AdminUserInfo>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    sess::require_admin(&session, &pool).await?;

    let rows: Vec<(i64, String, Option<String>, bool, bool, String, Option<String>)> = sqlx::query_as(
        "SELECT id, email, display_name, is_admin, locked, created_at, last_login_at \
         FROM users ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, email, display_name, is_admin, locked, created_at, last_login_at)| AdminUserInfo {
                id,
                email,
                display_name,
                is_admin,
                locked,
                created_at,
                last_login_at,
            },
        )
        .collect())
}

/// Generates a new random password for the target user and returns it in
/// plaintext — this is the only time it's ever visible, so the admin must
/// copy it now and pass it to the user out of band.
#[server(endpoint = "admin/users/reset-password")]
pub async fn admin_reset_password(user_id: i64) -> Result<String, ServerFnError> {
    use crate::auth::password as pw;
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    sess::require_admin(&session, &pool).await?;

    let temp_password = gen_temp_password();
    let hash = pw::hash(&temp_password).map_err(|e| srv("hash", e))?;

    let result = sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    if result.rows_affected() == 0 {
        return Err(srv("user", "not found"));
    }

    Ok(temp_password)
}

#[server(endpoint = "admin/users/set-locked")]
pub async fn set_user_locked(user_id: i64, locked: bool) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let admin = sess::require_admin(&session, &pool).await?;

    if user_id == admin.id && locked {
        return Err(srv("user", "cannot lock your own account"));
    }

    sqlx::query("UPDATE users SET locked = ? WHERE id = ?")
        .bind(locked)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}

#[server(endpoint = "admin/users/delete")]
pub async fn delete_user(user_id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let admin = sess::require_admin(&session, &pool).await?;

    if user_id == admin.id {
        return Err(srv("user", "cannot delete your own account"));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}
