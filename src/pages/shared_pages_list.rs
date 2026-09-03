use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::server::shared_page_fns::{list_my_shared_pages, CreateSharedPage};

#[component]
pub fn SharedPagesListPage() -> impl IntoView {
    let pages = Resource::new(|| (), |_| async move { list_my_shared_pages().await });
    let create = ServerAction::<CreateSharedPage>::new();
    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(id)) = create.value().get() {
            navigate(&format!("/app/shared/{id}"), Default::default());
        }
    });

    let delete = ServerAction::<crate::server::shared_page_fns::DeleteSharedPage>::new();
    let confirm_delete_id = RwSignal::new(Option::<i64>::None);

    Effect::new(move |_| {
        if delete.value().get().is_some() {
            pages.refetch();
        }
    });

    view! {
        <div>
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-xl font-bold text-slate-900 dark:text-slate-100">"Shared pages"</h1>
                    <p class="text-xs text-slate-500 mt-0.5">"Pages you collaborate on in real-time"</p>
                </div>
                <button
                    on:click=move |_| { create.dispatch(CreateSharedPage {}); }
                    class="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3.5 py-2 text-sm font-semibold text-white shadow-sm hover:bg-brand-700 transition"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New shared page"
                </button>
            </div>

            // Delete Shared Page Confirmation Modal
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
                                <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">"Delete shared page?"</h3>
                                <p class="text-xs text-slate-500 mt-1">"This action will delete the page for all members. This cannot be undone."</p>
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
                                        delete.dispatch(crate::server::shared_page_fns::DeleteSharedPage { id: target_id });
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

            <Suspense fallback=|| view! { <p class="mt-6 text-sm text-slate-500">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match pages.await {
                        Ok(list) if list.is_empty() => view! {
                            <div class="mt-8 rounded-xl border border-dashed border-slate-300 dark:border-slate-700 p-10 text-center">
                                <div class="w-12 h-12 mx-auto rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-slate-400">
                                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                                    </svg>
                                </div>
                                <h3 class="mt-3 text-sm font-semibold text-slate-900 dark:text-slate-100">"No shared pages yet"</h3>
                                <p class="mt-1 text-xs text-slate-500">"Create a page to collaborate with friends, teammates, or colleagues."</p>
                                <button
                                    on:click=move |_| { create.dispatch(CreateSharedPage {}); }
                                    class="mt-4 inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 transition"
                                >
                                    "Create first shared page"
                                </button>
                            </div>
                        }.into_any(),
                        Ok(list) => view! {
                            <ul class="mt-6 divide-y divide-slate-200 dark:divide-slate-800 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm overflow-hidden">
                                {list.into_iter().map(|page| {
                                    let id = page.id;
                                    let is_owner = page.my_role == crate::models::MemberRole::Owner;
                                    let role_badge_cls = match page.my_role {
                                        crate::models::MemberRole::Owner => "bg-purple-50 dark:bg-purple-950/50 text-purple-700 dark:text-purple-300 border-purple-200 dark:border-purple-800/60",
                                        crate::models::MemberRole::Editor => "bg-blue-50 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300 border-blue-200 dark:border-blue-800/60",
                                        crate::models::MemberRole::Viewer => "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-700",
                                    };
                                    let title = if page.title.trim().is_empty() {
                                        "Untitled page".to_string()
                                    } else {
                                        page.title.clone()
                                    };

                                    view! {
                                        <li class="flex items-center justify-between px-4 py-3.5 hover:bg-slate-50 dark:hover:bg-slate-800/50 transition">
                                            <A href=format!("/app/shared/{id}") attr:class="text-sm font-semibold text-slate-900 dark:text-slate-100 hover:text-brand-600 dark:hover:text-brand-400">
                                                {title}
                                            </A>
                                            <div class="flex items-center gap-3">
                                                <span class={format!("text-[11px] font-medium px-2 py-0.5 rounded-md border {role_badge_cls}")}>
                                                    {page.my_role.as_str()}
                                                </span>
                                                <span class="text-xs text-slate-400 dark:text-slate-500">{page.updated_at.clone()}</span>
                                                {is_owner.then(|| view! {
                                                    <button
                                                        on:click=move |_| { confirm_delete_id.set(Some(id)); }
                                                        class="inline-flex items-center gap-1 text-xs text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 transition ml-1"
                                                        title="Delete page"
                                                    >
                                                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                                        </svg>
                                                        "Delete"
                                                    </button>
                                                })}
                                            </div>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                        Err(_) => view! { <p class="mt-6 text-sm text-rose-500">"Failed to load shared pages."</p> }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}
