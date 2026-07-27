use super::helpers::escape_html;
use super::icons::*;

/// Base page layout wrapper. Supports the Sidebar + Top Nav + Main Content layout.
pub(crate) fn layout(title: &str, main_content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-gray-50">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Runner</title>
    <script src="{cdn}"></script>
    <style>
        :root {{
            --primary: #059669; /* emerald-600 */
            --primary-hover: #047857; /* emerald-700 */
        }}
    </style>
</head>
<body class="h-full flex flex-col md:flex-row overflow-hidden text-gray-900">
    {sidebar}
    <div class="flex-1 flex flex-col min-w-0 overflow-hidden md:pl-64">
        {top_nav}
        <main class="flex-1 overflow-y-auto p-4 md:p-8">
            <div class="mx-auto max-w-7xl">
                {main_content}
            </div>
        </main>
    </div>
    <script src="/js/common.js"></script>
    <script src="/js/api.js"></script>
    <script src="/js/notifications.js"></script>
    <script src="/js/validation.js"></script>
    <script src="/js/forms.js"></script>
</body>
</html>"#,
        title = escape_html(title),
        cdn = super::TAILWIND_CDN,
        sidebar = sidebar(),
        top_nav = top_nav(),
        main_content = main_content,
    )
}

fn sidebar() -> String {
    format!(
        r#"
<div class="hidden md:flex md:w-64 md:flex-col md:fixed md:inset-y-0 bg-gray-900 border-r border-gray-800 z-10 transition-transform">
    <div class="flex flex-col flex-grow pt-5 overflow-y-auto">
        <div class="flex items-center flex-shrink-0 px-4">
            <span class="text-white text-2xl font-bold tracking-tight">Runner</span>
        </div>
        <div class="mt-8 flex-1 flex flex-col">
            <nav class="flex-1 px-2 space-y-1">
                {nav_dashboard}
                {nav_apps}
                {nav_tasks}
                {nav_status}
            </nav>
        </div>
    </div>
</div>
<!-- Mobile Sidebar Overlay & Menu (controlled via JS) -->
<div id="mobile-menu" class="fixed inset-0 z-40 hidden md:hidden">
    <div class="fixed inset-0 bg-gray-600 bg-opacity-75" aria-hidden="true"></div>
    <div class="fixed inset-y-0 left-0 flex flex-col w-64 bg-gray-900 text-white z-50 transform -translate-x-full transition-transform" id="mobile-sidebar">
        <div class="flex items-center justify-between px-4 pt-5 pb-2">
            <span class="text-2xl font-bold tracking-tight">Runner</span>
            <button id="close-sidebar-btn" class="text-gray-300 hover:text-white">
                {icon_x}
            </button>
        </div>
        <nav class="mt-5 px-2 space-y-1">
            {nav_dashboard}
            {nav_apps}
            {nav_tasks}
            {nav_status}
        </nav>
    </div>
</div>
"#,
        icon_x = icon_x("w-6 h-6"),
        nav_dashboard = sidebar_link(
            "/",
            "Dashboard",
            &icon_dashboard("w-5 h-5 mr-3 flex-shrink-0")
        ),
        nav_apps = sidebar_link(
            "/apps",
            "Applications",
            &icon_cube("w-5 h-5 mr-3 flex-shrink-0")
        ),
        nav_tasks = sidebar_link(
            "/tasks",
            "Raw Tasks",
            &icon_code("w-5 h-5 mr-3 flex-shrink-0")
        ),
        nav_status = sidebar_link(
            "/status",
            "System Status",
            &icon_document_text("w-5 h-5 mr-3 flex-shrink-0")
        ),
    )
}

fn sidebar_link(href: &str, label: &str, icon_html: &str) -> String {
    format!(
        r#"<a href="{href}" class="text-gray-300 hover:bg-gray-800 hover:text-white group flex items-center px-2 py-2 text-sm font-medium rounded-md">
            {icon_html}
            {label}
        </a>"#,
        href = escape_html(href),
        label = escape_html(label),
        icon_html = icon_html
    )
}

