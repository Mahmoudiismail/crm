use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::runner::config::RunnerTask;
use crate::runner::engine::dispatcher::helpers::{load_config, save_config};
use crate::runner::engine::dispatcher::schedule::{
    advance_schedule, policy_from_config, set_schedule_enabled, update_next_run,
};
use crate::runner::engine::state::{ExecutionManagerCommand, RunnerStatus};

pub async fn run_due_tasks(
    path: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let mut cfg = load_config(path).await?;
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

    save_config(cfg, path).await?;
    Ok(())
}

pub(crate) async fn run_all_tasks_now(
    path: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
    is_manual: bool,
) -> Result<()> {
    let mut cfg = load_config(path).await?;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);
    for task in &mut cfg.tasks {
        if task.enabled {
            task.last_run_at = now.to_rfc3339();
            if !is_manual {
                update_next_run(task, now, policy.min_task_interval_seconds);
            }
            let _ = exec_tx
                .send(ExecutionManagerCommand::QueueTask {
                    task: Box::new(task.clone()),
                    policy: policy.clone(),
                })
                .await;
        }
    }
    save_config(cfg, path).await?;
    Ok(())
}

pub(crate) async fn run_task_by_id(
    path: &str,
    task_id: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
    is_manual: bool,
) -> Result<()> {
    let mut cfg = load_config(path).await?;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        task.last_run_at = now.to_rfc3339();

        if !is_manual {
            if !task.schedules.is_empty() {
                for schedule in &mut task.schedules {
                    if schedule.due_now(now) {
                        advance_schedule(schedule, now, policy.min_task_interval_seconds);
                    }
                }
            } else {
                update_next_run(task, now, policy.min_task_interval_seconds);
            }
        }

        let _ = exec_tx
            .send(ExecutionManagerCommand::QueueTask {
                task: Box::new(task.clone()),
                policy: policy.clone(),
            })
            .await;

        save_config(cfg, path).await?;
        return Ok(());
    }

    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

