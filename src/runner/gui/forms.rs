use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;

use crate::runner::config::{
    next_daily_run_after, parse_rfc3339_utc, Repetition, RunnerTask, ShellCommandMode,
    ShellCommandSpec, TaskKind, TaskSchedule,
};
use crate::runner::gui::validation::parse_checkbox;

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

    let post_run_action = values
        .get("post_run_action")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let mut post_run_script = String::new();
    let mut post_run_app_id = String::new();
    let mut post_run_app_args = HashMap::new();

    if post_run_action == "script" {
        post_run_script = values
            .get("post_run_script")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
    } else if post_run_action == "external_app" {
        post_run_app_id = values
            .get("post_run_app_id")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let args_json = values
            .get("post_run_app_args")
            .map(|s| s.as_str())
            .unwrap_or("{}");
        if let Ok(parsed_args) = serde_json::from_str(args_json) {
            post_run_app_args = parsed_args;
        }
    }

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
        .unwrap_or_else(|| "shell_command".to_string());

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
    } else if task_type == "external_app" {
        let app_id = values
            .get("external_app_id")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let args_json = values
            .get("external_app_args")
            .map(|s| s.as_str())
            .unwrap_or("{}");
        let args: HashMap<String, String> = serde_json::from_str(args_json)
            .with_context(|| format!("Invalid external app args JSON: {}", args_json))?;
        TaskKind::ExternalApp { app_id, args }
    } else {
        TaskKind::ShellCommand {
            mode: ShellCommandMode::Sequential,
            commands: Vec::new(),
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
        post_run_app_id,
        post_run_app_args,
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
                    let mut wh_map = HashMap::new();
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

                let mut base_str = every_str.strip_prefix("every").unwrap_or(every_str).trim();
                let mut start_time = None;
                if let Some((e, st_str)) = base_str.split_once("; st:") {
                    base_str = e.trim();
                    let st_val = st_str.trim();
                    if !st_val.is_empty() {
                        start_time = Some(st_val.to_string());
                    }
                }

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
                    let mut wh_map = HashMap::new();
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
                    let mut wh_map = HashMap::new();
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
                schedules.push(TaskSchedule::Weekly {
                    enabled: true,
                    day_of_week: rest_str.to_string(),
                    at_time: "09:00".to_string(),
                    next_run_at: Utc::now().to_rfc3339(),
                    working_hours,
                });
            }
            "monthly" => {
                let mut rest_str = rest;
                let mut working_hours = None;
                if let Some((r, wh_str)) = rest_str.split_once("; wh:") {
                    rest_str = r.trim();
                    let mut wh_map = HashMap::new();
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
                let day_str = rest_str
                    .strip_prefix("day")
                    .unwrap_or(rest_str)
                    .trim()
                    .parse::<u32>()
                    .with_context(|| format!("Invalid day of month '{}'", rest_str))?;
                schedules.push(TaskSchedule::Monthly {
                    enabled: true,
                    day_of_month: day_str.clamp(1, 31),
                    at_time: "09:00".to_string(),
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

pub(crate) fn parse_shell_commands_text(value: &str) -> Result<Vec<ShellCommandSpec>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_schedule_text() {
        let schedules = parse_schedules_text(
            "interval: every 1h\ndaily: 09:00, 13:00\nonce: 2026-04-15T09:30:00-05:00",
        )
        .unwrap();
        assert_eq!(schedules.len(), 3);
        match schedules.first().expect("No schedule") {
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
    fn test_parses_schedule_text_with_working_hours() {
        let schedules =
            parse_schedules_text("interval: every 2h; wh: Monday=09:00-17:00,Friday=10:00-15:00\n")
                .unwrap();
        assert_eq!(schedules.len(), 1);
        match schedules.first().expect("No schedule") {
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
    fn test_parses_shell_commands_text_correctly() {
        let commands = parse_shell_commands_text(
            "run: echo prepare\ncontinue: cleanup-if-present\necho fallback",
        )
        .unwrap();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands.first().unwrap().command, "echo prepare");
        assert!(!commands.first().unwrap().continue_on_error);
        assert_eq!(commands.get(1).unwrap().command, "cleanup-if-present");
        assert!(commands.get(1).unwrap().continue_on_error);
        assert_eq!(commands.get(2).unwrap().command, "echo fallback");
        assert!(!commands.get(2).unwrap().continue_on_error);
    }

    #[test]
    fn test_duration_parser_accepts_human_units() {
        assert_eq!(parse_duration_text("1h").unwrap(), 3_600);
        assert_eq!(parse_duration_text("1h 30m").unwrap(), 5_400);
        assert_eq!(parse_duration_text("90").unwrap(), 90);
    }
}
