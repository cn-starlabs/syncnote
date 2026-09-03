use std::time::Duration;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::client_upload::upload_from_change_event;
use crate::components::markdown::MarkdownPreview;
use crate::components::notes_sidebar::NotesSidebar;
use crate::models::Note;
use crate::server::note_fns::{get_note, SaveNote, SendNoteViaEmail};

#[component]
pub fn NoteEditorPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let note = Resource::new(id, |id| async move { get_note(id).await });
    let mobile_sidebar_open = RwSignal::new(false);
    let sidebar_refresh_tick = RwSignal::new(0u64);

    view! {
        <div class="flex flex-col md:flex-row gap-6 items-start">
            // Mobile sidebar toggle button
            <div class="w-full flex items-center justify-between md:hidden pb-2 border-b border-slate-200 dark:border-slate-800">
                <button
                    on:click=move |_| mobile_sidebar_open.update(|open| *open = !*open)
                    class="inline-flex items-center gap-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 text-xs font-medium text-slate-700 dark:text-slate-200 shadow-sm hover:bg-slate-50 dark:hover:bg-slate-800"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/>
                    </svg>
                    {move || if mobile_sidebar_open.get() { "Hide notes list" } else { "Show notes list" }}
                </button>
            </div>

            // Mobile expandable sidebar
            <Show when=move || mobile_sidebar_open.get()>
                <div class="w-full md:hidden">
                    <NotesSidebar
                        current_note_id=Signal::derive(move || Some(id()))
                        on_note_selected=Callback::new(move |_| mobile_sidebar_open.set(false))
                        refresh_trigger=Signal::derive(move || sidebar_refresh_tick.get())
                    />
                </div>
            </Show>

            // Desktop sticky sidebar
            <div class="hidden md:block w-72 shrink-0 sticky top-6">
                <NotesSidebar
                    current_note_id=Signal::derive(move || Some(id()))
                    refresh_trigger=Signal::derive(move || sidebar_refresh_tick.get())
                />
            </div>

            // Main note editor area
            <div class="flex-1 min-w-0 w-full">
                <Suspense fallback=|| view! { <p class="text-sm text-slate-500">"Loading…"</p> }>
                    {move || Suspend::new(async move {
                        match note.await {
                            Ok(n) => view! { <NoteEditor note=n on_saved=Callback::new(move |_| sidebar_refresh_tick.update(|t| *t = t.wrapping_add(1)))/> }.into_any(),
                            Err(_) => view! { <p class="text-sm text-rose-500">"Note not found."</p> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn NoteEditor(note: Note, #[prop(optional)] on_saved: Option<Callback<()>>) -> impl IntoView {
    let id = note.id;
    let title = RwSignal::new(note.title);
    let body = RwSignal::new(note.body);
    let save = ServerAction::<SaveNote>::new();
    let epoch = RwSignal::new(0u32);
    let saved = RwSignal::new(true);
    let upload_error = RwSignal::new(Option::<String>::None);

    let send_mail_action = ServerAction::<SendNoteViaEmail>::new();
    let email_modal_open = RwSignal::new(false);
    let recipient_email = RwSignal::new(String::new());
    let email_feedback = RwSignal::new(Option::<(bool, String)>::None);

    Effect::new(move |_| {
        if let Some(res) = save.value().get() {
            if res.is_ok() {
                if let Some(cb) = on_saved {
                    cb.run(());
                }
            }
        }
    });

    Effect::new(move |_| {
        if let Some(res) = send_mail_action.value().get() {
            match res {
                Ok(()) => {
                    email_feedback.set(Some((true, "Email sent successfully!".to_string())));
                    recipient_email.set(String::new());
                }
                Err(e) => {
                    email_feedback.set(Some((false, format!("Failed to send: {e}"))));
                }
            }
        }
    });

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

    let view_mode = RwSignal::new("split"); // "split" | "edit" | "preview"

    let insert_snippet = {
        let schedule_save = schedule_save.clone();
        move |prefix: &'static str, suffix: &'static str, placeholder: &'static str| {
            body.update(|b| {
                if b.ends_with('\n') || b.is_empty() {
                    b.push_str(&format!("{prefix}{placeholder}{suffix}"));
                } else {
                    b.push_str(&format!("\n{prefix}{placeholder}{suffix}"));
                }
            });
            schedule_save();
        }
    };

    let stats = move || {
        let text = body.get();
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        format!("{words} words · {chars} chars")
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
                    placeholder="Note subject / title…"
                    class="flex-1 text-xl font-semibold bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 hover:border-slate-400 dark:hover:border-slate-600 rounded-lg px-3.5 py-2 text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 shadow-sm focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                />
                <span class="text-xs font-medium text-slate-400 dark:text-slate-500 px-2 py-1 bg-slate-100 dark:bg-slate-800 rounded-md shrink-0">
                    {move || if saved.get() { "Saved" } else { "Saving…" }}
                </span>
            </div>

            // Editor Action Bar with Quick Snippet Buttons & View Mode Selector
            <div class="flex flex-wrap items-center justify-between gap-2.5 pb-1">
                // Quick Markdown & Math Action Chips
                <div class="flex flex-wrap items-center gap-1.5 text-xs">
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("**", "**", "bold text")
                        }
                        title="Insert Bold (**text**)"
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
                        title="Insert Italic (*text*)"
                        class="px-2.5 py-1 italic rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "I"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("### ", "", "Heading")
                        }
                        title="Insert Heading"
                        class="px-2 py-1 font-medium rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "H3"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("$", "$", "E = mc^2")
                        }
                        title="Insert Inline Math ($formula$)"
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
                        title="Insert Block Math ($$formula$$)"
                        class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md border border-brand-200 dark:border-brand-900/60 bg-brand-50/70 dark:bg-brand-950/40 text-brand-700 dark:text-brand-300 hover:bg-brand-100/70 dark:hover:bg-brand-900/40 transition font-mono font-medium"
                    >
                        "$$ Block $$"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("```rust\n", "\n```", "// code here")
                        }
                        title="Insert Code Block"
                        class="px-2 py-1 font-mono text-[11px] rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "{ }"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("- [ ] ", "", "task item")
                        }
                        title="Insert Task checklist"
                        class="px-2 py-1 rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "☑ Task"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("| Header 1 | Header 2 |\n| --- | --- |\n| Cell 1 | Cell 2 |", "", "")
                        }
                        title="Insert Table"
                        class="px-2 py-1 rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "⊞ Table"
                    </button>
                </div>

                // View Mode Toggles & Word Count
                <div class="flex items-center gap-3">
                    <span class="text-[11px] text-slate-400 dark:text-slate-500 hidden sm:inline">
                        {stats}
                    </span>

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
            </div>

            <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-2">
                    <label class="text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm transition">
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

                    <button
                        type="button"
                        on:click=move |_| {
                            email_feedback.set(None);
                            email_modal_open.update(|v| *v = !*v);
                        }
                        class="inline-flex items-center gap-1.5 text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm text-slate-700 dark:text-slate-200 transition"
                    >
                        <svg class="w-3.5 h-3.5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                        </svg>
                        "Send via email"
                    </button>
                </div>

                <Show when=move || upload_error.get().is_some()>
                    <span class="text-xs text-rose-500">{move || upload_error.get().unwrap_or_default()}</span>
                </Show>
            </div>

            // Send via Email Dialog Box
            <Show when=move || email_modal_open.get()>
                <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-sm space-y-3">
                    <div class="flex items-center justify-between">
                        <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-600 dark:text-slate-400">
                            "Send note copy via email"
                        </h3>
                        <button
                            on:click=move |_| email_modal_open.set(false)
                            class="text-xs text-slate-400 hover:text-slate-600"
                        >
                            "✕"
                        </button>
                    </div>

                    <form
                        on:submit=move |ev| {
                            ev.prevent_default();
                            email_feedback.set(None);
                            let recipient = recipient_email.get();
                            send_mail_action.dispatch(SendNoteViaEmail {
                                id,
                                recipient_email: recipient,
                            });
                        }
                        class="flex flex-wrap items-center gap-2"
                    >
                        <input
                            type="email"
                            required
                            placeholder="recipient@example.com"
                            prop:value=move || recipient_email.get()
                            on:input=move |ev| recipient_email.set(event_target_value(&ev))
                            class="flex-1 min-w-[220px] rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs focus:border-brand-500 focus:outline-none"
                        />
                        <button
                            type="submit"
                            disabled=move || send_mail_action.pending().get()
                            class="rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 disabled:opacity-60 transition"
                        >
                            {move || if send_mail_action.pending().get() { "Sending…" } else { "Send" }}
                        </button>
                    </form>

                    {move || email_feedback.get().map(|(ok, msg)| {
                        let class_str = if ok {
                            "text-xs text-emerald-700 bg-emerald-50 dark:bg-emerald-950/50 border border-emerald-200 dark:border-emerald-800/60 rounded p-2"
                        } else {
                            "text-xs text-rose-600 bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-800/60 rounded p-2"
                        };
                        view! { <p class=class_str>{msg}</p> }
                    })}
                </div>
            </Show>

            <div class=move || {
                match view_mode.get() {
                    "edit" => "grid grid-cols-1 gap-4",
                    "preview" => "grid grid-cols-1 gap-4",
                    _ => "grid grid-cols-1 lg:grid-cols-2 gap-4",
                }
            }>
                <Show when=move || view_mode.get() != "preview">
                    <textarea
                        prop:value=move || body.get()
                        on:input=move |ev| {
                            body.set(event_target_value(&ev));
                            schedule_save();
                        }
                        rows="26"
                        placeholder="Write Markdown (supports $math$ and $$block math$$)…"
                        class="w-full min-h-[550px] rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-4 font-mono text-sm leading-relaxed text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 hover:border-slate-400 dark:hover:border-slate-600 focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                    ></textarea>
                </Show>
                <Show when=move || view_mode.get() != "edit">
                    <div class="rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-4 overflow-auto min-h-[550px]">
                        <MarkdownPreview body=Signal::derive(move || body.get())/>
                    </div>
                </Show>
            </div>
        </div>
    }
}
