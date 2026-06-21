use anyhow::{Context, Result};
use chrono::{Local, Utc};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::runner::config::{
    human_datetime, next_daily_run_after, parse_rfc3339_utc, Repetition, ReportType, RunnerConfig,
    RunnerTask, ShellCommandMode, ShellCommandSpec, TaskKind, TaskSchedule,
};
use crate::runner::engine::{create_task, delete_task, update_task};
use crate::runner::engine::{RunnerCommand, RunnerHandle};

const TAILWIND_CDN: &str = "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4";

struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

pub fn start_gui_server(handle: RunnerHandle) {
    tokio::spawn(async move {
        if let Err(e) = run_server(handle).await {
            error!("Runner GUI server failed: {:#}", e);
        }
    });
}

async fn run_server(handle: RunnerHandle) -> Result<()> {
    let cfg = RunnerConfig::load(&handle.runner_config_path)?;
    let bind_addr = format!("{}:{}", cfg.gui_host, cfg.gui_port);
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("Runner GUI listening on http://{}", bind_addr);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let handle_clone = handle.clone();

        tokio::spawn(async move {
            let request = match read_http_request(&mut socket).await {
                Ok(Some(request)) => request,
                Ok(None) | Err(_) => return,
            };

            let (status, content_type, body) = match route_request(&request, &handle_clone).await {
                Ok(v) => v,
                Err(e) => (
                    500,
                    "text/html; charset=utf-8",
                    render_error_page("Request failed", &e.to_string()),
                ),
            };

            let response = format!(
                "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status,
                content_type,
                body.len(),
                body
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
    }
}

async fn read_http_request(socket: &mut tokio::net::TcpStream) -> Result<Option<HttpRequest>> {
    let mut buf = vec![0u8; 8192];
    let mut read = socket.read(&mut buf).await?;
    if read == 0 {
        return Ok(None);
    }

    let mut content_length = header_content_length(&buf[..read]).unwrap_or(0);
    while body_len(&buf[..read]) < content_length {
        if read == buf.len() {
            buf.resize(buf.len() * 2, 0);
        }
        let n = socket.read(&mut buf[read..]).await?;
        if n == 0 {
            break;
        }
        read += n;
        content_length = header_content_length(&buf[..read]).unwrap_or(content_length);
    }

    let req = String::from_utf8_lossy(&buf[..read]);
    let (headers, body) = req.split_once("\r\n\r\n").unwrap_or((req.as_ref(), ""));
    let first = headers.lines().next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();

    Ok(Some(HttpRequest {
        method,
        path,
        body: body.to_string(),
    }))
}

fn header_content_length(bytes: &[u8]) -> Option<usize> {
    let req = String::from_utf8_lossy(bytes);
    req.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

fn body_len(bytes: &[u8]) -> usize {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| bytes.len().saturating_sub(idx + 4))
        .unwrap_or(0)
}

async fn route_request(
    request: &HttpRequest,
    handle: &RunnerHandle,
) -> Result<(u16, &'static str, String)> {
    let (route_path, query_string) = split_path_and_query(&request.path);
    let query = parse_query_string(query_string);

    if request.method == "GET" && route_path == "/" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let status = handle.status.lock().await.clone();
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_dashboard(&cfg, &status, query.get("toast").map(String::as_str)),
        ));
    }

    if request.method == "GET" && route_path == "/status" {
        let status = handle.status.lock().await.clone();
        let body = serde_json::to_string_pretty(&status)?;
        return Ok((200, "application/json", body));
    }

    if request.method == "GET" && route_path == "/tasks" {
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        let body = serde_json::to_string_pretty(&cfg.tasks)?;
        return Ok((200, "application/json", body));
    }

    if request.method == "GET" && route_path == "/new-task" {
        let html = render_task_form("Create Task", "/create", "Create", None, None);
        return Ok((200, "text/html; charset=utf-8", html));
    }

    if request.method == "GET" && route_path.starts_with("/edit/") {
        let task_id = route_path.trim_start_matches("/edit/");
        let cfg = RunnerConfig::load(&handle.runner_config_path)?;
        if let Some(task) = cfg.tasks.iter().find(|t| t.id == task_id) {
            let action = format!("/update/{}", escape_html(task_id));
            let html = render_task_form("Edit Task", &action, "Update", Some(task), None);
            return Ok((200, "text/html; charset=utf-8", html));
        }
        return Ok((
            404,
            "text/html; charset=utf-8",
            render_error_page("Task not found", task_id),
        ));
    }

    if request.method == "POST" && route_path == "/create" {
        let values = parse_query_string(&request.body);
        let task = build_task_from_values(&values, None)?;
        create_task(&handle.runner_config_path, task).await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task created"),
        ));
    }

    if request.method == "POST" && route_path.starts_with("/update/") {
        let task_id = route_path.trim_start_matches("/update/").to_string();
        let values = parse_query_string(&request.body);
        let task = build_task_from_values(&values, Some(task_id.clone()))?;
        update_task(&handle.runner_config_path, &task_id, task).await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task updated"),
        ));
    }

    if request.method == "GET" && route_path == "/create" {
        let task = build_task_from_values(&query, None)?;
        create_task(&handle.runner_config_path, task).await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task created"),
        ));
    }

    if request.method == "GET" && route_path.starts_with("/update/") {
        let task_id = route_path.trim_start_matches("/update/").to_string();
        let task = build_task_from_values(&query, Some(task_id.clone()))?;
        update_task(&handle.runner_config_path, &task_id, task).await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task updated"),
        ));
    }

    if request.method == "GET" && route_path.starts_with("/delete/") {
        let task_id = route_path.trim_start_matches("/delete/").to_string();
        delete_task(&handle.runner_config_path, &task_id).await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task deleted"),
        ));
    }

    if request.method == "GET" && route_path == "/run-all" {
        handle.command_tx.send(RunnerCommand::RunAllNow).await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Run-all triggered"),
        ));
    }

    if request.method == "GET" && route_path == "/run-tickets" {
        handle
            .command_tx
            .send(RunnerCommand::RunAdhocCrm(ReportType::Tickets))
            .await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("CRM tickets run triggered"),
        ));
    }

    if request.method == "GET" && route_path.starts_with("/run/") {
        let task_id = route_path.trim_start_matches("/run/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::RunTaskNow(task_id.clone()))
            .await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task triggered"),
        ));
    }

    if request.method == "GET" && route_path.starts_with("/enable/") {
        let task_id = route_path.trim_start_matches("/enable/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::SetTaskEnabled {
                task_id,
                enabled: true,
            })
            .await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task enabled"),
        ));
    }

    if request.method == "GET" && route_path.starts_with("/disable/") {
        let task_id = route_path.trim_start_matches("/disable/").to_string();
        handle
            .command_tx
            .send(RunnerCommand::SetTaskEnabled {
                task_id,
                enabled: false,
            })
            .await?;
        return Ok((
            200,
            "text/html; charset=utf-8",
            render_redirect_to_dashboard("Task disabled"),
        ));
    }

    Ok((
        404,
        "text/html; charset=utf-8",
        render_error_page("Not found", route_path),
    ))
}

