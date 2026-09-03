use std::time::Duration;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::client_upload::upload_from_change_event;
use crate::client_ws::{connect_shared_page_ws, SharedPageSocket, WsStatus};
use crate::components::markdown::MarkdownPreview;
use crate::models::{MemberRole, PageEdit, SharedPage};
use crate::server::invite_fns::CreateInvite;
use crate::server::shared_page_fns::{list_members, RemoveMember, RenameSharedPage};

#[component]
pub fn SharedPageEditorPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let page = Resource::new(id, |id| async move { crate::server::shared_page_fns::get_shared_page(id).await });

    view! {
        <Suspense fallback=|| view! { <p class="text-sm text-slate-500">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match page.await {
                    Ok(p) => view! { <SharedPageEditor page=p/> }.into_any(),
                    Err(_) => view! { <p class="text-sm text-rose-500">"Page not found."</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn SharedPageEditor(page: SharedPage) -> impl IntoView {
    let page_id = page.id;
    let can_edit = page.my_role.can_edit();
    let is_owner = page.my_role == MemberRole::Owner;

    let title = RwSignal::new(page.title);
    let body = RwSignal::new(page.body);
    let version = RwSignal::new(page.version);
    let synced = RwSignal::new(true);
    let epoch = RwSignal::new(0u32);
    let ws_status = RwSignal::new(WsStatus::Connecting);
    let members = Resource::new(|| (), move |_| async move { list_members(page_id).await });

    let send_trigger = RwSignal::new(0u64);

    #[cfg(feature = "hydrate")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        let socket: Rc<RefCell<Option<SharedPageSocket>>> = Rc::new(RefCell::new(None));
        Effect::new({
            let socket = socket.clone();
            move |_| {
                let sock = connect_shared_page_ws(
                    page_id,
                    move |edit: PageEdit| {
                        body.set(edit.body);
                        version.set(edit.version);
                        synced.set(true);
                    },
                    move |st: WsStatus| {
                        ws_status.set(st);
                    },
                );
                *socket.borrow_mut() = Some(sock);
            }
        });

        Effect::new(move |_| {
            let tick = send_trigger.get();
            if tick > 0 {
                if let Some(sock) = socket.borrow().as_ref() {
                    sock.send(&PageEdit {
                        body: body.get_untracked(),
                        version: version.get_untracked(),
                    });
                }
            }
        });
    }

    let upload_error = RwSignal::new(Option::<String>::None);

    let rename = ServerAction::<RenameSharedPage>::new();
    let on_title_input = move |ev| {
        let new_title = event_target_value(&ev);
        title.set(new_title.clone());
        rename.dispatch(RenameSharedPage { id: page_id, title: new_title });
    };

    let view_mode = RwSignal::new("split"); // "split" | "edit" | "preview"

    let status_badge = move || {
        match ws_status.get() {
            WsStatus::Connected => view! {
                <span class="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-700 dark:text-emerald-300 px-2.5 py-1 bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-200 dark:border-emerald-800/60 rounded-full shrink-0 shadow-xs">
                    <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
                    "Live"
                </span>
            }.into_any(),
            WsStatus::Connecting => view! {
                <span class="inline-flex items-center gap-1.5 text-xs font-medium text-amber-700 dark:text-amber-300 px-2.5 py-1 bg-amber-50 dark:bg-amber-950/60 border border-amber-200 dark:border-amber-800/60 rounded-full shrink-0 shadow-xs">
                    <span class="w-2 h-2 rounded-full bg-amber-500 animate-ping"></span>
                    "Connecting…"
                </span>
            }.into_any(),
            WsStatus::Disconnected | WsStatus::Error => view! {
                <span class="inline-flex items-center gap-1.5 text-xs font-medium text-rose-700 dark:text-rose-300 px-2.5 py-1 bg-rose-50 dark:bg-rose-950/60 border border-rose-200 dark:border-rose-800/60 rounded-full shrink-0 shadow-xs">
                    <span class="w-2 h-2 rounded-full bg-rose-500"></span>
                    "Offline"
                </span>
            }.into_any(),
        }
    };

    view! {
        <div class="space-y-6">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                <div class="flex items-center gap-3 flex-1 min-w-0">
                    {if is_owner {
                        view! {
                            <input
                                type="text"
                                prop:value=move || title.get()
                                on:input=on_title_input
                                placeholder="Shared page title…"
                                class="flex-1 text-xl font-semibold bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 hover:border-slate-400 dark:hover:border-slate-600 rounded-lg px-3.5 py-2 text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 shadow-sm focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                            />
                        }.into_any()
                    } else {
                        view! { <h1 class="flex-1 text-xl font-bold text-slate-900 dark:text-slate-100 px-1">{move || title.get()}</h1> }.into_any()
                    }}
                </div>

                <div class="flex items-center gap-2 self-end sm:self-center">
                    // Collaborator Avatars
                    <Suspense fallback=|| ()>
                        {move || Suspend::new(async move {
                            match members.await {
                                Ok(list) => {
                                    let total = list.len();
                                    view! {
                                        <div class="flex items-center -space-x-1.5 mr-1" title=format!("{total} collaborators on this page")>
                                            {list.into_iter().take(4).map(|m| {
                                                let initial = m.email.chars().next().unwrap_or('?').to_uppercase().to_string();
                                                view! {
                                                    <div class="w-6 h-6 rounded-full bg-brand-600 border-2 border-white dark:border-slate-900 text-[10px] font-bold text-white flex items-center justify-center shadow-xs" title=m.email.clone()>
                                                        {initial}
                                                    </div>
                                                }
                                            }).collect_view()}
                                            {if total > 4 {
                                                view! {
                                                    <div class="w-6 h-6 rounded-full bg-slate-200 dark:bg-slate-700 border-2 border-white dark:border-slate-900 text-[9px] font-medium text-slate-600 dark:text-slate-300 flex items-center justify-center">
                                                        {format!("+{}", total - 4)}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span class="hidden"></span> }.into_any()
                                            }}
                                        </div>
                                    }.into_any()
                                }
                                Err(_) => view! { <span class="hidden"></span> }.into_any(),
                            }
                        })}
                    </Suspense>

                    // WebSocket Health Badge
                    {status_badge}

                    // Sync state badge
                    <span class="text-xs font-medium text-slate-400 dark:text-slate-500 px-2.5 py-1 bg-slate-100 dark:bg-slate-800 rounded-md shrink-0">
                        {move || if synced.get() { "Synced" } else { "Syncing…" }}
                    </span>
                </div>
            </div>

            {can_edit.then(move || {
                view! {
                    <div class="flex items-center gap-3">
                        <label class="text-sm rounded-md border border-slate-300 dark:border-slate-700 px-3 py-1.5 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800">
                            "Attach file"
                            <input
                                type="file"
                                class="hidden"
                                on:change=move |ev| {
                                    upload_error.set(None);
                                    upload_from_change_event(ev, "shared_page".to_string(), page_id, move |res| {
                                        match res {
                                            Ok(u) => {
                                                let md = if u.content_type.starts_with("image/") {
                                                    format!("\n\n![{}]({})\n\n", u.filename, u.url)
                                                } else {
                                                    format!("\n\n[{}]({})\n\n", u.filename, u.url)
                                                };
                                                body.update(|b| b.push_str(&md));
                                                synced.set(false);
                                                send_trigger.update(|t| *t = t.wrapping_add(1));
                                            }
                                            Err(e) => upload_error.set(Some(e)),
                                        }
                                    });
                                }
                            />
                        </label>
                        <Show when=move || upload_error.get().is_some()>
                            <span class="text-xs text-rose-500">{move || upload_error.get().unwrap_or_default()}</span>
                        </Show>
                    </div>
                }
            })}

            // Quick format bar & view mode
            {can_edit.then(move || {
                let insert_snippet = move |prefix: &'static str, suffix: &'static str, placeholder: &'static str| {
                    body.update(|b| {
                        if b.ends_with('\n') || b.is_empty() {
                            b.push_str(&format!("{prefix}{placeholder}{suffix}"));
                        } else {
                            b.push_str(&format!("\n{prefix}{placeholder}{suffix}"));
                        }
                    });
                    synced.set(false);
                    send_trigger.update(|t| *t = t.wrapping_add(1));
                };

                view! {
                    <div class="flex flex-wrap items-center justify-between gap-2.5 pb-1">
                        <div class="flex flex-wrap items-center gap-1.5 text-xs">
                            <button
                                type="button"
                                on:click={
                                    let insert = insert_snippet.clone();
                                    move |_| insert("**", "**", "bold text")
                                }
                                title="Bold"
                                class="px-2.5 py-1 font-semibold rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                            >
                                "B"
                            </button>
                            <button
                                type="button"
                                on:click={
                                    let insert = insert_snippet.clone();
                                    move |_| insert("*", "*", "italic text")
                                }
                                title="Italic"
                                class="px-2.5 py-1 italic rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                            >
                                "I"
                            </button>
                            <button
                                type="button"
                                on:click={
                                    let insert = insert_snippet.clone();
                                    move |_| insert("$", "$", "E = mc^2")
                                }
                                title="Inline Math"
                                class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md border border-brand-200 dark:border-brand-900/60 bg-brand-50/70 dark:bg-brand-950/40 text-brand-700 dark:text-brand-300 hover:bg-brand-100/70 dark:hover:bg-brand-900/40 transition font-mono font-medium"
                            >
                                "$f(x)$"
                            </button>
                            <button
                                type="button"
                                on:click={
                                    let insert = insert_snippet.clone();
                                    move |_| insert("$$\n", "\n$$", "\\sum_{i=1}^{n} x_i")
                                }
                                title="Block Math"
                                class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md border border-brand-200 dark:border-brand-900/60 bg-brand-50/70 dark:bg-brand-950/40 text-brand-700 dark:text-brand-300 hover:bg-brand-100/70 dark:hover:bg-brand-900/40 transition font-mono font-medium"
                            >
                                "$$ Block $$"
                            </button>
                            <button
                                type="button"
                                on:click={
                                    let insert = insert_snippet.clone();
                                    move |_| insert("```\n", "\n```", "// code")
                                }
                                title="Code block"
                                class="px-2 py-1 font-mono text-[11px] rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                            >
                                "{ }"
                            </button>
                            <button
                                type="button"
                                on:click={
                                    let insert = insert_snippet.clone();
                                    move |_| insert("- [ ] ", "", "task")
                                }
                                title="Task checklist"
                                class="px-2 py-1 rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                            >
                                "☑ Task"
                            </button>
                        </div>

                        <div class="inline-flex rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 p-0.5 text-xs shadow-xs">
                            <button
                                type="button"
                                on:click=move |_| view_mode.set("edit")
                                class=move || {
                                    if view_mode.get() == "edit" {
                                        "px-2.5 py-1 rounded-md bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-300 font-medium"
                                    } else {
                                        "px-2.5 py-1 rounded-md text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                                    }
                                }
                            >
                                "Edit"
                            </button>
                            <button
                                type="button"
                                on:click=move |_| view_mode.set("split")
                                class=move || {
                                    if view_mode.get() == "split" {
                                        "px-2.5 py-1 rounded-md bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-300 font-medium"
                                    } else {
                                        "px-2.5 py-1 rounded-md text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                                    }
                                }
                            >
                                "Split"
                            </button>
                            <button
                                type="button"
                                on:click=move |_| view_mode.set("preview")
                                class=move || {
                                    if view_mode.get() == "preview" {
                                        "px-2.5 py-1 rounded-md bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-300 font-medium"
                                    } else {
                                        "px-2.5 py-1 rounded-md text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                                    }
                                }
                            >
                                "Preview"
                            </button>
                        </div>
                    </div>
                }
            })}

            <div class=move || {
                match view_mode.get() {
                    "edit" => "grid grid-cols-1 gap-4",
                    "preview" => "grid grid-cols-1 gap-4",
                    _ => "grid grid-cols-1 md:grid-cols-2 gap-4",
                }
            }>
                <Show when=move || view_mode.get() != "preview">
                    {if can_edit {
                        view! {
                            <textarea
                                prop:value=move || body.get()
                                on:input=move |ev| {
                                    body.set(event_target_value(&ev));
                                    synced.set(false);
                                    let my_epoch = epoch.get_untracked() + 1;
                                    epoch.set(my_epoch);
                                    set_timeout(
                                        move || {
                                            if epoch.get_untracked() == my_epoch {
                                                send_trigger.update(|t| *t = t.wrapping_add(1));
                                            }
                                        },
                                        Duration::from_millis(400),
                                    );
                                }
                                rows="20"
                                placeholder="Write Markdown (supports $math$ and $$block math$$) — edits sync live to everyone viewing this page…"
                                class="w-full rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-4 font-mono text-sm leading-relaxed text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 hover:border-slate-400 dark:hover:border-slate-600 focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                            ></textarea>
                        }.into_any()
                    } else {
                        view! {
                            <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 text-sm text-slate-500 shadow-sm">
                                "You have view-only access to this page."
                            </div>
                        }.into_any()
                    }}
                </Show>
                <Show when=move || view_mode.get() != "edit">
                    <div class="rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-4 overflow-auto min-h-[400px]">
                        <MarkdownPreview body=Signal::derive(move || body.get())/>
                    </div>
                </Show>
            </div>

            {is_owner.then(|| view! { <OwnerPanel page_id=page_id/> })}
        </div>
    }
}

#[component]
fn OwnerPanel(page_id: i64) -> impl IntoView {
    let members = Resource::new(|| (), move |_| async move { list_members(page_id).await });
    let remove = ServerAction::<RemoveMember>::new();
    Effect::new(move |_| {
        if remove.value().get().is_some() {
            members.refetch();
        }
    });

    let invite = ServerAction::<CreateInvite>::new();
    let role = RwSignal::new("editor".to_string());
    let invite_link = move || {
        invite.value().get().and_then(|r| r.ok()).map(|token| {
            let origin = window_origin();
            format!("{origin}/join/{token}")
        })
    };

    view! {
        <div class="rounded-lg border border-slate-200 dark:border-slate-800 p-4 space-y-4">
            <div>
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Members"</h2>
                <Suspense fallback=|| view! { <p class="text-xs text-slate-500 mt-2">"Loading…"</p> }>
                    {move || Suspend::new(async move {
                        match members.await {
                            Ok(list) => view! {
                                <ul class="mt-2 space-y-1 text-sm">
                                    {list.into_iter().map(|m| {
                                        let uid = m.user_id;
                                        view! {
                                            <li class="flex items-center justify-between">
                                                <span>{m.email}" — "{m.role.as_str()}</span>
                                                {(m.role != MemberRole::Owner).then(|| view! {
                                                    <button
                                                        on:click=move |_| { remove.dispatch(RemoveMember { page_id, user_id: uid }); }
                                                        class="text-xs text-rose-500 hover:text-rose-700"
                                                    >
                                                        "Remove"
                                                    </button>
                                                })}
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            }.into_any(),
                            Err(_) => view! { <p class="text-xs text-rose-500 mt-2">"Failed to load members."</p> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>

            <div>
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Invite link"</h2>
                <div class="mt-2 flex items-center gap-2">
                    <select
                        on:change=move |ev| role.set(event_target_value(&ev))
                        class="rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-2 py-1.5 text-sm"
                    >
                        <option value="editor">"Can edit"</option>
                        <option value="viewer">"View only"</option>
                    </select>
                    <button
                        on:click=move |_| { invite.dispatch(CreateInvite { page_id, role: role.get_untracked(), expires_in_hours: None }); }
                        class="rounded-md bg-brand-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-brand-700"
                    >
                        "Generate link"
                    </button>
                </div>
                {move || invite_link().map(|link| view! {
                    <p class="mt-2 text-xs text-slate-600 dark:text-slate-400 break-all select-all">{link}</p>
                })}
            </div>
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
