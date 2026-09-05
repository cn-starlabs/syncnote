use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::auth::use_auth;
use crate::server::invite_fns::{get_invite_preview, JoinInvite};

#[component]
pub fn JoinInvitePage() -> impl IntoView {
    let params = use_params_map();
    let token = move || params.read().get("token").unwrap_or_default();
    let preview = Resource::new(token, |token| async move { get_invite_preview(token).await });
    let auth = use_auth();
    let join = ServerAction::<JoinInvite>::new();
    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(page_id)) = join.value().get() {
            navigate(&format!("/app/shared/{page_id}"), Default::default());
        }
    });

    view! {
        <div class="max-w-md mx-auto">
            <Suspense fallback=|| view! { <p class="text-sm text-slate-500 dark:text-slate-400">"Loading invite…"</p> }>
                {move || Suspend::new(async move {
                    match preview.await {
                        Ok(p) => view! {
                            <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6 sm:p-8">
                                <h1 class="text-lg font-semibold text-slate-900 dark:text-slate-100">
                                    "You've been invited to \""{p.page_title}"\""
                                </h1>
                                <p class="mt-1 text-sm text-slate-500 dark:text-slate-400">"Access level: "{p.role}</p>

                                <Suspense fallback=|| ()>
                                    {move || Suspend::new(async move {
                                        match auth.user.await {
                                            Ok(Some(_)) => view! {
                                                <button
                                                    on:click=move |_| { join.dispatch(JoinInvite { token: token() }); }
                                                    class="mt-6 w-full rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700"
                                                >
                                                    "Join page"
                                                </button>
                                            }.into_any(),
                                            _ => view! {
                                                <p class="mt-6 text-sm text-slate-600 dark:text-slate-400">
                                                    "Sign in or create an account to join this page."
                                                </p>
                                                <div class="mt-3 flex gap-3">
                                                    <A href="/login" attr:class="rounded-md border border-slate-300 dark:border-slate-700 px-4 py-2 text-sm font-medium hover:bg-slate-100 dark:hover:bg-slate-800">"Sign in"</A>
                                                    <A href="/register" attr:class="rounded-md bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700">"Sign up"</A>
                                                </div>
                                            }.into_any(),
                                        }
                                    })}
                                </Suspense>
                            </div>
                        }.into_any(),
                        Err(_) => view! { <p class="text-sm text-rose-500">"This invite link is invalid or has expired."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}