fn render_dashboard(
    cfg: &RunnerConfig,
    status: &crate::runner::engine::RunnerStatus,
    toast: Option<&str>,
) -> String {
    let rows = cfg
        .tasks
        .iter()
        .map(render_task_row)
        .collect::<Vec<_>>()
        .join("");

    let toast_html = toast.map(render_toast).unwrap_or_default();

    html_page(
        "Runner GUI",
        &format!(
            "{}<div class='space-y-6'>\
                <div class='flex flex-col md:flex-row md:items-end md:justify-between gap-4'>\
                    <div><p class='text-sm font-semibold text-emerald-700'>Runner</p><h1 class='text-3xl font-bold text-gray-900'>Task Dashboard</h1><p class='text-gray-600 mt-2'>Schedule CRM work and shell command groups from one local control panel.</p></div>\
                    <div class='flex flex-wrap gap-2'>\
                        <a class='rounded bg-gray-900 text-white px-4 py-2 text-sm font-semibold' href='/run-all'>Run All Now</a>\
                        <a class='rounded border border-gray-300 px-4 py-2 text-sm font-semibold text-gray-800' href='/run-tickets'>Run CRM Tickets</a>\
                        <a class='rounded bg-emerald-600 text-white px-4 py-2 text-sm font-semibold' href='/new-task'>New Task</a>\
                    </div>\
                </div>\
                <div class='grid md:grid-cols-4 gap-4'>\
                    {}\
                </div>\
                <div class='bg-white border border-gray-200 rounded shadow-sm overflow-hidden'>\
                    <div class='px-5 py-4 border-b border-gray-200 flex items-center justify-between'>\
                        <h2 class='text-lg font-semibold text-gray-900'>Tasks</h2>\
                        <div class='text-sm'><a class='text-emerald-700 font-semibold' href='/status'>JSON Status</a><span class='text-gray-300 mx-2'>|</span><a class='text-emerald-700 font-semibold' href='/tasks'>JSON Tasks</a></div>\
                    </div>\
                    <div class='overflow-x-auto'>\
                        <table class='min-w-full divide-y divide-gray-200 text-sm'>\
                            <thead class='bg-gray-50'><tr>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Task</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Schedule</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Next Run</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Last Run</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Status</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Actions</th>\
                            </tr></thead>\
                            <tbody class='bg-white divide-y divide-gray-100'>{}</tbody>\
                        </table>\
                    </div>\
                </div>\
            </div>",
            toast_html,
            render_status_cards(status, cfg.tasks.len()),
            rows
        ),
    )
}

fn render_status_cards(status: &crate::runner::engine::RunnerStatus, task_count: usize) -> String {
    let running = if status.currently_running {
        "Running"
    } else {
        "Idle"
    };
    let last_task = if status.last_task_id.is_empty() {
        "None"
    } else {
        &status.last_task_id
    };
    let last_run = if status.last_run_at.is_empty() {
        "Never".to_string()
    } else {
        human_datetime(&status.last_run_at)
    };
    let last_error = if status.last_error.is_empty() {
        "No current error"
    } else {
        &status.last_error
    };

    format!(
        "{}{}{}<div class='bg-white border border-gray-200 rounded shadow-sm p-4'>\
            <p class='text-xs uppercase tracking-wide text-gray-500 font-semibold'>Last Run</p>\
            <p class='mt-2 text-lg font-semibold text-gray-900 break-words'>{}\
                <span class='block text-xs text-gray-500 mt-1'>{}</span>\
            </p></div>",
        metric_card("State", running),
        metric_card("Tasks", &task_count.to_string()),
        metric_card("Last Task", last_task),
        escape_html(&last_run),
        escape_html(last_error)
    )
}

fn metric_card(label: &str, value: &str) -> String {
    format!(
        "<div class='bg-white border border-gray-200 rounded shadow-sm p-4'><p class='text-xs uppercase tracking-wide text-gray-500 font-semibold'>{}</p><p class='mt-2 text-lg font-semibold text-gray-900 break-words'>{}</p></div>",
        escape_html(label),
        escape_html(value)
    )
}

