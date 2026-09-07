//! Read-only note view for notes shared directly with a logged-in user
//! (via `note_shares` table, not a public token link).

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::markdown::MarkdownPreview;
use crate::server::note_share_fns::get_note_shared_with_me;

#[component]
pub fn SharedNoteUserViewPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let note = Resource::new(id, |id| async move { get_note_shared_with_me(id).await });

    view! {
        <div class="max-w-3xl mx-auto py-6">
            <Suspense fallback=|| view! {
                <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p>
            }>
                {move || Suspend::new(async move {
                    match note.await {
                        Err(_) => view! {
                            <div class="text-center py-24 space-y-3">
                                <p class="text-4xl">{"🔗"}</p>
                                <p class="text-lg font-semibold text-slate-700 dark:text-slate-300">
                                    "Note not available"
                                </p>
                                <p class="text-sm text-slate-500 dark:text-slate-400">
                                    "This note hasn't been shared with you, or it no longer exists."
                                </p>
                            </div>
                        }.into_any(),
                        Ok(n) => {
                            let updated_at = n.updated_at.clone();
                            view! {
                                // Print header (hidden on screen)
                                <div id="note-print-area">
                                    <div class="mb-6 pb-4 border-b border-slate-200 dark:border-slate-800">
                                        <div class="flex items-start justify-between gap-4 flex-wrap">
                                            <div>
                                                <h1 class="text-2xl font-bold text-slate-900 dark:text-slate-100">
                                                    {n.title.clone()}
                                                </h1>
                                                <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
                                                    "Last updated: " {updated_at}
                                                </p>
                                            </div>
                                            <div class="flex items-center gap-2">
                                                // PDF export
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
                                                    class="inline-flex items-center gap-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 text-xs font-medium text-slate-700 dark:text-slate-200 shadow-sm hover:bg-slate-50 dark:hover:bg-slate-800 transition print:hidden"
                                                >
                                                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                                            d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                                                    </svg>
                                                    "Export PDF"
                                                </button>
                                            </div>
                                        </div>
                                        <p class="mt-2 text-[11px] text-slate-400 dark:text-slate-500 print:hidden">
                                            "Shared with you · Read-only"
                                        </p>
                                    </div>

                                    <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm p-6 print:border-none print:shadow-none print:p-0">
                                        <MarkdownPreview body=Signal::derive(move || n.body.clone())/>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}
