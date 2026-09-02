use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

/// Returned by `login` / `register` so the UI can flash a message on failure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthOutcome {
    pub ok: bool,
    pub message: Option<String>,
}

#[server(endpoint = "auth/login")]
pub async fn login(email: String, password: String) -> Result<AuthOutcome, ServerFnError> {
    use crate::auth::{password as pw, session as sess};
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let email = email.trim().to_lowercase();
    if !is_email(&email) || password.is_empty() {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Invalid email or password".into()),
        });
    }

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;

    let row: Option<(i64, String, bool)> =
        sqlx::query_as("SELECT id, password_hash, locked FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&pool)
            .await
            .map_err(|e| srv("db", e))?;

    let Some((user_id, hash, locked)) = row else {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Wrong email or password".into()),
        });
    };

    let ok = pw::verify(&password, &hash).map_err(|e| srv("verify", e))?;
    if !ok {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Wrong email or password".into()),
        });
    }
    if locked {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("This account has been locked".into()),
        });
    }

    sqlx::query("UPDATE users SET last_login_at = datetime('now') WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    sess::login(&session, user_id).await?;
    leptos_axum::redirect("/app");
    Ok(AuthOutcome { ok: true, message: None })
}

#[server(endpoint = "auth/register")]
pub async fn register(email: String, password: String, invite_code: String) -> Result<AuthOutcome, ServerFnError> {
    use crate::auth::{password as pw, session as sess};
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let email = email.trim().to_lowercase();
    let invite_code = invite_code.trim().to_string();
    if !is_email(&email) {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Invalid email address".into()),
        });
    }
    if password.len() < 8 {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Password must be at least 8 characters".into()),
        });
    }
    if invite_code.is_empty() {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Invite code is required".into()),
        });
    }

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let mut tx = pool.begin().await.map_err(|e| srv("db", e))?;

    let invite: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT uses_left, expires_at FROM invite_codes WHERE code = ?")
            .bind(&invite_code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| srv("db", e))?;

    let Some((uses_left, expires_at)) = invite else {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Invalid invite code".into()),
        });
    };
    if uses_left <= 0 {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("That invite code has no uses left".into()),
        });
    }
    if let Some(exp) = expires_at {
        let now: (String,) = sqlx::query_as("SELECT datetime('now')")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| srv("db", e))?;
        if exp.as_str() < now.0.as_str() {
            return Ok(AuthOutcome {
                ok: false,
                message: Some("That invite code has expired".into()),
            });
        }
    }

    let taken: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| srv("db", e))?;
    if taken.is_some() {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("That email is already registered".into()),
        });
    }

    let hash = pw::hash(&password).map_err(|e| srv("hash", e))?;

    let user_id: (i64,) = sqlx::query_as("INSERT INTO users (email, password_hash) VALUES (?, ?) RETURNING id")
        .bind(&email)
        .bind(&hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| srv("db", e))?;

    sqlx::query("UPDATE invite_codes SET uses_left = uses_left - 1 WHERE code = ?")
        .bind(&invite_code)
        .execute(&mut *tx)
        .await
        .map_err(|e| srv("db", e))?;

    tx.commit().await.map_err(|e| srv("db", e))?;

    sess::login(&session, user_id.0).await?;
    leptos_axum::redirect("/app");
    Ok(AuthOutcome { ok: true, message: None })
}

#[server(endpoint = "auth/logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use tower_sessions::Session;
    let session: Session = leptos_axum::extract().await?;
    sess::logout(&session).await?;
    leptos_axum::redirect("/");
    Ok(())
}

/// Used by the client AuthContext resource to learn who's logged in.
#[server(endpoint = "auth/me")]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;
    // Use Axum Extension extraction (task-local) rather than Leptos expect_context
    // (thread-local), because this fn is called during SSR from a Resource future
    // that may be polled on a different thread where the Leptos Owner is absent.
    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    sess::current_user(&session, &pool).await
}

#[server(endpoint = "auth/update-profile")]
pub async fn update_profile(display_name: String) -> Result<AuthOutcome, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let display_name = display_name.trim();
    let display_name = if display_name.is_empty() { None } else { Some(display_name) };

    sqlx::query("UPDATE users SET display_name = ? WHERE id = ?")
        .bind(display_name)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(AuthOutcome { ok: true, message: None })
}

#[server(endpoint = "auth/change-password")]
pub async fn change_password(current_password: String, new_password: String) -> Result<AuthOutcome, ServerFnError> {
    use crate::auth::{password as pw, session as sess};
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    if new_password.len() < 8 {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("New password must be at least 8 characters".into()),
        });
    }

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let row: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    let ok = pw::verify(&current_password, &row.0).map_err(|e| srv("verify", e))?;
    if !ok {
        return Ok(AuthOutcome {
            ok: false,
            message: Some("Current password is incorrect".into()),
        });
    }

    let hash = pw::hash(&new_password).map_err(|e| srv("hash", e))?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(AuthOutcome { ok: true, message: None })
}

#[cfg(feature = "ssr")]
fn is_email(s: &str) -> bool {
    let s = s.as_bytes();
    let at = s.iter().position(|&b| b == b'@');
    let Some(at) = at else { return false };
    if at == 0 || at == s.len() - 1 {
        return false;
    }
    s[at + 1..].iter().any(|&b| b == b'.')
}

#[cfg(not(feature = "ssr"))]
#[allow(dead_code)]
fn is_email(_: &str) -> bool {
    true
}