fn render_task_row(task: &RunnerTask) -> String {
    let enabled_badge = if task.enabled {
        "<span class='inline-flex rounded bg-emerald-100 px-2 py-1 text-xs font-semibold text-emerald-800'>Enabled</span>"
    } else {
        "<span class='inline-flex rounded bg-gray-100 px-2 py-1 text-xs font-semibold text-gray-700'>Disabled</span>"
    };
    let kind = match &task.kind {
        TaskKind::CrmFetch { report } => format!("CRM {}", report.as_arg()),
        TaskKind::ShellCommand { mode, commands } => {
            let command_count = commands.len();
            let mode_str = match mode {
                ShellCommandMode::Sequential => "seq",
                ShellCommandMode::Parallel => "par",
            };
            format!(
                "Shell, {} cmd{} ({})",
                command_count,
                if command_count == 1 { "" } else { "s" },
                mode_str
            )
        }
        TaskKind::Yasweb {
            report_type,
            report_name,
            ..
        } => {
            format!("Yasweb {} ({})", report_name, report_type)
        }
    };
    let last_run = if task.last_run_at.is_empty() {
        "Never".to_string()
    } else {
        human_datetime(&task.last_run_at)
    };
    let last_status = if task.last_status.is_empty() {
        "No result yet".to_string()
    } else {
        escape_html(&task.last_status)
    };
    let id = escape_html(&task.id);

    format!(
        "<tr>\
            <td class='px-4 py-4 align-top'><div class='font-semibold text-gray-900'>{}</div><div class='text-xs text-gray-500 mt-1'>{}</div><div class='mt-2'>{}</div></td>\
            <td class='px-4 py-4 align-top text-gray-700'>{}</td>\
            <td class='px-4 py-4 align-top text-gray-700'>{}</td>\
            <td class='px-4 py-4 align-top text-gray-700'>{}</td>\
            <td class='px-4 py-4 align-top text-gray-700 max-w-xs break-words'>{}</td>\
            <td class='px-4 py-4 align-top'><div class='flex flex-wrap gap-2'>\
                <a class='rounded border border-gray-300 px-3 py-1 font-semibold text-gray-800' href='/run/{}'>Run</a>\
                {}\
                {}\
                <a class='rounded bg-emerald-600 text-white px-3 py-1 text-sm font-semibold hover:bg-emerald-700' href='/edit/{}'>Edit</a>\
                <a class='rounded bg-red-600 text-white px-3 py-1 text-sm font-semibold hover:bg-red-700' href='/delete/{}'>Delete</a>\
            </div></td>\
        </tr>",
        escape_html(&task.name),
        id,
        enabled_badge,
        escape_html(&format!("{} - {}", kind, task.schedule_summary())),
        escape_html(&task.next_run_summary()),
        escape_html(&last_run),
        last_status,
        id,
        if !task.enabled { format!("<a class='rounded border border-gray-300 px-3 py-1 font-semibold text-gray-800' href='/enable/{}'>Enable</a>", id) } else { "".to_string() },
        if task.enabled { format!("<a class='rounded border border-gray-300 px-3 py-1 font-semibold text-gray-800' href='/disable/{}'>Disable</a>", id) } else { "".to_string() },
        id,
        id
    )
}

fn render_task_form(
    title: &str,
    action: &str,
    submit_label: &str,
    task: Option<&RunnerTask>,
    error: Option<&str>,
) -> String {
    let id = task.map(|t| t.id.as_str()).unwrap_or_default();
    let name = task.map(|t| t.name.as_str()).unwrap_or_default();
    let enabled = task.map(|t| t.enabled).unwrap_or(true);
    let post_run_script = task.map(|t| t.post_run_script.as_str()).unwrap_or_default();
    let timeout_seconds = task.map(|t| t.timeout_seconds).unwrap_or(0);
    let timeout_seconds_str = if timeout_seconds > 0 {
        timeout_seconds.to_string()
    } else {
        String::new()
    };
    let mut yasweb_type = String::new();
    let mut yasweb_name = String::new();
    let mut yasweb_filters = String::new();

    let (task_type, report) = match task.map(|t| &t.kind) {
        Some(TaskKind::ShellCommand { .. }) => ("shell_command", "all"),
        Some(TaskKind::Yasweb {
            report_type,
            report_name,
            filters,
        }) => {
            yasweb_type = report_type.clone();
            yasweb_name = report_name.clone();
            yasweb_filters = serde_json::to_string_pretty(filters).unwrap_or_default();
            ("yasweb", "all")
        }
        Some(TaskKind::CrmFetch { report }) => (
            "crm_fetch",
            match report {
                ReportType::All => "all",
                ReportType::Tickets => "tickets",
                ReportType::Calls => "calls",
                ReportType::Leads => "leads",
                ReportType::None => "none",
            },
        ),
        None => ("crm_fetch", "all"),
    };

    let error_html = error
        .map(|message| {
            format!(
                "<div class='rounded border border-red-200 bg-red-50 px-4 py-3 text-red-800 text-sm'>{}</div>",
                escape_html(message)
            )
        })
        .unwrap_or_default();

    let form_html = format!(
        "<div class='max-w-4xl mx-auto space-y-5'>\
            <div><a class='text-sm font-semibold text-emerald-700' href='/'>Back to dashboard</a><h1 class='text-3xl font-bold text-gray-900 mt-3'>{}</h1></div>\
            {}\
            <form class='bg-white border border-gray-200 rounded shadow-sm p-5 space-y-5' method='post' action='{}'>\
                <div class='grid md:grid-cols-2 gap-4'>\
                    {}\
                    {}\
                </div>\
                <label class='flex items-center gap-2 text-sm font-semibold text-gray-800'><input type='checkbox' name='enabled' value='on' {}> Enabled</label>\
                <div class='grid md:grid-cols-2 gap-4'>\
                    {}\
                    {}\
                </div>\
                <label class='block mb-4'>\
                    <span class='text-sm font-semibold text-gray-700'>Post Run Script (Optional)</span>\
                    <input class='mt-1 block w-full rounded border border-gray-300 px-3 py-2' type='text' name='post_run_script' value='{}' placeholder='C:\\Scripts\\after_fetch.vbs'>\
                    <p class='text-xs text-gray-500 mt-1'>Runs a script after a task successfully completes (.txt/.vbs using cscript, .ps1, .bat, etc.)</p>\
                </label>\
                <label class='block mb-4'>\
                    <span class='text-sm font-semibold text-gray-700'>Timeout (Seconds)</span>\
                    <input class='mt-1 block w-full md:w-1/4 rounded border border-gray-300 px-3 py-2' type='number' name='timeout_seconds' value='{}' placeholder='0 (Global default)'>\
                    <p class='text-xs text-gray-500 mt-1'>Overrides the global timeout for this task and its post run script. Leave blank or 0 to use the global timeout.</p>\
                </label>\
                {}\
                {}\
                <div id='yasweb-container' class='hidden space-y-4 p-4 border border-blue-200 bg-blue-50 rounded'>\
                    <h3 class='text-lg font-semibold text-blue-800'>Yasweb Configuration</h3>\
                    <div class='grid md:grid-cols-2 gap-4'>\
                        <label class='block'><span class='text-sm font-semibold text-gray-800'>Report Type</span><input class='mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' name='yasweb_type' value='{}'></label>\
                        <label class='block'><span class='text-sm font-semibold text-gray-800'>Report Name</span><input class='mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' name='yasweb_name' value='{}'></label>\
                    </div>\
                    <label class='block'><span class='text-sm font-semibold text-gray-800'>Filters (JSON)</span><textarea class='mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm font-mono' name='yasweb_filters' rows='4' placeholder='{{\"From Date\": \"21-Jun-2025 00:00\"}}'>{}</textarea></label>\
                </div>\
                <button class='rounded bg-emerald-600 text-white px-4 py-2 text-sm font-semibold' type='submit'>{}</button>\
            </form>\
        </div>",
        escape_html(title),
        error_html,
        action,
        input_field("ID", "id", id),
        input_field("Name", "name", name),
        if enabled { "checked" } else { "" },
        select_task_type(task_type),
        select_report(report),
        escape_html(post_run_script),
        escape_html(&timeout_seconds_str),
        schedule_editor_html(task),
        shell_command_editor_html(task),
        escape_html(&yasweb_type),
        escape_html(&yasweb_name),
        escape_html(&yasweb_filters),
        escape_html(submit_label)
    );
    let page_html = form_html + &form_script();
    html_page(title, &page_html)
}

