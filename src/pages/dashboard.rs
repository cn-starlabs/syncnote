use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::server::note_fns::{list_my_notes, CreateNote, DeleteNote};

#[component]
pub fn DashboardPage() -> impl IntoView {
    let notes = Resource::new(|| (), |_| async move { list_my_notes().await });
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
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
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

            <Suspense fallback=|| view! { <p class="mt-6 text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match notes.await {
                        Ok(list) if list.is_empty() => view! {
                            <div class="mt-8 rounded-xl border border-dashed border-slate-300 dark:border-slate-700 p-10 text-center">
                                <div class="w-12 h-12 mx-auto rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-slate-400">
                                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
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
                            // Plain `let` closures outside the view! macro: writing the
                            // filter+collect inline as `each=move || ... .collect::<Vec<_>>()`
                            // puts a turbofish (`<Vec<_>>`) inside a component prop, which the
                            // macro's HTML-like tag parser misreads as tag syntax.
                            let filtered_notes = move || {
                                let q = search_query.get().trim().to_lowercase();
                                all_notes
                                    .get()
                                    .into_iter()
                                    .filter(move |n| {
                                        q.is_empty() || n.title.to_lowercase().contains(&q) || n.body.to_lowercase().contains(&q)
                                    })
                                    .collect::<Vec<_>>()
                            };
                            let has_results = move || {
                                let q = search_query.get().trim().to_lowercase();
                                q.is_empty()
                                    || all_notes
                                        .get()
                                        .iter()
                                        .any(|n| n.title.to_lowercase().contains(&q) || n.body.to_lowercase().contains(&q))
                            };
                            view! {
                                <div class="mt-4 relative">
                                    <svg class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M11 19a8 8 0 100-16 8 8 0 000 16z" />
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
                                                        <A href=format!("/app/note/{id}") attr:class="text-sm font-semibold text-slate-900 dark:text-slate-100 hover:text-brand-600 dark:hover:text-brand-400">
                                                            {title}
                                                        </A>
                                                        <div class="flex items-center gap-3">
                                                            <span class="text-xs text-slate-500 dark:text-slate-400">{note.updated_at.clone()}</span>
                                                            <button
                                                                on:click=move |_| { confirm_delete_id.set(Some(id)); }
                                                                class="inline-flex items-center gap-1 text-xs text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 transition"
                                                                title="Delete note"
                                                            >
                                                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
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
        </div>
    }
}
