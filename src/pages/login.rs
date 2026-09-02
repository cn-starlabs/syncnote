use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::auth::refresh_auth;
use crate::client_passkey::login_with_discoverable_passkey;
use crate::server::auth_fns::Login;

#[component]
pub fn LoginPage() -> impl IntoView {
    let action = ServerAction::<Login>::new();
    let pending = action.pending();
    let result = action.value();

    Effect::new(move |_| {
        if let Some(Ok(outcome)) = result.get() {
            if outcome.ok {
                refresh_auth();
            }
        }
    });

    let error_msg = move || match result.get() {
        Some(Ok(o)) if !o.ok => o.message.unwrap_or_else(|| "Sign in failed".into()),
        Some(Err(_)) => "Server error, please try again".to_string(),
        _ => String::new(),
    };

    let navigate = use_navigate();

    // Only one WebAuthn `get()` can be pending per tab at a time, so we don't
    // also auto-start a background conditional (autofill) request here — it
    // would collide with this explicit button ("OperationError: A request is
    // already pending") the moment the user clicked it.
    let passkey_pending = RwSignal::new(false);
    let passkey_error = RwSignal::new(Option::<String>::None);

    let on_passkey_button = move |_| {
        passkey_error.set(None);
        passkey_pending.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            let outcome = login_with_discoverable_passkey(false).await;
            passkey_pending.set(false);
            match outcome {
                Ok(o) if o.ok => {
                    refresh_auth();
                    navigate("/app", Default::default());
                }
                Ok(o) => passkey_error.set(Some(o.message.unwrap_or_else(|| "Sign in failed".into()))),
                Err(e) => passkey_error.set(Some(e)),
            }
        });
    };

    view! {
        <div class="max-w-md mx-auto">
            <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6 sm:p-8">
                <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Sign in"</h1>
                <p class="mt-1 text-sm text-slate-500">"Use your email and password."</p>

                <ActionForm action=action attr:class="mt-6 space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-slate-700 dark:text-slate-300">"Email"</label>
                        <input type="email" name="email" required autocomplete="username"
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 shadow-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm"
                            placeholder="you@example.com"/>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-slate-700 dark:text-slate-300">"Password"</label>
                        <input type="password" name="password" required minlength="1" autocomplete="current-password"
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 shadow-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm"/>
                    </div>

                    <Show when=move || !error_msg().is_empty()>
                        <p class="text-sm text-rose-600 bg-rose-50 border border-rose-200 rounded p-2">
                            {error_msg}
                        </p>
                    </Show>

                    <button type="submit" disabled=move || pending.get()
                        class="w-full inline-flex justify-center rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow hover:bg-brand-700 disabled:opacity-60">
                        {move || if pending.get() { "Signing in…" } else { "Sign in" }}
                    </button>
                </ActionForm>

                <div class="mt-6 pt-6 border-t border-slate-200 dark:border-slate-800">
                    <button
                        on:click=on_passkey_button
                        disabled=move || passkey_pending.get()
                        class="w-full inline-flex justify-center items-center gap-2 rounded-md border border-slate-300 dark:border-slate-700 px-4 py-2 text-sm font-medium hover:bg-slate-100 dark:hover:bg-slate-800 disabled:opacity-60"
                    >
                        {move || if passkey_pending.get() { "Verifying…" } else { "Sign in with a passkey" }}
                    </button>
                    <Show when=move || passkey_error.get().is_some()>
                        <p class="mt-2 text-sm text-rose-600 bg-rose-50 border border-rose-200 rounded p-2">
                            {move || passkey_error.get().unwrap_or_default()}
                        </p>
                    </Show>
                </div>

                <p class="mt-4 text-xs text-slate-500 text-center">
                    "No account? "
                    <A href="/register" attr:class="text-brand-600 hover:underline">"Sign up"</A>
                </p>
            </div>
        </div>
    }
}
