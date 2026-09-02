use leptos::prelude::*;

use crate::components::admin_tabs::AdminTabs;
use crate::server::admin_user_fns::{
    list_users, AdminCreateUser, AdminResetPassword, DeleteUser, SetUserLocked,
};

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    let users = Resource::new(|| (), |_| async move { list_users().await });
    let create_user = ServerAction::<AdminCreateUser>::new();
    let reset_password = ServerAction::<AdminResetPassword>::new();
    let set_locked = ServerAction::<SetUserLocked>::new();
    let delete_user = ServerAction::<DeleteUser>::new();
    let confirming_delete = RwSignal::new(Option::<i64>::None);
    let action_error = RwSignal::new(Option::<String>::None);
    let created_user_info = RwSignal::new(Option::<(String, String)>::None);

    let new_email = RwSignal::new(String::new());
    let new_display_name = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let new_is_admin = RwSignal::new(false);
    let show_create_form = RwSignal::new(false);

    Effect::new(move |_| {
        if let Some(result) = create_user.value().get() {
            match result {
                Ok(pw) => {
                    let email = new_email.get_untracked();
                    created_user_info.set(Some((email, pw)));
                    new_email.set(String::new());
                    new_display_name.set(String::new());
                    new_password.set(String::new());
                    new_is_admin.set(false);
                    show_create_form.set(false);
                    action_error.set(None);
                    users.refetch();
                }
                Err(e) => action_error.set(Some(e.to_string())),
            }
        }
    });

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
        <div class="space-y-6">
            <AdminTabs/>
            
            <div class="flex items-center justify-between">
                <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Users"</h1>
                <button
                    on:click=move |_| show_create_form.update(|v| *v = !*v)
                    class="rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 transition shadow-sm"
                >
                    {move || if show_create_form.get() { "Cancel" } else { "+ Create user" }}
                </button>
            </div>

            // Create User Form Card
            <Show when=move || show_create_form.get()>
                <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6 space-y-4">
                    <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Create a new user"</h2>
                    
                    <form
                        on:submit=move |ev| {
                            ev.prevent_default();
                            action_error.set(None);
                            let email = new_email.get();
                            let display_name = if new_display_name.get().trim().is_empty() {
                                None
                            } else {
                                Some(new_display_name.get().trim().to_string())
                            };
                            let password = if new_password.get().trim().is_empty() {
                                None
                            } else {
                                Some(new_password.get().trim().to_string())
                            };
                            let is_admin = new_is_admin.get();

                            create_user.dispatch(AdminCreateUser {
                                email,
                                display_name,
                                password,
                                is_admin,
                            });
                        }
                        class="space-y-4"
                    >
                        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                            <div>
                                <label class="block text-xs font-medium text-slate-600 dark:text-slate-400">"Email *"</label>
                                <input
                                    type="email"
                                    required
                                    placeholder="user@example.com"
                                    prop:value=move || new_email.get()
                                    on:input=move |ev| new_email.set(event_target_value(&ev))
                                    class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-sm"
                                />
                            </div>

                            <div>
                                <label class="block text-xs font-medium text-slate-600 dark:text-slate-400">"Display Name (optional)"</label>
                                <input
                                    type="text"
                                    placeholder="Jane Doe"
                                    prop:value=move || new_display_name.get()
                                    on:input=move |ev| new_display_name.set(event_target_value(&ev))
                                    class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-sm"
                                />
                            </div>

                            <div>
                                <label class="block text-xs font-medium text-slate-600 dark:text-slate-400">"Password (optional, min 8 chars)"</label>
                                <input
                                    type="text"
                                    placeholder="Leave blank to auto-generate"
                                    prop:value=move || new_password.get()
                                    on:input=move |ev| new_password.set(event_target_value(&ev))
                                    class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-sm"
                                />
                            </div>
                        </div>

                        <div class="flex items-center justify-between pt-2">
                            <label class="flex items-center gap-2 cursor-pointer text-xs font-medium text-slate-700 dark:text-slate-300">
                                <input
                                    type="checkbox"
                                    prop:checked=move || new_is_admin.get()
                                    on:change=move |ev| new_is_admin.set(event_target_checked(&ev))
                                    class="rounded border-slate-300 dark:border-slate-700 text-brand-600 focus:ring-brand-500"
                                />
                                "Grant Administrator privileges"
                            </label>

                            <button
                                type="submit"
                                class="rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700 shadow-sm"
                            >
                                "Create user"
                            </button>
                        </div>
                    </form>
                </div>
            </Show>

            <Show when=move || action_error.get().is_some()>
                <p class="text-sm text-rose-600 bg-rose-50 border border-rose-200 rounded p-2">
                    {move || action_error.get().unwrap_or_default()}
                </p>
            </Show>

            <Show when=move || created_user_info.get().is_some()>
                <div class="text-sm text-emerald-800 dark:text-emerald-300 bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 rounded-lg p-3 space-y-1">
                    <p class="font-semibold">"✓ User created successfully!"</p>
                    <p class="text-xs">
                        "User: "<span class="font-mono font-medium">{move || created_user_info.get().map(|(e, _)| e).unwrap_or_default()}</span>
                        " — Password: "<span class="font-mono font-bold select-all bg-emerald-100 dark:bg-emerald-900/60 px-1.5 py-0.5 rounded">{move || created_user_info.get().map(|(_, pw)| pw).unwrap_or_default()}</span>
                        " (copy it now to share with the user)"
                    </p>
                </div>
            </Show>

            <Show when=move || temp_password().is_some()>
                <div class="rounded-lg border border-emerald-200 dark:border-emerald-800 bg-emerald-50 dark:bg-emerald-950/40 p-3 space-y-1 text-sm text-emerald-800 dark:text-emerald-300">
                    <p class="font-semibold">"✓ Password reset successfully"</p>
                    <p class="text-xs">
                        "Temporary password: "
                        <span class="font-mono font-bold select-all bg-emerald-100 dark:bg-emerald-900/60 px-1.5 py-0.5 rounded">
                            {move || temp_password().unwrap_or_default()}
                        </span>
                    </p>
                    <p class="text-xs text-emerald-700 dark:text-emerald-400">
                        "📧 A notification email with this temporary password has also been sent to the user's email address."
                    </p>
                </div>
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
