import sys

with open('src/runner/gui/templates.rs', 'r') as f:
    content = f.read()

bad = """    format!(
        "<div class='p-3 border border-gray-200 rounded mb-2' data-schedule-row>\\
          <div class='grid md:grid-cols-6 gap-2 items-end'>\\
            <label class='block'>\\
                <span class='text-xs font-semibold text-gray-700'>Type</span>\\
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm schedule-kind' name='schedule_kind_{}'>\\
                    <option value='interval' {}>Interval</option>\\
                    <option value='once' {}>Once</option>\\
                    <option value='daily' {}>Daily</option>\\
                    <option value='weekly' {}>Weekly</option>\\
                    <option value='monthly' {}>Monthly</option>\\
                </select>\\
            </label>\\
            <label class='block schedule-interval {}'>\\
                <span class='text-xs font-semibold text-gray-700'>Interval</span>\\
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='schedule_interval_{}'>{}\\
                </select>\\
            </label>\\
            <label class='block schedule-interval schedule-start-time {}'>\\
                <span class='text-xs font-semibold text-gray-700'>Start Time (HH:MM)</span>\\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='time' name='schedule_start_time_{}' value='{}'>\\
            </label>\\
            <label class='block schedule-once {}'>\\
                <span class='text-xs font-semibold text-gray-700'>Date & Time</span>\\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='datetime-local' name='schedule_once_at_{}' value='{}'>\\
            </label>\\
            <label class='block schedule-daily {}'>\\
                <span class='text-xs font-semibold text-gray-700'>Times (HH:MM)</span>\\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' name='schedule_daily_at_{}' value='{}' placeholder='09:00, 13:00'>\\
            </label>\\
            <label class='block schedule-weekly {}'>\\
                <span class='text-xs font-semibold text-gray-700'>Day</span>\\
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' name='schedule_weekly_at_{}' data-weekly-day>\\
                    {}\\
                </select>\\
            </label>\\
            <label class='block schedule-monthly {}'>\\
                <span class='text-xs font-semibold text-gray-700'>Day (1-31)</span>\\
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='number' name='schedule_monthly_at_{}' value='{}' min='1' max='31'>\\
            </label>\\
            <button type='button' class='remove-schedule rounded border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-700'>Remove</button>\\
          </div>\\
          <div class='mt-3 schedule-wh {}'>\\
              <div class='flex items-center justify-between'>\\
                  <span class='text-xs font-semibold text-gray-700'>Working Hours (Optional)</span>\\
                  <button type='button' class='add-wh-row rounded border border-gray-300 bg-white px-2 py-1 text-xs font-semibold text-gray-700 hover:bg-gray-50'>+ Add Day</button>\\
              </div>\\
              <div class='wh-rows'>{}</div>\\
          </div>\\
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
    )"""

def get_wh_val(day, wh):
    if wh is None:
        return ""
    if day in wh:
        return f"{wh[day].start}-{wh[day].end}"
    return ""

