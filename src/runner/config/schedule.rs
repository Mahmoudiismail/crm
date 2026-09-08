use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::runner::config::models::*;

impl RunnerTask {
    pub fn due_now(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        if !self.schedules.is_empty() {
            return self.schedules.iter().any(|schedule| schedule.due_now(now));
        }
        if self.next_run_at.is_empty() {
            return true;
        }
        DateTime::parse_from_rfc3339(&self.next_run_at)
            .map(|dt| dt.with_timezone(&Utc) <= now)
            .unwrap_or(true)
    }

    pub fn schedule_summary(&self) -> String {
        if self.schedules.is_empty() {
            return match self.repetition {
                Repetition::Once => {
                    if self.next_run_at.is_empty() {
                        "Once, immediately".to_string()
                    } else {
                        format!("Once at {}", human_datetime(&self.next_run_at))
                    }
                }
                Repetition::Repeat => format!("Every {}", human_duration(self.frequency_seconds)),
            };
        }

        self.schedules
            .iter()
            .map(TaskSchedule::summary)
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn next_run_summary(&self) -> String {
        let mut dates = Vec::new();
        if self.schedules.is_empty() {
            if !self.next_run_at.is_empty() {
                dates.push(self.next_run_at.as_str());
            }
        } else {
            for schedule in &self.schedules {
                if let Some(next) = schedule.next_run_at() {
                    dates.push(next);
                }
            }
        }

        dates
            .into_iter()
            .filter_map(|value| parse_rfc3339_utc(value).ok())
            .min()
            .map(|dt| human_datetime(&dt.to_rfc3339()))
            .unwrap_or_else(|| "Immediate".to_string())
    }
}

impl TaskSchedule {
    pub fn due_now(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled() {
            return false;
        }

        match self {
            Self::Once { next_run_at, .. } => {
                if next_run_at.is_empty() {
                    true
                } else {
                    parse_rfc3339_utc(next_run_at)
                        .map(|dt| now >= dt)
                        .unwrap_or(false)
                }
            }
            Self::Interval {
                next_run_at,
                working_hours,
                start_time,
                ..
            } => {
                let is_due = if next_run_at.is_empty() {
                    true
                } else {
                    parse_rfc3339_utc(next_run_at)
                        .map(|dt| now >= dt)
                        .unwrap_or(false)
                };

                if is_due {
                    if let Some(wh) = working_hours {
                        if !is_within_working_hours(wh, now) {
                            return false;
                        }
                    }

                    if let Some(st) = start_time {
                        if !st.is_empty() {
                            if let Ok(st_time) = chrono::NaiveTime::parse_from_str(st, "%H:%M") {
                                let now_local = now.with_timezone(&chrono::Local);
                                if now_local.time() < st_time {
                                    return false;
                                }
                            }
                        }
                    }
                    true
                } else {
                    false
                }
            }
            Self::DailyTimes {
                next_run_at,
                working_hours,
                ..
            } => {
                let is_due = if next_run_at.is_empty() {
                    false
                } else {
                    parse_rfc3339_utc(next_run_at)
                        .map(|dt| now >= dt)
                        .unwrap_or(false)
                };

                if is_due {
                    if let Some(wh) = working_hours {
                        is_working_day(wh, now)
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
            Self::Weekly { next_run_at, .. } | Self::Monthly { next_run_at, .. } => {
                if next_run_at.is_empty() {
                    false
                } else {
                    parse_rfc3339_utc(next_run_at)
                        .map(|dt| now >= dt)
                        .unwrap_or(false)
                }
            }
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Once { enabled, .. }
            | Self::Interval { enabled, .. }
            | Self::DailyTimes { enabled, .. }
            | Self::Weekly { enabled, .. }
            | Self::Monthly { enabled, .. } => *enabled,
        }
    }

    pub fn next_run_at(&self) -> Option<&str> {
        match self {
            Self::Once { next_run_at, .. }
            | Self::Interval { next_run_at, .. }
            | Self::DailyTimes { next_run_at, .. }
            | Self::Weekly { next_run_at, .. }
            | Self::Monthly { next_run_at, .. } => {
                if next_run_at.is_empty() {
                    None
                } else {
                    Some(next_run_at.as_str())
                }
            }
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::Once {
                enabled,
                next_run_at,
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                if next_run_at.is_empty() {
                    format!("Once, immediately{}", state)
                } else {
                    format!("Once at {}{}", human_datetime(next_run_at), state)
                }
            }
            Self::Interval {
                enabled,
                every_seconds,
                start_time,
                ..
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                let start_info = if let Some(st) = start_time {
                    format!(" starting at {}", st)
                } else {
                    "".to_string()
                };
                format!(
                    "Every {}{}{}",
                    human_duration(*every_seconds),
                    start_info,
                    state
                )
            }
            Self::DailyTimes { enabled, times, .. } => {
                let state = if *enabled { "" } else { " (disabled)" };
                if times.is_empty() {
                    format!("Daily, no times{}", state)
                } else {
                    format!("Daily at {} local{}", times.join(", "), state)
                }
            }
            Self::Weekly {
                enabled,
                day_of_week,
                at_time,
                ..
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                let time_str = if at_time.is_empty() {
                    "default".to_string()
                } else {
                    at_time.clone()
                };
                format!("Weekly on {} at {}{}", day_of_week, time_str, state)
            }
            Self::Monthly {
                enabled,
                day_of_month,
                at_time,
                ..
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                let time_str = if at_time.is_empty() {
                    "default".to_string()
                } else {
                    at_time.clone()
                };
                format!("Monthly on day {} at {}{}", day_of_month, time_str, state)
            }
        }
    }
}

pub fn human_datetime(value: &str) -> String {
    parse_rfc3339_utc(value)
        .map(|dt| {
            let local = dt.with_timezone(&Local);
            format!(
                "{} ({})",
                local.format("%b %-d, %Y %-I:%M %p local"),
                relative_time(dt, Utc::now())
            )
        })
        .unwrap_or_else(|_| value.to_string())
}

pub fn is_within_working_hours(
    working_hours: &std::collections::HashMap<String, WorkingHours>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let now_local = now.with_timezone(&chrono::Local);
    let current_day_idx = now_local.weekday().num_days_from_monday();
    let current_time = now_local.time();

    let parse_day = |day: &str| -> Option<u32> {
        match day {
            "Mon" | "Monday" | "mon" => Some(0),
            "Tue" | "Tuesday" | "tue" => Some(1),
            "Wed" | "Wednesday" | "wed" => Some(2),
            "Thu" | "Thursday" | "thu" => Some(3),
            "Fri" | "Friday" | "fri" => Some(4),
            "Sat" | "Saturday" | "sat" => Some(5),
            "Sun" | "Sunday" | "sun" => Some(6),
            _ => None,
        }
    };

    let weekday_str = match now_local.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    };

    let mut matched_hours = None;

    for (days_str, hours) in working_hours {
        let parts: Vec<&str> = days_str.split('-').map(|s| s.trim()).collect();
        let is_match = if parts.len() == 2 {
            let start_day = parse_day(parts[0]);
            let end_day = parse_day(parts[1]);
            if let (Some(s), Some(e)) = (start_day, end_day) {
                if s <= e {
                    current_day_idx >= s && current_day_idx <= e
                } else {
                    current_day_idx >= s || current_day_idx <= e
                }
            } else {
                false
            }
        } else {
            days_str == weekday_str || parse_day(days_str) == Some(current_day_idx)
        };

        if is_match {
            matched_hours = Some(hours);
            break;
        }
    }

    if let Some(hours) = matched_hours {
        if let (Ok(start), Ok(end)) = (
            chrono::NaiveTime::parse_from_str(&hours.start, "%H:%M"),
            chrono::NaiveTime::parse_from_str(&hours.end, "%H:%M"),
        ) {
            if start <= end {
                return current_time >= start && current_time <= end;
            } else {
                return current_time >= start || current_time <= end;
            }
        }
        return false;
    }

    working_hours.is_empty()
}

pub fn next_working_time(
    working_hours: &std::collections::HashMap<String, WorkingHours>,
    now: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    if is_within_working_hours(working_hours, now) {
        return now;
    }

    let mut candidate = now;
    // Advance minute by minute up to 14 days
    for _ in 0..(14 * 24 * 60) {
        candidate += chrono::TimeDelta::minutes(1);
        if is_within_working_hours(working_hours, candidate) {
            return candidate;
        }
    }

    now
}

pub fn human_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0 seconds".to_string();
    }

