#![allow(unused_imports)]
use super::forms::*;
use super::helpers::*;
use super::HttpRequest;
use super::TAILWIND_CDN;
use crate::runner::config::*;
use crate::runner::engine::*;
use anyhow::{Context, Result};
use chrono::{Local, Utc};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};
pub(crate) fn render_dashboard(
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
                        \
                        <a class='rounded border border-gray-300 px-4 py-2 text-sm font-semibold text-gray-800' href='/apps'>Apps</a>\
                        \
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

pub(crate) fn render_status_cards(
    status: &crate::runner::engine::RunnerStatus,
    task_count: usize,
) -> String {
    let running = if status.running_tasks_count > 0 {
        format!(
            "Running ({} active, {} queued)",
            status.running_tasks_count, status.queued_tasks_count
        )
    } else if status.queued_tasks_count > 0 {
        format!("Idle ({} queued)", status.queued_tasks_count)
    } else {
        "Idle".to_string()
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
        metric_card("State", &running),
        metric_card("Tasks", &task_count.to_string()),
        metric_card("Last Task", last_task),
        escape_html(&last_run),
        escape_html(last_error)
    )
}

pub(crate) fn metric_card(label: &str, value: &str) -> String {
    format!(
        "<div class='bg-white border border-gray-200 rounded shadow-sm p-4'><p class='text-xs uppercase tracking-wide text-gray-500 font-semibold'>{}</p><p class='mt-2 text-lg font-semibold text-gray-900 break-words'>{}</p></div>",
        escape_html(label),
        escape_html(value)
    )
}

pub(crate) fn render_task_row(task: &RunnerTask) -> String {
    let enabled_badge = if task.enabled {
        "<span class='inline-flex rounded bg-emerald-100 px-2 py-1 text-xs font-semibold text-emerald-800'>Enabled</span>"
    } else {
        "<span class='inline-flex rounded bg-gray-100 px-2 py-1 text-xs font-semibold text-gray-700'>Disabled</span>"
    };
    let kind = match task.legacy_kind() {
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
        TaskKind::ExternalApp { app_id, .. } => {
            format!("External App ({})", app_id)
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

pub(crate) fn render_task_form(
    title: &str,
    action: &str,
    submit_label: &str,
    task: Option<&RunnerTask>,
    error: Option<&str>,
) -> String {
    let id = task.map(|t| t.id.as_str()).unwrap_or_default();
    let name = task.map(|t| t.name.as_str()).unwrap_or_default();
    let enabled = task.map(|t| t.enabled).unwrap_or(true);
    let post_run_script = task.map(|t| t.legacy_post_run_script()).unwrap_or_default();
    let post_run_app_id = task.map(|t| t.legacy_post_run_app_id()).unwrap_or_default();
    let post_run_app_args = task
        .map(|t| {
            serde_json::to_string(&t.legacy_post_run_app_args())
                .unwrap_or_else(|_| "{}".to_string())
        })
        .unwrap_or_else(|| "{}".to_string());

    let post_run_action = if !post_run_app_id.is_empty() {
        "external_app"
    } else if !post_run_script.is_empty() {
        "script"
    } else {
        "none"
    };

    let timeout_seconds = task.map(|t| t.timeout_seconds).unwrap_or(0);
    let timeout_seconds_str = if timeout_seconds > 0 {
        timeout_seconds.to_string()
    } else {
        String::new()
    };
    let mut ext_app_id = String::new();
    let mut ext_app_args = String::new();

    let (task_type, _report) = match task.map(|t| t.legacy_kind()) {
        Some(TaskKind::ShellCommand { .. }) => ("shell_command", "all"),
        Some(TaskKind::ExternalApp { app_id, args }) => {
            ext_app_id = app_id.clone();
            ext_app_args = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
            ("external_app", "all")
        }
        None => ("shell_command", "all"),
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
            <div><a class='text-sm font-semibold text-emerald-700' href='/'>Back to dashboard</a><h1 class='text-3xl font-bold text-gray-900 mt-3'>{title}</h1></div>\
            {error_html}\
            <form class='bg-white border border-gray-200 rounded shadow-sm p-5 space-y-5' method='post' action='{action}'>\
                <div class='grid md:grid-cols-2 gap-4'>\
                    {id_field}\
                    {name_field}\
                </div>\
                <div class='grid md:grid-cols-2 gap-4 items-center'>\
                    <label class='flex items-center gap-2 text-sm font-semibold text-gray-800 h-full mt-4'><input type='checkbox' name='enabled' value='on' {checked_attr}> Enabled</label>\
                    {type_select}\
                </div>\
                <div class='grid md:grid-cols-2 gap-4'>\
                    <label class='block mb-4'>\
                        <span class='text-sm font-semibold text-gray-700'>Timeout (Seconds)</span>\
                        <input class='mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm' type='number' name='timeout_seconds' value='{timeout_val}' placeholder='0 (Global default)'>\
                        <p class='text-xs text-gray-500 mt-1'>Overrides the global timeout.</p>\
                    </label>\
                </div>\
                <div class='mb-4 p-4 border border-gray-200 bg-gray-50 rounded'>
                    <h3 class='text-lg font-semibold text-gray-800 mb-2'>Post Run Action</h3>
                    <select id='post_run_action_select' name='post_run_action' class='mb-4 block w-full rounded border border-gray-300 px-3 py-2 text-sm'>
                        <option value='none' {post_run_none_selected}>None</option>
                        <option value='script' {post_run_script_selected}>Script</option>
                        <option value='external_app' {post_run_app_selected}>External Application</option>
                    </select>

                    <div id='post_run_script_container' class='{post_run_script_class}'>
                        <label class='block mb-2'>
                            <span class='text-sm font-semibold text-gray-700'>Post Run Script</span>
                            <input class='mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' id='post_run_script_input' name='post_run_script' value='{post_run_val}' placeholder='C:\\Scripts\\after_fetch.vbs'>
                            <p class='text-xs text-gray-500 mt-1'>Runs a script after a task successfully completes.</p>
                        </label>
                    </div>

                    <div id='post_run_app_container' class='{post_run_app_class} space-y-4'>
                        <div id='post-run-external-app-select-container' class='mb-4'></div>
                        <div id='post-run-external-app-dynamic-inputs' class='space-y-3'></div>
                        <input type='hidden' id='post_run_app_args' name='post_run_app_args' value='{post_run_args_val}'>
                        <input type='hidden' id='post_run_app_id' name='post_run_app_id' value='{post_run_id_val}'>
                    </div>
                </div>
                {schedule_editor}\
                {command_editor}\
                <div id='external-app-container' class='hidden space-y-4 p-4 border border-purple-200 bg-purple-50 rounded'>\
                    <h3 class='text-lg font-semibold text-purple-800'>External Application</h3>\
                    <div id='external-app-select-container' class='mb-4'></div>\
                    <div id='external-app-dynamic-inputs' class='space-y-3'></div>\
                    <input type='hidden' id='external_app_args' name='external_app_args' value='{ext_args}'>\
                    <input type='hidden' id='external_app_id' name='external_app_id' value='{ext_id}'>\
                </div>\
                <button class='rounded bg-emerald-600 text-white px-4 py-2 text-sm font-semibold' type='submit'>{submit_label}</button>\
            </form>\
        </div>",
        title = escape_html(title),
        error_html = error_html,
        action = action,
        id_field = input_field("ID", "id", id),
        name_field = input_field("Name", "name", name),
        checked_attr = if enabled { "checked" } else { "" },
        type_select = select_task_type(task_type),
        post_run_val = escape_html(&post_run_script),
        post_run_args_val = post_run_app_args.replace("'", "&#39;"),
        post_run_id_val = escape_html(&post_run_app_id),
        post_run_none_selected = if post_run_action == "none" { "selected" } else { "" },
        post_run_script_selected = if post_run_action == "script" { "selected" } else { "" },
        post_run_app_selected = if post_run_action == "external_app" { "selected" } else { "" },
        post_run_script_class = if post_run_action == "script" { "block" } else { "hidden" },
        post_run_app_class = if post_run_action == "external_app" { "block" } else { "hidden" },
        timeout_val = escape_html(&timeout_seconds_str),
        schedule_editor = schedule_editor_html(task),
        command_editor = shell_command_editor_html(task),
        ext_args = ext_app_args.replace("'", "&#39;"),
        ext_id = escape_html(&ext_app_id),
        submit_label = escape_html(submit_label)
    );
    html_page(title, &form_html)
}

pub(crate) fn schedule_editor_html(task: Option<&RunnerTask>) -> String {
    let rows = if let Some(task) = task {
        schedule_rows_html(task)
    } else {
        schedule_row_html(0, "interval", "1h", "", "", "", "", None, None)
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

pub(crate) fn shell_command_editor_html(task: Option<&RunnerTask>) -> String {
    let mode = match task.map(|t| t.legacy_kind()) {
        Some(TaskKind::ShellCommand { mode, .. }) => mode,
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

pub(crate) fn schedule_rows_html(task: &RunnerTask) -> String {
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
                    Some(task),
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
                    Some(task),
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
                    Some(task),
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
                    Some(task),
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
                    Some(task),
                ));
                index += 1;
            }
        }
    }
    if rows.is_empty() {
        rows.push(schedule_row_html(
            0, "interval", "1h", "", "", "", "", None, None,
        ));
    }
    rows.join("")
}

pub(crate) fn shell_command_rows_html(task: &RunnerTask) -> String {
    match task.legacy_kind() {
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
pub(crate) fn schedule_row_html(
    index: usize,
    kind: &str,
    interval_value: &str,
    once_value: &str,
    daily_value: &str,
    weekly_value: &str,
    monthly_value: &str,
    working_hours: Option<&std::collections::HashMap<String, crate::runner::config::WorkingHours>>,
    task: Option<&RunnerTask>,
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

    let mut start_time_val = String::new();
    if let Some(TaskSchedule::Interval { start_time, .. }) =
        task.and_then(|t| t.schedules.get(index))
    {
        start_time_val = start_time.clone().unwrap_or_default();
    }

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
          <div class='grid md:grid-cols-6 gap-2 items-end'>\
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
            <label class='block schedule-interval schedule-start-time {}'>\
                <span class='text-xs font-semibold text-gray-700'>Start Time (HH:MM)</span>\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='time' name='schedule_start_time_{}' value='{}'>\
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
        interval_hidden,
        index,
        start_time_val,
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

pub(crate) fn days_of_week_options(selected_day: &str) -> String {
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

pub(crate) fn command_row_html(index: usize, command: &str, continue_on_error: bool) -> String {
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

pub(crate) fn local_datetime_value(value: &str) -> String {
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

pub(crate) fn input_field(label: &str, name: &str, value: &str) -> String {
    format!(
        "<label class='block'><span class='text-sm font-semibold text-gray-800'>{}</span><input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' name='{}' value='{}'></label>",
        escape_html(label),
        escape_html(name),
        escape_html(value)
    )
}

pub(crate) fn select_task_type(value: &str) -> String {
    format!(
        "<label class='block'><span class='text-sm font-semibold text-gray-800'>Task Type</span><select id='task-type-select' class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='task_type'><option value='shell_command' {}>Shell Command</option><option value='external_app' {}>External App</option></select></label>",
        if value == "shell_command" { "selected" } else { "" },
        if value == "external_app" { "selected" } else { "" }
    )
}

pub(crate) fn html_page(title: &str, content: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width, initial-scale=1'><title>{}</title><script src='{}'></script></head><body class='bg-gray-50 text-gray-900'><main class='max-w-7xl mx-auto px-4 py-8'>{}</main><script src='/assets/js/common.js'></script><script src='/assets/js/api.js'></script><script src='/assets/js/validation.js'></script><script src='/assets/js/notifications.js'></script><script src='/assets/js/forms.js'></script></body></html>",
        escape_html(title),
        TAILWIND_CDN,
        content
    )
}

pub(crate) fn render_redirect_to_dashboard(message: &str) -> String {
    html_page(
        "Redirecting",
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-gray-200 rounded shadow-sm p-6'><h1 class='text-2xl font-bold text-gray-900'>Redirecting</h1><p class='mt-4 text-gray-700'>Returning to the dashboard...</p></div><script>window.addEventListener('DOMContentLoaded', function() {{ window.redirectToDashboard('{}'); }});</script>",
            js_escape(message)
        ),
    )
}

pub(crate) fn render_toast(message: &str) -> String {
    format!(
        "<div id='runner-toast' class='fixed right-4 top-4 z-50 max-w-sm rounded border border-gray-200 bg-white px-4 py-3 shadow-lg'>\
            <p class='text-sm font-semibold text-gray-900'>{}</p>\
        </div>",
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

pub(crate) fn render_apps_page(apps: &[crate::runner::config::RegisteredApp]) -> String {
    let rows = apps.iter().map(|app| {
        format!(
            "<tr>\
                <td class='px-4 py-3 align-top font-semibold text-gray-900'>{}</td>\
                <td class='px-4 py-3 align-top text-gray-700'>{}</td>\
                <td class='px-4 py-3 align-top text-gray-700 font-mono text-xs'>{}</td>\
                <td class='px-4 py-3 align-top text-gray-700 font-mono text-xs'>{}</td>\
                <td class='px-4 py-3 align-top'>\
                    <a class='rounded bg-blue-600 text-white px-3 py-1 text-sm font-semibold hover:bg-blue-700 mr-2' href='/apps/edit/{}'>Edit</a>\
                    <a class='rounded bg-red-600 text-white px-3 py-1 text-sm font-semibold hover:bg-red-700' href='/apps/delete/{}'>Delete</a>\
                </td>\
            </tr>",
            escape_html(&app.name),
            escape_html(&app.id),
            escape_html(&app.executable_path),
            escape_html(&app.config_path),
            escape_html(&app.id),
            escape_html(&app.id)
        )
    }).collect::<String>();

    html_page(
        "Registered Apps",
        &format!(
            "<div class='max-w-5xl mx-auto space-y-6'>\
                <div>\
                    <a class='text-sm font-semibold text-emerald-700' href='/'>Back to dashboard</a>\
                    <h1 class='text-3xl font-bold text-gray-900 mt-2'>Registered Applications</h1>\
                    <p class='text-gray-600 mt-1'>Manage external applications that can be scheduled as tasks.</p>\
                </div>\
                \
                <div class='bg-white border border-gray-200 rounded shadow-sm overflow-hidden'>\
                    <div class='px-5 py-4 border-b border-gray-200'>\
                        <h2 class='text-lg font-semibold text-gray-900'>App List</h2>\
                    </div>\
                    <div class='overflow-x-auto'>\
                        <table class='min-w-full divide-y divide-gray-200 text-sm'>\
                            <thead class='bg-gray-50'><tr>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Name</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>ID</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Executable</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Config Path</th>\
                                <th class='px-4 py-3 text-left font-semibold text-gray-700'>Actions</th>\
                            </tr></thead>\
                            <tbody class='bg-white divide-y divide-gray-100'>{}</tbody>\
                        </table>\
                    </div>\
                </div>\
                \
                <div class='bg-white border border-gray-200 rounded shadow-sm p-5'>\
                    <h2 class='text-lg font-semibold text-gray-900 mb-4'>Register New App</h2>\
                    <form method='post' action='/apps/create' class='space-y-4 max-w-2xl'>\
                        <div class='grid md:grid-cols-2 gap-4'>\
                            {}\
                            {}\
                        </div>\
                        {}\
                        {}\
                        <button class='rounded bg-emerald-600 text-white px-4 py-2 text-sm font-semibold' type='submit'>Register App</button>\
                    </form>\
                </div>\
            </div>",
            rows,
            input_field("Name", "name", ""),
            input_field("App ID", "id", ""),
            input_field("Executable Path (e.g. tasker.exe)", "executable_path", ""),
            input_field("Config Path (Optional override)", "config_path", "")
        )
    )
}

pub(crate) fn render_app_edit_page(app: &crate::runner::config::RegisteredApp) -> String {
    html_page(
        "Edit App",
        &format!(
            "<div class='max-w-2xl mx-auto'>                <h1 class='text-2xl font-bold text-gray-900 mb-6'>Edit App</h1>                <form action='/apps/update/{}' method='POST' class='space-y-4 bg-white p-6 rounded shadow-sm border border-gray-200'>                    {}                    {}                    {}                    <div class='pt-4 flex gap-3'>                        <button type='submit' class='rounded bg-emerald-600 px-4 py-2 text-sm font-semibold text-white'>Update App</button>                        <a href='/apps' class='rounded border border-gray-300 px-4 py-2 text-sm font-semibold text-gray-700'>Cancel</a>                    </div>                </form>            </div>",
            escape_html(&app.id),
            input_field("Name", "name", &app.name),
            input_field("Executable Path", "executable_path", &app.executable_path),
            input_field("Config Path", "config_path", &app.config_path)
        ),
    )
}