fn schedule_editor_html(task: Option<&RunnerTask>) -> String {
    let rows = if let Some(task) = task {
        schedule_rows_html(task)
    } else {
        schedule_row_html(0, "interval", "1h", "", "", "", "", None)
    };

    format!(
        "<div class='space-y-3'>\
            <div class='flex items-center justify-between'>\
                <span class='text-sm font-semibold text-gray-800'>Schedules</span>\
                <button type='button' id='add-schedule-row' class='rounded border border-gray-300 bg-emerald-600 text-white px-3 py-1 text-sm font-semibold hover:bg-emerald-700'>+ Add schedule</button>\
            </div>\
            <div id='schedule-rows' class='space-y-3'>{}</div>\
            <input type='hidden' id='schedules-hidden' name='schedules' value=''>\
            <p class='text-xs text-gray-500'>Select one or more schedules. Supports: Interval, Once, Daily at specific times, Weekly on day, or Monthly on day.</p>\
        </div>",
        rows
    )
}

fn shell_command_editor_html(task: Option<&RunnerTask>) -> String {
    let mode = match task.map(|t| &t.kind) {
        Some(TaskKind::ShellCommand { mode, .. }) => *mode,
        _ => ShellCommandMode::Sequential,
    };
    let mode_html = format!(
        "<label class='block mb-3'>\
            <span class='text-sm font-semibold text-gray-800'>Execution Mode</span>\
            <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='shell_command_mode'>\
                <option value='sequential' {}>Sequential</option>\
                <option value='parallel' {}>Parallel</option>\
            </select>\
        </label>",
        if mode == ShellCommandMode::Sequential { "selected" } else { "" },
        if mode == ShellCommandMode::Parallel { "selected" } else { "" }
    );

    let rows = if let Some(task) = task {
        shell_command_rows_html(task)
    } else {
        command_row_html(0, "", false)
    };

    format!(
        "<div id='shell-command-container' class='space-y-3 hidden'>\
            {}\
            <div class='flex items-center justify-between'>\
                <span class='text-sm font-semibold text-gray-800'>Shell Commands</span>\
                <button type='button' id='add-command-row' class='rounded border border-gray-300 bg-emerald-600 text-white px-3 py-1 text-sm font-semibold hover:bg-emerald-700'>+ Add command</button>\
            </div>\
            <div id='command-rows' class='space-y-3'>{}</div>\
            <input type='hidden' id='commands-hidden' name='commands' value=''>\
            <div class='text-xs text-gray-600 space-y-1'>\
                <p><strong>Modes:</strong></p>\
                <ul class='list-disc list-inside'>\
                    <li><strong>Run:</strong> Halt on error (default)</li>\
                    <li><strong>Continue:</strong> Ignore errors and proceed</li>\
                </ul>\
            </div>\
        </div>",
        mode_html,
        rows
    )
}

fn schedule_rows_html(task: &RunnerTask) -> String {
    let mut rows = Vec::new();
    let mut index = 0;
    for schedule in &task.schedules {
        match schedule {
            TaskSchedule::Interval {
                every_seconds,
                working_hours,
                ..
            } => {
                rows.push(schedule_row_html(
                    index,
                    "interval",
                    &compact_duration(*every_seconds),
                    "",
                    "",
                    "",
                    "",
                    working_hours.as_ref(),
                ));
                index += 1;
            }
            TaskSchedule::Once { next_run_at, .. } => {
                rows.push(schedule_row_html(
                    index,
                    "once",
                    "1h",
                    &local_datetime_value(next_run_at),
                    "",
                    "",
                    "",
                    None,
                ));
                index += 1;
            }
            TaskSchedule::DailyTimes {
                times,
                working_hours,
                ..
            } => {
                rows.push(schedule_row_html(
                    index,
                    "daily",
                    "1h",
                    "",
                    &times.join(", "),
                    "",
                    "",
                    working_hours.as_ref(),
                ));
                index += 1;
            }
            TaskSchedule::Weekly { day_of_week, .. } => {
                rows.push(schedule_row_html(
                    index,
                    "weekly",
                    "1h",
                    "",
                    "",
                    day_of_week,
                    "",
                    None,
                ));
                index += 1;
            }
            TaskSchedule::Monthly { day_of_month, .. } => {
                rows.push(schedule_row_html(
                    index,
                    "monthly",
                    "1h",
                    "",
                    "",
                    "",
                    &day_of_month.to_string(),
                    None,
                ));
                index += 1;
            }
        }
    }
    if rows.is_empty() {
        rows.push(schedule_row_html(0, "interval", "1h", "", "", "", "", None));
    }
    rows.join("")
}

