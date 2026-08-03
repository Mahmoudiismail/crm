use anyhow::Context;

use crate::runner::config::models::*;
use crate::runner::config::schedule::*;

impl RunnerConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.gui_port == 0 {
            anyhow::bail!("gui_port cannot be 0");
        }
        if self.poll_interval_seconds == 0 {
            anyhow::bail!("poll_interval_seconds cannot be 0");
        }
        Ok(())
    }
}

pub fn normalize_and_validate_task(
    task: &mut RunnerTask,
    cfg: &RunnerConfig,
) -> anyhow::Result<()> {
    task.id = task.id.trim().to_string();
    task.name = task.name.trim().to_string();
    task.next_run_at = task.next_run_at.trim().to_string();

    if task.id.is_empty() {
        return Err(anyhow::anyhow!("Task id is required"));
    }
    if !crate::runner::engine::helpers::is_valid_task_id(&task.id) {
        return Err(anyhow::anyhow!(
            "Invalid task id '{}'. Use letters, numbers, '-' or '_'",
            task.id
        ));
    }
    if task.name.is_empty() {
        return Err(anyhow::anyhow!("Task name is required"));
    }

    if !task.next_run_at.is_empty() {
        parse_rfc3339_utc(&task.next_run_at).with_context(|| {
            format!(
                "Invalid next_run_at timestamp '{}'. Use RFC3339",
                task.next_run_at
            )
        })?;
    }

    if matches!(task.repetition, Repetition::Repeat) {
        task.frequency_seconds = task
            .frequency_seconds
            .max(cfg.min_task_interval_seconds.max(1));
    }

    normalize_and_validate_schedules(task, cfg.min_task_interval_seconds.max(1))?;

    // Validate steps directly instead of converting to/from legacy kind
    for step in &mut task.steps {
        step.actions.retain(|action| match action {
            ActionSpec::ShellCommand(c) => !c.command.trim().is_empty(),
            ActionSpec::ExternalApp(_) => true,
        });

        for action in &mut step.actions {
            if let ActionSpec::ShellCommand(c) = action {
                c.command = c.command.trim().to_string();
            }
        }

        for action in &step.actions {
            if let ActionSpec::ExternalApp(app) = action {
                if app.app_id.trim().is_empty() {
                    return Err(anyhow::anyhow!("External App tasks require an app_id"));
                }
            }
        }
    }

    if task.steps.iter().all(|s| s.actions.is_empty()) {
        return Err(anyhow::anyhow!(
            "Task requires at least one non-empty action"
        ));
    }

    Ok(())
}

