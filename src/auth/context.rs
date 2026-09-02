use leptos::prelude::*;

use crate::auth::AuthUser;
use crate::server::auth_fns::get_current_user;

/// Resource that loads (or returns) the currently-signed-in user.
/// Re-runs whenever the version signal is bumped, e.g. after login/logout.
#[derive(Clone, Copy)]
pub struct AuthContext {
    pub version: RwSignal<u32>,
    pub user: Resource<Result<Option<AuthUser>, ServerFnError>>,
}

pub fn provide_auth_context() {
    let version = RwSignal::new(0u32);
    let user = Resource::new(move || version.get(), |_| async move { get_current_user().await });
    provide_context(AuthContext { version, user });
}

pub fn use_auth() -> AuthContext {
    expect_context::<AuthContext>()
}

/// Bump after login/logout to force re-fetch on the client.
pub fn refresh_auth() {
    if let Some(ctx) = use_context::<AuthContext>() {
        ctx.version.update(|v| *v = v.wrapping_add(1));
    }
}