fn shell_command_rows_html(task: &RunnerTask) -> String {
    match &task.kind {
        TaskKind::ShellCommand { commands, .. } => {
            let rows = commands
                .iter()
                .enumerate()
                .map(|(index, spec)| command_row_html(index, &spec.command, spec.continue_on_error))
                .collect::<Vec<_>>();
            if rows.is_empty() {
                command_row_html(0, "", false)
            } else {
                rows.join("")
            }
        }
        _ => command_row_html(0, "", false),
    }
}

#[allow(clippy::too_many_arguments)]
fn schedule_row_html(
    index: usize,
    kind: &str,
    interval_value: &str,
    once_value: &str,
    daily_value: &str,
    weekly_value: &str,
    monthly_value: &str,
    working_hours: Option<&std::collections::HashMap<String, crate::runner::config::WorkingHours>>,
) -> String {
    let interval_hidden = if kind == "interval" { "" } else { "hidden" };
    let once_hidden = if kind == "once" { "" } else { "hidden" };
    let daily_hidden = if kind == "daily" { "" } else { "hidden" };
    let weekly_hidden = if kind == "weekly" { "" } else { "hidden" };
    let monthly_hidden = if kind == "monthly" { "" } else { "hidden" };
    let interval_options = [
        "15m", "30m", "1h", "2h", "4h", "8h", "12h", "24h", "2d", "7d",
    ]
    .iter()
    .map(|value| {
        format!(
            "<option value='{}' {}>{}</option>",
            value,
            if *value == interval_value {
                "selected"
            } else {
                ""
            },
            value
        )
    })
    .collect::<Vec<_>>()
    .join("");

    let days_of_week = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ]
    .iter()
    .map(|day| {
        format!(
            "<option value='{}' {}>{}</option>",
            day,
            if weekly_value == *day { "selected" } else { "" },
            day
        )
    })
    .collect::<Vec<_>>()
    .join("");

    let mut working_hours_html = String::new();
    if let Some(wh) = working_hours {
        for (day, hours) in wh {
            let day_options = days_of_week_options(day);
            working_hours_html.push_str(&format!(
                "<div class='flex gap-2 items-center mt-2' data-wh-row>\
                    <select class='rounded border border-gray-300 px-2 py-1 text-sm wh-day'>{}</select>\
                    <input class='rounded border border-gray-300 px-2 py-1 text-sm w-24 wh-start' type='time' value='{}'>\
                    <span class='text-xs text-gray-500'>to</span>\
                    <input class='rounded border border-gray-300 px-2 py-1 text-sm w-24 wh-end' type='time' value='{}'>\
                    <button type='button' class='remove-wh rounded bg-red-100 px-2 py-1 text-xs font-semibold text-red-700'>&times;</button>\
                </div>",
                day_options, hours.start, hours.end
            ));
        }
    }

    format!(
        "<div class='p-3 border border-gray-200 rounded mb-2' data-schedule-row>\
          <div class='grid md:grid-cols-5 gap-2 items-end'>\
            <label class='block'>\
                <span class='text-xs font-semibold text-gray-700'>Type</span>\
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm schedule-kind' name='schedule_kind_{}'>\
                    <option value='interval' {}>Interval</option>\
                    <option value='once' {}>Once</option>\
                    <option value='daily' {}>Daily</option>\
                    <option value='weekly' {}>Weekly</option>\
                    <option value='monthly' {}>Monthly</option>\
                </select>\
            </label>\
            <label class='block schedule-interval {}'>\
                <span class='text-xs font-semibold text-gray-700'>Interval</span>\
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='schedule_interval_{}'>{}\
                </select>\
            </label>\
            <label class='block schedule-once {}'>\
                <span class='text-xs font-semibold text-gray-700'>Date & Time</span>\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='datetime-local' name='schedule_once_at_{}' value='{}'>\
            </label>\
            <label class='block schedule-daily {}'>\
                <span class='text-xs font-semibold text-gray-700'>Times (HH:MM)</span>\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' name='schedule_daily_at_{}' value='{}' placeholder='09:00, 13:00'>\
            </label>\
            <label class='block schedule-weekly {}'>\
                <span class='text-xs font-semibold text-gray-700'>Day</span>\
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='schedule_weekly_at_{}' data-weekly-day>\
                    {}\
                </select>\
            </label>\
            <label class='block schedule-monthly {}'>\
                <span class='text-xs font-semibold text-gray-700'>Day (1-31)</span>\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='number' name='schedule_monthly_at_{}' value='{}' min='1' max='31'>\
            </label>\
            <button type='button' class='remove-schedule rounded border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-700'>Remove</button>\
          </div>\
          <div class='mt-3 schedule-wh {}'>\
              <div class='flex items-center justify-between'>\
                  <span class='text-xs font-semibold text-gray-700'>Working Hours (Optional)</span>\
                  <button type='button' class='add-wh-row rounded border border-gray-300 bg-white px-2 py-1 text-xs font-semibold text-gray-700 hover:bg-gray-50'>+ Add Day</button>\
              </div>\
              <div class='wh-rows'>{}</div>\
          </div>\
        </div>",
        index,
        if kind == "interval" { "selected" } else { "" },
        if kind == "once" { "selected" } else { "" },
        if kind == "daily" { "selected" } else { "" },
        if kind == "weekly" { "selected" } else { "" },
        if kind == "monthly" { "selected" } else { "" },
        interval_hidden,
        index,
        interval_options,
        once_hidden,
        index,
        escape_html(once_value),
        daily_hidden,
        index,
        escape_html(daily_value),
        weekly_hidden,
        index,
        days_of_week,
        monthly_hidden,
        index,
        escape_html(monthly_value),
        interval_hidden,
        working_hours_html,
    )
}

