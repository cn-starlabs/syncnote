//! Browser-side WebAuthn ceremony glue. `webauthn-rs-proto`'s `wasm` feature
//! provides `From` conversions between its JSON-safe proto types and the real
//! `web_sys` Credential Management API types, so this is mostly plumbing: call
//! our own server functions (which just shuttle JSON strings, see
//! `server/passkey_fns.rs`) around the two `navigator.credentials` calls that
//! only the browser can make. No-op stub for the non-hydrate (ssr) build.

use crate::server::auth_fns::AuthOutcome;

/// Best-effort extraction of the real DOMException/Error name + message
/// (e.g. "NotAllowedError: The operation either timed out or was not
/// allowed...") — a caught/awaited promise rejection doesn't get logged to
/// the browser console on its own, so surfacing this in the UI is the only
/// way to see *why* a WebAuthn call was rejected.
#[cfg(feature = "hydrate")]
fn describe_js_error(e: &wasm_bindgen::JsValue) -> String {
    use wasm_bindgen::JsCast;
    if let Some(err) = e.dyn_ref::<web_sys::DomException>() {
        format!("{}: {}", err.name(), err.message())
    } else if let Some(err) = e.dyn_ref::<js_sys::Error>() {
        format!("{}: {}", err.name(), err.message())
    } else {
        format!("{e:?}")
    }
}

#[cfg(feature = "hydrate")]
pub async fn register_passkey(label: String) -> Result<(), String> {
    use crate::server::passkey_fns::{finish_passkey_registration, start_passkey_registration};
    use wasm_bindgen_futures::JsFuture;
    use webauthn_rs_proto::{CreationChallengeResponse, RegisterPublicKeyCredential, ResidentKeyRequirement};

    let ccr_json = start_passkey_registration().await.map_err(|e| e.to_string())?;
    let mut ccr: CreationChallengeResponse = serde_json::from_str(&ccr_json).map_err(|e| e.to_string())?;
    // The server asks for `residentKey: discouraged` (webauthn-rs's default for
    // `start_passkey_registration`), but our login flow is usernameless and needs
    // a discoverable credential to find anything. `residentKey` is only a hint to
    // the authenticator, not something the server verifies afterward, so it's
    // safe to override it here before handing the options to the browser.
    if let Some(selection) = ccr.public_key.authenticator_selection.as_mut() {
        selection.resident_key = Some(ResidentKeyRequirement::Required);
        selection.require_resident_key = true;
    }
    let c_options: web_sys::CredentialCreationOptions = ccr.into();

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let promise = window
        .navigator()
        .credentials()
        .create_with_options(&c_options)
        .map_err(|e| format!("could not start passkey creation: {}", describe_js_error(&e)))?;
    let jsval = JsFuture::from(promise)
        .await
        .map_err(|e| format!("passkey creation was cancelled or failed: {}", describe_js_error(&e)))?;
    let w_rpkc = web_sys::PublicKeyCredential::from(jsval);
    let rpkc = RegisterPublicKeyCredential::from(w_rpkc);
    let cred_json = serde_json::to_string(&rpkc).map_err(|e| e.to_string())?;

    finish_passkey_registration(label, cred_json).await.map_err(|e| e.to_string())
}

#[cfg(not(feature = "hydrate"))]
pub async fn register_passkey(_label: String) -> Result<(), String> {
    Err("passkeys are only available in the browser".to_string())
}

/// Checks whether this browser supports "conditional mediation" — the
/// autofill-style passkey suggestion UI. Where unsupported (older Firefox,
/// etc.), skip calling [`login_with_discoverable_passkey`] and just let
/// the user fall back to the password form.
#[cfg(feature = "hydrate")]
pub async fn conditional_mediation_available() -> bool {
    use wasm_bindgen_futures::JsFuture;
    use webauthn_rs_proto::PublicKeyCredentialExt;

    let Ok(promise) = PublicKeyCredentialExt::is_conditional_mediation_available() else {
        return false;
    };
    JsFuture::from(promise).await.map(|v| v.as_bool().unwrap_or(false)).unwrap_or(false)
}

#[cfg(not(feature = "hydrate"))]
pub async fn conditional_mediation_available() -> bool {
    false
}

/// Usernameless login: the server identifies the user from whatever
/// credential the browser returns (see `finish_discoverable_login`), so no
/// email/username is needed up front either way. `conditional` picks the UX:
/// - `true` — autofill-style suggestion under a field with
///   `autocomplete="webauthn"`; sits pending until the user picks a
///   suggestion or navigates away. Start this once, as soon as the login
///   page mounts, not on a button click.
/// - `false` — strips the `mediation` hint so the browser pops its normal
///   passkey picker dialog immediately; suitable for an explicit "Sign in
///   with a passkey" button.
#[cfg(feature = "hydrate")]
pub async fn login_with_discoverable_passkey(conditional: bool) -> Result<AuthOutcome, String> {
    use crate::server::passkey_fns::{finish_discoverable_login, start_discoverable_login};
    use wasm_bindgen_futures::JsFuture;
    use webauthn_rs_proto::{PublicKeyCredential as ProtoPublicKeyCredential, RequestChallengeResponse};

    let rcr_json = start_discoverable_login().await.map_err(|e| e.to_string())?;
    let mut rcr: RequestChallengeResponse = serde_json::from_str(&rcr_json).map_err(|e| e.to_string())?;
    if !conditional {
        rcr.mediation = None;
    }
    let c_options: web_sys::CredentialRequestOptions = rcr.into();

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let promise = window
        .navigator()
        .credentials()
        .get_with_options(&c_options)
        .map_err(|e| format!("could not start passkey sign-in: {}", describe_js_error(&e)))?;
    let jsval = JsFuture::from(promise)
        .await
        .map_err(|e| format!("passkey sign-in was cancelled or failed: {}", describe_js_error(&e)))?;
    let w_pkc = web_sys::PublicKeyCredential::from(jsval);
    let pkc = ProtoPublicKeyCredential::from(w_pkc);
    let cred_json = serde_json::to_string(&pkc).map_err(|e| e.to_string())?;

    finish_discoverable_login(cred_json).await.map_err(|e| e.to_string())
}

#[cfg(not(feature = "hydrate"))]
pub async fn login_with_discoverable_passkey(_conditional: bool) -> Result<AuthOutcome, String> {
    Err("passkeys are only available in the browser".to_string())
}
