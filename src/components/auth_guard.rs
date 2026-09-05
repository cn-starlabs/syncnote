use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::auth::{use_auth, AuthUser};

/// Wraps a subtree that requires an authenticated user. While the auth resource
/// is loading we render a small placeholder; once resolved we either render
/// children with the user injected as context, or redirect to /login.
#[component]
pub fn RequireAuth(children: ChildrenFn) -> impl IntoView {
    let auth = use_auth();
    let children = StoredValue::new(children);
    view! {
        <Suspense fallback=|| view! { <AuthLoading/> }>
            {move || Suspend::new(async move {
                let result = auth.user.await;
                match result {
                    Ok(Some(user)) => {
                        provide_context(user);
                        children.read_value()().into_any()
                    }
                    Ok(None) => view! { <Redirect path="/login"/> }.into_any(),
                    Err(_)   => view! { <Redirect path="/login"/> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
pub fn RequireAdmin(children: ChildrenFn) -> impl IntoView {
    let auth = use_auth();
    let children = StoredValue::new(children);
    view! {
        <Suspense fallback=|| view! { <AuthLoading/> }>
            {move || Suspend::new(async move {
                let result = auth.user.await;
                match result {
                    Ok(Some(user)) if user.is_admin => {
                        provide_context(user);
                        children.read_value()().into_any()
                    }
                    Ok(Some(_)) => view! { <Forbidden/> }.into_any(),
                    _ => view! { <Redirect path="/login"/> }.into_any(),
                }
            })}
        </Suspense>
    }
}

/// Reads the user injected by RequireAuth/RequireAdmin. Panics if used outside.
pub fn current_user() -> AuthUser {
    expect_context::<AuthUser>()
}

#[component]
fn AuthLoading() -> impl IntoView {
    view! {
        <div class="py-20 text-center text-sm text-slate-500 dark:text-slate-400">"Loading…"</div>
    }
}

#[component]
fn Forbidden() -> impl IntoView {
    view! {
        <div class="py-20 text-center">
            <h1 class="text-2xl font-bold text-slate-900 dark:text-slate-100">"403 — Forbidden"</h1>
            <p class="mt-2 text-slate-600 dark:text-slate-400">"This page is admin-only."</p>
        </div>
    }
}