fn days_of_week_options(selected_day: &str) -> String {
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ]
    .iter()
    .map(|day| {
        format!(
            "<option value='{}' {}>{}</option>",
            day,
            if selected_day == *day { "selected" } else { "" },
            day
        )
    })
    .collect::<Vec<_>>()
    .join("")
}

fn command_row_html(index: usize, command: &str, continue_on_error: bool) -> String {
    format!(
        "<div class='grid md:grid-cols-[1fr_100px_auto] gap-2 items-center p-2 bg-gray-50 border border-gray-200 rounded' data-command-row>\
            <div class='grid md:grid-cols-2 gap-2'>\
                <label class='block'>\
                    <span class='text-xs font-semibold text-gray-700'>Command</span>\
                    <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm command-text' type='text' value='{}' placeholder='echo hello'>\
                </label>\
                <label class='block'>\
                    <span class='text-xs font-semibold text-gray-700'>Mode</span>\
                    <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm command-mode' name='command_mode_{}'>\
                        <option value='run' {}>Run</option>\
                        <option value='continue' {}>Continue</option>\
                    </select>\
                </label>\
            </div>\
            <button type='button' class='remove-command rounded bg-red-600 text-white px-3 py-2 text-sm font-semibold hover:bg-red-700'>Remove</button>\
        </div>",
        escape_html(command),
        index,
        if !continue_on_error { "selected" } else { "" },
        if continue_on_error { "selected" } else { "" }
    )
}

fn local_datetime_value(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return dt
            .with_timezone(&Local)
            .format("%Y-%m-%dT%H:%M")
            .to_string();
    }
    String::new()
}

fn form_script() -> String {
    format!("<script>{}</script>", include_str!("form_script.js"))
}

fn input_field(label: &str, name: &str, value: &str) -> String {
    format!(
        "<label class='block'><span class='text-sm font-semibold text-gray-800'>{}</span><input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' name='{}' value='{}'></label>",
        escape_html(label),
        escape_html(name),
        escape_html(value)
    )
}

fn select_task_type(value: &str) -> String {
    format!(
        "<label class='block'><span class='text-sm font-semibold text-gray-800'>Task Type</span><select id='task-type-select' class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='task_type'><option value='crm_fetch' {}>CRM Fetch</option><option value='shell_command' {}>Shell Command</option><option value='yasweb' {}>Yasweb Report</option></select></label>",
        if value == "crm_fetch" { "selected" } else { "" },
        if value == "shell_command" { "selected" } else { "" },
        if value == "yasweb" { "selected" } else { "" }
    )
}

fn select_report(value: &str) -> String {
    let option = |raw: &str, label: &str| {
        format!(
            "<option value='{}' {}>{}</option>",
            raw,
            if value == raw { "selected" } else { "" },
            label
        )
    };
    format!(
        "<div id='crm-report-container' class='block'><label class='block'><span class='text-sm font-semibold text-gray-800'>CRM Report</span><select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='report'>{}</select></label></div>",
        [
            option("all", "All"),
            option("tickets", "Tickets"),
            option("calls", "Calls"),
            option("leads", "Leads"),
            option("none", "None"),
        ]
        .join("")
    )
}

fn html_page(title: &str, content: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width, initial-scale=1'><title>{}</title><script src='{}'></script></head><body class='bg-gray-50 text-gray-900'><main class='max-w-7xl mx-auto px-4 py-8'>{}</main></body></html>",
        escape_html(title),
        TAILWIND_CDN,
        content
    )
}

fn render_redirect_to_dashboard(message: &str) -> String {
    html_page(
        "Redirecting",
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-gray-200 rounded shadow-sm p-6'><h1 class='text-2xl font-bold text-gray-900'>Redirecting</h1><p class='mt-4 text-gray-700'>Returning to the dashboard...</p></div><script>const msg='{}'; window.location.replace('/?toast=' + encodeURIComponent(msg));</script>",
            js_escape(message)
        ),
    )
}

fn render_toast(message: &str) -> String {
    format!(
        "<div id='runner-toast' class='fixed right-4 top-4 z-50 max-w-sm rounded border border-gray-200 bg-white px-4 py-3 shadow-lg'>\
            <p class='text-sm font-semibold text-gray-900'>{}</p>\
        </div><script>setTimeout(()=>{{const t=document.getElementById('runner-toast'); if(t) t.remove();}},4000);</script>",
        escape_html(message)
    )
}

fn js_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn render_error_page(title: &str, message: &str) -> String {
    html_page(
        title,
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-red-200 rounded shadow-sm p-6'><h1 class='text-2xl font-bold text-red-800'>{}</h1><p class='mt-3 text-gray-700 break-words'>{}</p><p class='mt-4'><a class='rounded bg-gray-900 text-white px-4 py-2 text-sm font-semibold' href='/'>Open dashboard</a></p></div>",
            escape_html(title),
            escape_html(message)
        ),
    )
}

fn split_path_and_query(path: &str) -> (&str, &str) {
    match path.split_once('?') {
        Some((route, query)) => (route, query),
        None => (path, ""),
    }
}

fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };
        values.insert(url_decode(raw_key), url_decode(raw_value));
    }
    values
}

fn url_decode(value: &str) -> String {
    let mut decoded = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(code) = u8::from_str_radix(hex, 16) {
                    decoded.push(code as char);
                    index += 3;
                } else {
                    decoded.push('%');
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte as char);
                index += 1;
            }
        }
    }

    decoded
}

fn parse_checkbox(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn parse_report_type(raw: Option<&String>) -> ReportType {
    match raw.map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "tickets" => ReportType::Tickets,
        Some(v) if v == "calls" => ReportType::Calls,
        Some(v) if v == "leads" => ReportType::Leads,
        Some(v) if v == "none" => ReportType::None,
        _ => ReportType::All,
    }
}

