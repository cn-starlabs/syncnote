use leptos::prelude::*;

use crate::components::admin_tabs::AdminTabs;
use crate::server::admin_invite_fns::{list_signup_invites, CreateSignupInvite, DeleteSignupInvite};

#[component]
pub fn AdminInvitesPage() -> impl IntoView {
    let invites = Resource::new(|| (), |_| async move { list_signup_invites().await });
    let create = ServerAction::<CreateSignupInvite>::new();
    let delete = ServerAction::<DeleteSignupInvite>::new();

    Effect::new(move |_| {
        if create.value().get().is_some() {
            invites.refetch();
        }
    });
    Effect::new(move |_| {
        if delete.value().get().is_some() {
            invites.refetch();
        }
    });

    let new_code = move || create.value().get().and_then(|r| r.ok());

    view! {
        <div class="space-y-6">
            <AdminTabs/>
            <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Invite codes"</h1>

            <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6">
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Create a new code"</h2>
                <ActionForm action=create attr:class="mt-4 flex flex-wrap items-end gap-3">
                    <div>
                        <label class="block text-xs text-slate-500 dark:text-slate-400">"Uses"</label>
                        <input type="number" name="uses_left" value="1" min="1"
                            class="mt-1 w-24 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-2 py-1.5 text-sm"/>
                    </div>
                    <div>
                        <label class="block text-xs text-slate-500 dark:text-slate-400">"Expires in (hours, optional)"</label>
                        <input type="number" name="expires_in_hours" min="1"
                            class="mt-1 w-40 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-2 py-1.5 text-sm"/>
                    </div>
                    <div class="flex-1 min-w-40">
                        <label class="block text-xs text-slate-500 dark:text-slate-400">"Note (optional)"</label>
                        <input type="text" name="note" placeholder="e.g. design team"
                            class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-2 py-1.5 text-sm"/>
                    </div>
                    <button type="submit" class="rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700">
                        "Create"
                    </button>
                </ActionForm>
                <Show when=move || new_code().is_some()>
                    <p class="mt-3 text-xs text-slate-600 dark:text-slate-400">
                        "New code: "<span class="font-mono select-all">{move || new_code().unwrap_or_default()}</span>
                    </p>
                </Show>
            </div>

            <Suspense fallback=|| view! { <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match invites.await {
                        Ok(list) if list.is_empty() => view! {
                            <p class="text-sm text-slate-500 dark:text-slate-400">"No invite codes yet."</p>
                        }.into_any(),
                        Ok(list) => view! {
                            <ul class="divide-y divide-slate-200 dark:divide-slate-800 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
                                {list.into_iter().map(|inv| {
                                    let code = inv.code.clone();
                                    view! {
                                        <li class="flex items-center justify-between px-4 py-3 text-sm">
                                            <div>
                                                <span class="font-mono">{inv.code.clone()}</span>
                                                <span class="ml-3 text-slate-400">
                                                    {inv.uses_left}" uses left"
                                                    {inv.expires_at.clone().map(|e| format!(" · expires {e}")).unwrap_or_default()}
                                                    {inv.note.clone().map(|n| format!(" · {n}")).unwrap_or_default()}
                                                </span>
                                            </div>
                                            <button
                                                on:click=move |_| { delete.dispatch(DeleteSignupInvite { code: code.clone() }); }
                                                class="text-xs text-rose-500 hover:text-rose-700"
                                            >
                                                "Revoke"
                                            </button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                        Err(_) => view! { <p class="text-sm text-rose-500">"Failed to load invite codes."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}