    let units = [
        ("day", 86_400),
        ("hour", 3_600),
        ("minute", 60),
        ("second", 1),
    ];
    let mut remaining = seconds;
    let mut parts = Vec::new();

    for (name, unit_seconds) in units {
        let count = remaining / unit_seconds;
        if count > 0 {
            parts.push(format!(
                "{} {}{}",
                count,
                name,
                if count == 1 { "" } else { "s" }
            ));
            remaining %= unit_seconds;
        }
        if parts.len() == 2 {
            break;
        }
    }

    parts.join(" ")
}

pub fn relative_time(value: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = now.signed_duration_since(value).num_seconds();
    if seconds.abs() < 60 {
        return if seconds >= 0 {
            "just now".to_string()
        } else {
            "in less than 1 minute".to_string()
        };
    }

    let abs = seconds.unsigned_abs();
    let label = human_duration(abs);
    if seconds >= 0 {
        format!("{} ago", label)
    } else {
        format!("in {}", label)
    }
}

pub fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("Invalid RFC3339 timestamp '{}'", value))
}

pub fn next_daily_run_after(
    times: &[String],
    now: DateTime<Utc>,
    working_hours: Option<&std::collections::HashMap<String, WorkingHours>>,
) -> Result<String> {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let mut candidates = Vec::new();

    for raw in times {
        let time = NaiveTime::parse_from_str(raw.trim(), "%H:%M")
            .with_context(|| format!("Invalid daily time '{}'. Use HH:MM", raw))?;

        // Look ahead up to 14 days to find the next valid working day,
        // to avoid infinite loops if configuration is somehow completely disjoint,
        // though typically it would just look ahead a couple days at most.
        for day_offset in 0_i64..14 {
            let date = today + chrono::TimeDelta::days(day_offset);
            let local_dt = date.and_time(time);
            let candidate = Local
                .from_local_datetime(&local_dt)
                .earliest()
                .or_else(|| Local.from_local_datetime(&local_dt).latest())
                .with_context(|| format!("Local time '{}' could not be resolved", raw))?
                .with_timezone(&Utc);

            if candidate > now {
                if let Some(wh) = working_hours {
                    if is_working_day(wh, candidate) {
                        candidates.push(candidate);
                        break; // Only need the first valid future candidate for this specific time
                    }
                } else {
                    candidates.push(candidate);
                    break;
                }
            }
        }
    }

    candidates
        .into_iter()
        .min()
        .map(|dt| dt.to_rfc3339())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "daily_times schedule requires at least one HH:MM time that falls on a working day"
            )
        })
}

