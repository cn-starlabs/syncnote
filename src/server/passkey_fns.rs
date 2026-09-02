use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server::auth_fns::AuthOutcome;

#[cfg(feature = "ssr")]
fn srv<S: std::fmt::Display>(prefix: &str, e: S) -> ServerFnError {
    ServerFnError::ServerError(format!("{prefix}: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PasskeyInfo {
    pub id: i64,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Returns the registration challenge as a JSON string (rather than a typed
/// `CreationChallengeResponse`) so this signature doesn't need the
/// `webauthn-rs`/`webauthn-rs-proto` types to be nameable in both the ssr and
/// hydrate builds — the hydrate side only ever sees these types inside
/// `client_passkey.rs`, gated to that feature. See [[project_syncnote]] gotcha
/// about `web_sys` not existing at all under `ssr`; the same logic applies here.
#[server(endpoint = "passkeys/register-start")]
pub async fn start_passkey_registration() -> Result<String, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::{AppPool, WebauthnState};
    use axum::extract::Extension;
    use tower_sessions::Session;
    use webauthn_rs::prelude::*;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let Extension(WebauthnState(webauthn)) = leptos_axum::extract::<Extension<WebauthnState>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    // Deterministic per-user WebAuthn handle derived from our own integer id —
    // avoids a separate stored-UUID column; this handle never leaves the server
    // except embedded (opaquely) in the ceremony, so reversibility isn't a concern here.
    let user_unique_id = Uuid::from_u128(user.id as u128);

    let existing: Vec<(String,)> = sqlx::query_as("SELECT passkey_json FROM passkeys WHERE user_id = ?")
        .bind(user.id)
        .fetch_all(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    let exclude_credentials: Vec<CredentialID> = existing
        .iter()
        .filter_map(|(json,)| serde_json::from_str::<Passkey>(json).ok())
        .map(|pk| pk.cred_id().clone())
        .collect();

    let (ccr, reg_state) = webauthn
        .start_passkey_registration(
            user_unique_id,
            &user.email,
            user.display_name.as_deref().unwrap_or(&user.email),
            Some(exclude_credentials),
        )
        .map_err(|e| srv("webauthn", e))?;

    session
        .insert("passkey_reg_state", (user.id, reg_state))
        .await
        .map_err(|e| srv("session", e))?;

    serde_json::to_string(&ccr).map_err(|e| srv("json", e))
}

#[server(endpoint = "passkeys/register-finish")]
pub async fn finish_passkey_registration(label: String, credential_json: String) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::{AppPool, WebauthnState};
    use axum::extract::Extension;
    use tower_sessions::Session;
    use webauthn_rs::prelude::*;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let Extension(WebauthnState(webauthn)) = leptos_axum::extract::<Extension<WebauthnState>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let (reg_user_id, reg_state): (i64, PasskeyRegistration) = session
        .get("passkey_reg_state")
        .await
        .map_err(|e| srv("session", e))?
        .ok_or_else(|| srv("session", "no registration in progress"))?;
    let _ = session.remove_value("passkey_reg_state").await;

    if reg_user_id != user.id {
        return Err(srv("session", "user mismatch"));
    }

    let reg: RegisterPublicKeyCredential = serde_json::from_str(&credential_json).map_err(|e| srv("json", e))?;
    let passkey = webauthn
        .finish_passkey_registration(&reg, &reg_state)
        .map_err(|e| srv("webauthn", e))?;

    let passkey_json = serde_json::to_string(&passkey).map_err(|e| srv("json", e))?;
    let label = label.trim();
    let label = if label.is_empty() { "Passkey".to_string() } else { label.to_string() };

    sqlx::query("INSERT INTO passkeys (user_id, label, passkey_json) VALUES (?, ?, ?)")
        .bind(user.id)
        .bind(label)
        .bind(passkey_json)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}

#[server(endpoint = "passkeys/list")]
pub async fn list_passkeys() -> Result<Vec<PasskeyInfo>, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    let rows: Vec<(i64, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT id, label, created_at, last_used_at FROM passkeys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| srv("db", e))?;

    Ok(rows
        .into_iter()
        .map(|(id, label, created_at, last_used_at)| PasskeyInfo {
            id,
            label: label.unwrap_or_else(|| "Passkey".to_string()),
            created_at,
            last_used_at,
        })
        .collect())
}

#[server(endpoint = "passkeys/delete")]
pub async fn delete_passkey(id: i64) -> Result<(), ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::AppPool;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let session: Session = leptos_axum::extract().await?;
    let user = sess::require_user(&session, &pool).await?;

    sqlx::query("DELETE FROM passkeys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    Ok(())
}

/// Usernameless (discoverable-credential) login: the browser doesn't need to
/// know who's signing in ahead of time — it prompts the user with whichever
/// passkeys it holds for this site (or offers one as an autofill suggestion
/// under an `autocomplete="webauthn"` field, if the browser supports
/// conditional mediation), and we identify the user from the response
/// afterward via [`finish_discoverable_login`].
#[server(endpoint = "passkeys/login-start")]
pub async fn start_discoverable_login() -> Result<String, ServerFnError> {
    use crate::server_ctx::WebauthnState;
    use axum::extract::Extension;
    use tower_sessions::Session;

    let Extension(WebauthnState(webauthn)) = leptos_axum::extract::<Extension<WebauthnState>>().await?;
    let session: Session = leptos_axum::extract().await?;

    let (rcr, auth_state) = webauthn.start_discoverable_authentication().map_err(|e| srv("webauthn", e))?;

    session
        .insert("passkey_disc_auth_state", auth_state)
        .await
        .map_err(|e| srv("session", e))?;

    serde_json::to_string(&rcr).map_err(|e| srv("json", e))
}

#[server(endpoint = "passkeys/login-finish")]
pub async fn finish_discoverable_login(credential_json: String) -> Result<AuthOutcome, ServerFnError> {
    use crate::auth::session as sess;
    use crate::server_ctx::{AppPool, WebauthnState};
    use axum::extract::Extension;
    use tower_sessions::Session;
    use webauthn_rs::prelude::*;

    let Extension(AppPool(pool)) = leptos_axum::extract::<Extension<AppPool>>().await?;
    let Extension(WebauthnState(webauthn)) = leptos_axum::extract::<Extension<WebauthnState>>().await?;
    let session: Session = leptos_axum::extract().await?;

    let auth_state: DiscoverableAuthentication = session
        .get("passkey_disc_auth_state")
        .await
        .map_err(|e| srv("session", e))?
        .ok_or_else(|| srv("session", "no authentication in progress"))?;
    let _ = session.remove_value("passkey_disc_auth_state").await;

    let cred: PublicKeyCredential = serde_json::from_str(&credential_json).map_err(|e| srv("json", e))?;

    // Extract which user + credential this claims to be, *before* verifying —
    // the actual proof happens in finish_discoverable_authentication below.
    let (user_uuid, _cred_id) = webauthn
        .identify_discoverable_authentication(&cred)
        .map_err(|e| srv("webauthn", e))?;
    // Reverses the deterministic Uuid::from_u128(user.id) used at registration time.
    let user_id = user_uuid.as_u128() as i64;

    let locked: Option<(bool,)> = sqlx::query_as("SELECT locked FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    if locked.map(|(l,)| l).unwrap_or(true) {
        return Err(srv("auth", "this account has been locked"));
    }

    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, passkey_json FROM passkeys WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| srv("db", e))?;
    let passkeys: Vec<(i64, Passkey)> = rows
        .into_iter()
        .filter_map(|(id, json)| serde_json::from_str::<Passkey>(&json).ok().map(|pk| (id, pk)))
        .collect();
    if passkeys.is_empty() {
        return Err(srv("auth", "no passkeys registered for that account"));
    }

    let discoverable_keys: Vec<DiscoverableKey> = passkeys.iter().map(|(_, pk)| DiscoverableKey::from(pk)).collect();
    let auth_result = webauthn
        .finish_discoverable_authentication(&cred, auth_state, &discoverable_keys)
        .map_err(|e| srv("webauthn", e))?;

    // Update whichever stored credential matched (bumps its signature counter,
    // guarding against cloned authenticators) — `update_credential` is a no-op
    // for any passkey that wasn't the one used.
    for (row_id, mut pk) in passkeys {
        if pk.update_credential(&auth_result).is_some() {
            let updated_json = serde_json::to_string(&pk).map_err(|e| srv("json", e))?;
            sqlx::query("UPDATE passkeys SET passkey_json = ?, last_used_at = datetime('now') WHERE id = ?")
                .bind(updated_json)
                .bind(row_id)
                .execute(&pool)
                .await
                .map_err(|e| srv("db", e))?;
            break;
        }
    }

    sess::login(&session, user_id).await?;
    sqlx::query("UPDATE users SET last_login_at = datetime('now') WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| srv("db", e))?;

    leptos_axum::redirect("/app");
    Ok(AuthOutcome { ok: true, message: None })
}
