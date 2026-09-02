use leptos::prelude::*;

use super::footer::Footer;
use super::nav::Nav;

#[component]
pub fn Shell(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-full flex flex-col bg-slate-50 dark:bg-slate-950">
            <Nav/>
            <main class="flex-1 mx-auto w-full max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
                {children()}
            </main>
            <Footer/>
        </div>
    }
}