pub fn is_working_day(
    working_hours: &std::collections::HashMap<String, WorkingHours>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let now_local = now.with_timezone(&chrono::Local);
    let current_day_idx = now_local.weekday().num_days_from_monday();

    let parse_day = |day: &str| -> Option<u32> {
        match day {
            "Mon" | "Monday" | "mon" => Some(0),
            "Tue" | "Tuesday" | "tue" => Some(1),
            "Wed" | "Wednesday" | "wed" => Some(2),
            "Thu" | "Thursday" | "thu" => Some(3),
            "Fri" | "Friday" | "fri" => Some(4),
            "Sat" | "Saturday" | "sat" => Some(5),
            "Sun" | "Sunday" | "sun" => Some(6),
            _ => None,
        }
    };

    let weekday_str = match now_local.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    };

    for days_str in working_hours.keys() {
        let parts: Vec<&str> = days_str.split('-').map(|s| s.trim()).collect();
        let is_match = if parts.len() == 2 {
            let start_day = parse_day(parts[0]);
            let end_day = parse_day(parts[1]);
            if let (Some(s), Some(e)) = (start_day, end_day) {
                if s <= e {
                    current_day_idx >= s && current_day_idx <= e
                } else {
                    current_day_idx >= s || current_day_idx <= e
                }
            } else {
                false
            }
        } else {
            days_str == weekday_str || parse_day(days_str) == Some(current_day_idx)
        };

        if is_match {
            return true;
        }
    }

    working_hours.is_empty()
}

pub fn next_weekly_run_after(
    day_of_week: &str,
    at_time: &str,
    now: DateTime<Utc>,
    working_hours: Option<&std::collections::HashMap<String, WorkingHours>>,
) -> Result<String> {
    let day_lower = day_of_week.trim().to_lowercase();
    let target_weekday = match day_lower.as_str() {
        "sunday" | "sun" | "0" => chrono::Weekday::Sun,
        "monday" | "mon" | "1" => chrono::Weekday::Mon,
        "tuesday" | "tue" | "2" => chrono::Weekday::Tue,
        "wednesday" | "wed" | "3" => chrono::Weekday::Wed,
        "thursday" | "thu" | "4" => chrono::Weekday::Thu,
        "friday" | "fri" | "5" => chrono::Weekday::Fri,
        "saturday" | "sat" | "6" => chrono::Weekday::Sat,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid day of week '{}'. Use monday-sunday (or mon-sun, 0-6)",
                day_of_week
            ))
        }
    };

    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let now_weekday = today.weekday();

    let time = if at_time.is_empty() {
        NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is mathematically valid")
    } else {
        NaiveTime::parse_from_str(at_time.trim(), "%H:%M")
            .with_context(|| format!("Invalid weekly time '{}'. Use HH:MM", at_time))?
    };

    let days_until_target = (target_weekday.num_days_from_monday() as i64
        - now_weekday.num_days_from_monday() as i64
        + 7)
        % 7;

    // We will check up to 52 weeks ahead
    for week_offset in 0_i64..52 {
        let total_days_offset = days_until_target + (week_offset * 7);
        let target_date = today + chrono::TimeDelta::days(total_days_offset);
        let local_dt = target_date.and_time(time);
        let candidate = match Local.from_local_datetime(&local_dt) {
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(dt, _) => dt,
            chrono::LocalResult::None => continue,
        }
        .with_timezone(&Utc);

        if candidate > now {
            if let Some(wh) = working_hours {
                if is_working_day(wh, candidate) {
                    return Ok(candidate.to_rfc3339());
                }
            } else {
                return Ok(candidate.to_rfc3339());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Could not resolve weekly schedule time on a valid working day"
    ))
}

