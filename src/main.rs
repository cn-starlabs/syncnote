#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::extract::{DefaultBodyLimit, Extension};
    use axum::routing::{get, post};
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use syncnote::app::{shell, App};
    use syncnote::config::AppConfig;
    use syncnote::db;
    use syncnote::server::attachments::{serve_attachment, upload_handler};
    use syncnote::server::ws::ws_handler;
    use syncnote::server_ctx::{AppPool, AppState, Rooms, UploadsDir, WebauthnState};
    use tower_http::compression::CompressionLayer;
    use tower_http::trace::TraceLayer;
    use tower_sessions::cookie::time::Duration as CookieDuration;
    use tower_sessions::{Expiry, SessionManagerLayer};
    use tower_sessions_sqlx_store::SqliteStore;
    use webauthn_rs::prelude::*;

    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,syncnote=debug,tower_http=info".into()),
        )
        .init();

    let cfg = AppConfig::load().expect("config.toml");

    let mut conf = get_configuration(None).expect("leptos config");
    conf.leptos_options.site_addr = cfg.site_addr();
    conf.leptos_options.site_root = cfg.server.site_root.clone().into();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let pool = db::open_pool(&cfg.database.path).await.expect("open app.db");
    db::run_migrations(&pool).await.expect("migrations");
    db::bootstrap_admin_if_requested(&pool).await.expect("bootstrap admin");

    let uploads_dir = cfg.database.uploads_dir.clone();
    tokio::fs::create_dir_all(&uploads_dir).await.expect("create uploads dir");

    // Passkeys require rp_id to be the exact effective domain the browser sees —
    // see config.toml's [webauthn] section.
    let rp_origin = url::Url::parse(&cfg.webauthn.rp_origin).expect("config.toml webauthn.rp_origin must be a valid URL");
    let webauthn = WebauthnBuilder::new(&cfg.webauthn.rp_id, &rp_origin)
        .expect("invalid webauthn rp_id/origin")
        .rp_name("SyncNote")
        .build()
        .expect("failed to build webauthn");

    // Cookies can only carry the Secure flag over an https origin, so this
    // tracks whichever scheme config.toml's webauthn.rp_origin declares —
    // the same origin the app is actually served on.
    let cookie_secure = rp_origin.scheme() == "https";

    let state = AppState {
        leptos_options: leptos_options.clone(),
        pool: AppPool(pool),
        rooms: Rooms::default(),
        uploads_dir,
        webauthn: WebauthnState(std::sync::Arc::new(webauthn)),
    };

    let session_store = SqliteStore::new(state.pool.0.clone());
    session_store.migrate().await.expect("session store migrate");
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(cookie_secure)
        .with_name("syncnote_sid")
        .with_expiry(Expiry::OnInactivity(CookieDuration::days(30)));

    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/ws/page/{id}", get(ws_handler))
        .route(
            "/api/upload",
            post(upload_handler).layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
        .route("/attachments/{id}", get(serve_attachment))
        .leptos_routes_with_context(
            &state,
            routes,
            {
                let state = state.clone();
                move || {
                    provide_context(state.pool.clone());
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(Extension(AppPool(state.pool.0.clone())))
        .layer(Extension(state.webauthn.clone()))
        .layer(Extension(UploadsDir(state.uploads_dir.clone())))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(session_layer)
        .with_state(state);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // client-side: see lib.rs::hydrate
}
