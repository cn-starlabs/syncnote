use leptos::prelude::*;
use leptos_meta::{provide_meta_context, HashedStylesheet, MetaTags, Title};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::{path, StaticSegment};

use crate::auth::provide_auth_context;
use crate::components::auth_guard::{RequireAdmin, RequireAuth};
use crate::components::layout::Shell;
use crate::pages::account::AccountPage;
use crate::pages::admin_invites::AdminInvitesPage;
use crate::pages::admin_users::AdminUsersPage;
use crate::pages::dashboard::DashboardPage;
use crate::pages::home::HomePage;
use crate::pages::join_invite::JoinInvitePage;
use crate::pages::login::LoginPage;
use crate::pages::not_found::NotFound;
use crate::pages::note_editor::NoteEditorPage;
use crate::pages::register::RegisterPage;
use crate::pages::shared_page_editor::SharedPageEditorPage;
use crate::pages::shared_pages_list::SharedPagesListPage;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en" class="h-full bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 antialiased">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="description" content="SyncNote — personal notes and live-collaborative shared pages"/>
                <script>"(function(){try{if(localStorage.getItem('dark-mode')==='true'){document.documentElement.classList.add('dark');}}catch(e){}})();"</script>
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone()/>
                <HashedStylesheet id="leptos" options=options />
                <MetaTags/>
            </head>
            <body class="h-full bg-slate-50 dark:bg-slate-950">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_auth_context();

    view! {
        <Title text="SyncNote"/>
        <Router>
            <Shell>
                <Routes fallback=|| view! { <NotFound/> }.into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=path!("/login") view=LoginPage/>
                    <Route path=path!("/register") view=RegisterPage/>
                    <Route path=path!("/join/:token") view=JoinInvitePage/>
                    <ParentRoute path=path!("/app") view=AuthGate>
                        <Route path=path!("") view=DashboardPage/>
                        <Route path=path!("/note/:id") view=NoteEditorPage/>
                        <Route path=path!("/shared") view=SharedPagesListPage/>
                        <Route path=path!("/shared/:id") view=SharedPageEditorPage/>
                        <Route path=path!("/account") view=AccountPage/>
                        <ParentRoute path=path!("/admin") view=AdminGate>
                            <Route path=path!("/invites") view=AdminInvitesPage/>
                            <Route path=path!("/users") view=AdminUsersPage/>
                        </ParentRoute>
                    </ParentRoute>
                </Routes>
            </Shell>
        </Router>
    }
}

#[component]
fn AuthGate() -> impl IntoView {
    use leptos_router::components::Outlet;
    view! {
        <RequireAuth>
            <Outlet/>
        </RequireAuth>
    }
}

#[component]
fn AdminGate() -> impl IntoView {
    use leptos_router::components::Outlet;
    view! {
        <RequireAdmin>
            <Outlet/>
        </RequireAdmin>
    }
}
