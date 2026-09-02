use leptos::prelude::ServerFnError;
use sqlx::SqlitePool;
use tower_sessions::Session;

use super::model::AuthUser;

pub const SESSION_USER_ID: &str = "uid";

fn srv(msg: impl Into<String>) -> ServerFnError {
    ServerFnError::ServerError(msg.into())
}

/// Read the current user_id from the cookie session and load the row.
pub async fn current_user(session: &Session, pool: &SqlitePool) -> Result<Option<AuthUser>, ServerFnError> {
    let user_id: Option<i64> = session
        .get(SESSION_USER_ID)
        .await
        .map_err(|e| srv(format!("session read: {e}")))?;

    let Some(user_id) = user_id else {
        return Ok(None);
    };

    let row: Option<(i64, String, Option<String>, bool, bool)> =
        sqlx::query_as("SELECT id, email, display_name, is_admin, locked FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| srv(format!("db: {e}")))?;

    // A locked account is treated as logged out everywhere: RequireAuth
    // redirects to /login, and any in-flight session gets kicked out on its
    // next request, without needing to hunt down and invalidate the cookie.
    Ok(row.and_then(|(id, email, display_name, is_admin, locked)| {
        if locked {
            None
        } else {
            Some(AuthUser {
                id,
                email,
                display_name,
                is_admin,
            })
        }
    }))
}

pub async fn login(session: &Session, user_id: i64) -> Result<(), ServerFnError> {
    session
        .insert(SESSION_USER_ID, user_id)
        .await
        .map_err(|e| srv(format!("session write: {e}")))?;
    Ok(())
}

pub async fn logout(session: &Session) -> Result<(), ServerFnError> {
    session.flush().await.map_err(|e| srv(format!("session flush: {e}")))?;
    Ok(())
}

pub async fn require_user(session: &Session, pool: &SqlitePool) -> Result<AuthUser, ServerFnError> {
    match current_user(session, pool).await? {
        Some(u) => Ok(u),
        None => Err(srv("not signed in")),
    }
}

pub async fn require_admin(session: &Session, pool: &SqlitePool) -> Result<AuthUser, ServerFnError> {
    let user = require_user(session, pool).await?;
    if !user.is_admin {
        return Err(srv("admin access required"));
    }
    Ok(user)
}
