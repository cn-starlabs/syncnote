use leptos::prelude::*;

use crate::components::admin_tabs::AdminTabs;
use crate::server::admin_user_fns::{list_users, AdminResetPassword, DeleteUser, SetUserLocked};

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let users = Resource::new(|| (), |_| async move { list_users().await });
    let reset_password = ServerAction::<AdminResetPassword>::new();
    let set_locked = ServerAction::<SetUserLocked>::new();
    let delete_user = ServerAction::<DeleteUser>::new();
    let confirming_delete = RwSignal::new(Option::<i64>::None);
    let action_error = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        if let Some(result) = set_locked.value().get() {
            match result {
                Ok(()) => users.refetch(),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        }
    });
    Effect::new(move |_| {
        if let Some(result) = delete_user.value().get() {
            match result {
                Ok(()) => {
                    confirming_delete.set(None);
                    users.refetch();
                }
                Err(e) => action_error.set(Some(e.to_string())),
            }
        }
    });
    let temp_password = move || {
        reset_password.value().get().and_then(|r| match r {
            Ok(pw) => Some(pw),
            Err(e) => {
                action_error.set(Some(e.to_string()));
                None
            }
        })
    };

    view! {
        <div class="space-y-4">
            <AdminTabs/>
            <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Users"</h1>

            <Show when=move || action_error.get().is_some()>
                <p class="text-sm text-rose-600 bg-rose-50 border border-rose-200 rounded p-2">
                    {move || action_error.get().unwrap_or_default()}
                </p>
            </Show>
            <Show when=move || temp_password().is_some()>
                <p class="text-sm text-emerald-700 bg-emerald-50 border border-emerald-200 rounded p-2">
                    "New temporary password (copy it now, it won't be shown again): "
                    <span class="font-mono select-all">{move || temp_password().unwrap_or_default()}</span>
                </p>
            </Show>

            <Suspense fallback=|| view! { <p class="text-sm text-slate-500">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match users.await {
                        Ok(list) => view! {
                            <div class="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
                                <table class="min-w-full text-sm">
                                    <thead class="text-left text-xs text-slate-500 border-b border-slate-200 dark:border-slate-800">
                                        <tr>
                                            <th class="px-4 py-2">"Email"</th>
                                            <th class="px-4 py-2">"Name"</th>
                                            <th class="px-4 py-2">"Role"</th>
                                            <th class="px-4 py-2">"Status"</th>
                                            <th class="px-4 py-2">"Last login"</th>
                                            <th class="px-4 py-2">"Actions"</th>
                                        </tr>
                                    </thead>
                                    <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
                                        {list.into_iter().map(|u| {
                                            let id = u.id;
                                            let is_locked = u.locked;
                                            view! {
                                                <tr>
                                                    <td class="px-4 py-2">{u.email.clone()}</td>
                                                    <td class="px-4 py-2 text-slate-500">{u.display_name.clone().unwrap_or_default()}</td>
                                                    <td class="px-4 py-2 text-slate-500">{if u.is_admin { "admin" } else { "user" }}</td>
                                                    <td class="px-4 py-2">
                                                        {if is_locked {
                                                            view! { <span class="text-rose-600">"Locked"</span> }.into_any()
                                                        } else {
                                                            view! { <span class="text-emerald-600">"Active"</span> }.into_any()
                                                        }}
                                                    </td>
                                                    <td class="px-4 py-2 text-slate-400 text-xs">{u.last_login_at.clone().unwrap_or_else(|| "never".into())}</td>
                                                    <td class="px-4 py-2">
                                                        <div class="flex items-center gap-3 text-xs">
                                                            <button
                                                                on:click=move |_| { action_error.set(None); reset_password.dispatch(AdminResetPassword { user_id: id }); }
                                                                class="text-brand-600 hover:underline"
                                                            >
                                                                "Reset password"
                                                            </button>
                                                            <button
                                                                on:click=move |_| { action_error.set(None); set_locked.dispatch(SetUserLocked { user_id: id, locked: !is_locked }); }
                                                                class="text-amber-600 hover:underline"
                                                            >
                                                                {if is_locked { "Unlock" } else { "Lock" }}
                                                            </button>
                                                            {move || if confirming_delete.get() == Some(id) {
                                                                view! {
                                                                    <span class="text-slate-500">"Delete permanently?"</span>
                                                                    <button
                                                                        on:click=move |_| { action_error.set(None); delete_user.dispatch(DeleteUser { user_id: id }); }
                                                                        class="text-rose-600 font-semibold hover:underline"
                                                                    >
                                                                        "Confirm"
                                                                    </button>
                                                                    <button
                                                                        on:click=move |_| confirming_delete.set(None)
                                                                        class="text-slate-400 hover:underline"
                                                                    >
                                                                        "Cancel"
                                                                    </button>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <button
                                                                        on:click=move |_| confirming_delete.set(Some(id))
                                                                        class="text-rose-500 hover:underline"
                                                                    >
                                                                        "Delete"
                                                                    </button>
                                                                }.into_any()
                                                            }}
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any(),
                        Err(_) => view! { <p class="text-sm text-rose-500">"Failed to load users."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}
