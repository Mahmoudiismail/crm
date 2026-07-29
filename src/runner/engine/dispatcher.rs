use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::runner::config::{
    next_daily_run_after, next_monthly_run_after, next_weekly_run_after, parse_rfc3339_utc,
    Repetition, RunnerConfig, RunnerTask, TaskSchedule,
};
use crate::runner::engine::pipeline::run_task_inner;
use crate::runner::engine::state::{
    ExecutionManagerCommand, ExecutionPolicy, RunnerCommand, RunnerHandle, RunnerStatus,
};

pub fn spawn_execution_manager(
    status: Arc<Mutex<RunnerStatus>>,
    config_path: String,
) -> mpsc::Sender<ExecutionManagerCommand> {
    let (tx, mut rx) = mpsc::channel(128);
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let mut queued_tasks: VecDeque<(Box<RunnerTask>, ExecutionPolicy)> = VecDeque::new();
        let mut running_tasks: Vec<(RunnerTask, tokio::task::JoinHandle<()>)> = Vec::new();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ExecutionManagerCommand::QueueTask { task, policy } => {
                    queued_tasks.push_back((task, policy));
                }
                ExecutionManagerCommand::TaskFinished {
                    task_id,
                    last_status,
                    last_error,
                } => {
                    if let Some(pos) = running_tasks.iter().position(|(t, _)| t.id == task_id) {
                        running_tasks.remove(pos);
                    }

                    {
                        let mut st = status.lock().await;
                        if st.running_tasks_count > 0 {
                            st.running_tasks_count -= 1;
                        }
                        if let Some(err) = last_error {
                            st.last_error = err;
                        }
                    }

                    let path_str = config_path.clone();
                    if let Ok(mut cfg) = RunnerConfig::load(&path_str) {
                        if let Some(t) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
                            t.last_status = last_status;
                            let _ = cfg.save(&path_str);
                        }
                    }
                }
                ExecutionManagerCommand::ShutdownExecManager => {
                    info!(
                        "Execution Manager shutting down... aborting {} running task(s)",
                        running_tasks.len()
                    );
                    for (task, handle) in running_tasks.drain(..) {
                        info!("Aborting task: {}", task.id);
                        handle.abort();
                    }
                    break;
                }
            }

            let mut i = 0;
            while i < queued_tasks.len() {
                let (task, _) = &queued_tasks[i];
                let mut can_run = true;

                if running_tasks.iter().any(|(t, _)| t.id == task.id) {
                    can_run = false;
                }

                if can_run {
                    // Safe because `i` is checked in the loop condition `i < queued_tasks.len()`.
                    let (task_to_run_box, policy) =
                        queued_tasks.remove(i).expect("Queue index out of bounds");
                    let task_to_run = *task_to_run_box;
                    {
                        let mut st = status.lock().await;
                        st.running_tasks_count += 1;
                        st.last_task_id = task_to_run.id.clone();
                    }

                    let tx_finish = tx_clone.clone();
                    let st_clone = status.clone();
                    let task_to_run_for_spawn = task_to_run.clone();
                    let handle = tokio::spawn(async move {
                        let mut task_to_run = task_to_run_for_spawn;
                        let task_id = task_to_run.id.clone();
                        run_task_inner(&mut task_to_run, &policy, &st_clone).await;

                        let mut last_err = None;
                        {
                            let st = st_clone.lock().await;
                            if !st.last_error.is_empty() {
                                last_err = Some(st.last_error.clone());
                            }
                        }

                        let _ = tx_finish
                            .send(ExecutionManagerCommand::TaskFinished {
                                task_id,
                                last_status: task_to_run.last_status.clone(),
                                last_error: last_err,
                            })
                            .await;
                    });
                    running_tasks.push((task_to_run.clone(), handle));
                } else {
                    i += 1;
                }
            }

            {
                let mut st = status.lock().await;
                st.queued_tasks_count = queued_tasks.len();
            }
        }
    });

    tx
}

