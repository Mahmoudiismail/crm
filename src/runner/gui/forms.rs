#![allow(unused_imports)]
use super::helpers::*;
use super::HttpRequest;
use crate::runner::config::*;
use crate::runner::engine::*;
use anyhow::{Context, Result};
use chrono::{Local, Utc};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{error, info};
pub(crate) fn build_task_from_values(
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

    let steps_json = values.get("steps").map(|v| v.as_str()).unwrap_or("[]");
    let steps: Vec<TaskStep> = serde_json::from_str(steps_json)
        .with_context(|| format!("Invalid steps JSON: {}", steps_json))?;

    let post_run_steps_json = values
        .get("post_run_steps")
        .map(|v| v.as_str())
        .unwrap_or("[]");
    let post_run_steps: Vec<TaskStep> = serde_json::from_str(post_run_steps_json)
        .with_context(|| format!("Invalid post_run_steps JSON: {}", post_run_steps_json))?;

    let task = RunnerTask {
        id,
        name,
        enabled: parse_checkbox(values, "enabled"),
        repetition,
        frequency_seconds,
        next_run_at,
        schedules,
        steps,
        post_run_steps,
        last_run_at: String::new(),
        last_status: String::new(),
        timeout_seconds,
    };

    Ok(task)
}

pub(crate) fn legacy_fields_from_schedules(
    schedules: &[TaskSchedule],
) -> (Repetition, u64, String) {
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

pub(crate) fn legacy_fields_from_values(
    values: &HashMap<String, String>,
) -> (Repetition, u64, String) {
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

pub(crate) fn parse_schedules_text(value: &str) -> Result<Vec<TaskSchedule>> {
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

                let mut base_str = every_str;
                let mut start_time = None;
                if let Some((e, st_str)) = base_str.split_once("; st:") {
                    base_str = e.trim();
                    let st_val = st_str.trim();
                    if !st_val.is_empty() {
                        start_time = Some(st_val.to_string());
                    }
                }
                base_str = base_str.strip_prefix("every").unwrap_or(base_str).trim();

                schedules.push(TaskSchedule::Interval {
                    enabled: true,
                    every_seconds: parse_duration_text(base_str)?,
                    next_run_at: String::new(),
                    working_hours,
                    start_time,
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
                let mut rest_str = rest;
                let mut working_hours = None;
                if let Some((r, wh_str)) = rest_str.split_once("; wh:") {
                    rest_str = r.trim();
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

                let mut at_time = "09:00".to_string();
                if let Some((r, st_str)) = rest_str.split_once("; st:") {
                    rest_str = r.trim();
                    let st_val = st_str.trim();
                    if !st_val.is_empty() {
                        at_time = st_val.to_string();
                    }
                }

                schedules.push(TaskSchedule::Weekly {
                    enabled: true,
                    day_of_week: rest_str.to_string(),
                    at_time,
                    next_run_at: Utc::now().to_rfc3339(),
                    working_hours,
                });
            }
            "monthly" => {
                let mut rest_str = rest;
                let mut working_hours = None;
                if let Some((r, wh_str)) = rest_str.split_once("; wh:") {
                    rest_str = r.trim();
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

                let mut at_time = "09:00".to_string();
                if let Some((r, st_str)) = rest_str.split_once("; st:") {
                    rest_str = r.trim();
                    let st_val = st_str.trim();
                    if !st_val.is_empty() {
                        at_time = st_val.to_string();
                    }
                }

                let day_str = rest_str
                    .strip_prefix("day")
                    .unwrap_or(rest_str)
                    .trim()
                    .parse::<u32>()
                    .with_context(|| format!("Invalid day of month '{}'", rest_str))?;
                schedules.push(TaskSchedule::Monthly {
                    enabled: true,
                    day_of_month: day_str.clamp(1, 31),
                    at_time,
                    next_run_at: Utc::now().to_rfc3339(),
                    working_hours,
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

pub(crate) fn parse_duration_text(value: &str) -> Result<u64> {
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

pub(crate) fn parse_duration_token(token: &str) -> Result<u64> {
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

pub(crate) fn compact_duration(seconds: u64) -> String {
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

pub(crate) fn parse_checkbox(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}