pub(crate) async fn set_task_enabled(path: &str, task_id: &str, enabled: bool) -> Result<()> {
    let mut cfg = load_config(path).await?;
    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        let previous_status = task.enabled;
        task.enabled = enabled;
        if enabled && task.next_run_at.is_empty() {
            task.next_run_at = Utc::now().to_rfc3339();
        }
        for schedule in &mut task.schedules {
            set_schedule_enabled(schedule, enabled);
        }

        save_config(cfg, path).await?;

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

pub async fn create_task(path: &str, mut task: RunnerTask) -> Result<()> {
    let mut cfg = load_config(path).await?;
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
    save_config(cfg, path).await?;

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
    let mut cfg = load_config(path).await?;
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
                _ => false,
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
    save_config(cfg, path).await?;

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
    let mut cfg = load_config(path).await?;
    let initial_len = cfg.tasks.len();

    let task_to_delete = cfg.tasks.iter().find(|t| t.id == task_id).cloned();

    cfg.tasks.retain(|t| t.id != task_id);
    if cfg.tasks.len() == initial_len {
        return Err(anyhow::anyhow!("Task '{}' not found", task_id));
    }
    save_config(cfg, path).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::{RunnerConfig, RunnerTask, TaskSchedule};
    use crate::runner::engine::state::{ExecutionManagerCommand, RunnerStatus};
    use chrono::{TimeDelta, Utc};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_multiple_schedules_only_advance_due_schedules() {
        let now = Utc::now();
        let past = now - TimeDelta::seconds(3600); // 1 hour ago
        let future = now + TimeDelta::seconds(3600); // 1 hour in future

        let schedule_due = TaskSchedule::Once {
            enabled: true,
            next_run_at: past.to_rfc3339(),
        };

        let schedule_not_due = TaskSchedule::Once {
            enabled: true,
            next_run_at: future.to_rfc3339(),
        };

        let task = RunnerTask {
            id: "task_multi_schedule".to_string(),
            name: "Test multiple schedules".to_string(),
            enabled: true,
            schedules: vec![schedule_due.clone(), schedule_not_due.clone()],
            repetition: crate::runner::config::Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            steps: vec![],
            post_run_steps: vec![],
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 3600,
        };

        let mut cfg = RunnerConfig::default();
        cfg.tasks.push(task.clone());

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();

        save_config(cfg, path_str).await.unwrap();

        let status = Arc::new(Mutex::new(RunnerStatus {
            running_tasks_count: 0,
            queued_tasks_count: 0,
            last_task_id: String::new(),
            last_error: String::new(),
            last_run_at: String::new(),
        }));
        let (exec_tx, mut exec_rx) = mpsc::channel(128);

        // Run the task, simulating the scheduler loop discovering a schedule is due
        run_task_by_id(path_str, "task_multi_schedule", &status, &exec_tx, false)
            .await
            .unwrap();

        // Verify task was queued
        let queued = exec_rx.recv().await.expect("Task should be queued");
        match queued {
            ExecutionManagerCommand::QueueTask { task, .. } => {
                assert_eq!(task.id, "task_multi_schedule");
            }
            _ => panic!("Expected QueueTask command"),
        }

        // Verify the config on disk
        let cfg = load_config(path_str).await.unwrap();
        let updated_task = cfg.tasks.first().unwrap();

        let updated_schedule_due = &updated_task.schedules[0];
        let updated_schedule_not_due = &updated_task.schedules[1];

        // Due schedule should be advanced (for Once, it means disabled and next_run_at cleared)
        match updated_schedule_due {
            TaskSchedule::Once {
                enabled,
                next_run_at,
            } => {
                assert!(!enabled, "Due schedule should be disabled");
                assert!(
                    next_run_at.is_empty(),
                    "Due schedule next_run_at should be cleared"
                );
            }
            _ => panic!("Expected Once schedule"),
        }

        // Not due schedule should NOT be changed
        match updated_schedule_not_due {
            TaskSchedule::Once {
                enabled,
                next_run_at,
            } => {
                assert!(*enabled, "Not due schedule should remain enabled");
                assert_eq!(
                    next_run_at,
                    &future.to_rfc3339(),
                    "Not due schedule next_run_at should remain unchanged"
                );
            }
            _ => panic!("Expected Once schedule"),
        }
    }

    #[tokio::test]
    async fn test_multiple_schedules_interval_and_daily() {
        let now = Utc::now();
        let past = now - TimeDelta::seconds(3600); // 1 hour ago
        let future = now + TimeDelta::seconds(3600); // 1 hour in future

        let schedule_interval = TaskSchedule::Interval {
            enabled: true,
            every_seconds: 60,
            next_run_at: past.to_rfc3339(),
            working_hours: None,
            working_hours_profile_id: None,
            start_time: None,
        };

        let schedule_daily = TaskSchedule::DailyTimes {
            enabled: true,
            times: vec!["15:00".to_string()],
            working_hours: None,
            working_hours_profile_id: None,
            next_run_at: future.to_rfc3339(),
        };

        let task = RunnerTask {
            id: "task_multi_schedule_interval_daily".to_string(),
            name: "Test interval and daily".to_string(),
            enabled: true,
            schedules: vec![schedule_interval.clone(), schedule_daily.clone()],
            repetition: crate::runner::config::Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            steps: vec![],
            post_run_steps: vec![],
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 3600,
        };

        let mut cfg = RunnerConfig::default();
        cfg.tasks.push(task.clone());

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();

        save_config(cfg, path_str).await.unwrap();

        let status = Arc::new(Mutex::new(RunnerStatus {
            running_tasks_count: 0,
            queued_tasks_count: 0,
            last_task_id: String::new(),
            last_error: String::new(),
            last_run_at: String::new(),
        }));
        let (exec_tx, _exec_rx) = mpsc::channel(128);

        run_task_by_id(
            path_str,
            "task_multi_schedule_interval_daily",
            &status,
            &exec_tx,
            false,
        )
        .await
        .unwrap();

        let cfg = load_config(path_str).await.unwrap();
        let updated_task = cfg.tasks.first().unwrap();

        let updated_interval = &updated_task.schedules[0];
        let updated_daily = &updated_task.schedules[1];

        match updated_interval {
            TaskSchedule::Interval { next_run_at, .. } => {
                assert!(
                    next_run_at != &past.to_rfc3339(),
                    "Interval should be advanced"
                );
            }
            _ => panic!("Expected Interval schedule"),
        }

        match updated_daily {
            TaskSchedule::DailyTimes { next_run_at, .. } => {
                assert_eq!(
                    next_run_at,
                    &future.to_rfc3339(),
                    "Daily should remain unchanged"
                );
            }
            _ => panic!("Expected Daily schedule"),
        }
    }
}
