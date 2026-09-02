use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::auth::{refresh_auth, use_auth};
use crate::client_passkey::register_passkey;
use crate::server::auth_fns::{ChangePassword, UpdateProfile};
use crate::server::passkey_fns::{list_passkeys, DeletePasskey};

#[component]
pub fn AccountPage() -> impl IntoView {
    let auth = use_auth();

    view! {
        <div class="max-w-md mx-auto space-y-8">
            <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Account"</h1>
            <Suspense fallback=|| view! { <p class="text-sm text-slate-500">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match auth.user.await {
                        Ok(Some(user)) => view! {
                            <AccountForms email=user.email display_name=user.display_name.unwrap_or_default()/>
                        }.into_any(),
                        _ => view! { <p class="text-sm text-rose-500">"Not signed in."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn AccountForms(email: String, display_name: String) -> impl IntoView {
    let update_profile = ServerAction::<UpdateProfile>::new();
    let change_password = ServerAction::<ChangePassword>::new();

    Effect::new(move |_| {
        if let Some(Ok(outcome)) = update_profile.value().get() {
            if outcome.ok {
                refresh_auth();
            }
        }
    });

    let profile_msg = move || match update_profile.value().get() {
        Some(Ok(o)) if o.ok => Some(("Profile updated.".to_string(), false)),
        Some(Ok(o)) => Some((o.message.unwrap_or_else(|| "Update failed".into()), true)),
        Some(Err(_)) => Some(("Server error, please try again".to_string(), true)),
        None => None,
    };

    let password_msg = move || match change_password.value().get() {
        Some(Ok(o)) if o.ok => Some(("Password changed.".to_string(), false)),
        Some(Ok(o)) => Some((o.message.unwrap_or_else(|| "Change failed".into()), true)),
        Some(Err(_)) => Some(("Server error, please try again".to_string(), true)),
        None => None,
    };

    view! {
        <div class="space-y-8">
            <p class="text-sm text-slate-500">{email}</p>

            <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6">
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Display name"</h2>
                <ActionForm action=update_profile attr:class="mt-4 space-y-3">
                    <input
                        type="text"
                        name="display_name"
                        value=display_name
                        placeholder="Your name"
                        class="block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 text-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500"
                    />
                    <Show when=move || profile_msg().is_some()>
                        {move || {
                            let (msg, is_err) = profile_msg().unwrap();
                            let cls = if is_err {
                                "text-rose-600 bg-rose-50 border-rose-200"
                            } else {
                                "text-emerald-600 bg-emerald-50 border-emerald-200"
                            };
                            view! { <p class=format!("text-sm border rounded p-2 {cls}")>{msg}</p> }
                        }}
                    </Show>
                    <button type="submit" class="rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700">
                        "Save"
                    </button>
                </ActionForm>
            </div>

            <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6">
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Change password"</h2>
                <ActionForm action=change_password attr:class="mt-4 space-y-3">
                    <div>
                        <label class="block text-sm text-slate-700 dark:text-slate-300">"Current password"</label>
                        <input type="password" name="current_password" required
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 text-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500"/>
                    </div>
                    <div>
                        <label class="block text-sm text-slate-700 dark:text-slate-300">"New password"</label>
                        <input type="password" name="new_password" required minlength="8"
                            class="mt-1 block w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 text-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500"/>
                        <p class="mt-1 text-xs text-slate-400">"At least 8 characters."</p>
                    </div>
                    <Show when=move || password_msg().is_some()>
                        {move || {
                            let (msg, is_err) = password_msg().unwrap();
                            let cls = if is_err {
                                "text-rose-600 bg-rose-50 border-rose-200"
                            } else {
                                "text-emerald-600 bg-emerald-50 border-emerald-200"
                            };
                            view! { <p class=format!("text-sm border rounded p-2 {cls}")>{msg}</p> }
                        }}
                    </Show>
                    <button type="submit" class="rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700">
                        "Change password"
                    </button>
                </ActionForm>
            </div>

            <PasskeysCard/>
        </div>
    }
}

#[component]
fn PasskeysCard() -> impl IntoView {
    let passkeys = Resource::new(|| (), |_| async move { list_passkeys().await });
    let delete_passkey = ServerAction::<DeletePasskey>::new();
    let new_label = RwSignal::new(String::new());
    let registering = RwSignal::new(false);
    let passkey_status = RwSignal::new(Option::<(String, bool)>::None);

    Effect::new(move |_| {
        if delete_passkey.value().get().is_some() {
            passkeys.refetch();
        }
    });

    let on_register_passkey = move |_| {
        let label = new_label.get_untracked();
        passkey_status.set(None);
        registering.set(true);
        spawn_local(async move {
            let result = register_passkey(label).await;
            registering.set(false);
            match result {
                Ok(()) => {
                    passkey_status.set(Some(("Passkey added.".to_string(), false)));
                    new_label.set(String::new());
                    passkeys.refetch();
                }
                Err(e) => passkey_status.set(Some((e, true))),
            }
        });
    };

    view! {
        <div class="bg-white dark:bg-slate-900 rounded-xl shadow-sm ring-1 ring-slate-200 dark:ring-slate-800 p-6">
            <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Passkeys"</h2>
            <p class="mt-1 text-xs text-slate-500">
                "Sign in without a password using your device's fingerprint, face, or a security key."
            </p>

            <Suspense fallback=|| view! { <p class="mt-3 text-xs text-slate-500">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match passkeys.await {
                        Ok(list) if list.is_empty() => view! {
                            <p class="mt-3 text-xs text-slate-500">"No passkeys yet."</p>
                        }.into_any(),
                        Ok(list) => view! {
                            <ul class="mt-3 space-y-1 text-sm">
                                {list.into_iter().map(|pk| {
                                    let id = pk.id;
                                    view! {
                                        <li class="flex items-center justify-between">
                                            <span>{pk.label.clone()}" · added "{pk.created_at.clone()}</span>
                                            <button
                                                on:click=move |_| { delete_passkey.dispatch(DeletePasskey { id }); }
                                                class="text-xs text-rose-500 hover:text-rose-700"
                                            >
                                                "Remove"
                                            </button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                        Err(_) => view! { <p class="mt-3 text-xs text-rose-500">"Failed to load passkeys."</p> }.into_any(),
                    }
                })}
            </Suspense>

            <div class="mt-4 flex items-center gap-2">
                <input
                    type="text"
                    prop:value=move || new_label.get()
                    on:input=move |ev| new_label.set(event_target_value(&ev))
                    placeholder="e.g. MacBook Touch ID"
                    class="flex-1 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-2 text-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500"
                />
                <button
                    on:click=on_register_passkey
                    disabled=move || registering.get()
                    class="rounded-md bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
                >
                    {move || if registering.get() { "Adding…" } else { "Add passkey" }}
                </button>
            </div>
            <Show when=move || passkey_status.get().is_some()>
                {move || {
                    let (msg, is_err) = passkey_status.get().unwrap();
                    let cls = if is_err {
                        "text-rose-600 bg-rose-50 border-rose-200"
                    } else {
                        "text-emerald-600 bg-emerald-50 border-emerald-200"
                    };
                    view! { <p class=format!("mt-2 text-sm border rounded p-2 {cls}")>{msg}</p> }
                }}
            </Show>
        </div>
    }
}
