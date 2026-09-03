use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

use crate::auth::{refresh_auth, use_auth};
use crate::server::auth_fns::Logout;

#[component]
pub fn Nav() -> impl IntoView {
    let auth = use_auth();
    let logout = ServerAction::<Logout>::new();
    let location = use_location();

    Effect::new(move |_| {
        if logout.value().get().is_some() {
            refresh_auth();
        }
    });

    let link_class = move |path: &'static str, exact: bool| {
        move || {
            let current = location.pathname.get();
            let active = if exact { current == path } else { current.starts_with(path) };
            if active {
                "rounded-md px-3 py-1.5 text-sm font-medium bg-brand-50 dark:bg-brand-500/10 text-brand-700 dark:text-brand-100"
            } else {
                "rounded-md px-3 py-1.5 text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800"
            }
        }
    };

    let initial = move |user: &crate::auth::AuthUser| {
        user.display_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(Some(user.email.as_str()))
            .and_then(|s| s.chars().next())
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string())
    };

    view! {
        <header class="sticky top-0 z-10 border-b border-slate-200 dark:border-slate-800 bg-white/90 dark:bg-slate-900/90 backdrop-blur">
            <div class="mx-auto max-w-[1600px] px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
                <A href="/" attr:class="text-lg font-bold tracking-tight text-brand-600 dark:text-brand-100">"SyncNote"</A>
                <nav class="flex items-center gap-1 sm:gap-2 text-sm">
                    <Suspense fallback=|| ()>
                        {move || Suspend::new(async move {
                            match auth.user.await {
                                Ok(Some(user)) => view! {
                                    <A href="/app" attr:class=link_class("/app", true)>"My notes"</A>
                                    <A href="/app/shared" attr:class=link_class("/app/shared", false)>"Shared pages"</A>
                                    {user.is_admin.then(|| view! {
                                        <A href="/app/admin/users" attr:class=link_class("/app/admin", false)>"Admin"</A>
                                    })}

                                    <div class="mx-2 h-6 w-px bg-slate-200 dark:bg-slate-800 hidden sm:block"/>

                                    <A href="/app/account" attr:class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800">
                                        <span class="flex h-6 w-6 items-center justify-center rounded-full bg-brand-600 text-xs font-semibold text-white">
                                            {initial(&user)}
                                        </span>
                                        <span class="hidden md:inline text-slate-600 dark:text-slate-300">
                                            {user.display_name.clone().unwrap_or(user.email)}
                                        </span>
                                    </A>
                                    <button
                                        on:click=move |_| { logout.dispatch(Logout {}); }
                                        class="rounded-md px-3 py-1.5 text-sm font-medium text-slate-500 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 hover:text-slate-900 dark:hover:text-slate-200"
                                    >
                                        "Sign out"
                                    </button>
                                }.into_any(),
                                _ => view! {
                                    <A href="/login" attr:class="rounded-md px-3 py-1.5 text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800">"Sign in"</A>
                                    <A href="/register" attr:class="rounded-md bg-brand-600 px-3 py-1.5 text-sm font-medium text-white shadow-sm hover:bg-brand-700">"Sign up"</A>
                                }.into_any(),
                            }
                        })}
                    </Suspense>

                    <div class="ml-1 pl-2 border-l border-slate-200 dark:border-slate-800 flex items-center">
                        <ThemeToggle/>
                    </div>
                </nav>
            </div>
        </header>
    }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let dark_mode = RwSignal::new(false);

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let is_dark = storage.get_item("dark-mode").ok().flatten() == Some("true".to_string());
                    dark_mode.set(is_dark);
                }
            }
        });
    }

    let toggle_theme = move |_| {
        let new_val = !dark_mode.get();
        dark_mode.set(new_val);
        #[cfg(feature = "hydrate")]
        {
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if let Some(root) = doc.document_element() {
                        if new_val {
                            let _ = root.class_list().add_1("dark");
                        } else {
                            let _ = root.class_list().remove_1("dark");
                        }
                    }
                }
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item("dark-mode", if new_val { "true" } else { "false" });
                }
            }
        }
    };

    view! {
        <button
            type="button"
            on:click=toggle_theme
            title="Toggle theme (Light / Dark)"
            class="p-2 rounded-lg text-slate-500 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 focus:outline-none transition-colors"
        >
            <Show
                when=move || dark_mode.get()
                fallback=|| view! {
                    // Sun icon for switching to dark mode
                    <svg class="w-4 h-4 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                    </svg>
                }
            >
                // Moon icon for switching to light mode
                <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                </svg>
            </Show>
        </button>
    }
}
