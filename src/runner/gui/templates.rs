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
        .fold(String::new(), |mut acc, s| {
            if !acc.is_empty() {
                acc.push_str("<div class=\'mt-2\'></div>");
            }
            acc.push_str(&s);
            acc
        });

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
    _profiles: &[crate::runner::config::WorkingHoursProfile],
    title: &str,
    action: &str,
    submit_label: &str,
    task: Option<&RunnerTask>,
    error: Option<&str>,
) -> String {
    let id = task.map(|t| t.id.as_str()).unwrap_or_default();
    let name = task.map(|t| t.name.as_str()).unwrap_or_default();
    let enabled = task.map(|t| t.enabled).unwrap_or(true);

    let timeout_seconds = task.map(|t| t.timeout_seconds).unwrap_or(0);
    let timeout_seconds_str = if timeout_seconds > 0 {
        timeout_seconds.to_string()
    } else {
        String::new()
    };

    let steps_json = task
        .map(|t| serde_json::to_string(&t.steps).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "[]".to_string());
    let post_run_steps_json = task
        .map(|t| serde_json::to_string(&t.post_run_steps).unwrap_or_else(|_| "[]".to_string()))
        .unwrap_or_else(|| "[]".to_string());

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
                <div class='mb-2'>\
                    <label class='flex items-center gap-2 text-sm font-semibold text-gray-800 h-full mt-4'><input type='checkbox' name='enabled' value='on' {checked_attr}> Task Enabled</label>\
                </div>\
                <div class='grid md:grid-cols-2 gap-4'>\
                    <label class='block mb-4'>\
                        <span class='text-sm font-semibold text-gray-700'>Timeout (Seconds)</span>\
                        <input class='mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm' type='number' name='timeout_seconds' value='{timeout_val}' placeholder='0 (Global default)'>\
                        <p class='text-xs text-gray-500 mt-1'>Overrides the global timeout.</p>\
                    </label>\
                </div>\
                {schedule_editor}\
                <div class='mb-4'>\
                    <div class='flex items-center justify-between mb-2'>\
                        <h3 class='text-lg font-semibold text-gray-800'>Steps</h3>\
                        <button type='button' id='add-step-btn' class='rounded border border-gray-300 bg-emerald-600 text-white px-3 py-1 text-sm font-semibold hover:bg-emerald-700'>+ Add step</button>\
                    </div>\
                    <div id='steps-container' class='space-y-4'></div>\
                    <input type='hidden' id='steps-hidden' name='steps' value='{steps_val}'>\
                </div>\
                <div class='mb-4 p-4 border border-gray-200 bg-gray-50 rounded'>\
                    <div class='flex items-center justify-between mb-2'>\
                        <h3 class='text-lg font-semibold text-gray-800'>Post Run Steps</h3>\
                        <button type='button' id='add-post-run-step-btn' class='rounded border border-gray-300 bg-emerald-600 text-white px-3 py-1 text-sm font-semibold hover:bg-emerald-700'>+ Add post run step</button>\
                    </div>\
                    <p class='text-xs text-gray-500 mb-4'>These steps execute only if the main pipeline succeeds.</p>\
                    <div id='post-run-steps-container' class='space-y-4'></div>\
                    <input type='hidden' id='post-run-steps-hidden' name='post_run_steps' value='{post_run_steps_val}'>\
                </div>\
                <div class='pt-4 border-t border-gray-200 flex justify-end space-x-3'>\
                    <a href='/' class='inline-flex justify-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2'>Cancel</a>\
                    <button type='submit' class='inline-flex justify-center rounded-md border border-transparent bg-emerald-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2'>{submit_label}</button>\
                </div>\
            </form>\
        </div>",
        title = escape_html(title),
        error_html = error_html,
        action = action,
        id_field = input_field("ID", "id", id),
        name_field = input_field("Name", "name", name),
        checked_attr = if enabled { "checked" } else { "" },
        timeout_val = escape_html(&timeout_seconds_str),
        schedule_editor = schedule_editor_html(task),
        steps_val = escape_html(&steps_json),
        post_run_steps_val = escape_html(&post_run_steps_json),
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn schedule_row_html(
    _index: usize,
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

    let is_wh_hidden = if kind == "interval" || kind == "daily" {
        ""
    } else {
        "hidden"
    };
    let is_st_hidden = if kind == "interval" || kind == "weekly" || kind == "monthly" {
        ""
    } else {
        "hidden"
    };

    let mut start_time_val = String::new();
    if let Some(schedules) = task.map(|t| &t.schedules) {
        for s in schedules {
            match s {
                TaskSchedule::Interval {
                    start_time: Some(st),
                    ..
                } => start_time_val = st.clone(),
                TaskSchedule::Weekly { at_time, .. } => start_time_val = at_time.clone(),
                TaskSchedule::Monthly { at_time, .. } => start_time_val = at_time.clone(),
                _ => {}
            }
        }
    }

    let wh_mon = working_hours
        .and_then(|wh| wh.get("Monday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();
    let wh_tue = working_hours
        .and_then(|wh| wh.get("Tuesday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();
    let wh_wed = working_hours
        .and_then(|wh| wh.get("Wednesday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();
    let wh_thu = working_hours
        .and_then(|wh| wh.get("Thursday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();
    let wh_fri = working_hours
        .and_then(|wh| wh.get("Friday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();
    let wh_sat = working_hours
        .and_then(|wh| wh.get("Saturday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();
    let wh_sun = working_hours
        .and_then(|wh| wh.get("Sunday"))
        .map(|h| format!("{}-{}", h.start, h.end))
        .unwrap_or_default();

    format!(
        "<div class='flex flex-col gap-3 p-4 border border-gray-200 rounded-md bg-white'>\
            <div class='flex flex-wrap items-end gap-3 w-full'>\
                <div class='w-full sm:w-auto flex-1'>\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Type</label>\
                    <select class='schedule-kind shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2 bg-gray-50'>\
                        <option value='interval' {}>Interval</option>\
                        <option value='once' {}>Once</option>\
                        <option value='daily' {}>Daily</option>\
                        <option value='weekly' {}>Weekly</option>\
                        <option value='monthly' {}>Monthly</option>\
                    </select>\
                </div>\
                <div class='schedule-interval w-full sm:w-auto flex-1 {}'>\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Every</label>\
                    <select class='interval-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2'>\
                        {}\
                    </select>\
                </div>\
                <div class='schedule-once w-full sm:w-auto flex-1 {}'>\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>At</label>\
                    <input class='once-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='datetime-local' value='{}'>\
                </div>\
                <div class='schedule-daily w-full sm:w-auto flex-1 {}'>\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Times</label>\
                    <div class='daily-times-container flex flex-wrap gap-2 mb-2'></div>\
                    <button type='button' class='add-daily-time-btn inline-flex items-center px-2 py-1 border border-gray-300 text-xs font-medium rounded shadow-sm text-gray-700 bg-white hover:bg-gray-50 focus:outline-none'>+ Add Time</button>\
                    <input type='hidden' class='daily-value' value='{}'>\
                </div>\
                <div class='schedule-weekly w-full sm:w-auto flex-1 {}'>\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Day and Time</label>\
                    <div class='flex gap-2'>\
                        <select class='weekly-day shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2'>\
                            <option value='Monday'>Monday</option>\
                            <option value='Tuesday'>Tuesday</option>\
                            <option value='Wednesday'>Wednesday</option>\
                            <option value='Thursday'>Thursday</option>\
                            <option value='Friday'>Friday</option>\
                            <option value='Saturday'>Saturday</option>\
                            <option value='Sunday'>Sunday</option>\
                        </select>\
                        <input class='weekly-time shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='time'>\
                        <input class='weekly-value' type='hidden' value='{}'>\
                    </div>\
                </div>\
                <div class='schedule-monthly w-full sm:w-auto flex-1 {}'>\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Day and Time (1-31 or -1)</label>\
                    <div class='flex gap-2'>\
                        <input class='monthly-day shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='number' min='-1' max='31' placeholder='1'>\
                        <input class='monthly-time shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='time'>\
                        <input class='monthly-value' type='hidden' value='{}'>\
                    </div>\
                </div>\
                <div class='schedule-st w-full sm:w-auto flex-1 {}'>\
                  <label class='block text-xs font-medium text-gray-700 mb-1'>Start Time (Optional)</label>\
                  <input class='st-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='time' value='{}'>\
                </div>\
                <div>\
                    <button type='button' class='remove-schedule inline-flex items-center p-2 border border-transparent rounded-md shadow-sm text-white bg-red-600 hover:bg-red-700 focus:outline-none'>\
                        <svg class='h-4 w-4' fill='none' stroke='currentColor' viewBox='0 0 24 24'><path stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'></path></svg>\
                    </button>\
                </div>\
            </div>\
            <div class='schedule-wh w-full bg-gray-50 p-3 rounded border border-gray-200 {}'>\
               <div class='flex items-center justify-between mb-2'>\
                   <span class='text-xs font-medium text-gray-700'>Working Hours (Optional, e.g. 09:00-17:00)</span>\
               </div>\
               <div class='grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-4 text-xs'>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Monday</label><div class='flex items-center gap-1'><input type='time' class='wh-mon-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-mon-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-mon' value='{}'></div></div>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Tuesday</label><div class='flex items-center gap-1'><input type='time' class='wh-tue-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-tue-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-tue' value='{}'></div></div>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Wednesday</label><div class='flex items-center gap-1'><input type='time' class='wh-wed-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-wed-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-wed' value='{}'></div></div>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Thursday</label><div class='flex items-center gap-1'><input type='time' class='wh-thu-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-thu-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-thu' value='{}'></div></div>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Friday</label><div class='flex items-center gap-1'><input type='time' class='wh-fri-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-fri-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-fri' value='{}'></div></div>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Saturday</label><div class='flex items-center gap-1'><input type='time' class='wh-sat-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-sat-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-sat' value='{}'></div></div>\
                   <div><label class='block text-gray-600 mb-1 font-semibold'>Sunday</label><div class='flex items-center gap-1'><input type='time' class='wh-sun-start block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='From'><span class='text-gray-400'>-</span><input type='time' class='wh-sun-end block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1' title='To'><input type='hidden' class='wh-sun' value='{}'></div></div>\
               </div>\
            </div>\
        </div>",
        if kind == "interval" { "selected" } else { "" },
        if kind == "once" { "selected" } else { "" },
        if kind == "daily" { "selected" } else { "" },
        if kind == "weekly" { "selected" } else { "" },
        if kind == "monthly" { "selected" } else { "" },
        interval_hidden,
        {            let opts = vec!["15m", "30m", "1h", "2h", "4h", "8h", "12h", "24h", "2d", "7d"];            let mut found = false;            let mut html = String::new();            for opt in &opts {                if *opt == interval_value {                    html.push_str(&format!("<option value='{}' selected>{}</option>", opt, opt));                    found = true;                } else {                    html.push_str(&format!("<option value='{}'>{}</option>", opt, opt));                }            }            if !found && !interval_value.is_empty() {                html.push_str(&format!("<option value='{}' selected>{}</option>", interval_value, interval_value));            }            html        },
        once_hidden,
        escape_html(once_value),
        daily_hidden,
        escape_html(daily_value),
        weekly_hidden,
        escape_html(weekly_value),
        monthly_hidden,
        escape_html(monthly_value),
        is_st_hidden,
        escape_html(&start_time_val),
        is_wh_hidden,
        escape_html(&wh_mon),
        escape_html(&wh_tue),
        escape_html(&wh_wed),
        escape_html(&wh_thu),
        escape_html(&wh_fri),
        escape_html(&wh_sat),
        escape_html(&wh_sun)
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

pub(crate) fn render_wh_page(profiles: &[crate::runner::config::WorkingHoursProfile]) -> String {
    let mut rows = String::new();
    for profile in profiles {
        rows.push_str(&format!(
            "<tr>
                <td class='px-6 py-4 whitespace-nowrap text-sm text-gray-900'>{}</td>
                <td class='px-6 py-4 whitespace-nowrap text-sm text-gray-500'>{}</td>
                <td class='px-6 py-4 whitespace-nowrap text-right text-sm font-medium'>
                    <a href='/working-hours/edit/{}' class='text-emerald-600 hover:text-emerald-900'>Edit</a>
                    <span class='text-gray-300 mx-2'>|</span>
                    <a href='#' onclick='if(confirm('Delete profile?')) window.location.href='/working-hours/delete/{}'' class='text-red-600 hover:text-red-900'>Delete</a>
                </td>
            </tr>",
            escape_html(&profile.name),
            escape_html(&profile.id),
            escape_html(&profile.id),
            escape_html(&profile.id)
        ));
    }

    let body = format!(
        "<div class='px-4 sm:px-6 lg:px-8'>
            <div class='sm:flex sm:items-center'>
                <div class='sm:flex-auto'>
                    <h1 class='text-xl font-semibold text-gray-900'>Working Hours Profiles</h1>
                    <p class='mt-2 text-sm text-gray-700'>Manage reusable working hours definitions.</p>
                </div>
                <div class='mt-4 sm:mt-0 sm:ml-16 sm:flex-none'>
                    <a href='/working-hours/new' class='inline-flex items-center justify-center rounded-md border border-transparent bg-emerald-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-emerald-700'>Add Profile</a>
                </div>
            </div>
            <div class='mt-8 flex flex-col'>
                <div class='-my-2 -mx-4 overflow-x-auto sm:-mx-6 lg:-mx-8'>
                    <div class='inline-block min-w-full py-2 align-middle md:px-6 lg:px-8'>
                        <div class='overflow-hidden shadow ring-1 ring-black ring-opacity-5 md:rounded-lg'>
                            <table class='min-w-full divide-y divide-gray-300'>
                                <thead class='bg-gray-50'>
                                    <tr>
                                        <th class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>Name</th>
                                        <th class='px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider'>ID</th>
                                        <th class='px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider'>Actions</th>
                                    </tr>
                                </thead>
                                <tbody class='divide-y divide-gray-200 bg-white'>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>
        </div>",
        rows
    );
    layout("Working Hours", &body)
}

pub(crate) fn render_wh_edit_page(
    profile: Option<&crate::runner::config::WorkingHoursProfile>,
) -> String {
    let is_new = profile.is_none();
    let action_url = if is_new {
        "/working-hours/create".to_string()
    } else {
        format!("/working-hours/update/{}", profile.unwrap().id)
    };
    let title = if is_new {
        "Create Working Hours Profile"
    } else {
        "Edit Working Hours Profile"
    };
    let id_val = profile.map(|p| p.id.as_str()).unwrap_or("");
    let name_val = profile.map(|p| p.name.as_str()).unwrap_or("");
    let id_readonly = if is_new {
        ""
    } else {
        "readonly class='bg-gray-100'"
    };

    // Add grid for 7 days
    let days = vec![
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let mut days_html = String::new();

    for day in days {
        let (start, end) = if let Some(p) = profile {
            if let Some(wh) = p.days.get(day) {
                (wh.start.as_str(), wh.end.as_str())
            } else {
                ("", "")
            }
        } else {
            ("", "")
        };

        days_html.push_str(&format!(
            "<div><label class='block text-gray-600 mb-1 font-semibold'>{}</label><div class='flex items-center gap-1'><input type='time' name='{}_start' value='{}' class='block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1'><span class='text-gray-400'>-</span><input type='time' name='{}_end' value='{}' class='block w-full rounded border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1'></div></div>",
            day, day, start, day, end
        ));
    }

    let body = format!(
        "<div class='max-w-3xl mx-auto'>
            <form action='{}' method='POST' class='space-y-8 divide-y divide-gray-200'>
                <div class='space-y-8 divide-y divide-gray-200'>
                    <div>
                        <div>
                            <h3 class='text-lg leading-6 font-medium text-gray-900'>{}</h3>
                        </div>
                        <div class='mt-6 grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6'>
                            <div class='sm:col-span-3'>
                                <label for='id' class='block text-sm font-medium text-gray-700'>Profile ID</label>
                                <div class='mt-1'>
                                    <input type='text' name='id' id='id' required {} value='{}' class='shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md p-2 border'>
                                </div>
                            </div>
                            <div class='sm:col-span-3'>
                                <label for='name' class='block text-sm font-medium text-gray-700'>Profile Name</label>
                                <div class='mt-1'>
                                    <input type='text' name='name' id='name' required value='{}' class='shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md p-2 border'>
                                </div>
                            </div>
                            <div class='sm:col-span-6'>
                                <h4 class='text-sm font-medium text-gray-700 mb-2 mt-4 border-b pb-2'>Working Hours (e.g. 09:00 - 17:00)</h4>
                                <div class='grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-4 text-xs'>
                                    {}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class='pt-5'>
                    <div class='flex justify-end'>
                        <a href='/working-hours' class='bg-white py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500'>Cancel</a>
                        <button type='submit' class='ml-3 inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-emerald-600 hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-emerald-500'>Save Profile</button>
                    </div>
                </div>
            </form>
        </div>",
        action_url, title, id_readonly, escape_html(id_val), escape_html(name_val), days_html
    );
    layout(title, &body)
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
