#![allow(unused_imports)]
use super::components::*;
use super::forms::*;
use super::helpers::*;
use super::icons::*;
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
    let mut rows = String::new();
    if cfg.tasks.is_empty() {
        rows = "<tr><td colspan='5' class='px-6 py-12 text-center text-gray-500'>No tasks configured.</td></tr>".to_string();
    } else {
        for task in &cfg.tasks {
            rows.push_str(&render_task_row(task));
        }
    }

    let toast_html = toast.map(render_toast).unwrap_or_default();

    let header = page_header(
        "Dashboard",
        "Schedule CRM work and shell command groups from one local control panel.",
        &format!(
            "{} {} {}",
            secondary_button("Apps", Some("/apps"), Some(&icon_cube("w-4 h-4 mr-2"))),
            secondary_button(
                "Run All Now",
                Some("/run-all"),
                Some(&icon_play("w-4 h-4 mr-2"))
            ),
            primary_button(
                "New Task",
                Some("/new-task"),
                Some(&icon_plus("w-4 h-4 mr-2"))
            )
        ),
    );

    let stats_grid = format!(
        "<div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8'>{}</div>",
        render_status_cards(status)
    );

    let table_html = format!(
        "<div class='overflow-x-auto'>
            <table class='min-w-full divide-y divide-gray-200 text-sm'>
                <thead class='bg-gray-50'><tr>
                    <th scope='col' class='px-6 py-3 text-left font-semibold text-gray-900'>Task Name / ID</th>
                    <th scope='col' class='px-6 py-3 text-left font-semibold text-gray-900'>Status</th>
                    <th scope='col' class='px-6 py-3 text-left font-semibold text-gray-900'>Schedule</th>
                    <th scope='col' class='px-6 py-3 text-left font-semibold text-gray-900'>Actions</th>
                    <th scope='col' class='px-6 py-3 text-left font-semibold text-gray-900'>Controls</th>
                </tr></thead>
                <tbody class='bg-white divide-y divide-gray-200'>{}</tbody>
            </table>
        </div>",
        rows
    );

    let card_html = card("Configured Tasks", &table_html, None);

    layout(
        "Dashboard",
        &format!("{}{}{}{}", toast_html, header, stats_grid, card_html),
    )
}

pub(crate) fn render_status_cards(status: &crate::runner::engine::RunnerStatus) -> String {
    format!(
        "{}{}{}{}",
        stat_card(
            "Running Tasks",
            &status.running_tasks_count.to_string(),
            "Currently executing"
        ),
        stat_card(
            "Queued Tasks",
            &status.queued_tasks_count.to_string(),
            "Waiting in queue"
        ),
        stat_card(
            "Last Task Executed",
            if status.last_task_id.is_empty() {
                "None"
            } else {
                &status.last_task_id
            },
            "Recent activity"
        ),
        stat_card(
            "Last Error",
            if status.last_error.is_empty() {
                "None"
            } else {
                &status.last_error
            },
            "System health"
        )
    )
}