fn top_nav() -> String {
    format!(
        r#"
<div class="md:pl-64 flex flex-col flex-1">
    <div class="sticky top-0 z-10 flex-shrink-0 flex h-16 bg-white border-b border-gray-200">
        <button type="button" id="open-sidebar-btn" class="px-4 border-r border-gray-200 text-gray-500 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-emerald-500 md:hidden">
            <span class="sr-only">Open sidebar</span>
            {icon_menu}
        </button>
        <div class="flex-1 px-4 flex justify-between">
            <div class="flex-1 flex items-center">
                <span class="text-gray-800 font-semibold md:hidden">Runner</span>
            </div>
            <div class="ml-4 flex items-center md:ml-6 gap-2">
                <a href="/new-task" class="inline-flex items-center px-3 py-1.5 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-emerald-600 hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500">
                    {icon_plus}
                    New Task
                </a>
            </div>
        </div>
    </div>
</div>
"#,
        icon_menu = icon_menu("h-6 w-6"),
        icon_plus = icon_plus("h-4 w-4 mr-1.5"),
    )
}

pub(crate) fn page_header(title: &str, subtitle: &str, actions: &str) -> String {
    format!(
        r#"<div class="mb-8 md:flex md:items-center md:justify-between">
    <div class="min-w-0 flex-1">
        <h2 class="text-2xl font-bold leading-7 text-gray-900 sm:truncate sm:text-3xl sm:tracking-tight">{title}</h2>
        {subtitle_html}
    </div>
    <div class="mt-4 flex md:ml-4 md:mt-0 gap-3">
        {actions}
    </div>
</div>"#,
        title = escape_html(title),
        subtitle_html = if subtitle.is_empty() {
            String::new()
        } else {
            format!(
                "<p class='mt-1 text-sm text-gray-500'>{}</p>",
                escape_html(subtitle)
            )
        },
        actions = actions
    )
}

pub(crate) fn card(title: &str, body: &str, footer: Option<&str>) -> String {
    let footer_html = match footer {
        Some(f) => format!(
            "<div class='bg-gray-50 px-4 py-3 border-t border-gray-200 sm:px-6'>{}</div>",
            f
        ),
        None => String::new(),
    };
    let header = if title.is_empty() {
        String::new()
    } else {
        format!("<div class='px-4 py-4 sm:px-6 border-b border-gray-200'><h3 class='text-lg leading-6 font-medium text-gray-900'>{}</h3></div>", escape_html(title))
    };

    format!(
        r#"<div class="bg-white overflow-hidden shadow-sm sm:rounded-lg border border-gray-200">
            {header}
            <div class="px-4 py-5 sm:p-6">
                {body}
            </div>
            {footer_html}
        </div>"#,
        header = header,
        body = body,
        footer_html = footer_html
    )
}

pub(crate) fn stat_card(title: &str, value: &str, description: &str) -> String {
    format!(
        r#"<div class="bg-white overflow-hidden shadow-sm rounded-lg border border-gray-200 p-5">
            <dt class="text-sm font-medium text-gray-500 truncate">{title}</dt>
            <dd class="mt-1 text-3xl font-semibold text-gray-900">{value}</dd>
            <dd class="mt-1 text-sm text-gray-500">{description}</dd>
        </div>"#,
        title = escape_html(title),
        value = value, // Assumed pre-escaped or raw HTML/numbers
        description = escape_html(description)
    )
}

pub(crate) fn primary_button(label: &str, href: Option<&str>, icon: Option<&str>) -> String {
    let icon_html = icon.unwrap_or("");
    if let Some(url) = href {
        format!(
            r#"<a href="{}" class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-emerald-600 hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500">
                {}{}
            </a>"#,
            escape_html(url),
            icon_html,
            escape_html(label)
        )
    } else {
        format!(
            r#"<button type="submit" class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-emerald-600 hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500">
                {}{}
            </button>"#,
            icon_html,
            escape_html(label)
        )
    }
}

pub(crate) fn secondary_button(label: &str, href: Option<&str>, icon: Option<&str>) -> String {
    let icon_html = icon.unwrap_or("");
    if let Some(url) = href {
        format!(
            r#"<a href="{}" class="inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500 shadow-sm">
                {}{}
            </a>"#,
            escape_html(url),
            icon_html,
            escape_html(label)
        )
    } else {
        format!(
            r#"<button type="button" class="inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500 shadow-sm">
                {}{}
            </button>"#,
            icon_html,
            escape_html(label)
        )
    }
}

