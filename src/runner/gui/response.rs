use crate::runner::gui::helpers::{escape_html, js_escape};

const TAILWIND_CDN: &str = "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4";

pub(crate) fn html_page(title: &str, content: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width, initial-scale=1'><title>{}</title><script src='{}'></script></head><body class='bg-gray-50 text-gray-900'><main class='max-w-7xl mx-auto px-4 py-8'>{}</main></body></html>",
        escape_html(title),
        TAILWIND_CDN,
        content
    )
}

pub(crate) fn render_redirect_to_dashboard(message: &str) -> String {
    html_page(
        "Redirecting",
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-gray-200 rounded shadow-sm p-6'><h1 class='text-2xl font-bold text-gray-900'>Redirecting</h1><p class='mt-4 text-gray-700'>Returning to the dashboard...</p></div><script>const msg='{}'; window.location.replace('/?toast=' + encodeURIComponent(msg));</script>",
            js_escape(message)
        ),
    )
}

pub(crate) fn render_toast(message: &str) -> String {
    format!(
        "<div id='runner-toast' class='fixed right-4 top-4 z-50 max-w-sm rounded border border-gray-200 bg-white px-4 py-3 shadow-lg'>\
            <p class='text-sm font-semibold text-gray-900'>{}</p>\
        </div><script>setTimeout(()=>{{const t=document.getElementById('runner-toast'); if(t) t.remove();}},4000);</script>",
        escape_html(message)
    )
}

pub(crate) fn render_error_page(title: &str, message: &str) -> String {
    html_page(
        title,
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-red-200 rounded shadow-sm p-6'><h1 class='text-2xl font-bold text-red-800'>{}</h1><p class='mt-3 text-gray-700 break-words'>{}</p><p class='mt-4'><a class='rounded bg-gray-900 text-white px-4 py-2 text-sm font-semibold' href='/'>Open dashboard</a></p></div>",
            escape_html(title),
            escape_html(message)
        ),
    )
}
