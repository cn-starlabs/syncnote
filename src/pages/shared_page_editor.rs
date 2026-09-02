use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::client_upload::upload_from_change_event;
use crate::client_ws::{connect_shared_page_ws, SharedPageSocket};
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

    let socket: Rc<RefCell<Option<SharedPageSocket>>> = Rc::new(RefCell::new(None));
    {
        let socket = socket.clone();
        Effect::new(move |_| {
            let sock = connect_shared_page_ws(page_id, move |edit: PageEdit| {
                body.set(edit.body);
                version.set(edit.version);
                synced.set(true);
            });
            *socket.borrow_mut() = Some(sock);
        });
    }

    let send_edit = {
        let socket = socket.clone();
        move || {
            if let Some(sock) = socket.borrow().as_ref() {
                sock.send(&PageEdit {
                    body: body.get_untracked(),
                    version: version.get_untracked(),
                });
            }
        }
    };

    let send_edit_for_upload = send_edit.clone();
    let upload_error = RwSignal::new(Option::<String>::None);

    let on_body_input = move |ev| {
        body.set(event_target_value(&ev));
        synced.set(false);
        let my_epoch = epoch.get_untracked() + 1;
        epoch.set(my_epoch);
        let send_edit = send_edit.clone();
        set_timeout(
            move || {
                if epoch.get_untracked() == my_epoch {
                    send_edit();
                }
            },
            Duration::from_millis(400),
        );
    };

    let rename = ServerAction::<RenameSharedPage>::new();
    let on_title_input = move |ev| {
        let new_title = event_target_value(&ev);
        title.set(new_title.clone());
        rename.dispatch(RenameSharedPage { id: page_id, title: new_title });
    };

    view! {
        <div class="space-y-6">
            <div class="flex items-center gap-3">
                {if is_owner {
                    view! {
                        <input
                            type="text"
                            prop:value=move || title.get()
                            on:input=on_title_input
                            class="flex-1 text-xl font-semibold bg-transparent border-0 border-b border-transparent focus:border-brand-500 focus:ring-0 px-0 text-slate-900 dark:text-slate-100"
                        />
                    }.into_any()
                } else {
                    view! { <h1 class="flex-1 text-xl font-semibold text-slate-900 dark:text-slate-100">{move || title.get()}</h1> }.into_any()
                }}
                <span class="text-xs text-slate-400">{move || if synced.get() { "Synced" } else { "Syncing…" }}</span>
            </div>

            {can_edit.then(|| {
                let send_edit_for_upload = send_edit_for_upload.clone();
                view! {
                    <div class="flex items-center gap-3">
                        <label class="text-sm rounded-md border border-slate-300 dark:border-slate-700 px-3 py-1.5 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800">
                            "Attach file"
                            <input
                                type="file"
                                class="hidden"
                                on:change=move |ev| {
                                    upload_error.set(None);
                                    let send_edit_for_upload = send_edit_for_upload.clone();
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
                                                send_edit_for_upload();
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

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                {if can_edit {
                    view! {
                        <textarea
                            prop:value=move || body.get()
                            on:input=on_body_input
                            rows="20"
                            placeholder="Write Markdown — edits sync live to everyone viewing this page…"
                            class="w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-900 p-3 font-mono text-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500"
                        ></textarea>
                    }.into_any()
                } else {
                    view! {
                        <div class="rounded-md border border-slate-200 dark:border-slate-800 p-3 text-sm text-slate-500">
                            "You have view-only access to this page."
                        </div>
                    }.into_any()
                }}
                <div class="rounded-md border border-slate-200 dark:border-slate-800 p-3 overflow-auto">
                    <MarkdownPreview body=Signal::derive(move || body.get())/>
                </div>
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
