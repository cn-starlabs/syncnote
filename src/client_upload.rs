//! Uploads a file selected via `<input type="file">` to `/api/upload` and
//! reports back the stored attachment (id + URL) so the caller can insert a
//! Markdown reference into the note/page body. Only does anything under the
//! `hydrate` build — the browser File/FormData/fetch APIs don't exist during
//! SSR, so the non-hydrate build gets a no-op stub with the same signature.

use crate::models::UploadResult;

#[cfg(feature = "hydrate")]
pub fn upload_from_change_event(
    ev: web_sys::Event,
    scope: String,
    scope_id: i64,
    on_done: impl Fn(Result<UploadResult, String>) + 'static,
) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::{spawn_local, JsFuture};
    use web_sys::{FormData, HtmlInputElement, Request, RequestInit, RequestMode, Response};

    let Some(input) = ev.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) else {
        return;
    };
    let Some(files) = input.files() else { return };
    let Some(file) = files.get(0) else { return };
    // Reset now so selecting the same file again still fires `change`.
    input.set_value("");

    spawn_local(async move {
        let result: Result<UploadResult, String> = async {
            let form = FormData::new().map_err(|_| "could not build form".to_string())?;
            form.append_with_str("scope", &scope).map_err(|_| "form error".to_string())?;
            form.append_with_str("scope_id", &scope_id.to_string())
                .map_err(|_| "form error".to_string())?;
            form.append_with_blob("file", &file).map_err(|_| "form error".to_string())?;

            let opts = RequestInit::new();
            opts.set_method("POST");
            opts.set_mode(RequestMode::SameOrigin);
            opts.set_body(&form);

            let request =
                Request::new_with_str_and_init("/api/upload", &opts).map_err(|_| "could not build request".to_string())?;

            let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
            let resp_value = JsFuture::from(window.fetch_with_request(&request))
                .await
                .map_err(|_| "network error".to_string())?;
            let resp: Response = resp_value.dyn_into().map_err(|_| "bad response".to_string())?;

            if !resp.ok() {
                return Err(format!("upload failed (HTTP {})", resp.status()));
            }

            let json_promise = resp.json().map_err(|_| "bad response body".to_string())?;
            let json = JsFuture::from(json_promise).await.map_err(|_| "invalid JSON".to_string())?;
            serde_wasm_bindgen::from_value::<UploadResult>(json).map_err(|e| format!("could not parse response: {e}"))
        }
        .await;
        on_done(result);
    });
}

// Generic over the event type: under SSR, `web_sys` isn't even in the
// dependency graph (leptos only pulls it in for hydrate/csr), so this stub
// can't name `web_sys::Event` — it just needs to accept whatever type the
// shared `on:change` handler in the calling page component infers.
#[cfg(not(feature = "hydrate"))]
pub fn upload_from_change_event<E>(
    _ev: E,
    _scope: String,
    _scope_id: i64,
    _on_done: impl Fn(Result<UploadResult, String>) + 'static,
) {
}