#[allow(dead_code)]
pub(crate) fn danger_button(label: &str, href: Option<&str>, icon: Option<&str>) -> String {
    let icon_html = icon.unwrap_or("");
    if let Some(url) = href {
        format!(
            r#"<a href="{}" class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500">
                {}{}
            </a>"#,
            escape_html(url),
            icon_html,
            escape_html(label)
        )
    } else {
        format!(
            r#"<button type="button" class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500">
                {}{}
            </button>"#,
            icon_html,
            escape_html(label)
        )
    }
}

pub(crate) fn badge(text: &str, color_class: &str) -> String {
    format!(
        r#"<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}">
            {}
        </span>"#,
        color_class,
        escape_html(text)
    )
}

pub(crate) fn status_badge(is_active: bool, active_text: &str, inactive_text: &str) -> String {
    if is_active {
        badge(active_text, "bg-green-100 text-green-800")
    } else {
        badge(inactive_text, "bg-gray-100 text-gray-800")
    }
}

#[allow(dead_code)]
pub(crate) fn empty_state(title: &str, description: &str, action_html: Option<&str>) -> String {
    format!(
        r#"<div class="text-center py-12 border-2 border-dashed border-gray-300 rounded-lg">
            <h3 class="mt-2 text-sm font-semibold text-gray-900">{}</h3>
            <p class="mt-1 text-sm text-gray-500">{}</p>
            <div class="mt-6">
                {}
            </div>
        </div>"#,
        escape_html(title),
        escape_html(description),
        action_html.unwrap_or("")
    )
}

#[allow(dead_code)]
pub(crate) fn form_group(
    id: &str,
    label: &str,
    input_html: &str,
    help_text: Option<&str>,
) -> String {
    let help_html = match help_text {
        Some(text) => format!(
            "<p class='mt-1 text-sm text-gray-500'>{}</p>",
            escape_html(text)
        ),
        None => String::new(),
    };
    format!(
        r#"<div class="mb-4">
            <label for="{id}" class="block text-sm font-medium text-gray-700 mb-1">{label}</label>
            {input_html}
            {help_html}
        </div>"#,
        id = escape_html(id),
        label = escape_html(label),
        input_html = input_html,
        help_html = help_html
    )
}

#[allow(dead_code)]
pub(crate) fn text_input(id: &str, name: &str, value: &str, placeholder: &str) -> String {
    format!(
        r#"<input type="text" id="{id}" name="{name}" value="{value}" placeholder="{placeholder}" class="shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2">"#,
        id = escape_html(id),
        name = escape_html(name),
        value = escape_html(value),
        placeholder = escape_html(placeholder)
    )
}

#[allow(dead_code)]
pub(crate) fn checkbox(id: &str, name: &str, label: &str, checked: bool) -> String {
    let checked_attr = if checked { "checked" } else { "" };
    format!(
        r#"<div class="relative flex items-start mb-4">
            <div class="flex h-5 items-center">
                <input id="{id}" name="{name}" type="checkbox" value="true" {checked_attr} class="h-4 w-4 rounded border-gray-300 text-emerald-600 focus:ring-emerald-500">
            </div>
            <div class="ml-3 text-sm">
                <label for="{id}" class="font-medium text-gray-700">{label}</label>
            </div>
        </div>"#,
        id = escape_html(id),
        name = escape_html(name),
        label = escape_html(label),
        checked_attr = checked_attr
    )
}

#[allow(dead_code)]
pub(crate) fn textarea(id: &str, name: &str, value: &str, placeholder: &str, rows: u32) -> String {
    format!(
        r#"<textarea id="{id}" name="{name}" rows="{rows}" placeholder="{placeholder}" class="shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border border-gray-300 rounded-md p-2 font-mono">{value}</textarea>"#,
        id = escape_html(id),
        name = escape_html(name),
        rows = rows,
        placeholder = escape_html(placeholder),
        value = escape_html(value)
    )
}