fn build_task_from_values(
    values: &HashMap<String, String>,
    fallback_id: Option<String>,
) -> Result<RunnerTask> {
    let id = values
        .get("id")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or(fallback_id)
        .unwrap_or_default();

    let name = values
        .get("name")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let post_run_script = values
        .get("post_run_script")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let timeout_seconds = values
        .get("timeout_seconds")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);

    let schedules = values
        .get("schedules")
        .map(|value| parse_schedules_text(value))
        .transpose()?
        .unwrap_or_default();
    let (repetition, frequency_seconds, next_run_at) = if values.contains_key("schedules") {
        legacy_fields_from_schedules(&schedules)
    } else {
        legacy_fields_from_values(values)
    };

    let task_type = values
        .get("task_type")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "crm_fetch".to_string());

    let kind = if task_type == "shell_command" {
        let mode = match values.get("shell_command_mode").map(|v| v.as_str()) {
            Some("parallel") => ShellCommandMode::Parallel,
            _ => ShellCommandMode::Sequential,
        };
        TaskKind::ShellCommand {
            mode,
            commands: parse_shell_commands_text(
                values
                    .get("commands")
                    .map(String::as_str)
                    .unwrap_or_default(),
            )?,
        }
    } else if task_type == "yasweb" {
        let filters_str = values
            .get("yasweb_filters")
            .map(String::as_str)
            .unwrap_or("{}");
        let filters = if filters_str.trim().is_empty() {
            std::collections::HashMap::new()
        } else {
            serde_json::from_str(filters_str)
                .with_context(|| format!("Invalid filters JSON: {}", filters_str))?
        };
        TaskKind::Yasweb {
            report_type: values
                .get("yasweb_type")
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            report_name: values
                .get("yasweb_name")
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            filters,
        }
    } else {
        TaskKind::CrmFetch {
            report: parse_report_type(values.get("report")),
        }
    };

    Ok(RunnerTask {
        id,
        name,
        enabled: parse_checkbox(values, "enabled"),
        repetition,
        frequency_seconds,
        next_run_at,
        schedules,
        kind,
        last_run_at: String::new(),
        last_status: String::new(),
        post_run_script,
        timeout_seconds,
    })
}

fn legacy_fields_from_schedules(schedules: &[TaskSchedule]) -> (Repetition, u64, String) {
    if let Some(schedule) = schedules.first() {
        match schedule {
            TaskSchedule::Interval {
                every_seconds,
                next_run_at,
                ..
            } => (Repetition::Repeat, *every_seconds, next_run_at.clone()),
            TaskSchedule::Once { next_run_at, .. } => (Repetition::Once, 0, next_run_at.clone()),
            TaskSchedule::DailyTimes { next_run_at, .. } => {
                (Repetition::Repeat, 24 * 60 * 60, next_run_at.clone())
            }
            TaskSchedule::Weekly { next_run_at, .. } => {
                (Repetition::Repeat, 7 * 24 * 60 * 60, next_run_at.clone())
            }
            TaskSchedule::Monthly { next_run_at, .. } => {
                (Repetition::Repeat, 30 * 24 * 60 * 60, next_run_at.clone())
            }
        }
    } else {
        (Repetition::Once, 3600, String::new())
    }
}

fn legacy_fields_from_values(values: &HashMap<String, String>) -> (Repetition, u64, String) {
    let repetition = match values.get("repetition").map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "repeat" => Repetition::Repeat,
        _ => Repetition::Once,
    };
    let frequency_seconds = values
        .get("frequency_seconds")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);
    let next_run_at = values
        .get("next_run_at")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    (repetition, frequency_seconds, next_run_at)
}

fn parse_schedules_text(value: &str) -> Result<Vec<TaskSchedule>> {
    let mut schedules = Vec::new();
    for raw_line in value.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (kind, rest) = line
            .split_once(':')
            .with_context(|| format!("Invalid schedule '{}'. Use kind: value", line))?;
        let kind = kind.trim().to_ascii_lowercase();
        let rest = rest.trim();

        match kind.as_str() {
            "interval" => {
                let mut every_str = rest;
                let mut working_hours = None;
                if let Some((e, wh_str)) = rest.split_once("; wh:") {
                    every_str = e.trim();
                    let mut wh_map = std::collections::HashMap::new();
                    for part in wh_str.split(',') {
                        if let Some((day, times)) = part.split_once('=') {
                            if let Some((start, end)) = times.split_once('-') {
                                wh_map.insert(
                                    day.trim().to_string(),
                                    crate::runner::config::WorkingHours {
                                        start: start.trim().to_string(),
                                        end: end.trim().to_string(),
                                    },
                                );
                            }
                        }
                    }
                    if !wh_map.is_empty() {
                        working_hours = Some(wh_map);
                    }
                }

                let every_str = every_str.strip_prefix("every").unwrap_or(every_str).trim();
                schedules.push(TaskSchedule::Interval {
                    enabled: true,
                    every_seconds: parse_duration_text(every_str)?,
                    next_run_at: Utc::now().to_rfc3339(),
                    working_hours,
                });
            }
            "daily" => {
                let mut times_str = rest;
                let mut working_hours = None;
                if let Some((t, wh_str)) = rest.split_once("; wh:") {
                    times_str = t.trim();
                    let mut wh_map = std::collections::HashMap::new();
                    for part in wh_str.split(',') {
                        if let Some((day, day_times)) = part.split_once('=') {
                            if let Some((start, end)) = day_times.split_once('-') {
                                wh_map.insert(
                                    day.trim().to_string(),
                                    crate::runner::config::WorkingHours {
                                        start: start.trim().to_string(),
                                        end: end.trim().to_string(),
                                    },
                                );
                            }
                        }
                    }
                    if !wh_map.is_empty() {
                        working_hours = Some(wh_map);
                    }
                }

                let times = times_str
                    .split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                let next_run_at = next_daily_run_after(&times, Utc::now(), working_hours.as_ref())?;
                schedules.push(TaskSchedule::DailyTimes {
                    enabled: true,
                    times,
                    next_run_at,
                    working_hours,
                });
            }
            "weekly" => {
                schedules.push(TaskSchedule::Weekly {
                    enabled: true,
                    day_of_week: rest.to_string(),
                    at_time: "09:00".to_string(),
                    next_run_at: Utc::now().to_rfc3339(),
                });
            }
            "monthly" => {
                let day_str = rest
                    .strip_prefix("day")
                    .unwrap_or(rest)
                    .trim()
                    .parse::<u32>()
                    .with_context(|| format!("Invalid day of month '{}'", rest))?;
                schedules.push(TaskSchedule::Monthly {
                    enabled: true,
                    day_of_month: day_str.clamp(1, 31),
                    at_time: "09:00".to_string(),
                    next_run_at: Utc::now().to_rfc3339(),
                });
            }
            "once" => {
                if !rest.is_empty() {
                    parse_rfc3339_utc(rest)?;
                }
                schedules.push(TaskSchedule::Once {
                    enabled: true,
                    next_run_at: rest.to_string(),
                });
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown schedule '{}'. Use interval, daily, weekly, monthly, or once",
                    kind
                ));
            }
        }
    }
    Ok(schedules)
}

