use leptos::prelude::*;

use crate::client_upload::upload_from_change_event;
use crate::server::attachment_fns::{list_my_attachments, DeleteAttachment};
use crate::server::file_share_fns::{
    list_file_share_links, list_file_shares, CreateFileShareLink, RevokeFileShareLink, ShareFileWithUser, UnshareFile,
};

fn human_size(bytes: i64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{bytes} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    }
}

#[component]
pub fn FilesPage() -> impl IntoView {
    let attachments = Resource::new(|| (), |_| async move { list_my_attachments().await });
    let delete = ServerAction::<DeleteAttachment>::new();
    let confirm_delete_id = RwSignal::new(Option::<i64>::None);
    let sharing_id = RwSignal::new(Option::<i64>::None);
    let search_query = RwSignal::new(String::new());
    let upload_error = RwSignal::new(Option::<String>::None);
    let uploading = RwSignal::new(false);

    Effect::new(move |_| {
        if delete.value().get().is_some() {
            confirm_delete_id.set(None);
            attachments.refetch();
        }
    });

    let on_upload = move |ev| {
        upload_error.set(None);
        uploading.set(true);
        upload_from_change_event(ev, "library".to_string(), None, move |res| {
            uploading.set(false);
            match res {
                Ok(_) => attachments.refetch(),
                Err(e) => upload_error.set(Some(e)),
            }
        });
    };

    view! {
        <div>
            <div class="flex items-center justify-between gap-3">
                <div>
                    <h1 class="text-xl font-bold text-slate-900 dark:text-slate-100">"My files"</h1>
                    <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
                        "Your personal drive — upload a file here, then insert it into any note"
                    </p>
                </div>
                <label class="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3.5 py-2 text-sm font-semibold text-white shadow-sm hover:bg-brand-700 transition cursor-pointer shrink-0">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                    </svg>
                    {move || if uploading.get() { "Uploading…" } else { "Upload file" }}
                    <input type="file" class="hidden" disabled=move || uploading.get() on:change=on_upload/>
                </label>
            </div>
            <Show when=move || upload_error.get().is_some()>
                <p class="mt-2 text-sm text-rose-600 bg-rose-50 border border-rose-200 rounded p-2">
                    {move || upload_error.get().unwrap_or_default()}
                </p>
            </Show>

            <Suspense fallback=|| view! { <p class="mt-6 text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match attachments.await {
                        Ok(list) if list.is_empty() => view! {
                            <div class="mt-8 rounded-xl border border-dashed border-slate-300 dark:border-slate-700 p-10 text-center">
                                <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"No files uploaded yet"</h3>
                                <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
                                    "Upload a file above, or attach one directly from a note or shared page."
                                </p>
                            </div>
                        }.into_any(),
                        Ok(list) => {
                            let total_count = list.len();
                            let total_bytes: i64 = list.iter().map(|a| a.byte_size).sum();
                            let all_attachments = RwSignal::new(list);

                            // Hoisted outside the view! macro: `.collect::<Vec<_>>()` and a
                            // bare `>` comparison both contain `<`/`>`, which the macro's
                            // HTML-like tag parser misreads as tag syntax when written inline
                            // as a component prop closure (bit us before in shared_page_editor
                            // and dashboard — see those files for the full story).
                            let filtered_attachments = move || {
                                let q = search_query.get().trim().to_lowercase();
                                all_attachments
                                    .get()
                                    .into_iter()
                                    .filter(move |a| q.is_empty() || a.filename.to_lowercase().contains(&q))
                                    .collect::<Vec<_>>()
                            };
                            let has_results = move || {
                                let q = search_query.get().trim().to_lowercase();
                                q.is_empty() || all_attachments.get().iter().any(|a| a.filename.to_lowercase().contains(&q))
                            };

                            view! {
                                <div class="mt-4 flex flex-col sm:flex-row sm:items-center gap-3 sm:justify-between">
                                    <p class="text-xs text-slate-500 dark:text-slate-400">
                                        {format!("{total_count} file{} · {} used", if total_count == 1 { "" } else { "s" }, human_size(total_bytes))}
                                    </p>
                                    <input
                                        type="text"
                                        placeholder="Search files by name…"
                                        prop:value=move || search_query.get()
                                        on:input=move |ev| search_query.set(event_target_value(&ev))
                                        class="w-full sm:w-64 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 text-sm shadow-sm focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                                    />
                                </div>

                                <Show
                                    when=has_results
                                    fallback=|| view! { <p class="mt-6 text-sm text-slate-500 dark:text-slate-400 text-center">"No files match your search."</p> }
                                >
                                    <ul class="mt-4 divide-y divide-slate-200 dark:divide-slate-800 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm overflow-hidden">
                                        <For
                                            each=filtered_attachments
                                            key=|a| a.id
                                            children=move |a| {
                                                let id = a.id;
                                                let is_image = a.content_type.starts_with("image/");
                                                view! {
                                                    <li class="px-4 py-3 hover:bg-slate-50 dark:hover:bg-slate-800/50 transition">
                                                        <div class="flex items-center justify-between gap-3">
                                                        <div class="flex items-center gap-3 min-w-0">
                                                            <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-slate-100 dark:bg-slate-800 text-slate-400 dark:text-slate-500">
                                                                {if is_image {
                                                                    view! {
                                                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14M4 8h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                                                                        </svg>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"/>
                                                                        </svg>
                                                                    }.into_any()
                                                                }}
                                                            </span>
                                                            <div class="min-w-0">
                                                                <p class="text-sm font-medium text-slate-900 dark:text-slate-100 truncate">{a.filename.clone()}</p>
                                                                <p class="text-xs text-slate-500 dark:text-slate-400">
                                                                    {human_size(a.byte_size)}" · "{a.created_at.clone()}
                                                                    {match &a.shared_by_email {
                                                                        Some(email) => view! { <span>" · Shared by "{email.clone()}</span> }.into_any(),
                                                                        None => match (a.scope.as_str(), &a.scope_link) {
                                                                            ("library", _) => view! { <span>" · Personal library"</span> }.into_any(),
                                                                            (_, Some(link)) => view! {
                                                                                " · "<a href=link.clone() class="text-brand-600 dark:text-brand-400 hover:underline">
                                                                                    {a.scope_title.clone().unwrap_or_default()}
                                                                                </a>
                                                                            }.into_any(),
                                                                            (_, None) => view! { <span class="italic">" · (original note/page deleted)"</span> }.into_any(),
                                                                        },
                                                                    }}
                                                                </p>
                                                            </div>
                                                        </div>
                                                        <div class="flex items-center gap-3 text-xs shrink-0">
                                                            <a href=a.url.clone() target="_blank" rel="noopener" class="text-brand-600 dark:text-brand-400 hover:underline">
                                                                "Open"
                                                            </a>
                                                            <Show when=move || a.is_owner>
                                                                <button
                                                                    on:click=move |_| {
                                                                        sharing_id.update(|v| *v = if *v == Some(id) { None } else { Some(id) });
                                                                    }
                                                                    class="text-brand-600 dark:text-brand-400 hover:underline"
                                                                >
                                                                    "Share"
                                                                </button>
                                                            </Show>
                                                            <Show when=move || a.is_owner>
                                                                {move || if confirm_delete_id.get() == Some(id) {
                                                                    view! {
                                                                        <span class="text-slate-500 dark:text-slate-400">"Delete?"</span>
                                                                        <button
                                                                            on:click=move |_| { delete.dispatch(DeleteAttachment { id }); }
                                                                            class="text-rose-600 font-semibold hover:underline"
                                                                        >
                                                                            "Confirm"
                                                                        </button>
                                                                        <button
                                                                            on:click=move |_| confirm_delete_id.set(None)
                                                                            class="text-slate-400 hover:underline"
                                                                        >
                                                                            "Cancel"
                                                                        </button>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <button
                                                                            on:click=move |_| confirm_delete_id.set(Some(id))
                                                                            title="If this file is still referenced in the note/page's content, that link will break after deleting."
                                                                            class="text-rose-500 hover:underline"
                                                                        >
                                                                            "Delete"
                                                                        </button>
                                                                    }.into_any()
                                                                }}
                                                            </Show>
                                                        </div>
                                                        </div>
                                                        <Show when=move || sharing_id.get() == Some(id)>
                                                            <SharePanel attachment_id=id/>
                                                        </Show>
                                                    </li>
                                                }
                                            }
                                        />
                                    </ul>
                                </Show>
                            }.into_any()
                        }
                        Err(_) => view! { <p class="mt-6 text-sm text-rose-500">"Failed to load files."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}

fn window_origin() -> String {
    #[cfg(feature = "hydrate")]
    {
        web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        String::new()
    }
}

#[component]
fn SharePanel(attachment_id: i64) -> impl IntoView {
    let shares = Resource::new(|| (), move |_| async move { list_file_shares(attachment_id).await });
    let links = Resource::new(|| (), move |_| async move { list_file_share_links(attachment_id).await });

    let share_action = ServerAction::<ShareFileWithUser>::new();
    let unshare_action = ServerAction::<UnshareFile>::new();
    let create_link_action = ServerAction::<CreateFileShareLink>::new();
    let revoke_link_action = ServerAction::<RevokeFileShareLink>::new();

    let share_email = RwSignal::new(String::new());
    let share_error = RwSignal::new(Option::<String>::None);
    let duration_value = RwSignal::new(String::new());
    let duration_unit = RwSignal::new("hours".to_string());
    let max_downloads_input = RwSignal::new(String::new());
    let link_error = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        if let Some(result) = share_action.value().get() {
            match result {
                Ok(_) => {
                    share_email.set(String::new());
                    share_error.set(None);
                    shares.refetch();
                }
                Err(e) => share_error.set(Some(e.to_string())),
            }
        }
    });
    Effect::new(move |_| {
        if unshare_action.value().get().is_some() {
            shares.refetch();
        }
    });
    Effect::new(move |_| {
        if let Some(result) = create_link_action.value().get() {
            match result {
                Ok(_) => {
                    duration_value.set(String::new());
                    max_downloads_input.set(String::new());
                    link_error.set(None);
                    links.refetch();
                }
                Err(e) => link_error.set(Some(e.to_string())),
            }
        }
    });
    Effect::new(move |_| {
        if revoke_link_action.value().get().is_some() {
            links.refetch();
        }
    });

    let on_share = move |_| {
        let email = share_email.get_untracked();
        if !email.trim().is_empty() {
            share_action.dispatch(ShareFileWithUser { attachment_id, email });
        }
    };

    let on_create_link = move |_| {
        let hours = duration_value.get_untracked().trim().parse::<i64>().ok().map(|n| {
            if duration_unit.get_untracked() == "days" {
                n * 24
            } else {
                n
            }
        });
        let max_downloads = max_downloads_input.get_untracked().trim().parse::<i64>().ok();
        create_link_action.dispatch(CreateFileShareLink { attachment_id, expires_in_hours: hours, max_downloads });
    };

    view! {
        <div class="mt-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50 p-3 space-y-4 text-xs">
            <div>
                <h4 class="font-semibold text-slate-700 dark:text-slate-300">"Share with a user"</h4>
                <p class="mt-0.5 text-slate-500 dark:text-slate-400">"They'll see this file in their own Files page."</p>
                <div class="mt-1.5 flex items-center gap-2">
                    <input
                        type="email"
                        placeholder="user@example.com"
                        prop:value=move || share_email.get()
                        on:input=move |ev| share_email.set(event_target_value(&ev))
                        class="flex-1 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-900 px-2 py-1.5 text-xs"
                    />
                    <button on:click=on_share class="rounded-md bg-brand-600 px-3 py-1.5 font-medium text-white hover:bg-brand-700 shrink-0">
                        "Share"
                    </button>
                </div>
                <Show when=move || share_error.get().is_some()>
                    <p class="mt-1 text-rose-500">{move || share_error.get().unwrap_or_default()}</p>
                </Show>
                <Suspense fallback=|| ()>
                    {move || Suspend::new(async move {
                        match shares.await {
                            Ok(list) if !list.is_empty() => view! {
                                <ul class="mt-2 space-y-1">
                                    <For
                                        each=move || list.clone()
                                        key=|s| s.user_id
                                        children=move |s| {
                                            let uid = s.user_id;
                                            view! {
                                                <li class="flex items-center justify-between text-slate-600 dark:text-slate-400">
                                                    <span>{s.email.clone()}</span>
                                                    <button
                                                        on:click=move |_| { unshare_action.dispatch(UnshareFile { attachment_id, user_id: uid }); }
                                                        class="text-rose-500 hover:underline"
                                                    >
                                                        "Remove"
                                                    </button>
                                                </li>
                                            }
                                        }
                                    />
                                </ul>
                            }.into_any(),
                            _ => view! { <span class="hidden"></span> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>

            <div>
                <h4 class="font-semibold text-slate-700 dark:text-slate-300">"Public download link"</h4>
                <p class="mt-0.5 text-slate-500 dark:text-slate-400">"Anyone with the link can download it — no account needed."</p>
                <div class="mt-1.5 flex flex-wrap items-center gap-2">
                    <input
                        type="number"
                        min="1"
                        placeholder="Never expires"
                        prop:value=move || duration_value.get()
                        on:input=move |ev| duration_value.set(event_target_value(&ev))
                        class="w-28 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-900 px-2 py-1.5 text-xs"
                    />
                    <select
                        on:change=move |ev| duration_unit.set(event_target_value(&ev))
                        class="rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-900 px-2 py-1.5 text-xs"
                    >
                        <option value="hours">"Hours"</option>
                        <option value="days">"Days"</option>
                    </select>
                    <input
                        type="number"
                        min="1"
                        placeholder="Unlimited downloads"
                        prop:value=move || max_downloads_input.get()
                        on:input=move |ev| max_downloads_input.set(event_target_value(&ev))
                        class="w-36 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-900 px-2 py-1.5 text-xs"
                    />
                    <button on:click=on_create_link class="rounded-md bg-brand-600 px-3 py-1.5 font-medium text-white hover:bg-brand-700">
                        "Create link"
                    </button>
                </div>
                <Show when=move || link_error.get().is_some()>
                    <p class="mt-1 text-rose-500">{move || link_error.get().unwrap_or_default()}</p>
                </Show>
                <Suspense fallback=|| ()>
                    {move || Suspend::new(async move {
                        match links.await {
                            Ok(list) if !list.is_empty() => view! {
                                <ul class="mt-2 space-y-1.5">
                                    <For
                                        each=move || list.clone()
                                        key=|l| l.token.clone()
                                        children=move |l| {
                                            let token = l.token.clone();
                                            let full_url = format!("{}{}", window_origin(), l.url);
                                            let usage = match l.max_downloads {
                                                Some(max) => format!("{} of {} downloads used", l.download_count, max),
                                                None => format!("{} downloads, unlimited", l.download_count),
                                            };
                                            let expiry = l.expires_at.clone().unwrap_or_else(|| "never expires".to_string());
                                            view! {
                                                <li class="rounded-md border border-slate-200 dark:border-slate-800 p-2 space-y-1">
                                                    <div class="flex items-center justify-between gap-2">
                                                        <span class="font-mono truncate select-all text-slate-700 dark:text-slate-300">{full_url}</span>
                                                        <button
                                                            on:click=move |_| { revoke_link_action.dispatch(RevokeFileShareLink { token: token.clone() }); }
                                                            class="text-rose-500 hover:underline shrink-0"
                                                        >
                                                            "Revoke"
                                                        </button>
                                                    </div>
                                                    <p class="text-slate-400 dark:text-slate-500">{usage}" · expires "{expiry}</p>
                                                </li>
                                            }
                                        }
                                    />
                                </ul>
                            }.into_any(),
                            _ => view! { <span class="hidden"></span> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}
