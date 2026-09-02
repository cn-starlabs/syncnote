use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::models::shared_page::PageEdit;

#[derive(Clone)]
pub struct AppPool(pub SqlitePool);

#[derive(Clone)]
pub struct WebauthnState(pub Arc<webauthn_rs::prelude::Webauthn>);

/// One broadcast channel per open shared page, created lazily on first
/// connect and left in the map afterward (cheap: an idle channel with no
/// subscribers costs almost nothing, and pages get revisited often enough
/// that pruning isn't worth the complexity yet).
#[derive(Clone, Default)]
pub struct Rooms(pub Arc<Mutex<HashMap<i64, broadcast::Sender<PageEdit>>>>);

impl Rooms {
    pub fn sender(&self, page_id: i64) -> broadcast::Sender<PageEdit> {
        let mut rooms = self.0.lock().expect("rooms lock poisoned");
        rooms
            .entry(page_id)
            .or_insert_with(|| broadcast::channel(64).0)
            .clone()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: AppPool,
    pub rooms: Rooms,
    pub uploads_dir: PathBuf,
    pub webauthn: WebauthnState,
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

impl FromRef<AppState> for AppPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Rooms {
    fn from_ref(state: &AppState) -> Self {
        state.rooms.clone()
    }
}

impl FromRef<AppState> for WebauthnState {
    fn from_ref(state: &AppState) -> Self {
        state.webauthn.clone()
    }
}