fn parse_duration_text(value: &str) -> Result<u64> {
    let mut total = 0_u64;
    for token in value.split_whitespace() {
        total += parse_duration_token(token)?;
    }
    if total == 0 {
        parse_duration_token(value)
    } else {
        Ok(total)
    }
}

fn parse_duration_token(token: &str) -> Result<u64> {
    let token = token.trim();
    if token.is_empty() {
        return Err(anyhow::anyhow!("Duration is required"));
    }
    if let Ok(seconds) = token.parse::<u64>() {
        return Ok(seconds);
    }

    let split_at = token
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(token.len());
    let amount = token[..split_at]
        .parse::<u64>()
        .with_context(|| format!("Invalid duration '{}'", token))?;
    let unit = token[split_at..].to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3_600,
        "d" | "day" | "days" => 86_400,
        _ => return Err(anyhow::anyhow!("Invalid duration unit '{}'", unit)),
    };
    Ok(amount * multiplier)
}

fn compact_duration(seconds: u64) -> String {
    if seconds.is_multiple_of(86_400) {
        format!("{}d", seconds / 86_400)
    } else if seconds.is_multiple_of(3_600) {
        format!("{}h", seconds / 3_600)
    } else if seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{}s", seconds)
    }
}

fn parse_shell_commands_text(value: &str) -> Result<Vec<ShellCommandSpec>> {
    let mut commands = Vec::new();

    for raw_line in value.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(command) = line.strip_prefix("run:") {
            commands.push(ShellCommandSpec {
                command: command.trim().to_string(),
                continue_on_error: false,
            });
        } else if let Some(command) = line.strip_prefix("continue:") {
            commands.push(ShellCommandSpec {
                command: command.trim().to_string(),
                continue_on_error: true,
            });
        } else {
            commands.push(ShellCommandSpec {
                command: line.to_string(),
                continue_on_error: false,
            });
        }
    }

    Ok(commands)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schedule_text() {
        let schedules = parse_schedules_text(
            "interval: every 1h\ndaily: 09:00, 13:00\nonce: 2026-04-15T09:30:00-05:00",
        )
        .unwrap();
        assert_eq!(schedules.len(), 3);
        match &schedules[0] {
            TaskSchedule::Interval {
                every_seconds,
                working_hours,
                ..
            } => {
                assert_eq!(*every_seconds, 3_600);
                assert!(working_hours.is_none());
            }
            _ => panic!("expected interval"),
        }
    }

    #[test]
    fn parses_schedule_text_with_working_hours() {
        let schedules =
            parse_schedules_text("interval: every 2h; wh: Monday=09:00-17:00,Friday=10:00-15:00\n")
                .unwrap();
        assert_eq!(schedules.len(), 1);
        match &schedules[0] {
            TaskSchedule::Interval {
                every_seconds,
                working_hours,
                ..
            } => {
                assert_eq!(*every_seconds, 7_200);
                let wh = working_hours.as_ref().unwrap();
                assert_eq!(wh.len(), 2);
                assert_eq!(wh.get("Monday").unwrap().start, "09:00");
                assert_eq!(wh.get("Monday").unwrap().end, "17:00");
                assert_eq!(wh.get("Friday").unwrap().start, "10:00");
                assert_eq!(wh.get("Friday").unwrap().end, "15:00");
            }
            _ => panic!("expected interval"),
        }
    }

    #[test]
    fn parses_shell_commands_text_correctly() {
        let commands = parse_shell_commands_text(
            "run: echo prepare\ncontinue: cleanup-if-present\necho fallback",
        )
        .unwrap();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].command, "echo prepare");
        assert!(!commands[0].continue_on_error);
        assert_eq!(commands[1].command, "cleanup-if-present");
        assert!(commands[1].continue_on_error);
        assert_eq!(commands[2].command, "echo fallback");
        assert!(!commands[2].continue_on_error);
    }

    #[test]
    fn duration_parser_accepts_human_units() {
        assert_eq!(parse_duration_text("1h").unwrap(), 3_600);
        assert_eq!(parse_duration_text("1h 30m").unwrap(), 5_400);
        assert_eq!(parse_duration_text("90").unwrap(), 90);
    }

    #[test]
    fn human_datetime_accepts_rfc3339() {
        let text = human_datetime(&Utc::now().to_rfc3339());
        assert!(text.contains("local"));
    }

    #[test]
    fn date_type_import_keeps_rfc3339_parse_available() {
        let parsed: chrono::DateTime<Utc> = parse_rfc3339_utc("2026-04-15T09:30:00Z").unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-04-15T09:30:00+00:00");
    }

    #[test]
    fn metric_card_escapes_html_value() {
        let html = metric_card("Alert", "<script>alert(1)</script>");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
