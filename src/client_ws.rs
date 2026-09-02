//! Thin wrapper around the browser WebSocket API for shared-page live sync.
//! Only does anything under the `hydrate` build — under `ssr` it's a no-op
//! stub so the shared page-editor component compiles for both targets.

use crate::models::PageEdit;

#[cfg(feature = "hydrate")]
pub struct SharedPageSocket {
    ws: web_sys::WebSocket,
    // Keeps the JS closure alive for the socket's lifetime.
    _onmessage: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
}

#[cfg(feature = "hydrate")]
pub fn connect_shared_page_ws(page_id: i64, on_message: impl Fn(PageEdit) + 'static) -> SharedPageSocket {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{MessageEvent, WebSocket};

    let window = web_sys::window().expect("no window");
    let location = window.location();
    let proto = if location.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = location.host().unwrap_or_default();
    let url = format!("{proto}://{host}/ws/page/{page_id}");
    let ws = WebSocket::new(&url).expect("failed to open shared-page websocket");

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
        if let Some(text) = e.data().as_string() {
            if let Ok(edit) = serde_json::from_str::<PageEdit>(&text) {
                on_message(edit);
            }
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    SharedPageSocket { ws, _onmessage: onmessage }
}

#[cfg(feature = "hydrate")]
impl SharedPageSocket {
    pub fn send(&self, edit: &PageEdit) {
        if self.ws.ready_state() == web_sys::WebSocket::OPEN {
            if let Ok(json) = serde_json::to_string(edit) {
                let _ = self.ws.send_with_str(&json);
            }
        }
    }
}

#[cfg(feature = "hydrate")]
impl Drop for SharedPageSocket {
    fn drop(&mut self) {
        let _ = self.ws.close();
    }
}

#[cfg(not(feature = "hydrate"))]
pub struct SharedPageSocket;

#[cfg(not(feature = "hydrate"))]
pub fn connect_shared_page_ws(_page_id: i64, _on_message: impl Fn(PageEdit) + 'static) -> SharedPageSocket {
    SharedPageSocket
}

#[cfg(not(feature = "hydrate"))]
impl SharedPageSocket {
    pub fn send(&self, _edit: &PageEdit) {}
}