fn normalize_and_validate_schedules(
    task: &mut RunnerTask,
    min_task_interval_seconds: u64,
) -> anyhow::Result<()> {
    for schedule in &mut task.schedules {
        match schedule {
            TaskSchedule::Once { next_run_at, .. } => {
                *next_run_at = next_run_at.trim().to_string();
                if !next_run_at.is_empty() {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!("Invalid once schedule '{}'. Use RFC3339", next_run_at)
                    })?;
                }
            }
            TaskSchedule::Interval {
                every_seconds,
                next_run_at,
                start_time,
                ..
            } => {
                *every_seconds = (*every_seconds).max(min_task_interval_seconds);
                *next_run_at = next_run_at.trim().to_string();
                if let Some(st) = start_time {
                    *st = st.trim().to_string();
                    if !st.is_empty() {
                        chrono::NaiveTime::parse_from_str(st, "%H:%M").with_context(|| {
                            format!("Invalid interval start time '{}'. Use HH:MM", st)
                        })?;
                    }
                }

                // If it doesn't have a next_run_at, we need to compute it.
                // If start_time is set, we find the first valid future slot
                if next_run_at.is_empty() {
                    let now = chrono::Utc::now();
                    if let Some(st) = start_time {
                        if !st.is_empty() {
                            if let Ok(st_time) = chrono::NaiveTime::parse_from_str(st, "%H:%M") {
                                let now_local = now.with_timezone(&chrono::Local);
                                let now_naive = now_local.naive_local();
                                let mut candidate_local = now_local.date_naive().and_time(st_time);

                                while candidate_local <= now_naive {
                                    candidate_local +=
                                        chrono::Duration::seconds(*every_seconds as i64);
                                }

                                use chrono::TimeZone;
                                let next = chrono::Local
                                    .from_local_datetime(&candidate_local)
                                    .earliest()
                                    .or_else(|| {
                                        chrono::Local.from_local_datetime(&candidate_local).latest()
                                    })
                                    .map(|dt: chrono::DateTime<chrono::Local>| {
                                        dt.with_timezone(&chrono::Utc)
                                    })
                                    .unwrap_or(
                                        now + chrono::TimeDelta::seconds(*every_seconds as i64),
                                    );
                                *next_run_at = next.to_rfc3339();
                            } else {
                                let next = now + chrono::TimeDelta::seconds(*every_seconds as i64);
                                *next_run_at = next.to_rfc3339();
                            }
                        } else {
                            let next = now + chrono::TimeDelta::seconds(*every_seconds as i64);
                            *next_run_at = next.to_rfc3339();
                        }
                    } else {
                        let next = now + chrono::TimeDelta::seconds(*every_seconds as i64);
                        *next_run_at = next.to_rfc3339();
                    }
                } else {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!(
                            "Invalid interval next_run_at '{}'. Use RFC3339",
                            next_run_at
                        )
                    })?;
                }
            }
            TaskSchedule::DailyTimes {
                times,
                next_run_at,
                working_hours,
                ..
            } => {
                times.retain(|time| !time.trim().is_empty());
                for time in times.iter_mut() {
                    *time = time.trim().to_string();
                    chrono::NaiveTime::parse_from_str(time, "%H:%M")
                        .with_context(|| format!("Invalid daily time '{}'. Use HH:MM", time))?;
                }
                if times.is_empty() {
                    return Err(anyhow::anyhow!(
                        "daily_times schedule requires at least one HH:MM time"
                    ));
                }
                *next_run_at = next_run_at.trim().to_string();
                if next_run_at.is_empty() {
                    *next_run_at =
                        next_daily_run_after(times, chrono::Utc::now(), working_hours.as_ref())?;
                } else {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!(
                            "Invalid daily_times next_run_at '{}'. Use RFC3339",
                            next_run_at
                        )
                    })?;
                }
            }
            TaskSchedule::Weekly {
                day_of_week,
                at_time,
                next_run_at,
                working_hours,
                ..
            } => {
                *day_of_week = day_of_week.trim().to_string();
                *at_time = at_time.trim().to_string();
                if !at_time.is_empty() {
                    chrono::NaiveTime::parse_from_str(at_time, "%H:%M")
                        .with_context(|| format!("Invalid weekly time '{}'. Use HH:MM", at_time))?;
                }
                *next_run_at = next_run_at.trim().to_string();
                if next_run_at.is_empty() {
                    *next_run_at = next_weekly_run_after(
                        day_of_week,
                        at_time,
                        chrono::Utc::now(),
                        working_hours.as_ref(),
                    )?;
                } else {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!("Invalid weekly next_run_at '{}'. Use RFC3339", next_run_at)
                    })?;
                }
            }
            TaskSchedule::Monthly {
                day_of_month,
                at_time,
                next_run_at,
                working_hours,
                ..
            } => {
                *day_of_month = (*day_of_month).clamp(1, 31);
                *at_time = at_time.trim().to_string();
                if !at_time.is_empty() {
                    chrono::NaiveTime::parse_from_str(at_time, "%H:%M").with_context(|| {
                        format!("Invalid monthly time '{}'. Use HH:MM", at_time)
                    })?;
                }
                // Compute next_run_at
                *next_run_at = next_monthly_run_after(
                    *day_of_month,
                    at_time,
                    chrono::Utc::now(),
                    working_hours.as_ref(),
                )?;
            }
        }
    }

    Ok(())
}
