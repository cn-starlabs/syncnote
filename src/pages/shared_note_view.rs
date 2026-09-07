use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

use crate::components::markdown::MarkdownPreview;
use crate::models::Note;
use crate::server::note_share_fns::{check_share_link_protected, get_shared_note};

#[component]
pub fn SharedNoteViewPage() -> impl IntoView {
    let params = use_params_map();
    let token = move || params.read().get("token").unwrap_or_default();

    // Whether the link needs a password (None = not yet checked)
    let is_protected = Resource::new(token, |t| async move {
        check_share_link_protected(t).await
    });

    view! {
        <div class="min-h-screen bg-slate-50 dark:bg-slate-950 py-10 px-4">
            <div class="max-w-3xl mx-auto">
                <Suspense fallback=|| view! {
                    <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p>
                }>
                    {move || Suspend::new(async move {
                        match is_protected.await {
                            Err(_) => view! {
                                <div class="text-center py-24 space-y-3">
                                    <p class="text-4xl">{"🔗"}</p>
                                    <p class="text-lg font-semibold text-slate-700 dark:text-slate-300">
                                        "This note is not available"
                                    </p>
                                    <p class="text-sm text-slate-500 dark:text-slate-400">
                                        "The share link may be invalid or the note has been removed."
                                    </p>
                                </div>
                            }.into_any(),
                            Ok(protected) => view! {
                                <SharedNoteContent token=token() protected=protected/>
                            }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn SharedNoteContent(token: String, protected: bool) -> impl IntoView {
    let note: RwSignal<Option<Note>> = RwSignal::new(None);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let password_input = RwSignal::new(String::new());
    let tok = token.clone();

    // If not password-protected, fetch immediately
    if !protected {
        let tok2 = tok.clone();
        spawn_local(async move {
            loading.set(true);
            match get_shared_note(tok2, None).await {
                Ok(n) => note.set(Some(n)),
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    }

    view! {
        {move || {
            if loading.get() {
                return view! {
                    <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p>
                }.into_any();
            }

            if let Some(n) = note.get() {
                // ── Render the note ──────────────────────────────────────────
                let updated_at = n.updated_at.clone();
                return view! {
                    <div id="note-print-area">
                        <div class="print:hidden mb-6 pb-4 border-b border-slate-200 dark:border-slate-800">
                            <div class="flex items-start justify-between gap-4 flex-wrap">
                                <div>
                                    <h1 class="text-2xl font-bold text-slate-900 dark:text-slate-100">
                                        {n.title.clone()}
                                    </h1>
                                    <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
                                        "Last updated: " {updated_at}
                                    </p>
                                </div>
                                <button
                                    type="button"
                                    on:click=|_| {
                                        #[cfg(feature = "hydrate")]
                                        {
                                            use wasm_bindgen::prelude::*;
                                            #[wasm_bindgen]
                                            extern "C" {
                                                #[wasm_bindgen(js_namespace = window)]
                                                fn print();
                                            }
                                            print();
                                        }
                                    }
                                    class="inline-flex items-center gap-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 text-xs font-medium text-slate-700 dark:text-slate-200 shadow-sm hover:bg-slate-50 dark:hover:bg-slate-800 transition"
                                >
                                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                            d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                                    </svg>
                                    "Download PDF"
                                </button>
                            </div>
                            <p class="mt-2 text-[11px] text-slate-400 dark:text-slate-500">
                                "Shared via SyncNote · Read-only"
                            </p>
                        </div>
                        <div class="rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-6 print:border-none print:shadow-none print:p-0">
                            <MarkdownPreview body=Signal::derive(move || n.body.clone())/>
                        </div>
                    </div>
                }.into_any();
            }

            // ── Password gate ────────────────────────────────────────────────
            view! {
                <div class="max-w-sm mx-auto mt-20">
                    <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm p-8 space-y-5 text-center">
                        <div class="w-14 h-14 mx-auto rounded-full bg-brand-50 dark:bg-brand-950/40 flex items-center justify-center">
                            <svg class="w-7 h-7 text-brand-600 dark:text-brand-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
                            </svg>
                        </div>
                        <div>
                            <h2 class="text-base font-semibold text-slate-900 dark:text-slate-100">
                                "Password protected"
                            </h2>
                            <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
                                "Enter the password to view this note."
                            </p>
                        </div>

                        <Show when=move || error.get().is_some()>
                            <p class="text-xs text-rose-600 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800/60 rounded-lg px-3 py-2">
                                {move || error.get().unwrap_or_default()}
                            </p>
                        </Show>

                        <form
                            on:submit={
                                let tok = tok.clone();
                                move |ev: leptos::ev::SubmitEvent| {
                                    ev.prevent_default();
                                    let pw = password_input.get_untracked();
                                    if pw.trim().is_empty() { return; }
                                    let t = tok.clone();
                                    error.set(None);
                                    loading.set(true);
                                    spawn_local(async move {
                                        match get_shared_note(t, Some(pw)).await {
                                            Ok(n) => note.set(Some(n)),
                                            Err(e) => {
                                                let msg = e.to_string();
                                                error.set(Some(if msg.contains("wrong_password") {
                                                    "Incorrect password. Please try again.".into()
                                                } else {
                                                    msg
                                                }));
                                            }
                                        }
                                        loading.set(false);
                                    });
                                }
                            }
                            class="space-y-3"
                        >
                            <input
                                type="password"
                                required
                                placeholder="Enter password…"
                                autofocus
                                prop:value=move || password_input.get()
                                on:input=move |ev| password_input.set(event_target_value(&ev))
                                class="w-full rounded-lg border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-4 py-2.5 text-sm focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                            />
                            <button
                                type="submit"
                                class="w-full rounded-lg bg-brand-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-brand-700 transition"
                            >
                                "Unlock note"
                            </button>
                        </form>
                    </div>
                </div>
            }.into_any()
        }}
    }
}
