use std::time::Duration;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::client_upload::upload_from_change_event;
use crate::components::markdown::MarkdownPreview;
use crate::models::Note;
use crate::server::note_fns::{get_note, SaveNote};

#[component]
pub fn NoteEditorPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let note = Resource::new(id, |id| async move { get_note(id).await });

    view! {
        <Suspense fallback=|| view! { <p class="text-sm text-slate-500">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match note.await {
                    Ok(n) => view! { <NoteEditor note=n/> }.into_any(),
                    Err(_) => view! { <p class="text-sm text-rose-500">"Note not found."</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn NoteEditor(note: Note) -> impl IntoView {
    let id = note.id;
    let title = RwSignal::new(note.title);
    let body = RwSignal::new(note.body);
    let save = ServerAction::<SaveNote>::new();
    let epoch = RwSignal::new(0u32);
    let saved = RwSignal::new(true);
    let upload_error = RwSignal::new(Option::<String>::None);

    let schedule_save = move || {
        saved.set(false);
        let my_epoch = epoch.get_untracked() + 1;
        epoch.set(my_epoch);
        set_timeout(
            move || {
                if epoch.get_untracked() == my_epoch {
                    save.dispatch(SaveNote {
                        id,
                        title: title.get_untracked(),
                        body: body.get_untracked(),
                    });
                    saved.set(true);
                }
            },
            Duration::from_millis(500),
        );
    };

    view! {
        <div class="space-y-4">
            <div class="flex items-center gap-3">
                <input
                    type="text"
                    prop:value=move || title.get()
                    on:input=move |ev| {
                        title.set(event_target_value(&ev));
                        schedule_save();
                    }
                    class="flex-1 text-xl font-semibold bg-transparent border-0 border-b border-transparent focus:border-brand-500 focus:ring-0 px-0 text-slate-900 dark:text-slate-100"
                />
                <span class="text-xs text-slate-400">{move || if saved.get() { "Saved" } else { "Saving…" }}</span>
            </div>

            <div class="flex items-center gap-3">
                <label class="text-sm rounded-md border border-slate-300 dark:border-slate-700 px-3 py-1.5 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800">
                    "Attach file"
                    <input
                        type="file"
                        class="hidden"
                        on:change=move |ev| {
                            upload_error.set(None);
                            upload_from_change_event(ev, "note".to_string(), id, move |res| {
                                match res {
                                    Ok(u) => {
                                        let md = if u.content_type.starts_with("image/") {
                                            format!("\n\n![{}]({})\n\n", u.filename, u.url)
                                        } else {
                                            format!("\n\n[{}]({})\n\n", u.filename, u.url)
                                        };
                                        body.update(|b| b.push_str(&md));
                                        schedule_save();
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

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <textarea
                    prop:value=move || body.get()
                    on:input=move |ev| {
                        body.set(event_target_value(&ev));
                        schedule_save();
                    }
                    rows="20"
                    placeholder="Write Markdown…"
                    class="w-full rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-900 p-3 font-mono text-sm focus:border-brand-500 focus:ring-1 focus:ring-brand-500"
                ></textarea>
                <div class="rounded-md border border-slate-200 dark:border-slate-800 p-3 overflow-auto">
                    <MarkdownPreview body=Signal::derive(move || body.get())/>
                </div>
            </div>
        </div>
    }
}