pub(crate) fn render_task_row(task: &RunnerTask) -> String {
    let task_status = if task.enabled {
        status_badge(true, "Active", "Inactive")
    } else {
        status_badge(false, "Active", "Inactive")
    };

    let schedules_text = task
        .schedules
        .iter()
        .map(|s| {
            let next = match s {
                TaskSchedule::Interval { next_run_at, .. } => next_run_at,
                TaskSchedule::DailyTimes { next_run_at, .. } => next_run_at,
                TaskSchedule::Weekly { next_run_at, .. } => next_run_at,
                TaskSchedule::Monthly { next_run_at, .. } => next_run_at,
                TaskSchedule::Once { next_run_at, .. } => next_run_at,
            };
            let mut base = human_schedule(s);
            if !next.is_empty() {
                base.push_str(" <br><span class='text-xs text-gray-500'>Next: ");
                base.push_str(&human_datetime(next));
                base.push_str("</span>");
            }
            base
        })
        .collect::<Vec<_>>()
        .join("<div class='mt-2'></div>");

    let schedule_display = if schedules_text.is_empty() {
        "<span class='text-gray-400 italic text-xs'>Manual</span>".to_string()
    } else {
        schedules_text
    };

    format!(
        "<tr>
            <td class='px-6 py-4 align-top'>
                <div class='font-medium text-gray-900'>{}</div>
                <div class='text-xs font-mono text-gray-500 mt-1'>{}</div>
            </td>
            <td class='px-6 py-4 align-top whitespace-nowrap'>{}</td>
            <td class='px-6 py-4 align-top'>{}</td>
            <td class='px-6 py-4 align-top whitespace-nowrap text-sm font-medium'>
                <a href='/edit/{}' class='text-emerald-600 hover:text-emerald-900 mr-4 inline-flex items-center'>{} Edit</a>
                <a href='/delete/{}' class='text-red-600 hover:text-red-900 inline-flex items-center'>{} Delete</a>
            </td>
            <td class='px-6 py-4 align-top whitespace-nowrap space-x-2'>
                <form action='/run/{}' method='POST' class='inline-block'>
                    <button type='submit' class='inline-flex items-center px-2.5 py-1.5 border border-gray-300 shadow-sm text-xs font-medium rounded text-gray-700 bg-white hover:bg-gray-50 focus:outline-none'>
                        {} Run Now
                    </button>
                </form>
                <form action='/{}' method='POST' class='inline-block'>
                    <input type='hidden' name='enabled' value='{}'>
                    <button type='submit' class='inline-flex items-center px-2.5 py-1.5 border border-gray-300 shadow-sm text-xs font-medium rounded text-gray-700 bg-white hover:bg-gray-50 focus:outline-none'>
                        {}
                    </button>
                </form>
            </td>
        </tr>",
        escape_html(&task.name),
        escape_html(&task.id),
        task_status,
        schedule_display,
        escape_html(&task.id),
        icon_edit("w-4 h-4 mr-1"),
        escape_html(&task.id),
        icon_trash("w-4 h-4 mr-1"),
        escape_html(&task.id),
        icon_play("w-4 h-4 mr-1"),
        if task.enabled { format!("disable/{}", escape_html(&task.id)) } else { format!("enable/{}", escape_html(&task.id)) },
        if task.enabled { "false" } else { "true" },
        if task.enabled { "Disable" } else { "Enable" },
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
    layout(title, &form_html)
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

pub(crate) fn command_row_html(_index: usize, command: &str, continue_on_error: bool) -> String {
    format!(
        "<div class='grid grid-cols-1 md:grid-cols-[1fr_120px_auto] gap-3 items-end p-4 mb-3 bg-gray-50 border border-gray-200 rounded-md' data-command-row>
            <div class='w-full'>
                <label class='block text-xs font-medium text-gray-700 mb-1'>Command</label>
                <input class='command-text shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' value='{}' placeholder='echo hello'>
            </div>
            <div class='w-full'>
                <label class='block text-xs font-medium text-gray-700 mb-1'>Mode</label>
                <select class='command-mode shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2'>
                    <option value='run' {}>Run</option>
                    <option value='continue' {}>Continue on Error</option>
                </select>
            </div>
            <div>
                <button type='button' class='remove-command inline-flex items-center p-2 border border-transparent rounded-md shadow-sm text-white bg-red-600 hover:bg-red-700 focus:outline-none'>
                    {}
                </button>
            </div>
        </div>",
        escape_html(command),
        if !continue_on_error { "selected" } else { "" },
        if continue_on_error { "selected" } else { "" },
        icon_trash("w-4 h-4")
    )
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) fn select_task_type(value: &str) -> String {
    format!(
        "<label class='block'><span class='text-sm font-semibold text-gray-800'>Task Type</span><select id='task-type-select' class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='task_type'><option value='shell_command' {}>Shell Command</option><option value='external_app' {}>External App</option></select></label>",
        if value == "shell_command" { "selected" } else { "" },
        if value == "external_app" { "selected" } else { "" }
    )
}

pub(crate) fn render_redirect_to_dashboard(message: &str) -> String {
    layout(
        "Redirecting",
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-gray-200 rounded-lg shadow-sm p-6 text-center mt-12'>
                <div class='mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-emerald-100 mb-4'>
                    {}
                </div>
                <h1 class='text-2xl font-bold text-gray-900'>Success</h1>
                <p class='mt-2 text-sm text-gray-500'>Returning to the dashboard...</p>
            </div>
            <script>window.addEventListener('DOMContentLoaded', function() {{ window.redirectToDashboard('{}'); }});</script>",
            icon_check("h-6 w-6 text-emerald-600"),
            js_escape(message)
        ),
    )
}

pub(crate) fn render_toast(message: &str) -> String {
    format!(
        "<div id='runner-toast' class='fixed right-4 top-4 z-50 max-w-sm rounded border border-gray-200 bg-white px-4 py-3 shadow-lg flex items-start gap-3'>
            <div class='flex-shrink-0'>
                {}
            </div>
            <p class='text-sm font-medium text-gray-900'>{}</p>
        </div>",
        icon_code("w-5 h-5 text-emerald-500"),
        escape_html(message)
    )
}

pub(crate) fn render_error_page(title: &str, message: &str) -> String {
    layout(
        title,
        &format!(
            "<div class='max-w-xl mx-auto bg-white border border-red-200 rounded-lg shadow-sm p-6 mt-12 text-center'>
                <div class='mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-red-100 mb-4'>
                    {}
                </div>
                <h1 class='text-2xl font-bold text-red-800'>{}</h1>
                <p class='mt-3 text-sm text-gray-700 break-words'>{}</p>
                <div class='mt-6'>
                    {}
                </div>
            </div>",
            icon_exclamation_triangle("h-6 w-6 text-red-600"),
            escape_html(title),
            escape_html(message),
            secondary_button("Return to Dashboard", Some("/"), None)
        ),
    )
}

pub(crate) fn render_apps_page(apps: &[crate::runner::config::RegisteredApp]) -> String {
    let mut rows = String::new();
    if apps.is_empty() {
        rows = "<tr><td colspan='5' class='px-6 py-12 text-center text-gray-500'>No applications registered.</td></tr>".to_string();
    } else {
        for app in apps {
            rows.push_str(&format!(
                "<tr>
                    <td class='px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900'>{}</td>
                    <td class='px-6 py-4 whitespace-nowrap text-sm text-gray-500'>{}</td>
                    <td class='px-6 py-4 whitespace-nowrap text-xs font-mono text-gray-500'>{}</td>
                    <td class='px-6 py-4 whitespace-nowrap text-xs font-mono text-gray-500'>{}</td>
                    <td class='px-6 py-4 whitespace-nowrap text-sm font-medium'>
                        <a href='/apps/edit/{}' class='text-emerald-600 hover:text-emerald-900 mr-4 inline-flex items-center'>{} Edit</a>
                        <a href='/apps/delete/{}' class='text-red-600 hover:text-red-900 inline-flex items-center'>{} Delete</a>
                    </td>
                </tr>",
                escape_html(&app.name),
                escape_html(&app.id),
                escape_html(&app.executable_path),
                escape_html(&app.config_path),
                escape_html(&app.id),
                icon_edit("w-4 h-4 mr-1"),
                escape_html(&app.id),
                icon_trash("w-4 h-4 mr-1"),
            ));
        }
    }

    let header = page_header(
        "Registered Applications",
        "Manage external applications that can be scheduled as tasks.",
        &secondary_button("Back to Dashboard", Some("/"), None),
    );

    let table_html = format!(
        "<div class='overflow-x-auto'>
            <table class='min-w-full divide-y divide-gray-200'>
                <thead class='bg-gray-50'><tr>
                    <th scope='col' class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>Name</th>
                    <th scope='col' class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>ID</th>
                    <th scope='col' class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>Executable</th>
                    <th scope='col' class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>Config Path</th>
                    <th scope='col' class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>Actions</th>
                </tr></thead>
                <tbody class='bg-white divide-y divide-gray-200'>{}</tbody>
            </table>
        </div>",
        rows
    );

    let list_card = card("App List", &table_html, None);

    let form_html = format!(
        "<form method='post' action='/apps/create'>
            <div class='grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-2'>
                <div class='sm:col-span-1'>{}</div>
                <div class='sm:col-span-1'>{}</div>
                <div class='sm:col-span-2'>{}</div>
                <div class='sm:col-span-2'>{}</div>
            </div>
            <div class='mt-6 pt-5 border-t border-gray-200 flex justify-end'>
                {}
            </div>
        </form>",
        input_field("Name", "name", ""),
        input_field("App ID", "id", ""),
        input_field("Executable Path (e.g. tasker.exe)", "executable_path", ""),
        input_field("Config Path (Optional override)", "config_path", ""),
        primary_button("Register App", None, Some(&icon_plus("w-4 h-4 mr-2")))
    );

    let register_card = card("Register New App", &form_html, None);

    layout(
        "Registered Apps",
        &format!(
            "{}<div class='space-y-8'>{}<div class='max-w-4xl'>{}</div></div>",
            header, list_card, register_card
        ),
    )
}

pub(crate) fn render_app_edit_page(app: &crate::runner::config::RegisteredApp) -> String {
    let form_html = format!(
        "<form action='/apps/update/{}' method='POST'>
            <div class='space-y-6'>
                {}
                {}
                {}
            </div>
            <div class='mt-8 pt-5 border-t border-gray-200 flex justify-end space-x-3'>
                {}
                {}
            </div>
        </form>",
        escape_html(&app.id),
        input_field("Name", "name", &app.name),
        input_field("Executable Path", "executable_path", &app.executable_path),
        input_field("Config Path", "config_path", &app.config_path),
        secondary_button("Cancel", Some("/apps"), None),
        primary_button("Update App", None, Some(&icon_check("w-4 h-4 mr-2")))
    );

    layout(
        "Edit App",
        &format!(
            "{}<div class='max-w-2xl mx-auto'>{}</div>",
            page_header(&format!("Edit {}", app.name), "", ""),
            card("Application Details", &form_html, None)
        ),
    )
}

pub(crate) fn human_schedule(schedule: &TaskSchedule) -> String {
    match schedule {
        TaskSchedule::Interval { every_seconds, .. } => {
            format!("Every {}", compact_duration(*every_seconds))
        }
        TaskSchedule::DailyTimes { times, .. } => format!("Daily at {}", times.join(", ")),
        TaskSchedule::Weekly {
            day_of_week,
            at_time,
            ..
        } => format!("Weekly on {} at {}", day_of_week, at_time),
        TaskSchedule::Monthly {
            day_of_month,
            at_time,
            ..
        } => format!("Monthly on day {} at {}", day_of_month, at_time),
        TaskSchedule::Once { .. } => "Run Once".to_string(),
    }
}
