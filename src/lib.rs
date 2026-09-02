#![recursion_limit = "256"]

pub mod app;
pub mod auth;
pub mod client_passkey;
pub mod client_upload;
pub mod client_ws;
pub mod components;
pub mod models;
pub mod pages;
pub mod server;

#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod server_ctx;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
