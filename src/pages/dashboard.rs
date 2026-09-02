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

    view! {
        <div>
            <div class="flex items-center justify-between">
                <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"My notes"</h1>
                <button
                    on:click=move |_| { create.dispatch(CreateNote {}); }
                    class="rounded-md bg-brand-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-brand-700"
                >
                    "New note"
                </button>
            </div>

            <Suspense fallback=|| view! { <p class="mt-6 text-sm text-slate-500">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match notes.await {
                        Ok(list) if list.is_empty() => view! {
                            <p class="mt-6 text-sm text-slate-500">"No notes yet — create one to get started."</p>
                        }.into_any(),
                        Ok(list) => view! {
                            <ul class="mt-6 divide-y divide-slate-200 dark:divide-slate-800 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
                                {list.into_iter().map(|note| {
                                    let id = note.id;
                                    view! {
                                        <li class="flex items-center justify-between px-4 py-3">
                                            <A href=format!("/app/note/{id}") attr:class="text-sm font-medium text-slate-900 dark:text-slate-100 hover:text-brand-600">
                                                {note.title.clone()}
                                            </A>
                                            <div class="flex items-center gap-3">
                                                <span class="text-xs text-slate-400">{note.updated_at.clone()}</span>
                                                <button
                                                    on:click=move |_| { delete.dispatch(DeleteNote { id }); }
                                                    class="text-xs text-rose-500 hover:text-rose-700"
                                                >
                                                    "Delete"
                                                </button>
                                            </div>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                        Err(_) => view! { <p class="mt-6 text-sm text-rose-500">"Failed to load notes."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}