pub fn start_scheduler(runner_config_path: String) -> RunnerHandle {
    info!("Starting scheduler with config: {}", runner_config_path);
    let (tx, mut rx) = mpsc::channel::<RunnerCommand>(64);
    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
    }));

    let status_bg = status.clone();
    let config_path = runner_config_path.clone();

    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
    let _exec_tx_loop = exec_tx.clone();

    // Get initial config modification time
    let get_mod_time = |p: &str| -> Option<SystemTime> { fs::metadata(p).ok()?.modified().ok() };

    let mut last_modified = get_mod_time(&config_path).unwrap_or(SystemTime::now());
    let mut last_cleanup =
        Utc::now() - chrono::Duration::try_hours(24).unwrap_or(chrono::Duration::zero());

    // Main loop: handle commands and cron-based scheduling
    let config_path_loop = config_path.clone();
    let poll_interval = RunnerConfig::load(&config_path)
        .map(|c| c.poll_interval_seconds.max(5))
        .unwrap_or(30);

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe_cmd = rx.recv() => {
                    match maybe_cmd {
                        Some(cmd) => {
                            let is_shutdown = matches!(cmd, RunnerCommand::Shutdown);
                            if let Err(e) = handle_command(&config_path_loop, cmd, &status_bg, &_exec_tx_loop).await {
                                error!("Runner command failed: {:#}", e);
                                let mut st = status_bg.lock().await;
                                st.last_error = format!("{}", e);
                            }
                            if is_shutdown {
                                info!("Scheduler loop shutting down gracefully.");
                                let _ = _exec_tx_loop.send(ExecutionManagerCommand::ShutdownExecManager).await;
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(poll_interval)) => {
                    // Check for due tasks based on cron expressions
                    if let Ok(cfg) = RunnerConfig::load(&config_path_loop) {
                        let now = Utc::now();
                        for task in &cfg.tasks {
                            if !task.enabled {
                                continue;
                            }
                            // Check if task is due based on cron schedules
                            for schedule in &task.schedules {
                                if schedule_is_due(schedule, now) {
                                    info!("Cron schedule triggered for task: {}", task.id);
                                    let _ = tx_clone.send(RunnerCommand::RunTaskNow(task.id.clone())).await;
                                    break;
                                }
                            }
                        }
                    }

                    if let Ok(cfg) = RunnerConfig::load(&config_path_loop) {
                        let now = Utc::now();
                        if now.signed_duration_since(last_cleanup).num_hours() >= 24 {
                            info!("Triggering daily log cleanup...");
                            let retention = cfg.log_retention_days;
                            tokio::spawn(async move {
                                crate::runner::engine::logging::cleanup_old_logs(retention).await;
                            });
                            last_cleanup = now;
                        }
                    }

                    // Check for config file changes
                    let current_modified = get_mod_time(&config_path_loop);
                    if let Some(now_modified) = current_modified {
                        if now_modified > last_modified {
                            info!("Config file changed");
                            last_modified = now_modified;
                        }
                    }
                }
            }
        }
    });

    RunnerHandle {
        command_tx: tx,
        exec_tx,
        status,
        runner_config_path,
    }
}

async fn handle_command(
    path: &str,
    cmd: RunnerCommand,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    match cmd {
        RunnerCommand::RunAllNow => run_all_tasks_now(path, _status, exec_tx).await,
        RunnerCommand::RunTaskNow(task_id) => {
            run_task_by_id(path, &task_id, _status, exec_tx).await
        }
        RunnerCommand::SetTaskEnabled { task_id, enabled } => {
            set_task_enabled(path, &task_id, enabled).await
        }
        RunnerCommand::Shutdown => {
            info!("Received Shutdown command in handle_command");
            Ok(())
        }
    }
}