new_schedule_row_html_def = """pub(crate) fn schedule_row_html(
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

    let is_wh_hidden = if kind == "interval" || kind == "daily" { "" } else { "hidden" };
    let is_st_hidden = if kind == "interval" || kind == "weekly" || kind == "monthly" { "" } else { "hidden" };

    let mut start_time_val = String::new();
    if let Some(schedules) = task.map(|t| &t.schedules) {
        for s in schedules {
           match s {
               TaskSchedule::Interval { start_time: Some(st), .. } => start_time_val = st.clone(),
               TaskSchedule::Weekly { at_time, .. } => start_time_val = at_time.clone(),
               TaskSchedule::Monthly { at_time, .. } => start_time_val = at_time.clone(),
               _ => {}
           }
        }
    }

    let wh_mon = working_hours.and_then(|wh| wh.get("Monday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();
    let wh_tue = working_hours.and_then(|wh| wh.get("Tuesday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();
    let wh_wed = working_hours.and_then(|wh| wh.get("Wednesday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();
    let wh_thu = working_hours.and_then(|wh| wh.get("Thursday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();
    let wh_fri = working_hours.and_then(|wh| wh.get("Friday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();
    let wh_sat = working_hours.and_then(|wh| wh.get("Saturday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();
    let wh_sun = working_hours.and_then(|wh| wh.get("Sunday")).map(|h| format!("{}-{}", h.start, h.end)).unwrap_or_default();

    format!(
        "<div class='flex flex-col gap-3 p-4 border border-gray-200 rounded-md bg-white'>\\
            <div class='flex flex-wrap items-end gap-3 w-full'>\\
                <div class='w-full sm:w-auto flex-1'>\\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Type</label>\\
                    <select class='schedule-kind shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2 bg-gray-50'>\\
                        <option value='interval' {}>Interval</option>\\
                        <option value='once' {}>Once</option>\\
                        <option value='daily' {}>Daily</option>\\
                        <option value='weekly' {}>Weekly</option>\\
                        <option value='monthly' {}>Monthly</option>\\
                    </select>\\
                </div>\\
                <div class='schedule-interval w-full sm:w-auto flex-1 {}'>\\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Every</label>\\
                    <select class='interval-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2'>\\
                        <option value='15m' {}>15m</option>\\
                        <option value='30m' {}>30m</option>\\
                        <option value='1h' {}>1h</option>\\
                        <option value='2h' {}>2h</option>\\
                        <option value='4h' {}>4h</option>\\
                        <option value='8h' {}>8h</option>\\
                        <option value='12h' {}>12h</option>\\
                        <option value='24h' {}>24h</option>\\
                        <option value='2d' {}>2d</option>\\
                        <option value='7d' {}>7d</option>\\
                    </select>\\
                </div>\\
                <div class='schedule-once w-full sm:w-auto flex-1 {}'>\\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>At</label>\\
                    <input class='once-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='datetime-local' value='{}'>\\
                </div>\\
                <div class='schedule-daily w-full sm:w-auto flex-1 {}'>\\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Times (comma sep, e.g. 09:00,15:30)</label>\\
                    <input class='daily-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' placeholder='09:00' value='{}'>\\
                </div>\\
                <div class='schedule-weekly w-full sm:w-auto flex-1 {}'>\\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Day and Time (e.g. Monday@09:00)</label>\\
                    <input class='weekly-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' placeholder='Monday@09:00' value='{}'>\\
                </div>\\
                <div class='schedule-monthly w-full sm:w-auto flex-1 {}'>\\
                    <label class='block text-xs font-medium text-gray-700 mb-1'>Day and Time (e.g. 15@09:00 or -1@09:00)</label>\\
                    <input class='monthly-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' placeholder='1@09:00' value='{}'>\\
                </div>\\
                <div class='schedule-st w-full sm:w-auto flex-1 {}'>\\
                  <label class='block text-xs font-medium text-gray-700 mb-1'>Start Time (Optional)</label>\\
                  <input class='st-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='time' value='{}'>\\
                </div>\\
                <div>\\
                    <button type='button' class='remove-schedule inline-flex items-center p-2 border border-transparent rounded-md shadow-sm text-white bg-red-600 hover:bg-red-700 focus:outline-none'>\\
                        <svg class='h-4 w-4' fill='none' stroke='currentColor' viewBox='0 0 24 24'><path stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'></path></svg>\\
                    </button>\\
                </div>\\
            </div>\\
            <div class='schedule-wh w-full bg-gray-50 p-3 rounded border border-gray-200 {}'>\\
               <div class='flex items-center justify-between mb-2'>\\
                   <span class='text-xs font-medium text-gray-700'>Working Hours (Optional, e.g. 09:00-17:00)</span>\\
               </div>\\
               <div class='grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs'>\\
                   <div><label class='block text-gray-600 mb-1'>Monday</label><input type='text' class='wh-mon block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
                   <div><label class='block text-gray-600 mb-1'>Tuesday</label><input type='text' class='wh-tue block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
                   <div><label class='block text-gray-600 mb-1'>Wednesday</label><input type='text' class='wh-wed block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
                   <div><label class='block text-gray-600 mb-1'>Thursday</label><input type='text' class='wh-thu block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
                   <div><label class='block text-gray-600 mb-1'>Friday</label><input type='text' class='wh-fri block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
                   <div><label class='block text-gray-600 mb-1'>Saturday</label><input type='text' class='wh-sat block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
                   <div><label class='block text-gray-600 mb-1'>Sunday</label><input type='text' class='wh-sun block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00' value='{}'></div>\\
               </div>\\
            </div>\\
        </div>",
        if kind == "interval" { "selected" } else { "" },
        if kind == "once" { "selected" } else { "" },
        if kind == "daily" { "selected" } else { "" },
        if kind == "weekly" { "selected" } else { "" },
        if kind == "monthly" { "selected" } else { "" },
        interval_hidden,
        if interval_value == "15m" { "selected" } else { "" },
        if interval_value == "30m" { "selected" } else { "" },
        if interval_value == "1h" { "selected" } else { "" },
        if interval_value == "2h" { "selected" } else { "" },
        if interval_value == "4h" { "selected" } else { "" },
        if interval_value == "8h" { "selected" } else { "" },
        if interval_value == "12h" { "selected" } else { "" },
        if interval_value == "24h" { "selected" } else { "" },
        if interval_value == "2d" { "selected" } else { "" },
        if interval_value == "7d" { "selected" } else { "" },
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
}"""

import re

# find the definition of schedule_row_html and replace it
content = re.sub(r'pub\(crate\) fn schedule_row_html\(.*?\(crate\) fn days_of_week_options\(selected_day: &str\) -> String {', new_schedule_row_html_def + '\n\npub(crate) fn days_of_week_options(selected_day: &str) -> String {', content, flags=re.DOTALL)

with open('src/runner/gui/templates.rs', 'w') as f:
    f.write(content)

print("Done")
