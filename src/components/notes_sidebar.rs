use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::models::Note;
use crate::server::note_fns::{list_my_notes, CreateNote};

#[component]
pub fn NotesSidebar(
    #[prop(into, optional)] current_note_id: Signal<Option<i64>>,
    #[prop(optional)] on_note_selected: Option<Callback<()>>,
) -> impl IntoView {
    let notes = Resource::new(|| (), |_| async move { list_my_notes().await });
    let search_query = RwSignal::new(String::new());
    let display_limit = RwSignal::new(30usize);
    let create = ServerAction::<CreateNote>::new();
    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(id)) = create.value().get() {
            notes.refetch();
            navigate(&format!("/app/note/{id}"), Default::default());
        }
    });

    let filtered_notes = move || {
        let q = search_query.get().to_lowercase();
        let list = notes.get().and_then(|res| res.ok()).unwrap_or_default();
        if q.trim().is_empty() {
            list
        } else {
            list.into_iter()
                .filter(|n| {
                    n.title.to_lowercase().contains(&q)
                        || n.body.to_lowercase().contains(&q)
                })
                .collect::<Vec<Note>>()
        }
    };

    view! {
        <aside class="flex flex-col h-full w-full rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-3 shadow-sm">
            <div class="flex items-center justify-between gap-2 pb-3 border-b border-slate-100 dark:border-slate-800">
                <span class="text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                    "Notes"
                </span>
                <button
                    on:click=move |_| { create.dispatch(CreateNote {}); }
                    title="Create new note"
                    class="inline-flex items-center gap-1 rounded-md bg-brand-600 px-2.5 py-1 text-xs font-medium text-white hover:bg-brand-700 transition"
                >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New"
                </button>
            </div>

            <div class="mt-3">
                <div class="relative">
                    <input
                        type="text"
                        placeholder="Search notes…"
                        prop:value=move || search_query.get()
                        on:input=move |ev| search_query.set(event_target_value(&ev))
                        class="w-full rounded-md border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 px-2.5 py-1.5 text-xs text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
                    />
                    <Show when=move || !search_query.get().is_empty()>
                        <button
                            on:click=move |_| search_query.set(String::new())
                            class="absolute right-2 top-1.5 text-xs text-slate-400 hover:text-slate-600"
                        >
                            "×"
                        </button>
                    </Show>
                </div>
            </div>

            <div class="mt-3 flex-1 overflow-y-auto space-y-1 pr-1 max-h-[calc(100vh-16rem)] min-h-[14rem]">
                <Suspense fallback=|| view! {
                    <div class="space-y-2 p-2">
                        <div class="h-4 bg-slate-100 dark:bg-slate-800 rounded animate-pulse"></div>
                        <div class="h-4 bg-slate-100 dark:bg-slate-800 rounded animate-pulse w-3/4"></div>
                        <div class="h-4 bg-slate-100 dark:bg-slate-800 rounded animate-pulse w-5/6"></div>
                    </div>
                }>
                    {move || {
                        let all_filtered = filtered_notes();
                        let total_count = all_filtered.len();
                        let limit = display_limit.get();
                        let visible = all_filtered.into_iter().take(limit).collect::<Vec<_>>();

                        if visible.is_empty() {
                            view! {
                                <p class="text-xs text-slate-400 text-center py-6">
                                    {if search_query.get().is_empty() {
                                        "No notes yet."
                                    } else {
                                        "No matching notes."
                                    }}
                                </p>
                            }.into_any()
                        } else {
                            view! {
                                <ul class="space-y-1">
                                    {visible.into_iter().map(|note| {
                                        let id = note.id;
                                        let on_selected = on_note_selected;
                                        let title = if note.title.trim().is_empty() {
                                            "Untitled".to_string()
                                        } else {
                                            note.title.clone()
                                        };

                                        view! {
                                            <li>
                                                <A
                                                    href=format!("/app/note/{id}")
                                                    attr:class=move || {
                                                        let is_active = current_note_id.get() == Some(id);
                                                        if is_active {
                                                            "flex flex-col rounded-lg px-2.5 py-2 bg-brand-50 dark:bg-brand-950/50 text-brand-700 dark:text-brand-300 font-medium text-xs border border-brand-200 dark:border-brand-800/60"
                                                        } else {
                                                            "flex flex-col rounded-lg px-2.5 py-2 text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800/60 text-xs transition"
                                                        }
                                                    }
                                                    on:click=move |_| {
                                                        if let Some(cb) = on_selected {
                                                            cb.run(());
                                                        }
                                                    }
                                                >
                                                    <span class="truncate font-medium">{title}</span>
                                                    <span class="text-[10px] text-slate-400 dark:text-slate-500 mt-0.5">{note.updated_at}</span>
                                                </A>
                                            </li>
                                        }
                                    }).collect::<Vec<_>>()}
                                </ul>

                                {if total_count > limit {
                                    view! {
                                        <div class="pt-2 pb-1 text-center">
                                            <button
                                                on:click=move |_| display_limit.update(|l| *l += 30)
                                                class="w-full text-[11px] font-medium text-brand-600 dark:text-brand-400 hover:underline py-1"
                                            >
                                                {format!("Load more ({total_count} total)")}
                                            </button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span class="hidden"></span> }.into_any()
                                }}
                            }.into_any()
                        }
                    }}
                </Suspense>
            </div>
        </aside>
    }
}