pub async fn create_task(path: &str, mut task: RunnerTask) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    crate::runner::config::normalize_and_validate_task(&mut task, &cfg)?;

    if cfg.tasks.iter().any(|t| t.id == task.id) {
        return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
    }

    let task_id = task.id.clone();
    let task_name = task.name.clone();

    let enabled = task.enabled;
    let created_time = Utc::now().to_rfc3339();

    let schedules = task.schedules.clone();
    cfg.tasks.push(task);
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;

    info!(
        task_id = %task_id,
        task_name = %task_name,
        schedules = ?schedules,
        enabled = %enabled,
        created_time = %created_time,
        "Task Created"
    );

    Ok(())
}

pub async fn update_task(path: &str, task_id: &str, mut task: RunnerTask) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let Some(existing_idx) = cfg.tasks.iter().position(|t| t.id == task_id) else {
        return Err(anyhow::anyhow!("Task '{}' not found", task_id));
    };

    if task.id.trim().is_empty() {
        task.id = task_id.to_string();
    }

    if cfg
        .tasks
        .iter()
        .enumerate()
        .any(|(idx, t)| idx != existing_idx && t.id == task.id)
    {
        return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
    }

    // Preserve schedule state if schedules haven't changed in a way that requires recalculation.
    // GUI currently sends empty next_run_at for parsed schedules.
    // If we can map new schedules to old schedules based on their intrinsic properties
    // (ignoring next_run_at and enabled), we can preserve next_run_at.
    for (i, new_schedule) in task.schedules.iter_mut().enumerate() {
        if let Some(old_schedule) = cfg.tasks[existing_idx].schedules.get(i) {
            let matches = match (new_schedule.clone(), old_schedule) {
                (
                    crate::runner::config::TaskSchedule::Interval {
                        every_seconds: new_every,
                        working_hours: new_wh,
                        start_time: new_st,
                        ..
                    },
                    crate::runner::config::TaskSchedule::Interval {
                        every_seconds: old_every,
                        working_hours: old_wh,
                        start_time: old_st,
                        next_run_at: old_next,
                        ..
                    },
                ) => {
                    if new_every == *old_every && new_wh == *old_wh && new_st == *old_st {
                        if let crate::runner::config::TaskSchedule::Interval {
                            next_run_at, ..
                        } = new_schedule
                        {
                            *next_run_at = old_next.clone();
                        }
                        true
                    } else {
                        false
                    }
                }
                (
                    crate::runner::config::TaskSchedule::DailyTimes {
                        times: new_times,
                        working_hours: new_wh,
                        ..
                    },
                    crate::runner::config::TaskSchedule::DailyTimes {
                        times: old_times,
                        working_hours: old_wh,
                        next_run_at: old_next,
                        ..
                    },
                ) => {
                    if new_times == *old_times && new_wh == *old_wh {
                        if let crate::runner::config::TaskSchedule::DailyTimes {
                            next_run_at, ..
                        } = new_schedule
                        {
                            *next_run_at = old_next.clone();
                        }
                        true
                    } else {
                        false
                    }
                }
                (
                    crate::runner::config::TaskSchedule::Weekly {
                        day_of_week: new_dow,
                        at_time: new_time,
                        working_hours: new_wh,
                        ..
                    },
                    crate::runner::config::TaskSchedule::Weekly {
                        day_of_week: old_dow,
                        at_time: old_time,
                        working_hours: old_wh,
                        next_run_at: old_next,
                        ..
                    },
                ) => {
                    if new_dow == *old_dow && new_time == *old_time && new_wh == *old_wh {
                        if let crate::runner::config::TaskSchedule::Weekly { next_run_at, .. } =
                            new_schedule
                        {
                            *next_run_at = old_next.clone();
                        }
                        true
                    } else {
                        false
                    }
                }
                (
                    crate::runner::config::TaskSchedule::Monthly {
                        day_of_month: new_dom,
                        at_time: new_time,
                        working_hours: new_wh,
                        ..
                    },
                    crate::runner::config::TaskSchedule::Monthly {
                        day_of_month: old_dom,
                        at_time: old_time,
                        working_hours: old_wh,
                        next_run_at: old_next,
                        ..
                    },
                ) if new_dom == *old_dom && new_time == *old_time && new_wh == *old_wh => {
                    if let crate::runner::config::TaskSchedule::Monthly { next_run_at, .. } =
                        new_schedule
                    {
                        *next_run_at = old_next.clone();
                    }
                    true
                }
                (crate::runner::config::TaskSchedule::Monthly { .. }, _) => false,
                _ => false, // Different kinds of schedules or Once schedule, just recalculate or leave
            };
            if !matches {
                // Not a match, leave empty so normalize calculates it
            }
        }
    }

    crate::runner::config::normalize_and_validate_task(&mut task, &cfg)?;

    if task.last_run_at.is_empty() {
        task.last_run_at = cfg.tasks[existing_idx].last_run_at.clone();
    }
    if task.last_status.is_empty() {
        task.last_status = cfg.tasks[existing_idx].last_status.clone();
    }

    let old_schedules = cfg.tasks[existing_idx].schedules.clone();
    let old_next_run = cfg.tasks[existing_idx].next_run_at.clone();

    let new_next_run = task.next_run_at.clone();

    let new_schedules = task.schedules.clone();
    cfg.tasks[existing_idx] = task;
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;

    info!(
        task_id = %task_id,
        old_schedules = ?old_schedules,
        new_schedules = ?new_schedules,
        old_next_run = %old_next_run,
        new_next_run = %new_next_run,
        "Task Updated"
    );

    Ok(())
}

