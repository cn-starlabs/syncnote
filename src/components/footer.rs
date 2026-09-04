use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const BUILD_TIME: &str = env!("BUILD_TIME");
    view! {
        <footer class="border-t border-slate-200 dark:border-slate-800 py-6 text-center text-xs text-slate-400 dark:text-slate-500 flex flex-wrap items-center justify-center gap-2">
            <span>"SyncNote"</span>
            <span class="font-mono text-[11px] px-1.5 py-0.5 rounded bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400">
                {format!("v{VERSION}")}
            </span>
            <span class="text-slate-300 dark:text-slate-700">"•"</span>
            <span class="text-[11px] text-slate-400 dark:text-slate-500">
                {format!("Built {BUILD_TIME}")}
            </span>
        </footer>
    }
}
