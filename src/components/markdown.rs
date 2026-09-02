use leptos::prelude::*;
use pulldown_cmark::{Event, Options, Parser};

/// Renders Markdown to HTML. Raw HTML embedded in the source (`Event::Html` /
/// `Event::InlineHtml`) is downgraded to plain text instead of passed through —
/// shared pages render other users' Markdown via `inner_html`, so passing raw
/// HTML through unfiltered would let any editor inject a `<script>` that runs
/// in every other viewer's browser.
pub fn render_markdown(src: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(src, options).map(|event| match event {
        Event::Html(html) => Event::Text(html),
        Event::InlineHtml(html) => Event::Text(html),
        other => other,
    });
    let mut html_out = String::new();
    pulldown_cmark::html::push_html(&mut html_out, parser);

    // Leptos Router intercepts clicks on <a> tags that don't have rel="external" or target="_blank".
    // For /attachments/ links, ensure they bypass client-side routing.
    html_out = html_out.replace(
        "<a href=\"/attachments/",
        "<a target=\"_blank\" rel=\"external\" download href=\"/attachments/",
    );

    html_out
}

#[component]
pub fn MarkdownPreview(#[prop(into)] body: Signal<String>) -> impl IntoView {
    view! { <div class="prose-note" inner_html=move || render_markdown(&body.get())></div> }
}
