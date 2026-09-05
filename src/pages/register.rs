use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::auth::refresh_auth;
use crate::server::auth_fns::Register;

#[component]
pub fn RegisterPage() -> impl IntoView {
    let action = ServerAction::<Register>::new();
    let pending = action.pending();
    let result = action.value();
    let navigate = use_navigate();

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if let Some(Ok(outcome)) = result.get() {
                if outcome.ok {
                    refresh_auth();
                    #[cfg(feature = "hydrate")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().assign("/app");
                            return;
                        }
                    }
                    navigate("/app", Default::default());
                }
            }
        });
    }

    let error_msg = move || match result.get() {
        Some(Ok(o)) if !o.ok => o.message.unwrap_or_else(|| "Registration failed".into()),
        Some(Err(_)) => "Server error, please try again".to_string(),
        _ => String::new(),
    };

    view! {
        <div class="max-w-md mx-auto">
            <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6 sm:p-8">
                <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Sign up"</h1>
                <p class="mt-1 text-sm text-slate-500 dark:text-slate-400">"Create an account to start taking notes."</p>

                <ActionForm action=action attr:class="mt-6 space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-slate-700 dark:text-slate-300">"Email"</label>
                        <input type="email" name="email" required
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 shadow-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm"
                            placeholder="you@example.com"/>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-slate-700 dark:text-slate-300">"Password"</label>
                        <input type="password" name="password" required minlength="8"
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 shadow-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm"/>
                        <p class="mt-1 text-xs text-slate-400">"At least 8 characters."</p>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-slate-700 dark:text-slate-300">"Invite code"</label>
                        <input type="text" name="invite_code" required
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 shadow-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm"/>
                    </div>

                    <Show when=move || !error_msg().is_empty()>
                        <p class="text-sm text-rose-600 bg-rose-50 border border-rose-200 rounded p-2">
                            {error_msg}
                        </p>
                    </Show>

                    <button type="submit" disabled=move || pending.get()
                        class="w-full inline-flex justify-center rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow hover:bg-brand-700 disabled:opacity-60">
                        {move || if pending.get() { "Creating account…" } else { "Sign up" }}
                    </button>
                </ActionForm>

                <p class="mt-4 text-xs text-slate-500 dark:text-slate-400 text-center">
                    "Already have an account? "
                    <A href="/login" attr:class="text-brand-600 hover:underline">"Sign in"</A>
                </p>
            </div>
        </div>
    }
}
