use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::models::NoteShareInfo;
use crate::server::note_fns::{list_my_notes, CreateNote, DeleteNote};
use crate::server::note_share_fns::{
    create_note_share_link, get_note_share_token, list_note_user_shares, list_notes_shared_with_me,
    revoke_note_share_link, share_note_with_user, unshare_note_from_user,
};

#[component]
pub fn DashboardPage() -> impl IntoView {
    let notes = Resource::new(|| (), |_| async move { list_my_notes().await });
    let shared_with_me = Resource::new(|| (), |_| async move { list_notes_shared_with_me().await });
    let create = ServerAction::<CreateNote>::new();
    let delete = ServerAction::<DeleteNote>::new();
    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(id)) = create.value().get() {
            notes.refetch();
            navigate(&format!("/app/note/{id}"), Default::default());
        }
    });
    Effect::new(move |_| {
        if delete.value().get().is_some() {
            notes.refetch();
        }
    });

    let confirm_delete_id = RwSignal::new(Option::<i64>::None);
    let search_query = RwSignal::new(String::new());

    // ── Share modal state ─────────────────────────────────────────────────────
    // Which note's share panel is open (None = closed)
    let share_modal_note_id = RwSignal::new(Option::<i64>::None);
    let share_token: RwSignal<Option<String>> = RwSignal::new(None);
    let share_users: RwSignal<Vec<NoteShareInfo>> = RwSignal::new(vec![]);
    let share_loading = RwSignal::new(false);
    let share_error = RwSignal::new(Option::<String>::None);
    let share_copied = RwSignal::new(false);
    let share_email_input = RwSignal::new(String::new());
    let share_email_error = RwSignal::new(Option::<String>::None);
    let share_email_pending = RwSignal::new(false);
    let share_email_info = RwSignal::new(Option::<String>::None);
    let share_new_password = RwSignal::new(String::new());

    let open_share_modal = move |note_id: i64| {
        share_modal_note_id.set(Some(note_id));
        share_token.set(None);
        share_users.set(vec![]);
        share_error.set(None);
        share_email_input.set(String::new());
        share_email_error.set(None);
        share_email_info.set(None);
        share_new_password.set(String::new());
        share_loading.set(true);
        spawn_local(async move {
            let token_res = get_note_share_token(note_id).await;
            let users_res = list_note_user_shares(note_id).await;
            match token_res {
                Ok(t) => share_token.set(t),
                Err(e) => share_error.set(Some(e.to_string())),
            }
            match users_res {
                Ok(u) => share_users.set(u),
                Err(_) => {}
            }
            share_loading.set(false);
        });
    };

    let close_share_modal = move |_| {
        share_modal_note_id.set(None);
    };

    let create_link = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(note_id) = share_modal_note_id.get_untracked() {
            let pw = share_new_password.get_untracked();
            let password = if pw.trim().is_empty() { None } else { Some(pw) };
            share_error.set(None);
            share_loading.set(true);
            spawn_local(async move {
                match create_note_share_link(note_id, password).await {
                    Ok(token) => {
                        share_token.set(Some(token));
                        share_new_password.set(String::new());
                    }
                    Err(e) => share_error.set(Some(e.to_string())),
                }
                share_loading.set(false);
            });
        }
    };

    let revoke_link = move |_| {
        if let Some(note_id) = share_modal_note_id.get_untracked() {
            share_error.set(None);
            share_loading.set(true);
            spawn_local(async move {
                match revoke_note_share_link(note_id).await {
                    Ok(()) => share_token.set(None),
                    Err(e) => share_error.set(Some(e.to_string())),
                }
                share_loading.set(false);
            });
        }
    };

    let copy_link = move |_| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::prelude::*;
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = ["window", "navigator", "clipboard"], js_name = writeText)]
                fn clipboard_write(s: &str);
            }
            if let Some(token) = share_token.get_untracked() {
                let url = format!(
                    "{}/note/shared/{}",
                    web_sys::window().unwrap().location().origin().unwrap_or_default(),
                    token
                );
                clipboard_write(&url);
                share_copied.set(true);
                leptos::prelude::set_timeout(
                    move || share_copied.set(false),
                    std::time::Duration::from_secs(2),
                );
            }
        }
    };

    let add_user_share = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(note_id) = share_modal_note_id.get_untracked() else { return; };
        let email = share_email_input.get_untracked();
        if email.trim().is_empty() { return; }
        share_email_error.set(None);
        share_email_info.set(None);
        share_email_pending.set(true);
        spawn_local(async move {
            match share_note_with_user(note_id, email).await {
                Ok(crate::models::ShareOutcome::SharedWithUser(info)) => {
                    share_users.update(|v| v.push(info));
                    share_email_input.set(String::new());
                }
                Ok(crate::models::ShareOutcome::LinkEmailed) => {
                    share_email_info.set(Some(
                        "No SyncNote account for that email — emailed them a read-only link instead.".to_string(),
                    ));
                    share_email_input.set(String::new());
                }
                Err(e) => share_email_error.set(Some(e.to_string())),
            }
            share_email_pending.set(false);
        });
    };

    let remove_user = move |note_id: i64, uid: i64| {
        spawn_local(async move {
            if unshare_note_from_user(note_id, uid).await.is_ok() {
                share_users.update(|v| v.retain(|u| u.user_id != uid));
            }
        });
    };

    view! {
        <div>
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-xl font-bold text-slate-900 dark:text-slate-100">"My notes"</h1>
                    <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">"All your private notes in one place"</p>
                </div>
                <button
                    on:click=move |_| { create.dispatch(CreateNote {}); }
                    class="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3.5 py-2 text-sm font-semibold text-white shadow-sm hover:bg-brand-700 transition"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New note"
                </button>
            </div>

            // Delete Confirmation Modal
            <Show when=move || confirm_delete_id.get().is_some()>
                <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/40 backdrop-blur-xs">
                    <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-xl max-w-sm w-full p-5 space-y-4">
                        <div class="flex items-start gap-3">
                            <div class="flex-shrink-0 w-8 h-8 rounded-full bg-rose-100 dark:bg-rose-900/40 flex items-center justify-center text-rose-600 dark:text-rose-400">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                </svg>
                            </div>
                            <div>
                                <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Delete note?"</h3>
                                <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">"This action will permanently delete this note. You cannot undo this."</p>
                            </div>
                        </div>
                        <div class="flex justify-end gap-2 pt-2">
                            <button
                                type="button"
                                on:click=move |_| confirm_delete_id.set(None)
                                class="rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-1.5 text-xs font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700 transition"
                            >
                                "Cancel"
                            </button>
                            <button
                                type="button"
                                on:click=move |_| {
                                    if let Some(target_id) = confirm_delete_id.get_untracked() {
                                        delete.dispatch(DeleteNote { id: target_id });
                                        confirm_delete_id.set(None);
                                    }
                                }
                                class="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-rose-700 transition"
                            >
                                "Delete permanently"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Share Modal Overlay
            <Show when=move || share_modal_note_id.get().is_some()>
                <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/40 backdrop-blur-xs">
                    <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-xl w-full max-w-md p-5 space-y-4">
                        // Header
                        <div class="flex items-center justify-between">
                            <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Share note"</h3>
                            <button on:click=close_share_modal class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 text-lg leading-none">"✕"</button>
                        </div>

                        <Show when=move || share_error.get().is_some()>
                            <p class="text-xs text-rose-500 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800/60 rounded p-2">
                                {move || share_error.get().unwrap_or_default()}
                            </p>
                        </Show>

                        // ── Public link section ───────────────────────────────
                        <div class="space-y-2">
                            <p class="text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">"Public link"</p>
                            {move || {
                                if share_loading.get() {
                                    view! { <p class="text-xs text-slate-500 dark:text-slate-400">"Loading…"</p> }.into_any()
                                } else if let Some(token) = share_token.get() {
                                    let share_url = format!("/note/shared/{token}");
                                    view! {
                                        <div class="space-y-2">
                                            <p class="text-xs text-slate-500 dark:text-slate-400">
                                                "Anyone with this link can view a read-only copy."
                                            </p>
                                            <div class="flex items-center gap-2">
                                                <input
                                                    type="text"
                                                    readonly
                                                    prop:value=share_url.clone()
                                                    class="flex-1 min-w-0 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs font-mono text-slate-700 dark:text-slate-300 focus:outline-none"
                                                />
                                                <button
                                                    type="button"
                                                    on:click=copy_link
                                                    class="shrink-0 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 transition"
                                                >
                                                    {move || if share_copied.get() { "Copied!" } else { "Copy" }}
                                                </button>
                                            </div>
                                            <div class="flex items-center gap-3">
                                                <a href=share_url target="_blank" rel="external noopener"
                                                    class="text-xs text-brand-600 dark:text-brand-400 hover:underline">
                                                    "Open link ↗"
                                                </a>
                                                <button type="button" on:click=revoke_link
                                                    class="text-xs text-rose-500 dark:text-rose-400 hover:underline">
                                                    "Revoke link"
                                                </button>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <form on:submit=create_link class="space-y-2">
                                            <p class="text-xs text-slate-500 dark:text-slate-400">
                                                "Create a public read-only link anyone can view."
                                            </p>
                                            <div class="flex items-center gap-2">
                                                <input
                                                    type="password"
                                                    placeholder="Protect with password (optional)"
                                                    prop:value=move || share_new_password.get()
                                                    on:input=move |ev| share_new_password.set(event_target_value(&ev))
                                                    class="flex-1 min-w-0 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs focus:border-brand-500 focus:outline-none"
                                                />
                                                <button
                                                    type="submit"
                                                    disabled=move || share_loading.get()
                                                    class="shrink-0 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 disabled:opacity-60 transition"
                                                >
                                                    {move || if share_loading.get() { "Creating…" } else { "Create share link" }}
                                                </button>
                                            </div>
                                        </form>
                                    }.into_any()
                                }
                            }}
                        </div>

                        // Divider
                        <div class="border-t border-slate-200 dark:border-slate-700"/>

                        // ── Share with specific people ────────────────────────
                        <div class="space-y-2">
                            <p class="text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                                "Share with people"
                            </p>
                            <form on:submit=add_user_share class="flex items-center gap-2">
                                <input
                                    type="email"
                                    required
                                    placeholder="colleague@example.com"
                                    prop:value=move || share_email_input.get()
                                    on:input=move |ev| share_email_input.set(event_target_value(&ev))
                                    class="flex-1 min-w-0 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs focus:border-brand-500 focus:outline-none"
                                />
                                <button
                                    type="submit"
                                    disabled=move || share_email_pending.get()
                                    class="shrink-0 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 disabled:opacity-60 transition"
                                >
                                    {move || if share_email_pending.get() { "Adding…" } else { "Add" }}
                                </button>
                            </form>

                            <Show when=move || share_email_error.get().is_some()>
                                <p class="text-xs text-rose-500">{move || share_email_error.get().unwrap_or_default()}</p>
                            </Show>
                            <Show when=move || share_email_info.get().is_some()>
                                <p class="text-xs text-emerald-600 dark:text-emerald-400">{move || share_email_info.get().unwrap_or_default()}</p>
                            </Show>

                            // List of already-shared users
                            {move || {
                                let users = share_users.get();
                                let note_id = share_modal_note_id.get().unwrap_or(0);
                                if users.is_empty() {
                                    view! {
                                        <p class="text-xs text-slate-400 dark:text-slate-500 italic">
                                            "Not shared with anyone yet."
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <ul class="space-y-1 mt-1">
                                            <For
                                                each=move || share_users.get()
                                                key=|u| u.user_id
                                                children=move |u| {
                                                    let uid = u.user_id;
                                                    let remove = remove_user.clone();
                                                    view! {
                                                        <li class="flex items-center justify-between rounded-md bg-slate-50 dark:bg-slate-800 px-3 py-1.5">
                                                            <span class="text-xs text-slate-700 dark:text-slate-300 truncate">{u.email.clone()}</span>
                                                            <button
                                                                type="button"
                                                                on:click=move |_| remove(note_id, uid)
                                                                class="ml-2 text-xs text-rose-400 hover:text-rose-600 dark:hover:text-rose-300 shrink-0"
                                                                title="Remove access"
                                                            >
                                                                "✕"
                                                            </button>
                                                        </li>
                                                    }
                                                }
                                            />
                                        </ul>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </div>
            </Show>

            // ── My notes list ─────────────────────────────────────────────────
            <Suspense fallback=|| view! { <p class="mt-6 text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match notes.await {
                        Ok(list) if list.is_empty() => view! {
                            <div class="mt-8 rounded-xl border border-dashed border-slate-300 dark:border-slate-700 p-10 text-center">
                                <div class="w-12 h-12 mx-auto rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-slate-400">
                                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                                    </svg>
                                </div>
                                <h3 class="mt-3 text-sm font-semibold text-slate-900 dark:text-slate-100">"No notes yet"</h3>
                                <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">"Get started by creating your first note or document."</p>
                                <button
                                    on:click=move |_| { create.dispatch(CreateNote {}); }
                                    class="mt-4 inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 transition"
                                >
                                    "Create first note"
                                </button>
                            </div>
                        }.into_any(),
                        Ok(list) => {
                            let all_notes = RwSignal::new(list);
                            let filtered_notes = move || {
                                let q = search_query.get().trim().to_lowercase();
                                all_notes.get().into_iter().filter(move |n| {
                                    q.is_empty() || n.title.to_lowercase().contains(&q) || n.body.to_lowercase().contains(&q)
                                }).collect::<Vec<_>>()
                            };
                            let has_results = move || {
                                let q = search_query.get().trim().to_lowercase();
                                q.is_empty() || all_notes.get().iter().any(|n| {
                                    n.title.to_lowercase().contains(&q) || n.body.to_lowercase().contains(&q)
                                })
                            };
                            view! {
                                <div class="mt-4 relative">
                                    <svg class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M11 19a8 8 0 100-16 8 8 0 000 16z"/>
                                    </svg>
                                    <input
                                        type="text"
                                        placeholder="Search notes by title or content…"
                                        prop:value=move || search_query.get()
                                        on:input=move |ev| search_query.set(event_target_value(&ev))
                                        class="w-full rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 pl-9 pr-3 py-2 text-sm shadow-sm focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                                    />
                                </div>

                                <Show
                                    when=has_results
                                    fallback=|| view! { <p class="mt-6 text-sm text-slate-500 dark:text-slate-400 text-center">"No notes match your search."</p> }
                                >
                                    <ul class="mt-6 divide-y divide-slate-200 dark:divide-slate-800 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm overflow-hidden">
                                        <For
                                            each=filtered_notes
                                            key=|n| n.id
                                            children=move |note| {
                                                let id = note.id;
                                                let title = if note.title.trim().is_empty() {
                                                    "Untitled note".to_string()
                                                } else {
                                                    note.title.clone()
                                                };
                                                view! {
                                                    <li class="flex items-center justify-between px-4 py-3.5 hover:bg-slate-50 dark:hover:bg-slate-800/50 transition">
                                                        <A href=format!("/app/note/{id}") attr:class="text-sm font-semibold text-slate-900 dark:text-slate-100 hover:text-brand-600 dark:hover:text-brand-400 truncate mr-3">
                                                            {title}
                                                        </A>
                                                        <div class="flex items-center gap-2 shrink-0">
                                                            <span class="text-xs text-slate-500 dark:text-slate-400 hidden sm:inline">{note.updated_at.clone()}</span>
                                                            // Share button
                                                            <button
                                                                on:click=move |_| open_share_modal(id)
                                                                class="inline-flex items-center gap-1 text-xs text-slate-400 hover:text-brand-600 dark:hover:text-brand-400 transition"
                                                                title="Share note"
                                                            >
                                                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                                                        d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"/>
                                                                </svg>
                                                                "Share"
                                                            </button>
                                                            // Delete button
                                                            <button
                                                                on:click=move |_| { confirm_delete_id.set(Some(id)); }
                                                                class="inline-flex items-center gap-1 text-xs text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 transition"
                                                                title="Delete note"
                                                            >
                                                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                                                </svg>
                                                                "Delete"
                                                            </button>
                                                        </div>
                                                    </li>
                                                }
                                            }
                                        />
                                    </ul>
                                </Show>
                            }.into_any()
                        }
                        Err(_) => view! { <p class="mt-6 text-sm text-rose-500">"Failed to load notes."</p> }.into_any(),
                    }
                })}
            </Suspense>

            // ── Shared with me ────────────────────────────────────────────────
            <div class="mt-10">
                <div class="mb-4">
                    <h2 class="text-base font-bold text-slate-900 dark:text-slate-100">"Shared with me"</h2>
                    <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">"Notes other users have shared with you (read-only)"</p>
                </div>
                <Suspense fallback=|| view! { <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
                    {move || Suspend::new(async move {
                        match shared_with_me.await {
                            Ok(list) if list.is_empty() => view! {
                                <p class="text-sm text-slate-400 dark:text-slate-500 italic">"No notes shared with you yet."</p>
                            }.into_any(),
                            Ok(list) => view! {
                                <ul class="divide-y divide-slate-200 dark:divide-slate-800 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm overflow-hidden">
                                    <For
                                        each=move || list.clone()
                                        key=|n| n.id
                                        children=move |note| {
                                            let id = note.id;
                                            let title = if note.title.trim().is_empty() { "Untitled".to_string() } else { note.title.clone() };
                                            let share_url = format!("/note/shared/");
                                            // Use the note id to load a view — shared with me uses note_id directly
                                            // We link to the read-only shared view by finding the token — for simplicity
                                            // link to the note editor (they are allowed read via note_shares).
                                            // Actually, since they only have a note_shares entry (no token), we can't
                                            // use the public URL. We'll add a server fn get_shared_note_for_user.
                                            // For now link to a dedicated "shared note" page using the note id.
                                            view! {
                                                <li class="flex items-center justify-between px-4 py-3.5 hover:bg-slate-50 dark:hover:bg-slate-800/50 transition">
                                                    <a
                                                        href=format!("/note/shared-user/{id}")
                                                        class="text-sm font-semibold text-slate-900 dark:text-slate-100 hover:text-brand-600 dark:hover:text-brand-400 truncate mr-3"
                                                    >
                                                        {title}
                                                    </a>
                                                    <span class="text-xs text-slate-500 dark:text-slate-400 shrink-0">{note.updated_at.clone()}</span>
                                                </li>
                                            }
                                        }
                                    />
                                </ul>
                            }.into_any(),
                            Err(_) => view! { <p class="text-sm text-rose-500">"Failed to load shared notes."</p> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}
