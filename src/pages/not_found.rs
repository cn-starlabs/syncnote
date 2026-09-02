use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="py-20 text-center">
            <h1 class="text-2xl font-bold text-slate-900 dark:text-slate-100">"404 — Not found"</h1>
            <p class="mt-2 text-slate-600 dark:text-slate-400">"That page doesn't exist."</p>
        </div>
    }
}
