use crate::runner::config::{
    next_daily_run_after, next_monthly_run_after, next_weekly_run_after, parse_rfc3339_utc,
    Repetition, RunnerConfig, RunnerTask, TaskSchedule,
};
use crate::runner::engine::state::ExecutionPolicy;
use chrono::{DateTime, TimeZone, Utc};

pub fn policy_from_config(cfg: &RunnerConfig) -> ExecutionPolicy {
    ExecutionPolicy {
        allow_shell_tasks: cfg.allow_shell_tasks,
        shell_timeout_seconds: cfg.shell_timeout_seconds,
        post_run_timeout_seconds: cfg.post_run_timeout_seconds,
        min_task_interval_seconds: cfg.min_task_interval_seconds.max(1),
        registered_apps: cfg.registered_apps.clone(),
        log_retention_days: cfg.log_retention_days,
    }
}

pub fn set_schedule_enabled(schedule: &mut TaskSchedule, enabled_value: bool) {
    match schedule {
        TaskSchedule::Once { enabled, .. }
        | TaskSchedule::Interval { enabled, .. }
        | TaskSchedule::DailyTimes { enabled, .. }
        | TaskSchedule::Weekly { enabled, .. }
        | TaskSchedule::Monthly { enabled, .. } => *enabled = enabled_value,
    }
}

pub fn advance_schedule(
    schedule: &mut TaskSchedule,
    now: DateTime<Utc>,
    min_task_interval_seconds: u64,
) {
    match schedule {
        TaskSchedule::Once {
            enabled,
            next_run_at,
        } => {
            *enabled = false;
            next_run_at.clear();
        }
        TaskSchedule::Interval {
            every_seconds,
            next_run_at,
            start_time,
            working_hours,
            ..
        } => {
            let effective_frequency = (*every_seconds).max(min_task_interval_seconds.max(1));

            // If there's a start_time, we align with it
            let next = if let Some(st) = start_time {
                if !st.is_empty() {
                    match chrono::NaiveTime::parse_from_str(st, "%H:%M") {
                        Ok(st_time) => {
                            let now_local = now.with_timezone(&chrono::Local);
                            let now_naive = now_local.naive_local();
                            let mut candidate_local = now_local.date_naive().and_time(st_time);

                            // Calculate slots from start_time
                            while candidate_local <= now_naive {
                                candidate_local +=
                                    chrono::Duration::seconds(effective_frequency as i64);
                            }

                            chrono::Local
                                .from_local_datetime(&candidate_local)
                                .earliest()
                                .or_else(|| {
                                    chrono::Local.from_local_datetime(&candidate_local).latest()
                                })
                                .map(|dt: DateTime<chrono::Local>| dt.with_timezone(&Utc))
                                .unwrap_or(
                                    now + chrono::TimeDelta::seconds(effective_frequency as i64),
                                )
                        }
                        Err(_) => now + chrono::TimeDelta::seconds(effective_frequency as i64),
                    }
                } else {
                    now + chrono::TimeDelta::seconds(effective_frequency as i64)
                }
            } else {
                now + chrono::TimeDelta::seconds(effective_frequency as i64)
            };

            let next = if let Some(wh) = working_hours {
                crate::runner::config::next_working_time(wh, next)
            } else {
                next
            };

            *every_seconds = effective_frequency;
            *next_run_at = next.to_rfc3339();
        }
        TaskSchedule::DailyTimes {
            times,
            next_run_at,
            working_hours,
            ..
        } => match next_daily_run_after(times, now, working_hours.as_ref()) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
        TaskSchedule::Weekly {
            next_run_at,
            day_of_week,
            at_time,
            working_hours,
            ..
        } => match next_weekly_run_after(day_of_week, at_time, now, working_hours.as_ref()) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
        TaskSchedule::Monthly {
            next_run_at,
            day_of_month,
            at_time,
            working_hours,
            ..
        } => match next_monthly_run_after(*day_of_month, at_time, now, working_hours.as_ref()) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
    }
}

pub fn schedule_is_due(schedule: &TaskSchedule, now: DateTime<Utc>) -> bool {
    if !schedule.enabled() {
        return false;
    }

    match schedule {
        TaskSchedule::Once { next_run_at, .. } => {
            if next_run_at.is_empty() {
                // Run immediately
                true
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(scheduled_time) => now >= scheduled_time,
                    Err(_) => false,
                }
            }
        }
        TaskSchedule::Interval {
            next_run_at,
            working_hours,
            start_time,
            ..
        } => {
            let is_due = if next_run_at.is_empty() {
                true
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            };

            if is_due {
                if let Some(wh) = working_hours {
                    if !crate::runner::config::is_within_working_hours(wh, now) {
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
        TaskSchedule::DailyTimes {
            next_run_at,
            working_hours,
            ..
        } => {
            let is_due = if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            };

            if is_due {
                if let Some(wh) = working_hours {
                    crate::runner::config::is_working_day(wh, now)
                } else {
                    true
                }
            } else {
                false
            }
        }
        TaskSchedule::Weekly { next_run_at, .. } => {
            if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            }
        }
        TaskSchedule::Monthly { next_run_at, .. } => {
            if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            }
        }
    }
}

pub fn update_next_run(task: &mut RunnerTask, now: DateTime<Utc>, min_task_interval_seconds: u64) {
    task.last_run_at = now.to_rfc3339();
    if !task.schedules.is_empty() {
        for schedule in &mut task.schedules {
            if schedule.due_now(now) {
                advance_schedule(schedule, now, min_task_interval_seconds);
            }
        }
        return;
    }

    match task.repetition {
        Repetition::Once => {
            task.enabled = false;
            task.next_run_at = String::new();
        }
        Repetition::Repeat => {
            let effective_frequency = task.frequency_seconds.max(min_task_interval_seconds.max(1));
            let next = now + chrono::TimeDelta::seconds(effective_frequency as i64);
            task.next_run_at = next.to_rfc3339();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::{next_daily_run_after, Repetition};

    #[test]
    fn legacy_repeat_task_is_due_without_next_run() {
        let task = RunnerTask {
            id: "legacy".to_string(),
            name: "Legacy".to_string(),
            enabled: true,
            repetition: Repetition::Repeat,
            frequency_seconds: 60,
            next_run_at: String::new(),
            schedules: Vec::new(),
            steps: Vec::new(),
            post_run_steps: Vec::new(),
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 0,
        };

        assert!(task.due_now(Utc::now()));
    }

    #[test]
    fn daily_local_schedule_gets_future_next_run() {
        let now = Utc::now();
        let next =
            next_daily_run_after(&["00:00".to_string(), "23:59".to_string()], now, None).unwrap();
        let next = parse_rfc3339_utc(&next).unwrap();
        assert!(next > now);
    }
}
