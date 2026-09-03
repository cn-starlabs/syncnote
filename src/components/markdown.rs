use leptos::prelude::*;
use pulldown_cmark::{Event, Options, Parser};

/// Renders Markdown to HTML with math formula support ($...$ inline, $$...$$ block).
/// Raw HTML embedded in the source (`Event::Html` / `Event::InlineHtml`) is downgraded
/// to plain text instead of passed through — shared pages render other users' Markdown
/// via `inner_html`, so passing raw HTML through unfiltered would let any editor inject
/// a `<script>` that runs in every other viewer's browser.
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

    // Render block math $$...$$ and inline math $...$
    html_out = render_math_formulas(&html_out);

    html_out
}

/// Transforms block `$$...$$` into `<div class="math-block">...</div>`
/// and inline `$...$` into `<span class="math-inline">...</span>`.
/// Skips delimiters that occur inside `<pre><code>...</code></pre>` blocks or `<code>...</code>` spans.
fn render_math_formulas(html: &str) -> String {
    // Split HTML into tokens: code tags vs non-code content
    let mut result = String::with_capacity(html.len());
    let mut in_code = false;
    let mut last_idx = 0;

    let mut i = 0;
    let bytes = html.as_bytes();
    let len = bytes.len();

    while i < len {
        if !in_code && (html[i..].starts_with("<code") || html[i..].starts_with("<pre")) {
            // Find closing '>'
            if let Some(tag_end) = html[i..].find('>') {
                let chunk = &html[last_idx..i];
                result.push_str(&process_math_in_text(chunk));
                let end = i + tag_end + 1;
                result.push_str(&html[i..end]);
                last_idx = end;
                i = end;
                in_code = true;
                continue;
            }
        } else if in_code && (html[i..].starts_with("</code>") || html[i..].starts_with("</pre>")) {
            if let Some(tag_end) = html[i..].find('>') {
                let end = i + tag_end + 1;
                result.push_str(&html[last_idx..end]);
                last_idx = end;
                i = end;
                in_code = false;
                continue;
            }
        }
        i += 1;
    }

    if last_idx < len {
        let chunk = &html[last_idx..];
        if in_code {
            result.push_str(chunk);
        } else {
            result.push_str(&process_math_in_text(chunk));
        }
    }

    result
}

fn process_math_in_text(text: &str) -> String {
    // First process block math: $$...$$
    let mut out = String::with_capacity(text.len());
    let mut remainder = text;

    while let Some(start) = remainder.find("$$") {
        out.push_str(&process_inline_math(&remainder[..start]));
        let after_start = &remainder[start + 2..];
        if let Some(end) = after_start.find("$$") {
            let formula = &after_start[..end];
            out.push_str("<div class=\"katex-math-block\" data-expr=\"");
            escape_attribute(&mut out, formula);
            out.push_str("\">$$");
            out.push_str(formula);
            out.push_str("$$</div>");
            remainder = &after_start[end + 2..];
        } else {
            // Unclosed $$
            out.push_str("$$");
            remainder = after_start;
        }
    }
    out.push_str(&process_inline_math(remainder));

    out
}

fn process_inline_math(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if chars[i] == '$' {
            // Check that it is not preceded by backslash or digit (e.g. $10)
            let is_escaped = i > 0 && chars[i - 1] == '\\';
            let next_is_digit = i + 1 < n && chars[i + 1].is_ascii_digit();
            let next_is_space = i + 1 < n && (chars[i + 1].is_whitespace() || chars[i + 1] == '$');

            if !is_escaped && !next_is_digit && !next_is_space {
                // Look for matching closing '$'
                let mut found_end = None;
                let mut j = i + 1;
                while j < n {
                    if chars[j] == '$' && chars[j - 1] != '\\' {
                        // Ensure closing $ is not preceded by a space
                        if !chars[j - 1].is_whitespace() {
                            found_end = Some(j);
                            break;
                        }
                    }
                    // Prevent multiline inline formula or crossing HTML tags
                    if chars[j] == '\n' || chars[j] == '<' {
                        break;
                    }
                    j += 1;
                }

                if let Some(end) = found_end {
                    let formula: String = chars[i + 1..end].iter().collect();
                    out.push_str("<span class=\"katex-math-inline\" data-expr=\"");
                    escape_attribute(&mut out, &formula);
                    out.push_str("\">$");
                    out.push_str(&formula);
                    out.push_str("$</span>");
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    out
}

fn escape_attribute(out: &mut String, val: &str) {
    for c in val.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

/// Triggers KaTeX auto-rendering on the client after preview updates.
#[cfg(feature = "hydrate")]
pub fn trigger_katex_render() {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = renderMathInSyncNote)]
        fn render_math_in_sync_note();
    }

    let _ = render_math_in_sync_note();
}

#[cfg(not(feature = "hydrate"))]
pub fn trigger_katex_render() {}

#[component]
pub fn MarkdownPreview(#[prop(into)] body: Signal<String>) -> impl IntoView {
    Effect::new(move |_| {
        let _ = body.get();
        // Give DOM brief tick to hydrate/paint before invoking KaTeX renderer
        set_timeout(
            move || {
                trigger_katex_render();
            },
            std::time::Duration::from_millis(20),
        );
    });

    view! { <div class="prose-note" inner_html=move || render_markdown(&body.get())></div> }
}
