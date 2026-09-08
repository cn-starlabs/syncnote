use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::client_upload::upload_from_change_event;
use crate::components::markdown::{render_markdown, MarkdownPreview};
use crate::components::notes_sidebar::NotesSidebar;
use crate::models::{AttachmentInfo, Note};
use crate::server::attachment_fns::list_library_attachments;
use crate::server::note_fns::{get_note, DeleteNote, SaveNote, SendNoteViaEmail};
use crate::server::note_share_fns::{
    create_note_share_link, get_note_share_token, list_note_user_shares, revoke_note_share_link,
    share_note_with_user, unshare_note_from_user,
};

#[component]
pub fn NoteEditorPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let note = Resource::new(id, |id| async move { get_note(id).await });
    let mobile_sidebar_open = RwSignal::new(false);
    let sidebar_refresh_tick = RwSignal::new(0u64);

    view! {
        <div class="flex flex-col md:flex-row gap-6 items-start">
            // Mobile sidebar toggle button
            <div class="w-full flex items-center justify-between md:hidden pb-2 border-b border-slate-200 dark:border-slate-800">
                <button
                    on:click=move |_| mobile_sidebar_open.update(|open| *open = !*open)
                    class="inline-flex items-center gap-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 text-xs font-medium text-slate-700 dark:text-slate-200 shadow-sm hover:bg-slate-50 dark:hover:bg-slate-800"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/>
                    </svg>
                    {move || if mobile_sidebar_open.get() { "Hide notes list" } else { "Show notes list" }}
                </button>
            </div>

            // Mobile expandable sidebar
            <Show when=move || mobile_sidebar_open.get()>
                <div class="w-full md:hidden">
                    <NotesSidebar
                        current_note_id=Signal::derive(move || Some(id()))
                        on_note_selected=Callback::new(move |_| mobile_sidebar_open.set(false))
                        refresh_trigger=Signal::derive(move || sidebar_refresh_tick.get())
                    />
                </div>
            </Show>

            // Desktop sticky sidebar
            <div class="hidden md:block w-72 shrink-0 sticky top-6">
                <NotesSidebar
                    current_note_id=Signal::derive(move || Some(id()))
                    refresh_trigger=Signal::derive(move || sidebar_refresh_tick.get())
                />
            </div>

            // Main note editor area
            <div class="flex-1 min-w-0 w-full">
                <Suspense fallback=|| view! { <p class="text-sm text-slate-500 dark:text-slate-400">"Loading…"</p> }>
                    {move || Suspend::new(async move {
                        match note.await {
                            Ok(n) => view! { <NoteEditor note=n on_saved=Callback::new(move |_| sidebar_refresh_tick.update(|t| *t = t.wrapping_add(1)))/> }.into_any(),
                            Err(_) => view! { <p class="text-sm text-rose-500">"Note not found."</p> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn NoteEditor(note: Note, #[prop(optional)] on_saved: Option<Callback<()>>) -> impl IntoView {
    let id = note.id;
    let updated_at = note.updated_at.clone();
    let title = RwSignal::new(note.title);
    let body = RwSignal::new(note.body);
    let save = ServerAction::<SaveNote>::new();
    let epoch = RwSignal::new(0u32);
    let saved = RwSignal::new(true);
    let upload_error = RwSignal::new(Option::<String>::None);
    let library_open = RwSignal::new(false);
    let library_files = RwSignal::new(Option::<Vec<AttachmentInfo>>::None);
    let library_error = RwSignal::new(Option::<String>::None);

    // ── Delete ──────────────────────────────────────────────────────────────
    let delete_action = ServerAction::<DeleteNote>::new();
    let confirm_delete = RwSignal::new(false);
    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(())) = delete_action.value().get() {
            navigate("/app", Default::default());
        }
    });

    // ── Share link ───────────────────────────────────────────────────────────
    let share_modal_open = RwSignal::new(false);
    let share_token: RwSignal<Option<String>> = RwSignal::new(None);
    let share_users: RwSignal<Vec<crate::models::NoteShareInfo>> = RwSignal::new(vec![]);
    let share_loading = RwSignal::new(false);
    let share_error = RwSignal::new(Option::<String>::None);
    let share_copied = RwSignal::new(false);
    let share_email_input = RwSignal::new(String::new());
    let share_email_error = RwSignal::new(Option::<String>::None);
    let share_email_pending = RwSignal::new(false);
    let share_new_password = RwSignal::new(String::new()); // optional password when creating a link

    // Load existing share token + shared users when share modal opens
    let open_share_modal = move |_| {
        share_error.set(None);
        share_modal_open.set(true);
        if share_token.get_untracked().is_none() && !share_loading.get_untracked() {
            share_loading.set(true);
            spawn_local(async move {
                let token_res = get_note_share_token(id).await;
                let users_res = list_note_user_shares(id).await;
                match token_res {
                    Ok(t) => share_token.set(t),
                    Err(e) => share_error.set(Some(e.to_string())),
                }
                if let Ok(u) = users_res {
                    share_users.set(u);
                }
                share_loading.set(false);
            });
        }
    };

    let create_share_link = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let pw = share_new_password.get_untracked();
        let password = if pw.trim().is_empty() { None } else { Some(pw) };
        share_error.set(None);
        share_loading.set(true);
        spawn_local(async move {
            match create_note_share_link(id, password).await {
                Ok(token) => {
                    share_token.set(Some(token));
                    share_new_password.set(String::new());
                }
                Err(e) => share_error.set(Some(e.to_string())),
            }
            share_loading.set(false);
        });
    };

    let revoke_share = move |_| {
        share_error.set(None);
        share_loading.set(true);
        spawn_local(async move {
            match revoke_note_share_link(id).await {
                Ok(()) => share_token.set(None),
                Err(e) => share_error.set(Some(e.to_string())),
            }
            share_loading.set(false);
        });
    };

    let copy_share_link = move |_| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::prelude::*;
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = ["window", "navigator", "clipboard"], js_name = writeText)]
                fn clipboard_write(s: &str);
            }
            if let Some(token) = share_token.get_untracked() {
                let url = format!("{}/note/shared/{}", web_sys::window().unwrap().location().origin().unwrap_or_default(), token);
                clipboard_write(&url);
                share_copied.set(true);
                set_timeout(move || share_copied.set(false), Duration::from_secs(2));
            }
        }
    };

    // ── Email ────────────────────────────────────────────────────────────────
    let send_mail_action = ServerAction::<SendNoteViaEmail>::new();
    let email_modal_open = RwSignal::new(false);
    let recipient_email = RwSignal::new(String::new());
    let email_feedback = RwSignal::new(Option::<(bool, String)>::None);

    Effect::new(move |_| {
        if let Some(res) = save.value().get() {
            if res.is_ok() {
                if let Some(cb) = on_saved {
                    cb.run(());
                }
            }
        }
    });

    Effect::new(move |_| {
        if let Some(res) = send_mail_action.value().get() {
            match res {
                Ok(()) => {
                    email_feedback.set(Some((true, "Email sent successfully!".to_string())));
                    recipient_email.set(String::new());
                }
                Err(e) => {
                    email_feedback.set(Some((false, format!("Failed to send: {e}"))));
                }
            }
        }
    });

    let schedule_save = move || {
        saved.set(false);
        let my_epoch = epoch.get_untracked() + 1;
        epoch.set(my_epoch);
        set_timeout(
            move || {
                if epoch.get_untracked() == my_epoch {
                    save.dispatch(SaveNote {
                        id,
                        title: title.get_untracked(),
                        body: body.get_untracked(),
                    });
                    saved.set(true);
                }
            },
            Duration::from_millis(500),
        );
    };

    let view_mode = RwSignal::new("split"); // "split" | "edit" | "preview"

    let insert_snippet = {
        let schedule_save = schedule_save.clone();
        move |prefix: &'static str, suffix: &'static str, placeholder: &'static str| {
            body.update(|b| {
                if b.ends_with('\n') || b.is_empty() {
                    b.push_str(&format!("{prefix}{placeholder}{suffix}"));
                } else {
                    b.push_str(&format!("\n{prefix}{placeholder}{suffix}"));
                }
            });
            schedule_save();
        }
    };

    let stats = move || {
        let text = body.get();
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        format!("{words} words · {chars} chars")
    };

    // ── PDF export: render the note's Markdown (incl. math) into a standalone
    // print document opened in a new window, then trigger that window's print
    // dialog. This renders the actual note content instead of whatever the
    // main app page happens to look like. ───────────────────────────────────
    let export_pdf = move |_| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::prelude::*;

            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = window, js_name = exportHtmlToPdf, catch)]
                fn export_html_to_pdf(html: &str) -> Result<(), JsValue>;
            }

            fn escape_html(s: &str) -> String {
                s.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            }

            let raw_title = title.get_untracked();
            let raw_title = if raw_title.trim().is_empty() {
                "Untitled Note"
            } else {
                raw_title.trim()
            };
            let doc_title = escape_html(raw_title);
            let doc_updated = escape_html(&updated_at);
            let body_text = body.get_untracked();
            let content_html = if body_text.trim().is_empty() {
                "<p style=\"color: #94a3b8; font-style: italic;\">No content in this note.</p>".to_string()
            } else {
                render_markdown(&body_text)
            };

            let base_href = web_sys::window()
                .and_then(|w| w.location().origin().ok())
                .unwrap_or_default();
            let base_tag = if !base_href.is_empty() {
                format!("<base href=\"{base_href}/\">")
            } else {
                String::new()
            };

            let full_html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
{base_tag}
<title>{doc_title}</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" crossorigin="anonymous">
<style>
  * {{ box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    color: #0f172a;
    background-color: #ffffff;
    padding: 2cm;
    line-height: 1.6;
    margin: 0;
  }}
  .preview-bar {{
    position: sticky;
    top: 0;
    left: 0;
    right: 0;
    background: #f8fafc;
    border-bottom: 1px solid #e2e8f0;
    padding: 10px 20px;
    margin: -2cm -2cm 1.5cm -2cm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    box-shadow: 0 1px 3px rgba(0,0,0,0.05);
    z-index: 100;
  }}
  .preview-title {{
    font-size: 13px;
    font-weight: 600;
    color: #475569;
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .preview-actions {{
    display: flex;
    gap: 8px;
  }}
  .btn {{
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    border: none;
    transition: background 0.15s;
    text-decoration: none;
  }}
  .btn-primary {{
    background: #4f46e5;
    color: #ffffff;
  }}
  .btn-primary:hover {{
    background: #4338ca;
  }}
  .btn-secondary {{
    background: #e2e8f0;
    color: #334155;
  }}
  .btn-secondary:hover {{
    background: #cbd5e1;
  }}
  h1.note-title {{
    font-size: 1.875rem;
    font-weight: 700;
    margin-top: 0;
    margin-bottom: 0.35rem;
    color: #0f172a;
  }}
  p.note-meta {{
    font-size: 0.8125rem;
    color: #64748b;
    margin-top: 0;
    margin-bottom: 1.5rem;
    border-bottom: 1px solid #e2e8f0;
    padding-bottom: 0.75rem;
  }}
  .prose-note h1 {{ font-size: 1.5rem; font-weight: 700; margin: 1.25rem 0 0.5rem; color: #0f172a; }}
  .prose-note h2 {{ font-size: 1.25rem; font-weight: 700; margin: 1.25rem 0 0.5rem; color: #1e293b; }}
  .prose-note h3 {{ font-size: 1.1rem; font-weight: 600; margin: 1rem 0 0.25rem; color: #334155; }}
  .prose-note p {{ margin: 0.75rem 0; }}
  .prose-note ul {{ list-style: disc; padding-left: 1.5rem; margin: 0.75rem 0; }}
  .prose-note ol {{ list-style: decimal; padding-left: 1.5rem; margin: 0.75rem 0; }}
  .prose-note li {{ margin: 0.25rem 0; }}
  .prose-note a {{ color: #2563eb; text-decoration: underline; }}
  .prose-note code {{ background: #f1f5f9; border-radius: 0.25rem; padding: 0.15rem 0.35rem; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 0.875em; }}
  .prose-note pre {{ background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 0.5rem; padding: 1rem; overflow-x: auto; margin: 0.75rem 0; }}
  .prose-note pre code {{ background: transparent; padding: 0; font-size: 0.875rem; }}
  .prose-note blockquote {{ border-left: 4px solid #cbd5e1; padding-left: 1rem; color: #475569; font-style: italic; margin: 0.75rem 0; }}
  .prose-note table {{ width: 100%; border-collapse: collapse; margin: 1rem 0; }}
  .prose-note th, .prose-note td {{ border: 1px solid #cbd5e1; padding: 0.5rem 0.75rem; text-align: left; font-size: 0.875rem; }}
  .prose-note th {{ background: #f8fafc; font-weight: 600; }}
  .prose-note hr {{ border: none; border-top: 1px solid #e2e8f0; margin: 1.5rem 0; }}
  .prose-note img {{ max-width: 100%; height: auto; border-radius: 0.375rem; margin: 0.75rem 0; }}
  .katex-math-block {{ display: flex; justify-content: center; margin: 1rem 0; overflow-x: auto; }}
  .katex-math-inline {{ display: inline-block; }}
  @media print {{
    body {{ padding: 0 !important; }}
    .no-print {{ display: none !important; }}
    h1, h2, h3 {{ page-break-after: avoid; }}
    pre, blockquote, table, img {{ page-break-inside: avoid; }}
  }}
</style>
</head>
<body>
<div class="preview-bar no-print">
  <div class="preview-title">
    <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
    </svg>
    <span>SyncNote — PDF Export Preview</span>
  </div>
  <div class="preview-actions">
    <button class="btn btn-primary" onclick="window.print()">
      <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"/>
      </svg>
      <span>Print / Save as PDF</span>
    </button>
    <button class="btn btn-secondary" onclick="window.close()">Close</button>
  </div>
</div>
<h1 class="note-title">{doc_title}</h1>
<p class="note-meta">Last updated: {doc_updated}</p>
<div class="prose-note">{content_html}</div>
<script src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js" crossorigin="anonymous"></script>
<script>
  function renderMathAndPrint() {{
    var k = (typeof katex !== 'undefined') ? katex : (window.opener && window.opener.katex ? window.opener.katex : null);
    if (k) {{
      document.querySelectorAll('.katex-math-inline').forEach(function(el) {{
        var expr = el.getAttribute('data-expr');
        if (expr) {{ try {{ k.render(expr, el, {{ throwOnError: false, displayMode: false }}); }} catch (e) {{}} }}
      }});
      document.querySelectorAll('.katex-math-block').forEach(function(el) {{
        var expr = el.getAttribute('data-expr');
        if (expr) {{ try {{ k.render(expr, el, {{ throwOnError: false, displayMode: true }}); }} catch (e) {{}} }}
      }});
    }}
    setTimeout(function() {{
      window.focus();
      window.print();
    }}, 250);
  }}
  if (document.readyState === 'complete') {{
    renderMathAndPrint();
  }} else {{
    window.addEventListener('load', renderMathAndPrint);
  }}
</script>
</body>
</html>"#
            );

            // Safety guard: ensure window.exportHtmlToPdf is defined even on stale/cached shells
            let _ = js_sys::eval(r#"
                if (typeof window.exportHtmlToPdf !== 'function') {
                    window.exportHtmlToPdf = function(fullHtml) {
                        var printWin = null;
                        try { printWin = window.open('', '_blank'); } catch(e) {}
                        if (printWin && printWin.document) {
                            try {
                                printWin.document.open();
                                printWin.document.write(fullHtml);
                                printWin.document.close();
                                return;
                            } catch(e) {
                                try { printWin.close(); } catch(_) {}
                            }
                        }
                        try {
                            var blob = new Blob([fullHtml], { type: 'text/html;charset=utf-8' });
                            var blobUrl = URL.createObjectURL(blob);
                            var blobWin = window.open(blobUrl, '_blank');
                            if (blobWin) {
                                setTimeout(function() { try { URL.revokeObjectURL(blobUrl); } catch(_) {} }, 60000);
                                return;
                            }
                        } catch(e) {}
                        try {
                            var frame = document.createElement('iframe');
                            frame.style.position = 'fixed';
                            frame.style.right = '100%';
                            frame.style.bottom = '100%';
                            frame.style.width = '0';
                            frame.style.height = '0';
                            frame.style.border = '0';
                            document.body.appendChild(frame);
                            var frameDoc = frame.contentWindow.document;
                            frameDoc.open();
                            frameDoc.write(fullHtml);
                            frameDoc.close();
                            setTimeout(function() {
                                try { frame.contentWindow.focus(); frame.contentWindow.print(); }
                                finally { setTimeout(function() { try { document.body.removeChild(frame); } catch(_) {} }, 2000); }
                            }, 300);
                        } catch(e) {}
                    };
                }
            "#);

            if let Err(e) = export_html_to_pdf(&full_html) {
                leptos::logging::warn!("export_html_to_pdf call failed: {e:?}");
            }
        }
    };

    view! {
        <div class="space-y-4">
            <div class="flex items-center gap-3">
                <input
                    type="text"
                    prop:value=move || title.get()
                    on:input=move |ev| {
                        title.set(event_target_value(&ev));
                        schedule_save();
                    }
                    placeholder="Note subject / title…"
                    class="flex-1 text-xl font-semibold bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 hover:border-slate-400 dark:hover:border-slate-600 rounded-lg px-3.5 py-2 text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 shadow-sm focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                />
                <span class="text-xs font-medium text-slate-500 dark:text-slate-400 px-2 py-1 bg-slate-100 dark:bg-slate-800 rounded-md shrink-0">
                    {move || if saved.get() { "Saved" } else { "Saving…" }}
                </span>
            </div>

            // Editor Action Bar with Quick Snippet Buttons & View Mode Selector
            <div class="flex flex-wrap items-center justify-between gap-2.5 pb-1">
                // Quick Markdown & Math Action Chips
                <div class="flex flex-wrap items-center gap-1.5 text-xs">
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("**", "**", "bold text")
                        }
                        title="Insert Bold (**text**)"
                        class="px-2.5 py-1 font-semibold rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "B"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("*", "*", "italic text")
                        }
                        title="Insert Italic (*text*)"
                        class="px-2.5 py-1 italic rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "I"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("### ", "", "Heading")
                        }
                        title="Insert Heading"
                        class="px-2 py-1 font-medium rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "H3"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("$", "$", "E = mc^2")
                        }
                        title="Insert Inline Math ($formula$)"
                        class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md border border-brand-200 dark:border-brand-900/60 bg-brand-50/70 dark:bg-brand-950/40 text-brand-700 dark:text-brand-300 hover:bg-brand-100/70 dark:hover:bg-brand-900/40 transition font-mono font-medium"
                    >
                        "$f(x)$"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("$$\n", "\n$$", "\\sum_{i=1}^{n} x_i")
                        }
                        title="Insert Block Math ($$formula$$)"
                        class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md border border-brand-200 dark:border-brand-900/60 bg-brand-50/70 dark:bg-brand-950/40 text-brand-700 dark:text-brand-300 hover:bg-brand-100/70 dark:hover:bg-brand-900/40 transition font-mono font-medium"
                    >
                        "$$ Block $$"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("```rust\n", "\n```", "// code here")
                        }
                        title="Insert Code Block"
                        class="px-2 py-1 font-mono text-[11px] rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "{ }"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("- [ ] ", "", "task item")
                        }
                        title="Insert Task checklist"
                        class="px-2 py-1 rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "☑ Task"
                    </button>
                    <button
                        type="button"
                        on:click={
                            let insert = insert_snippet.clone();
                            move |_| insert("| Header 1 | Header 2 |\n| --- | --- |\n| Cell 1 | Cell 2 |", "", "")
                        }
                        title="Insert Table"
                        class="px-2 py-1 rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                    >
                        "⊞ Table"
                    </button>
                </div>

                // View Mode Toggles & Word Count
                <div class="flex items-center gap-3">
                    <span class="text-[11px] text-slate-500 dark:text-slate-400 hidden sm:inline">
                        {stats}
                    </span>

                    <div class="inline-flex rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 p-0.5 text-xs shadow-xs">
                        <button
                            type="button"
                            on:click=move |_| view_mode.set("edit")
                            class=move || {
                                if view_mode.get() == "edit" {
                                    "px-2.5 py-1 rounded-md bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-300 font-medium"
                                } else {
                                    "px-2.5 py-1 rounded-md text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                                }
                            }
                        >
                            "Edit"
                        </button>
                        <button
                            type="button"
                            on:click=move |_| view_mode.set("split")
                            class=move || {
                                if view_mode.get() == "split" {
                                    "px-2.5 py-1 rounded-md bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-300 font-medium"
                                } else {
                                    "px-2.5 py-1 rounded-md text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                                }
                            }
                        >
                            "Split"
                        </button>
                        <button
                            type="button"
                            on:click=move |_| view_mode.set("preview")
                            class=move || {
                                if view_mode.get() == "preview" {
                                    "px-2.5 py-1 rounded-md bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-300 font-medium"
                                } else {
                                    "px-2.5 py-1 rounded-md text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200"
                                }
                            }
                        >
                            "Preview"
                        </button>
                    </div>
                </div>
            </div>

            // ── Toolbar: attach / library / share / email / PDF / delete ─────
            <div class="flex items-center justify-between gap-3">
                <div class="flex flex-wrap items-center gap-2">

                    // Attach file
                    <label class="text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm transition">
                        "Attach file"
                        <input
                            type="file"
                            class="hidden"
                            on:change=move |ev| {
                                upload_error.set(None);
                                upload_from_change_event(ev, "note".to_string(), Some(id), move |res| {
                                    match res {
                                        Ok(u) => {
                                            let md = if u.content_type.starts_with("image/") {
                                                format!("\n\n![{}]({})\n\n", u.filename, u.url)
                                            } else {
                                                format!("\n\n[{}]({})\n\n", u.filename, u.url)
                                            };
                                            body.update(|b| b.push_str(&md));
                                            schedule_save();
                                        }
                                        Err(e) => upload_error.set(Some(e)),
                                    }
                                });
                            }
                        />
                    </label>

                    // Delete button (next to Attach file)
                    <Show
                        when=move || !confirm_delete.get()
                        fallback=move || view! {
                            <div class="inline-flex items-center gap-1.5">
                                <span class="text-xs text-rose-600 dark:text-rose-400 font-medium">
                                    "Delete this note?"
                                </span>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        delete_action.dispatch(DeleteNote { id });
                                    }
                                    disabled=move || delete_action.pending().get()
                                    class="text-xs rounded-md border border-rose-400 dark:border-rose-600 bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 px-2.5 py-1.5 hover:bg-rose-100 dark:hover:bg-rose-900/40 disabled:opacity-60 transition"
                                >
                                    {move || if delete_action.pending().get() { "Deleting…" } else { "Yes, delete" }}
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| confirm_delete.set(false)
                                    class="text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-2.5 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800 transition"
                                >
                                    "Cancel"
                                </button>
                            </div>
                        }
                    >
                        <button
                            type="button"
                            on:click=move |_| confirm_delete.set(true)
                            title="Delete note"
                            class="inline-flex items-center gap-1.5 text-xs rounded-md border border-rose-300 dark:border-rose-800 bg-white dark:bg-slate-900 px-3 py-1.5 hover:bg-rose-50 dark:hover:bg-rose-950/30 shadow-sm text-rose-600 dark:text-rose-400 transition"
                        >
                            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                            </svg>
                            "Delete"
                        </button>
                    </Show>

                    // From library
                    <div class="relative">
                        <button
                            type="button"
                            on:click=move |_| {
                                let now_open = !library_open.get_untracked();
                                library_open.set(now_open);
                                if now_open && library_files.get_untracked().is_none() {
                                    library_error.set(None);
                                    spawn_local(async move {
                                        match list_library_attachments().await {
                                            Ok(files) => library_files.set(Some(files)),
                                            Err(e) => library_error.set(Some(e.to_string())),
                                        }
                                    });
                                }
                            }
                            class="text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 cursor-pointer hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm transition"
                        >
                            "From library"
                        </button>
                        <Show when=move || library_open.get()>
                            <div class="absolute left-0 top-full mt-1 z-20 w-64 max-h-72 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-lg p-2">
                                <Show when=move || library_error.get().is_some()>
                                    <p class="text-xs text-rose-500 p-1">{move || library_error.get().unwrap_or_default()}</p>
                                </Show>
                                {move || match library_files.get() {
                                    None => view! { <p class="text-xs text-slate-500 dark:text-slate-400 p-1">"Loading…"</p> }.into_any(),
                                    Some(files) if files.is_empty() => view! {
                                        <p class="text-xs text-slate-500 dark:text-slate-400 p-1">
                                            "No files in your library yet — upload some from the Files page."
                                        </p>
                                    }.into_any(),
                                    Some(files) => view! {
                                        <ul class="space-y-0.5">
                                            <For
                                                each=move || files.clone()
                                                key=|f| f.id
                                                children=move |f| {
                                                    let filename = f.filename.clone();
                                                    let content_type = f.content_type.clone();
                                                    let url = f.url.clone();
                                                    view! {
                                                        <li>
                                                            <button
                                                                type="button"
                                                                on:click=move |_| {
                                                                    let md = if content_type.starts_with("image/") {
                                                                        format!("\n\n![{filename}]({url})\n\n")
                                                                    } else {
                                                                        format!("\n\n[{filename}]({url})\n\n")
                                                                    };
                                                                    body.update(|b| b.push_str(&md));
                                                                    schedule_save();
                                                                    library_open.set(false);
                                                                }
                                                                class="w-full text-left text-xs px-2 py-1.5 rounded-md hover:bg-slate-100 dark:hover:bg-slate-800 truncate text-slate-700 dark:text-slate-300"
                                                            >
                                                                {f.filename.clone()}
                                                            </button>
                                                        </li>
                                                    }
                                                }
                                            />
                                        </ul>
                                    }.into_any(),
                                }}
                            </div>
                        </Show>
                    </div>

                    // Share button
                    <button
                        type="button"
                        on:click=open_share_modal
                        class="inline-flex items-center gap-1.5 text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm text-slate-700 dark:text-slate-200 transition"
                    >
                        <svg class="w-3.5 h-3.5 text-slate-500 dark:text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"/>
                        </svg>
                        "Share"
                    </button>

                    // Send via email
                    <button
                        type="button"
                        on:click=move |_| {
                            email_feedback.set(None);
                            email_modal_open.update(|v| *v = !*v);
                        }
                        class="inline-flex items-center gap-1.5 text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm text-slate-700 dark:text-slate-200 transition"
                    >
                        <svg class="w-3.5 h-3.5 text-slate-500 dark:text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                        </svg>
                        "Send via email"
                    </button>

                    // Export to PDF
                    <button
                        type="button"
                        on:click=export_pdf
                        title="Export to PDF (browser print)"
                        class="inline-flex items-center gap-1.5 text-xs rounded-md border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800 shadow-sm text-slate-700 dark:text-slate-200 transition"
                    >
                        <svg class="w-3.5 h-3.5 text-slate-500 dark:text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                        </svg>
                        "Export PDF"
                    </button>
                </div>

                <Show when=move || upload_error.get().is_some()>
                    <span class="text-xs text-rose-500">{move || upload_error.get().unwrap_or_default()}</span>
                </Show>
            </div>

            // ── Share modal ──────────────────────────────────────────────────
            <Show when=move || share_modal_open.get()>
                <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-sm space-y-4">
                    <div class="flex items-center justify-between">
                        <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-600 dark:text-slate-400">
                            "Share note"
                        </h3>
                        <button
                            on:click=move |_| share_modal_open.set(false)
                            class="text-xs text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
                        >
                            "✕"
                        </button>
                    </div>

                    <Show when=move || share_error.get().is_some()>
                        <p class="text-xs text-rose-500 bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-800/60 rounded p-2">
                            {move || share_error.get().unwrap_or_default()}
                        </p>
                    </Show>

                    // ── Public link ───────────────────────────────────────────
                    <div class="space-y-2">
                        <p class="text-[11px] font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                            "Public link"
                        </p>
                        {move || {
                            if share_loading.get() {
                                view! { <p class="text-xs text-slate-500 dark:text-slate-400">"Loading…"</p> }.into_any()
                            } else if let Some(token) = share_token.get() {
                                let share_url = format!("/note/shared/{token}");
                                view! {
                                    <div class="space-y-2">
                                        <p class="text-xs text-slate-500 dark:text-slate-400">
                                            "Anyone with this link can view a read-only copy."
                                        </p>
                                        <div class="flex items-center gap-2">
                                            <input
                                                type="text"
                                                readonly
                                                prop:value=share_url.clone()
                                                class="flex-1 min-w-0 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs font-mono text-slate-700 dark:text-slate-300 focus:outline-none"
                                            />
                                            <button
                                                type="button"
                                                on:click=copy_share_link
                                                class="shrink-0 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 transition"
                                            >
                                                {move || if share_copied.get() { "Copied!" } else { "Copy" }}
                                            </button>
                                        </div>
                                        <div class="flex items-center gap-3">
                                            <a href=share_url target="_blank" rel="external noopener"
                                                class="text-xs text-brand-600 dark:text-brand-400 hover:underline">
                                                "Open link ↗"
                                            </a>
                                            <button type="button" on:click=revoke_share
                                                class="text-xs text-rose-500 dark:text-rose-400 hover:underline">
                                                "Revoke link"
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <form on:submit=create_share_link class="space-y-2">
                                        <p class="text-xs text-slate-500 dark:text-slate-400">
                                            "Create a public read-only link anyone can view."
                                        </p>
                                        <div class="flex items-center gap-2">
                                            <input
                                                type="password"
                                                placeholder="Protect with password (optional)"
                                                prop:value=move || share_new_password.get()
                                                on:input=move |ev| share_new_password.set(event_target_value(&ev))
                                                class="flex-1 min-w-0 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs focus:border-brand-500 focus:outline-none"
                                            />
                                            <button type="submit"
                                                disabled=move || share_loading.get()
                                                class="shrink-0 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 disabled:opacity-60 transition">
                                                {move || if share_loading.get() { "Creating…" } else { "Create share link" }}
                                            </button>
                                        </div>
                                    </form>
                                }.into_any()
                            }
                        }}
                    </div>

                    // Divider
                    <div class="border-t border-slate-200 dark:border-slate-700"/>

                    // ── Share with specific people ────────────────────────────
                    <div class="space-y-2">
                        <p class="text-[11px] font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                            "Share with people"
                        </p>
                        <form
                            on:submit=move |ev| {
                                ev.prevent_default();
                                let email = share_email_input.get_untracked();
                                if email.trim().is_empty() { return; }
                                share_email_error.set(None);
                                share_email_pending.set(true);
                                spawn_local(async move {
                                    match share_note_with_user(id, email).await {
                                        Ok(info) => {
                                            share_users.update(|v| v.push(info));
                                            share_email_input.set(String::new());
                                        }
                                        Err(e) => share_email_error.set(Some(e.to_string())),
                                    }
                                    share_email_pending.set(false);
                                });
                            }
                            class="flex items-center gap-2"
                        >
                            <input
                                type="email"
                                required
                                placeholder="colleague@example.com"
                                prop:value=move || share_email_input.get()
                                on:input=move |ev| share_email_input.set(event_target_value(&ev))
                                class="flex-1 min-w-0 rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs focus:border-brand-500 focus:outline-none"
                            />
                            <button
                                type="submit"
                                disabled=move || share_email_pending.get()
                                class="shrink-0 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 disabled:opacity-60 transition"
                            >
                                {move || if share_email_pending.get() { "Adding…" } else { "Add" }}
                            </button>
                        </form>

                        <Show when=move || share_email_error.get().is_some()>
                            <p class="text-xs text-rose-500">
                                {move || share_email_error.get().unwrap_or_default()}
                            </p>
                        </Show>

                        // List of shared users
                        {move || {
                            let users = share_users.get();
                            if users.is_empty() {
                                view! {
                                    <p class="text-xs text-slate-400 dark:text-slate-500 italic">
                                        "Not shared with anyone yet."
                                    </p>
                                }.into_any()
                            } else {
                                view! {
                                    <ul class="space-y-1 mt-1">
                                        <For
                                            each=move || share_users.get()
                                            key=|u| u.user_id
                                            children=move |u| {
                                                let uid = u.user_id;
                                                view! {
                                                    <li class="flex items-center justify-between rounded-md bg-slate-50 dark:bg-slate-800 px-3 py-1.5">
                                                        <span class="text-xs text-slate-700 dark:text-slate-300 truncate">
                                                            {u.email.clone()}
                                                        </span>
                                                        <button
                                                            type="button"
                                                            on:click=move |_| {
                                                                spawn_local(async move {
                                                                    if unshare_note_from_user(id, uid).await.is_ok() {
                                                                        share_users.update(|v| v.retain(|u| u.user_id != uid));
                                                                    }
                                                                });
                                                            }
                                                            class="ml-2 shrink-0 text-xs text-rose-400 hover:text-rose-600 dark:hover:text-rose-300"
                                                            title="Remove access"
                                                        >
                                                            "✕"
                                                        </button>
                                                    </li>
                                                }
                                            }
                                        />
                                    </ul>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </Show>

            // ── Send via Email modal ─────────────────────────────────────────
            <Show when=move || email_modal_open.get()>
                <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-sm space-y-3">
                    <div class="flex items-center justify-between">
                        <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-600 dark:text-slate-400">
                            "Send note copy via email"
                        </h3>
                        <button
                            on:click=move |_| email_modal_open.set(false)
                            class="text-xs text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
                        >
                            "✕"
                        </button>
                    </div>

                    <form
                        on:submit=move |ev| {
                            ev.prevent_default();
                            email_feedback.set(None);
                            let recipient = recipient_email.get();
                            send_mail_action.dispatch(SendNoteViaEmail {
                                id,
                                recipient_email: recipient,
                            });
                        }
                        class="flex flex-wrap items-center gap-2"
                    >
                        <input
                            type="email"
                            required
                            placeholder="recipient@example.com"
                            prop:value=move || recipient_email.get()
                            on:input=move |ev| recipient_email.set(event_target_value(&ev))
                            class="flex-1 min-w-[220px] rounded-md border border-slate-300 dark:border-slate-700 dark:bg-slate-800 px-3 py-1.5 text-xs focus:border-brand-500 focus:outline-none"
                        />
                        <button
                            type="submit"
                            disabled=move || send_mail_action.pending().get()
                            class="rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-brand-700 disabled:opacity-60 transition"
                        >
                            {move || if send_mail_action.pending().get() { "Sending…" } else { "Send" }}
                        </button>
                    </form>

                    {move || email_feedback.get().map(|(ok, msg)| {
                        let class_str = if ok {
                            "text-xs text-emerald-700 bg-emerald-50 dark:bg-emerald-950/50 border border-emerald-200 dark:border-emerald-800/60 rounded p-2"
                        } else {
                            "text-xs text-rose-600 bg-rose-50 dark:bg-rose-950/50 border border-rose-200 dark:border-rose-800/60 rounded p-2"
                        };
                        view! { <p class=class_str>{msg}</p> }
                    })}
                </div>
            </Show>

            <div class=move || {
                match view_mode.get() {
                    "edit" => "grid grid-cols-1 gap-4",
                    "preview" => "grid grid-cols-1 gap-4",
                    _ => "grid grid-cols-1 lg:grid-cols-2 gap-4",
                }
            }>
                <Show when=move || view_mode.get() != "preview">
                    <textarea
                        prop:value=move || body.get()
                        on:input=move |ev| {
                            body.set(event_target_value(&ev));
                            schedule_save();
                        }
                        rows="26"
                        placeholder="Write Markdown (supports $math$ and $$block math$$)…"
                        class="w-full min-h-[550px] rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-4 font-mono text-sm leading-relaxed text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 hover:border-slate-400 dark:hover:border-slate-600 focus:border-brand-500 focus:ring-2 focus:ring-brand-500/20 focus:outline-none transition"
                    ></textarea>
                </Show>
                <Show when=move || view_mode.get() != "edit">
                    <div
                        id="note-print-area"
                        class="rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-sm p-4 overflow-auto min-h-[550px]"
                    >
                        <MarkdownPreview body=Signal::derive(move || body.get())/>
                    </div>
                </Show>
            </div>
        </div>
    }
}
