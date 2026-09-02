use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="border-t border-slate-200 dark:border-slate-800 py-6 text-center text-xs text-slate-400">
            "SyncNote"
        </footer>
    }
}
