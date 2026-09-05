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

    let user_row: Option<(String,)> = sqlx::query_as("SELECT email FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    let Some((user_email,)) = user_row else {
        return Err(srv("user", "user not found"));
    };

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

    let recipient = user_email.clone();
    let temp_pw = temp_password.clone();
    tokio::spawn(async move {
        let subject = "SyncNote: Password Reset by Administrator";
        let text = format!(
            "Hello,\n\nAn administrator has reset the password for your SyncNote account ({recipient}).\n\n\
             Your Temporary Password: {temp_pw}\n\n\
             Please log in with this temporary password and change it in your Account settings."
        );
        let html = format!(
            "<div style=\"font-family: sans-serif; line-height: 1.6; color: #334155; max-width: 560px;\">\
                <h2 style=\"color: #0f172a;\">Password Reset</h2>\
                <p>Hello,</p>\
                <p>An administrator has reset the password for your SyncNote account (<strong>{recipient}</strong>):</p>\
                <div style=\"background: #f1f5f9; padding: 12px 16px; border-radius: 6px; font-family: monospace; font-size: 16px; font-weight: bold; margin: 16px 0; letter-spacing: 1px;\">\
                    {temp_pw}\
                </div>\
                <p>Please log in using this temporary password and update it under <strong>Account settings</strong>.</p>\
                <hr style=\"border: none; border-top: 1px solid #e2e8f0; margin: 24px 0;\"/>\
                <p style=\"font-size: 12px; color: #94a3b8;\">SyncNote Administration</p>\
            </div>"
        );
        if let Err(e) = crate::server::mailer::send_email(&recipient, subject, &text, Some(&html)).await {
            tracing::error!("admin_reset_password: failed to send email to {recipient}: {e}");
        } else {
            tracing::info!("admin_reset_password: password reset email sent to {recipient}");
        }
    });

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

#[server(endpoint = "admin/users/set-admin")]
pub async fn set_user_admin(user_id: i64, is_admin: bool) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let admin = sess::require_admin(&session, &pool).await?;

    if user_id == admin.id && !is_admin {
        return Err(srv("user", "cannot remove your own admin rights"));
    }

    sqlx::query("UPDATE users SET is_admin = ? WHERE id = ?")
        .bind(is_admin)
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

#[server(endpoint = "admin/users/create")]
pub async fn admin_create_user(
    email: String,
    display_name: Option<String>,
    password: Option<String>,
    is_admin: bool,
) -> Result<String, ServerFnError> {
    use crate::auth::password as pw;
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    sess::require_admin(&session, &pool).await?;

    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(srv("input", "a valid email address is required"));
    }

    let raw_password = match password {
        Some(p) if !p.trim().is_empty() => {
            if p.trim().len() < 8 {
                return Err(srv("input", "password must be at least 8 characters"));
            }
            p.trim().to_string()
        }
        _ => gen_temp_password(),
    };

    let hash = pw::hash(&raw_password).map_err(|e| srv("hash", e))?;
    let clean_name = display_name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    if existing.is_some() {
        return Err(srv("input", "a user with this email already exists"));
    }

    sqlx::query(
        "INSERT INTO users (email, display_name, password_hash, is_admin) VALUES (?, ?, ?, ?)",
    )
    .bind(&email)
    .bind(clean_name)
    .bind(&hash)
    .bind(is_admin)
    .execute(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(raw_password)
}

