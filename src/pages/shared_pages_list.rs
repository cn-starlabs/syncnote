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

    view! {
        <div>
            <div class="flex items-center justify-between">
                <h1 class="text-xl font-semibold text-slate-900 dark:text-slate-100">"Shared pages"</h1>
                <button
                    on:click=move |_| { create.dispatch(CreateSharedPage {}); }
                    class="rounded-md bg-brand-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-brand-700"
                >
                    "New shared page"
                </button>
            </div>

            <Suspense fallback=|| view! { <p class="mt-6 text-sm text-slate-500">"Loading…"</p> }>
                {move || Suspend::new(async move {
                    match pages.await {
                        Ok(list) if list.is_empty() => view! {
                            <p class="mt-6 text-sm text-slate-500">"No shared pages yet — create one or ask for an invite link."</p>
                        }.into_any(),
                        Ok(list) => view! {
                            <ul class="mt-6 divide-y divide-slate-200 dark:divide-slate-800 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
                                {list.into_iter().map(|page| {
                                    let id = page.id;
                                    view! {
                                        <li class="flex items-center justify-between px-4 py-3">
                                            <A href=format!("/app/shared/{id}") attr:class="text-sm font-medium text-slate-900 dark:text-slate-100 hover:text-brand-600">
                                                {page.title.clone()}
                                            </A>
                                            <span class="text-xs text-slate-400">{page.my_role.as_str()}</span>
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
