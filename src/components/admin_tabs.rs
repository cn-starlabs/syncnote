use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

#[component]
pub fn AdminTabs() -> impl IntoView {
    let location = use_location();
    let is_active = move |path: &'static str| move || location.pathname.get().starts_with(path);

    let tab_class = move |active: bool| {
        if active {
            "rounded-md bg-white dark:bg-slate-900 px-3 py-1.5 text-sm font-medium text-slate-900 dark:text-slate-100 shadow-sm"
        } else {
            "rounded-md px-3 py-1.5 text-sm font-medium text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
        }
    };

    let users_active = is_active("/app/admin/users");
    let invites_active = is_active("/app/admin/invites");

    view! {
        <div class="inline-flex items-center gap-1 rounded-lg bg-slate-100 dark:bg-slate-800 p-1 mb-4">
            <A href="/app/admin/users" attr:class=move || tab_class(users_active())>"Users"</A>
            <A href="/app/admin/invites" attr:class=move || tab_class(invites_active())>"Invite codes"</A>
        </div>
    }
}
