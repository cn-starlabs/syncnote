use std::time::Duration;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::client_upload::upload_from_change_event;
use crate::client_ws::{connect_shared_page_ws, SharedPageSocket, WsStatus};
use crate::components::markdown::MarkdownPreview;
use crate::models::{MemberRole, PageEdit, SharedPage, SharedPageMember};
use crate::server::invite_fns::CreateInvite;
use crate::server::shared_page_fns::{
    get_shared_page, list_members, AddMemberByEmail, RemoveMember, RenameSharedPage,
};

#[component]
pub fn SharedPageEditorPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    // Fetch the page and its member roster together in one Resource. Doing the
    // roster as a *second, nested* Resource/Suspense (as this used to) hit a
    // Leptos SSR/hydration marker bug on a hard refresh — "the framework
    // expected a marker node" — that broke hydration of everything after it in
    // the tree, including the editor textarea itself. Folding both into a
    // single already-awaited fetch avoids the nested-Suspense pattern entirely.
    let data = Resource::new(id, |id| async move {
        let page = get_shared_page(id).await?;
        let members = list_members(id).await.unwrap_or_default();
        Ok::<_, ServerFnError>((page, members))
    });

    view! {
        <Suspense fallback=|| view! { <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok((p, m)) => view! { <SharedPageEditor page=p initial_members=m/> }.into_any(),
                    Err(_) => view! { <p class="text-sm text-rose-500">"Page not found."</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn SharedPageEditor(page: SharedPage, initial_members: Vec<SharedPageMember>) -> impl IntoView {
    let page_id = page.id;
    let can_edit = page.my_role.can_edit();
    let is_owner = page.my_role == MemberRole::Owner;
    let members = RwSignal::new(initial_members);

    let title = RwSignal::new(page.title);
    let body = RwSignal::new(page.body);
    let version = RwSignal::new(page.version);
    let synced = RwSignal::new(true);
    let epoch = RwSignal::new(0u32);
    let ws_status = RwSignal::new(WsStatus::Connecting);

    let send_trigger = RwSignal::new(0u64);
    // Off by default: while off, incoming updates (including our own echoed
    // saves) are not applied to the editor, so nothing can interrupt active
    // typing. Our own edits are still sent/saved in the background regardless
    // — this only gates what gets pulled in from the socket, not persistence.
    let live_sync_enabled = RwSignal::new(false);
    // Tracks exactly what body text we last sent over the socket. The server
    // broadcasts every accepted write back to the sender too, and that echo
    // can arrive after we've typed more (debounce + network round-trip both
    // take time). Without this, blindly applying every incoming message would
    // overwrite newer local keystrokes with the older text we just sent —
    // which read as "characters disappearing" while typing.
    let last_sent_body = RwSignal::new(body.get_untracked());

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
                        // Keep `version` current regardless — the next send has to
                        // be based on the server's real version to be accepted —
                        // but only touch the editor's visible text when the user
                        // has opted into live sync.
                        version.set(edit.version);
                        if !live_sync_enabled.get_untracked() {
                            return;
                        }
                        let local_body = body.get_untracked();
                        let is_own_stale_echo = edit.body == last_sent_body.get_untracked() && local_body != edit.body;
                        if is_own_stale_echo {
                            // Our own echo, but we've since typed more — keep the
                            // newer local text; the next debounced send catches
                            // the server up to it.
                            synced.set(false);
                        } else {
                            body.set(edit.body);
                            synced.set(true);
                        }
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
                    let current_body = body.get_untracked();
                    last_sent_body.set(current_body.clone());
                    sock.send(&PageEdit {
                        body: current_body,
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

    // Defined outside the view! macro: turbofish (`::<Vec<_>>`) contains `<`/`>`,
    // which confuses the macro's HTML-like tag parser when written inline as a
    // component prop value (`each=move || ... .collect::<Vec<_>>()`) — it reads
    // as malformed tag syntax. A plain identifier for `each=` sidesteps that.
    let avatar_members = move || members.get().into_iter().take(4).collect::<Vec<_>>();
    // Same reason as avatar_members above: a bare `>` inline in a component
    // prop closure (`when=move || ... > 4`) gets read as closing the `<Show>`
    // tag itself, not as "greater than" — hoisting avoids that ambiguity too.
    let has_overflow_members = move || members.get().len() > 4;

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
                    // Collaborator Avatars — `<For>` (not a plain reactive closure
                    // manually collecting views) is required here: an ad-hoc
                    // `move || list.collect_view()` fragment hit a Leptos SSR/
                    // hydration marker mismatch ("expected a marker node") on a
                    // hard refresh, breaking hydration of everything after it.
                    // `<For>`/`<Show>` are the primitives Leptos actually keeps
                    // SSR and hydration aligned for.
                    <Show when=move || !members.get().is_empty()>
                        <div
                            class="flex items-center -space-x-1.5 mr-1"
                            title=move || format!("{} collaborators on this page", members.get().len())
                        >
                            <For
                                each=avatar_members
                                key=|m| m.user_id
                                children=move |m| {
                                    let initial = m.email.chars().next().unwrap_or('?').to_uppercase().to_string();
                                    view! {
                                        <div class="w-6 h-6 rounded-full bg-brand-600 border-2 border-white dark:border-slate-900 text-[10px] font-bold text-white flex items-center justify-center shadow-xs" title=m.email.clone()>
                                            {initial}
                                        </div>
                                    }
                                }
                            />
                            <Show when=has_overflow_members>
                                <div class="w-6 h-6 rounded-full bg-slate-200 dark:bg-slate-700 border-2 border-white dark:border-slate-900 text-[9px] font-medium text-slate-600 dark:text-slate-300 flex items-center justify-center">
                                    {move || format!("+{}", members.get().len().saturating_sub(4))}
                                </div>
                            </Show>
                        </div>
                    </Show>

                    // WebSocket Health Badge
                    {status_badge}

                    // Sync state badge — while live sync is off this just says so,
                    // since incoming updates aren't being applied to the editor.
                    <span class="text-xs font-medium text-slate-500 dark:text-slate-400 px-2.5 py-1 bg-slate-100 dark:bg-slate-800 rounded-md shrink-0">
                        {move || {
                            if !live_sync_enabled.get() {
                                "Live sync off"
                            } else if synced.get() {
                                "Synced"
                            } else {
                                "Syncing…"
                            }
                        }}
                    </span>

                    // Live sync toggle — off by default so nothing from other
                    // viewers (or a delayed echo of your own save) can interrupt
                    // you mid-typing. Your own edits still save either way.
                    <button
                        type="button"
                        role="switch"
                        aria-checked=move || live_sync_enabled.get().to_string()
                        title="Toggle whether other viewers' live edits appear in your editor"
                        on:click=move |_| live_sync_enabled.update(|v| *v = !*v)
                        class=move || {
                            let base = "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-brand-500/30";
                            if live_sync_enabled.get() {
                                format!("{base} bg-brand-600")
                            } else {
                                format!("{base} bg-slate-300 dark:bg-slate-700")
                            }
                        }
                    >
                        <span
                            class=move || {
                                let base = "inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform";
                                if live_sync_enabled.get() {
                                    format!("{base} translate-x-6")
                                } else {
                                    format!("{base} translate-x-1")
                                }
                            }
                        ></span>
                    </button>
                    <span class="text-xs font-medium text-slate-500 dark:text-slate-400 shrink-0">"Live sync"</span>
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
                                    upload_from_change_event(ev, "shared_page".to_string(), Some(page_id), move |res| {
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
                            <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 text-sm text-slate-500 dark:text-slate-400 shadow-sm">
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

            {is_owner.then(move || view! { <OwnerPanel page_id=page_id members=members/> })}
        </div>
    }
}

#[component]
fn OwnerPanel(page_id: i64, members: RwSignal<Vec<SharedPageMember>>) -> impl IntoView {
    let remove = ServerAction::<RemoveMember>::new();
    Effect::new(move |_| {
        if let Some(Ok(())) = remove.value().get() {
            if let Some(RemoveMember { user_id, .. }) = remove.input().get_untracked() {
                members.update(|list| list.retain(|m| m.user_id != user_id));
            }
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

    let add_member = ServerAction::<AddMemberByEmail>::new();
    let add_email = RwSignal::new(String::new());
    let add_role = RwSignal::new("editor".to_string());
    let add_error = RwSignal::new(Option::<String>::None);
    Effect::new(move |_| {
        if let Some(result) = add_member.value().get() {
            match result {
                Ok(new_member) => {
                    members.update(|list| {
                        if !list.iter().any(|m| m.user_id == new_member.user_id) {
                            list.push(new_member);
                        }
                    });
                    add_email.set(String::new());
                    add_error.set(None);
                }
                Err(e) => add_error.set(Some(e.to_string())),
            }
        }
    });
    let on_add_member = move |_| {
        let email = add_email.get_untracked();
        if !email.trim().is_empty() {
            add_error.set(None);
            add_member.dispatch(AddMemberByEmail { page_id, email, role: add_role.get_untracked() });
        }
    };

    view! {
        <div class="rounded-lg border border-slate-200 dark:border-slate-800 p-4 space-y-4">
            <div>
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Members"</h2>
                <ul class="mt-2 space-y-1 text-sm">
                    <For
                        each=move || members.get()
                        key=|m| m.user_id
                        children=move |m| {
                            let uid = m.user_id;
                            let is_owner_row = m.role == MemberRole::Owner;
                            view! {
                                <li class="flex items-center justify-between">
                                    <span>{m.email}" — "{m.role.as_str()}</span>
                                    <Show when=move || !is_owner_row>
                                        <button
                                            on:click=move |_| { remove.dispatch(RemoveMember { page_id, user_id: uid }); }
                                            class="text-xs text-rose-500 hover:text-rose-700"
                                        >
                                            "Remove"
                                        </button>
                                    </Show>
                                </li>
                            }
                        }
                    />
                </ul>
            </div>

            <div>
                <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Share with a user"</h2>
                <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
                    "If they already have an account, add them directly — no invite link needed."
                </p>
                <div class="mt-2 flex flex-wrap items-center gap-2">
                    <input
                        type="email"
                        placeholder="user@example.com"
                        prop:value=move || add_email.get()
                        on:input=move |ev| add_email.set(event_target_value(&ev))
                        class="flex-1 min-w-40 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-2 py-1.5 text-sm"
                    />
                    <select
                        on:change=move |ev| add_role.set(event_target_value(&ev))
                        class="rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-2 py-1.5 text-sm"
                    >
                        <option value="editor">"Can edit"</option>
                        <option value="viewer">"View only"</option>
                    </select>
                    <button
                        on:click=on_add_member
                        class="rounded-md bg-brand-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-brand-700"
                    >
                        "Share"
                    </button>
                </div>
                <Show when=move || add_error.get().is_some()>
                    <p class="mt-2 text-xs text-rose-500">{move || add_error.get().unwrap_or_default()}</p>
                </Show>
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