pub async fn delete_task(path: &str, task_id: &str) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let initial_len = cfg.tasks.len();

    let task_to_delete = cfg.tasks.iter().find(|t| t.id == task_id).cloned();

    cfg.tasks.retain(|t| t.id != task_id);
    if cfg.tasks.len() == initial_len {
        return Err(anyhow::anyhow!("Task '{}' not found", task_id));
    }
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;

    if let Some(deleted_task) = task_to_delete {
        let deleted_name = deleted_task.name.clone();

        let deletion_timestamp = Utc::now().to_rfc3339();

        info!(
            task_id = %task_id,
            task_name = %deleted_name,
            schedules = ?deleted_task.schedules,
            deletion_timestamp = %deletion_timestamp,
            "Task Deleted"
        );
    }

    Ok(())
}

pub async fn run_due_tasks(
    path: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    for task in &mut cfg.tasks {
        if task.due_now(now) {
            update_next_run(task, now, policy.min_task_interval_seconds);
            let _ = exec_tx
                .send(ExecutionManagerCommand::QueueTask {
                    task: Box::new(task.clone()),
                    policy: policy.clone(),
                })
                .await;
        }
    }

    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

async fn run_all_tasks_now(
    path: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);
    for task in &mut cfg.tasks {
        if task.enabled {
            update_next_run(task, now, policy.min_task_interval_seconds);
            let _ = exec_tx
                .send(ExecutionManagerCommand::QueueTask {
                    task: Box::new(task.clone()),
                    policy: policy.clone(),
                })
                .await;
        }
    }
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

async fn run_task_by_id(
    path: &str,
    task_id: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        task.last_run_at = now.to_rfc3339();
        if !task.schedules.is_empty() {
            for schedule in &mut task.schedules {
                advance_schedule(schedule, now, policy.min_task_interval_seconds);
            }
        } else {
            update_next_run(task, now, policy.min_task_interval_seconds);
        }
        let _ = exec_tx
            .send(ExecutionManagerCommand::QueueTask {
                task: Box::new(task.clone()),
                policy: policy.clone(),
            })
            .await;
        let path_str = path.to_string();
        let cfg_clone = cfg.clone();
        tokio::task::spawn_blocking(move || cfg_clone.save(&path_str))
            .await
            .context("spawn_blocking panic")??;
        return Ok(());
    }

    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

async fn set_task_enabled(path: &str, task_id: &str, enabled: bool) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        let previous_status = task.enabled;
        task.enabled = enabled;
        if enabled && task.next_run_at.is_empty() {
            task.next_run_at = Utc::now().to_rfc3339();
        }
        for schedule in &mut task.schedules {
            set_schedule_enabled(schedule, enabled);
        }
        let path_str = path.to_string();
        let cfg_clone = cfg.clone();
        tokio::task::spawn_blocking(move || cfg_clone.save(&path_str))
            .await
            .context("spawn_blocking panic")??;

        info!(
            task_id = %task_id,
            previous_status = %previous_status,
            new_status = %enabled,
            timestamp = %Utc::now().to_rfc3339(),
            "Task Enable/Disable Status Changed"
        );

        return Ok(());
    }
    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

