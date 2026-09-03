//! Thin wrapper around the browser WebSocket API for shared-page live sync.
//! Only does anything under the `hydrate` build — under `ssr` it's a no-op
//! stub so the shared page-editor component compiles for both targets.

use crate::models::PageEdit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsStatus {
    Connecting,
    Connected,
    Disconnected,
    Error,
}

#[cfg(feature = "hydrate")]
pub struct SharedPageSocket {
    ws: web_sys::WebSocket,
    _onmessage: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
    _onopen: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
    _onclose: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::CloseEvent)>,
    _onerror: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

#[cfg(feature = "hydrate")]
pub fn connect_shared_page_ws(
    page_id: i64,
    on_message: impl Fn(PageEdit) + 'static,
    on_status: impl Fn(WsStatus) + 'static,
) -> SharedPageSocket {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

    let on_status = std::rc::Rc::new(on_status);
    on_status(WsStatus::Connecting);

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

    let on_status_open = on_status.clone();
    let onopen = Closure::<dyn FnMut(Event)>::new(move |_| {
        on_status_open(WsStatus::Connected);
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

    let on_status_close = on_status.clone();
    let onclose = Closure::<dyn FnMut(CloseEvent)>::new(move |_| {
        on_status_close(WsStatus::Disconnected);
    });
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));

    let on_status_err = on_status.clone();
    let onerror = Closure::<dyn FnMut(ErrorEvent)>::new(move |_| {
        on_status_err(WsStatus::Error);
    });
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    SharedPageSocket {
        ws,
        _onmessage: onmessage,
        _onopen: onopen,
        _onclose: onclose,
        _onerror: onerror,
    }
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
pub fn connect_shared_page_ws(
    _page_id: i64,
    _on_message: impl Fn(PageEdit) + 'static,
    _on_status: impl Fn(WsStatus) + 'static,
) -> SharedPageSocket {
    SharedPageSocket
}

#[cfg(not(feature = "hydrate"))]
impl SharedPageSocket {
    pub fn send(&self, _edit: &PageEdit) {}
}