pub fn next_monthly_run_after(
    day_of_month: u32,
    at_time: &str,
    now: DateTime<Utc>,
    working_hours: Option<&std::collections::HashMap<String, WorkingHours>>,
) -> Result<String> {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let current_year = today.year();
    let current_month = today.month();

    let time = if at_time.is_empty() {
        NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is mathematically valid")
    } else {
        NaiveTime::parse_from_str(at_time.trim(), "%H:%M")
            .with_context(|| format!("Invalid monthly time '{}'. Use HH:MM", at_time))?
    };

    for month_offset in 0..12 {
        let target_month = current_month + month_offset;
        let (year, month) = if target_month > 12 {
            (current_year + 1, target_month - 12)
        } else {
            (current_year, target_month)
        };

        let day = day_of_month.min(days_in_month(year, month));
        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .with_context(|| format!("Invalid date for month {}-{}", year, month))?;

        let local_dt = date.and_time(time);
        let candidate = match Local.from_local_datetime(&local_dt) {
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(dt, _) => dt,
            chrono::LocalResult::None => continue,
        }
        .with_timezone(&Utc);

        if candidate > now {
            if let Some(wh) = working_hours {
                if is_working_day(wh, candidate) {
                    return Ok(candidate.to_rfc3339());
                }
            } else {
                return Ok(candidate.to_rfc3339());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Could not find a valid monthly schedule date on a valid working day"
    ))
}

pub fn quarter_start_month(month: u32) -> u32 {
    match month {
        1..=3 => 1,
        4..=6 => 4,
        7..=9 => 7,
        10..=12 => 10,
        _ => 1,
    }
}

pub fn quarter_end_month(month: u32) -> u32 {
    match month {
        1..=3 => 3,
        4..=6 => 6,
        7..=9 => 9,
        10..=12 => 12,
        _ => 12,
    }
}

/// Generates execution periods according to the specified `PeriodMode`.
///
/// Converts resolved `start_date` and `end_date` into concrete `ExecutionPeriod`s.
pub fn generate_execution_periods(
    mode: PeriodMode,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Vec<ExecutionPeriod> {
    use chrono::{Datelike, NaiveDate};

    if start_date > end_date && mode == PeriodMode::Custom {
        return vec![ExecutionPeriod {
            start_date,
            end_date,
        }];
    }

    match mode {
        PeriodMode::Custom => {
            vec![ExecutionPeriod {
                start_date,
                end_date,
            }]
        }
        PeriodMode::Monthly => {
            let mut periods = Vec::new();
            let mut cur_year = start_date.year();
            let mut cur_month = start_date.month();

            let end_year = end_date.year();
            let end_month = end_date.month();

            loop {
                let period_start =
                    NaiveDate::from_ymd_opt(cur_year, cur_month, 1).unwrap_or(start_date);
                let last_day = days_in_month(cur_year, cur_month);
                let period_end =
                    NaiveDate::from_ymd_opt(cur_year, cur_month, last_day).unwrap_or(end_date);

                periods.push(ExecutionPeriod {
                    start_date: period_start,
                    end_date: period_end,
                });

                if cur_year > end_year || (cur_year == end_year && cur_month >= end_month) {
                    break;
                }

                if cur_month == 12 {
                    cur_year += 1;
                    cur_month = 1;
                } else {
                    cur_month += 1;
                }
            }

            periods
        }
        PeriodMode::Quarterly => {
            let mut periods = Vec::new();
            let mut cur_year = start_date.year();
            let mut cur_q_start_month = quarter_start_month(start_date.month());

            let end_year = end_date.year();
            let end_q_end_month = quarter_end_month(end_date.month());

            loop {
                let q_end_month = quarter_end_month(cur_q_start_month);
                let period_start =
                    NaiveDate::from_ymd_opt(cur_year, cur_q_start_month, 1).unwrap_or(start_date);
                let last_day = days_in_month(cur_year, q_end_month);
                let period_end =
                    NaiveDate::from_ymd_opt(cur_year, q_end_month, last_day).unwrap_or(end_date);

                periods.push(ExecutionPeriod {
                    start_date: period_start,
                    end_date: period_end,
                });

                if cur_year > end_year || (cur_year == end_year && q_end_month >= end_q_end_month) {
                    break;
                }

                if cur_q_start_month >= 10 {
                    cur_year += 1;
                    cur_q_start_month = 1;
                } else {
                    cur_q_start_month += 3;
                }
            }

            periods
        }
        PeriodMode::AMonth => {
            let mut periods = Vec::new();
            let target_month = start_date.month();
            let start_year = start_date.year();
            let end_year = end_date.year();

            for yr in start_year..=end_year {
                let period_start =
                    NaiveDate::from_ymd_opt(yr, target_month, 1).unwrap_or(start_date);
                let last_day = days_in_month(yr, target_month);
                let period_end =
                    NaiveDate::from_ymd_opt(yr, target_month, last_day).unwrap_or(end_date);

                periods.push(ExecutionPeriod {
                    start_date: period_start,
                    end_date: period_end,
                });
            }

            periods
        }
        PeriodMode::AQuarter => {
            let mut periods = Vec::new();
            let q_start = quarter_start_month(start_date.month());
            let q_end = quarter_end_month(start_date.month());
            let start_year = start_date.year();
            let end_year = end_date.year();

            for yr in start_year..=end_year {
                let period_start = NaiveDate::from_ymd_opt(yr, q_start, 1).unwrap_or(start_date);
                let last_day = days_in_month(yr, q_end);
                let period_end = NaiveDate::from_ymd_opt(yr, q_end, last_day).unwrap_or(end_date);

                periods.push(ExecutionPeriod {
                    start_date: period_start,
                    end_date: period_end,
                });
            }

            periods
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionOccurrence {
    pub period: ExecutionPeriod,
    pub scheduled_at: DateTime<Utc>,
}

pub fn generate_upcoming_executions(
    task: &RunnerTask,
    now: DateTime<Utc>,
    limit: usize,
) -> Result<Vec<ExecutionOccurrence>> {
    use chrono::{NaiveDate, TimeZone};

    if limit == 0 {
        return Ok(Vec::new());
    }

    // 1. Resolve start_date and end_date if present
    let start_date_str = task.start_date.as_deref().unwrap_or("today");
    let raw_start_date = crate::utils::parse_flexible_date(start_date_str)
        .unwrap_or_else(|| now.with_timezone(&chrono::Local).date_naive());

    let end_date_str = task.end_date.as_deref();
    let raw_end_date = if let Some(ed_str) = end_date_str {
        crate::utils::parse_flexible_date_with_base(ed_str, Some(start_date_str))
            .unwrap_or(raw_start_date)
    } else {
        raw_start_date
    };

    // 2. Generate execution periods using task.period_mode
    let periods = generate_execution_periods(task.period_mode, raw_start_date, raw_end_date);

    let mut results = Vec::new();

    // 3. Evaluate schedule over periods
    for period in periods {
        let period_start_utc = Local
            .from_local_datetime(&period.start_date.and_hms_opt(0, 0, 0).unwrap())
            .earliest()
            .unwrap_or_else(|| Utc::now().with_timezone(&Local))
            .with_timezone(&Utc);

        let period_end_utc = Local
            .from_local_datetime(&period.end_date.and_hms_opt(23, 59, 59).unwrap())
            .latest()
            .unwrap_or_else(|| Utc::now().with_timezone(&Local))
            .with_timezone(&Utc);

        if period_end_utc < now {
            continue;
        }

        if task.schedules.is_empty() {
            let scheduled_at = if period_start_utc > now {
                period_start_utc
            } else {
                now
            };
            results.push(ExecutionOccurrence {
                period,
                scheduled_at,
            });
            if results.len() >= limit {
                break;
            }
            continue;
        }

        for schedule in &task.schedules {
            if !schedule.enabled() {
                continue;
            }

            match schedule {
                TaskSchedule::Once { next_run_at, .. } => {
                    let dt = if next_run_at.is_empty() {
                        period_start_utc
                    } else if let Ok(parsed) = parse_rfc3339_utc(next_run_at) {
                        parsed
                    } else {
                        period_start_utc
                    };

                    if dt >= now && dt >= period_start_utc && dt <= period_end_utc {
                        results.push(ExecutionOccurrence {
                            period: period.clone(),
                            scheduled_at: dt,
                        });
                        if results.len() >= limit {
                            return Ok(results);
                        }
                    }
                }
                TaskSchedule::Interval {
                    every_seconds,
                    working_hours,
                    start_time,
                    ..
                } => {
                    let interval = (*every_seconds).max(1);
                    let mut cursor = if period_start_utc > now {
                        period_start_utc
                    } else {
                        now
                    };

                    if let Some(st) = start_time {
                        if !st.is_empty() {
                            if let Ok(st_time) = NaiveTime::parse_from_str(st.trim(), "%H:%M") {
                                let local_cursor = cursor.with_timezone(&Local);
                                if local_cursor.time() < st_time {
                                    if let Some(naive_dt) = local_cursor
                                        .date_naive()
                                        .and_time(st_time)
                                        .and_local_timezone(Local)
                                        .single()
                                    {
                                        let candidate = naive_dt.with_timezone(&Utc);
                                        if candidate > cursor {
                                            cursor = candidate;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let mut steps_count = 0;
                    while cursor <= period_end_utc && results.len() < limit && steps_count < 1000 {
                        steps_count += 1;
                        let is_wh_ok = if let Some(wh) = working_hours {
                            is_within_working_hours(wh, cursor)
                        } else {
                            true
                        };

                        if is_wh_ok && cursor >= now {
                            results.push(ExecutionOccurrence {
                                period: period.clone(),
                                scheduled_at: cursor,
                            });
                            if results.len() >= limit {
                                return Ok(results);
                            }
                        }

                        cursor += chrono::TimeDelta::seconds(interval as i64);
                        if let Some(wh) = working_hours {
                            if !is_within_working_hours(wh, cursor) {
                                cursor = next_working_time(wh, cursor);
                            }
                        }
                    }
                }
                TaskSchedule::DailyTimes {
                    times,
                    working_hours,
                    ..
                } => {
                    let mut cur_date = period.start_date;
                    while cur_date <= period.end_date && results.len() < limit {
                        for raw_time in times {
                            if let Ok(time) = NaiveTime::parse_from_str(raw_time.trim(), "%H:%M") {
                                if let Some(local_dt) =
                                    cur_date.and_time(time).and_local_timezone(Local).single()
                                {
                                    let dt = local_dt.with_timezone(&Utc);
                                    if dt >= now && dt >= period_start_utc && dt <= period_end_utc {
                                        let is_wh_ok = if let Some(wh) = working_hours {
                                            is_working_day(wh, dt)
                                        } else {
                                            true
                                        };
                                        if is_wh_ok {
                                            results.push(ExecutionOccurrence {
                                                period: period.clone(),
                                                scheduled_at: dt,
                                            });
                                            if results.len() >= limit {
                                                return Ok(results);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        cur_date += chrono::TimeDelta::days(1);
                    }
                }
                TaskSchedule::Weekly {
                    day_of_week,
                    at_time,
                    working_hours,
                    ..
                } => {
                    let mut cur_date = period.start_date;
                    let target_weekday = match day_of_week.trim().to_lowercase().as_str() {
                        "sunday" | "sun" | "0" => chrono::Weekday::Sun,
                        "monday" | "mon" | "1" => chrono::Weekday::Mon,
                        "tuesday" | "tue" | "2" => chrono::Weekday::Tue,
                        "wednesday" | "wed" | "3" => chrono::Weekday::Wed,
                        "thursday" | "thu" | "4" => chrono::Weekday::Thu,
                        "friday" | "fri" | "5" => chrono::Weekday::Fri,
                        "saturday" | "sat" | "6" => chrono::Weekday::Sat,
                        _ => chrono::Weekday::Mon,
                    };
                    let time = if at_time.is_empty() {
                        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                    } else {
                        NaiveTime::parse_from_str(at_time.trim(), "%H:%M")
                            .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    };

                    while cur_date <= period.end_date && results.len() < limit {
                        if cur_date.weekday() == target_weekday {
                            if let Some(local_dt) =
                                cur_date.and_time(time).and_local_timezone(Local).single()
                            {
                                let dt = local_dt.with_timezone(&Utc);
                                if dt >= now && dt >= period_start_utc && dt <= period_end_utc {
                                    let is_wh_ok = if let Some(wh) = working_hours {
                                        is_working_day(wh, dt)
                                    } else {
                                        true
                                    };
                                    if is_wh_ok {
                                        results.push(ExecutionOccurrence {
                                            period: period.clone(),
                                            scheduled_at: dt,
                                        });
                                        if results.len() >= limit {
                                            return Ok(results);
                                        }
                                    }
                                }
                            }
                        }
                        cur_date += chrono::TimeDelta::days(1);
                    }
                }
                TaskSchedule::Monthly {
                    day_of_month,
                    at_time,
                    working_hours,
                    ..
                } => {
                    let mut cur_year = period.start_date.year();
                    let mut cur_month = period.start_date.month();
                    let end_year = period.end_date.year();
                    let end_month = period.end_date.month();

                    let time = if at_time.is_empty() {
                        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                    } else {
                        NaiveTime::parse_from_str(at_time.trim(), "%H:%M")
                            .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    };

                    loop {
                        let day = (*day_of_month).min(days_in_month(cur_year, cur_month));
                        if let Some(cur_date) = NaiveDate::from_ymd_opt(cur_year, cur_month, day) {
                            if cur_date >= period.start_date && cur_date <= period.end_date {
                                if let Some(local_dt) =
                                    cur_date.and_time(time).and_local_timezone(Local).single()
                                {
                                    let dt = local_dt.with_timezone(&Utc);
                                    if dt >= now && dt >= period_start_utc && dt <= period_end_utc {
                                        let is_wh_ok = if let Some(wh) = working_hours {
                                            is_working_day(wh, dt)
                                        } else {
                                            true
                                        };
                                        if is_wh_ok {
                                            results.push(ExecutionOccurrence {
                                                period: period.clone(),
                                                scheduled_at: dt,
                                            });
                                            if results.len() >= limit {
                                                return Ok(results);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if cur_year > end_year || (cur_year == end_year && cur_month >= end_month) {
                            break;
                        }

                        if cur_month == 12 {
                            cur_year += 1;
                            cur_month = 1;
                        } else {
                            cur_month += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_execution_periods() {
        use chrono::NaiveDate;

        let jan15 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let sep10 = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();

        // 1. Custom mode
        let custom = generate_execution_periods(PeriodMode::Custom, jan15, sep10);
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].start_date, jan15);
        assert_eq!(custom[0].end_date, sep10);

        // 2. Monthly mode (Jan 15 to Sep 10 => 9 full months)
        let monthly = generate_execution_periods(PeriodMode::Monthly, jan15, sep10);
        assert_eq!(monthly.len(), 9);
        assert_eq!(
            monthly[0].start_date,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert_eq!(
            monthly[0].end_date,
            NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()
        );
        assert_eq!(
            monthly[1].start_date,
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
        );
        assert_eq!(
            monthly[1].end_date,
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
        assert_eq!(
            monthly[8].start_date,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        );
        assert_eq!(
            monthly[8].end_date,
            NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
        );

        // Single day monthly
        let sep1 = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let single_m = generate_execution_periods(PeriodMode::Monthly, sep1, sep1);
        assert_eq!(single_m.len(), 1);
        assert_eq!(
            single_m[0].start_date,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        );
        assert_eq!(
            single_m[0].end_date,
            NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
        );

        // 3. Quarterly mode (Feb 15 to Aug 10 => Q1, Q2, Q3)
        let feb15 = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        let aug10 = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
        let quarterly = generate_execution_periods(PeriodMode::Quarterly, feb15, aug10);
        assert_eq!(quarterly.len(), 3);
        assert_eq!(
            quarterly[0].start_date,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
        assert_eq!(
            quarterly[0].end_date,
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()
        );
        assert_eq!(
            quarterly[1].start_date,
            NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()
        );
        assert_eq!(
            quarterly[1].end_date,
            NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()
        );
        assert_eq!(
            quarterly[2].start_date,
            NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()
        );
        assert_eq!(
            quarterly[2].end_date,
            NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
        );

        // 4. A Month mode (Sep 15, 2022 to Sep 20, 2026)
        let start_2022 = NaiveDate::from_ymd_opt(2022, 9, 15).unwrap();
        let end_2026 = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let a_month = generate_execution_periods(PeriodMode::AMonth, start_2022, end_2026);
        assert_eq!(a_month.len(), 5);
        for (i, yr) in (2022..=2026).enumerate() {
            assert_eq!(
                a_month[i].start_date,
                NaiveDate::from_ymd_opt(yr, 9, 1).unwrap()
            );
            assert_eq!(
                a_month[i].end_date,
                NaiveDate::from_ymd_opt(yr, 9, 30).unwrap()
            );
        }

        // 5. A Quarter mode (Aug 15, 2022 to Sep 20, 2026 => Q3 for every year)
        let start_q3 = NaiveDate::from_ymd_opt(2022, 8, 15).unwrap();
        let a_quarter = generate_execution_periods(PeriodMode::AQuarter, start_q3, end_2026);
        assert_eq!(a_quarter.len(), 5);
        for (i, yr) in (2022..=2026).enumerate() {
            assert_eq!(
                a_quarter[i].start_date,
                NaiveDate::from_ymd_opt(yr, 7, 1).unwrap()
            );
            assert_eq!(
                a_quarter[i].end_date,
                NaiveDate::from_ymd_opt(yr, 9, 30).unwrap()
            );
        }

        // 6. Leap year February test
        let feb_leap_start = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();
        let feb_leap_end = NaiveDate::from_ymd_opt(2024, 2, 20).unwrap();
        let leap_m = generate_execution_periods(PeriodMode::Monthly, feb_leap_start, feb_leap_end);
        assert_eq!(leap_m.len(), 1);
        assert_eq!(
            leap_m[0].end_date,
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
    }

    #[test]
    fn test_generate_upcoming_executions_limit_and_bounded() {
        use chrono::TimeZone;

        let now = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let task = RunnerTask {
            id: "t1".to_string(),
            name: "Interval Task".to_string(),
            enabled: true,
            repetition: Repetition::Repeat,
            frequency_seconds: 3600,
            next_run_at: String::new(),
            schedules: vec![TaskSchedule::Interval {
                enabled: true,
                every_seconds: 3600,
                next_run_at: String::new(),
                working_hours: None,
                working_hours_profile_id: None,
                start_time: None,
            }],
            steps: vec![],
            post_run_steps: vec![],
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 0,
            period_mode: PeriodMode::Monthly,
            start_date: Some("2026-01-01".to_string()),
            end_date: Some("2026-12-31".to_string()),
        };

        let occurrences = generate_upcoming_executions(&task, now, 10).unwrap();
        assert_eq!(occurrences.len(), 10);
        assert_eq!(occurrences[0].scheduled_at, now);
        assert_eq!(
            occurrences[1].scheduled_at,
            now + chrono::TimeDelta::hours(1)
        );

        // Historical date test (0 results)
        let past_task = RunnerTask {
            id: "t2".to_string(),
            name: "Past Task".to_string(),
            enabled: true,
            repetition: Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            schedules: vec![],
            steps: vec![],
            post_run_steps: vec![],
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 0,
            period_mode: PeriodMode::Custom,
            start_date: Some("2020-01-01".to_string()),
            end_date: Some("2020-01-05".to_string()),
        };

        let past_occurrences = generate_upcoming_executions(&past_task, now, 10).unwrap();
        assert!(past_occurrences.is_empty());
    }

    #[test]
    fn human_duration_uses_largest_units() {
        assert_eq!(human_duration(3_660), "1 hour 1 minute");
        assert_eq!(human_duration(86_400), "1 day");
    }

    #[test]
    fn test_next_daily_run_after() {
        use chrono::TimeZone;

        // Base date: 2024-01-01 12:00:00 UTC
        let base_now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // 1. Same day, future time
        let times = vec!["15:00".to_string()];
        let res = next_daily_run_after(&times, base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        // It should be strictly after base_now
        assert!(dt.with_timezone(&Utc) > base_now);

        // 2. Same day, past time (should wrap to next day)
        let times = vec!["10:00".to_string()];
        let res = next_daily_run_after(&times, base_now, None).unwrap();
        let dt2 = DateTime::parse_from_rfc3339(&res).unwrap();
        assert!(dt2.with_timezone(&Utc) > base_now);

        // 3. Multiple times, picks the earliest valid one
        let times = vec![
            "10:00".to_string(),
            "15:00".to_string(),
            "18:00".to_string(),
        ];
        let res = next_daily_run_after(&times, base_now, None).unwrap();
        let dt3 = DateTime::parse_from_rfc3339(&res).unwrap();
        assert!(dt3.with_timezone(&Utc) > base_now);
        assert!(dt3 <= dt);

        // 4. Empty times array
        let empty_times: Vec<String> = vec![];
        assert!(next_daily_run_after(&empty_times, base_now, None).is_err());

        // 5. Invalid time formats
        let invalid_times = vec!["25:00".to_string()];
        assert!(next_daily_run_after(&invalid_times, base_now, None).is_err());

        let invalid_format = vec!["3 PM".to_string()];
        assert!(next_daily_run_after(&invalid_format, base_now, None).is_err());
    }

    #[test]
    fn test_next_weekly_run_after() {
        use chrono::TimeZone;

        // Base date: Monday 2024-01-01 12:00:00 UTC
        let base_now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // 1. Same day, future time
        let res = next_weekly_run_after("Monday", "15:00", base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).year(), 2024);
        assert_eq!(dt.with_timezone(&Utc).month(), 1);
        assert_eq!(dt.with_timezone(&Utc).day(), 1);
        assert!(dt.with_timezone(&Utc) > base_now);

        // 2. Same day, past time (should wrap to next week)
        let res = next_weekly_run_after("Monday", "10:00", base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert!(dt.with_timezone(&Utc) > base_now);
        let diff = dt.with_timezone(&Utc) - base_now;
        assert!(diff.num_days() >= 6);

        // 3. Different day formats
        assert!(next_weekly_run_after("mon", "10:00", base_now, None).is_ok());
        assert!(next_weekly_run_after("1", "10:00", base_now, None).is_ok());
        assert!(next_weekly_run_after("monday", "10:00", base_now, None).is_ok());

        // 4. Later in the week
        let res = next_weekly_run_after("Wednesday", "12:00", base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).weekday(), chrono::Weekday::Wed);

        // 5. Earlier in the week (should wrap)
        let res = next_weekly_run_after("Sunday", "12:00", base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).weekday(), chrono::Weekday::Sun);
        assert!(dt.with_timezone(&Utc) > base_now);

        // 6. Invalid inputs
        assert!(next_weekly_run_after("InvalidDay", "12:00", base_now, None).is_err());
        assert!(next_weekly_run_after("Monday", "25:00", base_now, None).is_err());
        assert!(next_weekly_run_after("Monday", "not-a-time", base_now, None).is_err());

        // 7. Empty time (should default to 00:00)
        let res = next_weekly_run_after("Tuesday", "", base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).weekday(), chrono::Weekday::Tue);
    }

    #[test]
    fn test_relative_time() {
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // 1. just now (past, < 60s)
        assert_eq!(relative_time(now, now), "just now");
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(59), now),
            "just now"
        );

        // 2. in less than 1 minute (future, < 60s)
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(1), now),
            "in less than 1 minute"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(59), now),
            "in less than 1 minute"
        );

        // 3. past, >= 60s
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(60), now),
            "1 minute ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(119), now),
            "1 minute 59 seconds ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(120), now),
            "2 minutes ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(3600), now),
            "1 hour ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(7200), now),
            "2 hours ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(86400), now),
            "1 day ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(172800), now),
            "2 days ago"
        );

        // 4. future, >= 60s
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(60), now),
            "in 1 minute"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(119), now),
            "in 1 minute 59 seconds"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(120), now),
            "in 2 minutes"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(3600), now),
            "in 1 hour"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(7200), now),
            "in 2 hours"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(86400), now),
            "in 1 day"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(172800), now),
            "in 2 days"
        );

        // 5. compound values (human_duration returns up to two units)
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(3600 + 120), now),
            "1 hour 2 minutes ago"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(86400 + 7200), now),
            "in 1 day 2 hours"
        );
    }

    #[test]
    fn test_is_within_working_hours() {
        use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
        use std::collections::HashMap;

        let mut working_hours = HashMap::new();
        working_hours.insert(
            "Monday".to_string(),
            WorkingHours {
                start: "09:00".to_string(),
                end: "17:00".to_string(),
            },
        );
        working_hours.insert(
            "Friday".to_string(),
            WorkingHours {
                start: "10:00".to_string(),
                end: "15:00".to_string(),
            },
        );

        let date_mon = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();

        let time_10am = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let dt_mon_10am = Local
            .from_local_datetime(&NaiveDateTime::new(date_mon, time_10am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        let time_8am = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let dt_mon_8am = Local
            .from_local_datetime(&NaiveDateTime::new(date_mon, time_8am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        let time_6pm = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        let dt_mon_6pm = Local
            .from_local_datetime(&NaiveDateTime::new(date_mon, time_6pm))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert!(is_within_working_hours(&working_hours, dt_mon_10am));
        assert!(!is_within_working_hours(&working_hours, dt_mon_8am));
        assert!(!is_within_working_hours(&working_hours, dt_mon_6pm));

        let date_tue = NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();
        let dt_tue_10am = Local
            .from_local_datetime(&NaiveDateTime::new(date_tue, time_10am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert!(!is_within_working_hours(&working_hours, dt_tue_10am));

        let date_fri = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let dt_fri_10am = Local
            .from_local_datetime(&NaiveDateTime::new(date_fri, time_10am))
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let dt_fri_8am = Local
            .from_local_datetime(&NaiveDateTime::new(date_fri, time_8am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert!(is_within_working_hours(&working_hours, dt_fri_10am));
        assert!(!is_within_working_hours(&working_hours, dt_fri_8am));
    }
}