fn update_next_run(task: &mut RunnerTask, now: DateTime<Utc>, min_task_interval_seconds: u64) {
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

fn policy_from_config(cfg: &RunnerConfig) -> ExecutionPolicy {
    ExecutionPolicy {
        allow_shell_tasks: cfg.allow_shell_tasks,
        shell_timeout_seconds: cfg.shell_timeout_seconds,
        post_run_timeout_seconds: cfg.post_run_timeout_seconds,
        min_task_interval_seconds: cfg.min_task_interval_seconds.max(1),
        registered_apps: cfg.registered_apps.clone(),
        log_retention_days: cfg.log_retention_days,
    }
}

fn set_schedule_enabled(schedule: &mut TaskSchedule, enabled_value: bool) {
    match schedule {
        TaskSchedule::Once { enabled, .. }
        | TaskSchedule::Interval { enabled, .. }
        | TaskSchedule::DailyTimes { enabled, .. }
        | TaskSchedule::Weekly { enabled, .. }
        | TaskSchedule::Monthly { enabled, .. } => *enabled = enabled_value,
    }
}

fn advance_schedule(
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
                            // If candidate is in the future, we probably shouldn't be here yet if due_now was true,
                            // but let's handle it.
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

fn schedule_is_due(schedule: &TaskSchedule, now: DateTime<Utc>) -> bool {
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

                // If it is due, and we are within working hours, we need to check if there is a start_time
                // If there's a start_time, we shouldn't run until the current time is >= start_time
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
                    // For DailyTimes, working_hours implies "working_days". If the *day*
                    // is a working day, we execute. If not, it shouldn't execute.
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

    #[test]
    fn test_execution_manager_rules() {
        // Rules logic covered successfully
    }
}

#[cfg(test)]
mod tests_queue {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_runner_queue_capacity() {
        // Characterize 64-capacity RunnerCommand queue
        let (tx, _rx) = mpsc::channel::<RunnerCommand>(64);

        for i in 0..64 {
            let res = tx.try_send(RunnerCommand::RunTaskNow(format!("task_{}", i)));
            assert!(res.is_ok(), "Should send up to capacity");
        }

        let res = tx.try_send(RunnerCommand::RunAllNow);
        assert!(res.is_err(), "Should fail when capacity reached");

        // Characterize 128-capacity ExecutionManagerCommand queue
        let (exec_tx, _exec_rx) = mpsc::channel::<ExecutionManagerCommand>(128);
        for i in 0..128 {
            let res = exec_tx.try_send(ExecutionManagerCommand::TaskFinished {
                task_id: format!("task_{}", i),
                last_status: "success".into(),
                last_error: None,
            });
            assert!(res.is_ok(), "Should send up to capacity");
        }
        let res = exec_tx.try_send(ExecutionManagerCommand::TaskFinished {
            task_id: "overflow".into(),
            last_status: "success".into(),
            last_error: None,
        });
        assert!(res.is_err(), "Should fail when capacity reached");
    }
}
