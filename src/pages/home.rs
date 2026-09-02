use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="max-w-2xl mx-auto text-center py-16">
            <h1 class="text-3xl sm:text-4xl font-bold text-slate-900 dark:text-slate-100">"SyncNote"</h1>
            <p class="mt-4 text-slate-600 dark:text-slate-400">
                "Personal notes that stay yours, and shared pages you can edit together in real time."
            </p>
            <div class="mt-8 flex justify-center gap-3">
                <A href="/register" attr:class="rounded-md bg-brand-600 px-4 py-2 text-white font-medium hover:bg-brand-700">"Get started"</A>
                <A href="/login" attr:class="rounded-md border border-slate-300 dark:border-slate-700 px-4 py-2 font-medium hover:bg-slate-100 dark:hover:bg-slate-800">"Sign in"</A>
            </div>
        </div>
    }
}
