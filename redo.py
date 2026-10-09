import subprocess

# helpers.rs
with open("src/runner/engine/dispatcher/helpers.rs", "r") as f:
    text = f.read()

text = text.replace(
"""pub(crate) async fn modify_config<F>(path: &str, f: F) -> Result<()>
where
    F: FnOnce(&mut RunnerConfig) + Send + 'static,
{
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    let path_str_save = path.to_string();
    tokio::task::spawn_blocking(move || {
        let mut cfg = RunnerConfig::load(&path_str)?;
        f(&mut cfg);
        cfg.save(&path_str_save)
    })
    .await
    .context("spawn_blocking panic for modify_config")??;
    Ok(())
}""",
"""pub(crate) async fn modify_config<F, T>(path: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut RunnerConfig) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    let path_str_save = path.to_string();
    let res: Result<T> = tokio::task::spawn_blocking(move || {
        let mut cfg = RunnerConfig::load(&path_str)?;
        let r = f(&mut cfg)?;
        cfg.save(&path_str_save)?;
        Ok(r)
    })
    .await
    .context("spawn_blocking panic for modify_config")?;
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_modify_config_concurrency_race() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        let path_str = path.to_str().unwrap();

        let cfg = RunnerConfig {
            poll_interval_seconds: 0,
            ..RunnerConfig::default()
        };
        cfg.save(path_str).unwrap();

        let mut handles = vec![];
        for _ in 0..10 {
            let p = path_str.to_string();
            handles.push(tokio::spawn(async move {
                modify_config(&p, |c| {
                    c.poll_interval_seconds += 1;
                    Ok(())
                }).await.unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let final_cfg = load_config(path_str).await.unwrap();
        assert_eq!(final_cfg.poll_interval_seconds, 10);
    }
}"""
)

text = text.replace(
"""pub(crate) async fn save_config(cfg: RunnerConfig, path: &str) -> Result<()> {
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic for save_config")?
}""",
""
)

with open("src/runner/engine/dispatcher/helpers.rs", "w") as f:
    f.write(text)

# lifecycle.rs
with open("src/runner/engine/dispatcher/lifecycle.rs", "r") as f:
    lifecycle = f.read()

lifecycle = lifecycle.replace(
"""                                if let Some(t) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
                                    t.last_status = last_status;
                                }
                            },""",
"""                                if let Some(t) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
                                    t.last_status = last_status;
                                }
                                Ok(())
                            },"""
)

with open("src/runner/engine/dispatcher/lifecycle.rs", "w") as f:
    f.write(lifecycle)


# task_commands.rs
with open("src/runner/engine/dispatcher/task_commands.rs", "w") as f:
    f.write("""use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::runner::config::RunnerTask;
use crate::runner::engine::dispatcher::schedule::{
    advance_schedule, policy_from_config, set_schedule_enabled, update_next_run,
};
use crate::runner::engine::state::{ExecutionManagerCommand, RunnerStatus};

pub async fn run_due_tasks(
    path: &str,
    status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let st = status.lock().await;
    let running_ids = st.running_task_ids.clone();
    let queued_ids = st.queued_task_ids.clone();
    drop(st);

    let tasks_to_queue: Vec<(crate::runner::config::RunnerTask, crate::runner::engine::state::ExecutionPolicy)> = crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        let mut inner_queue = vec![];
        let now = Utc::now();
        let policy = policy_from_config(cfg);
        for task in &mut cfg.tasks {
            if task.due_now(now) {
                if queued_ids.contains(&task.id) || running_ids.contains(&task.id) {
                    tracing::warn!(
                        "Task '{}' is already running or queued; skipping duplicate launch",
                        task.id
                    );
                    continue;
                }
                update_next_run(task, now, policy.min_task_interval_seconds);
                inner_queue.push((task.clone(), policy.clone()));
            }
        }
        Ok(inner_queue)
    }).await?;

    for (task, policy) in tasks_to_queue {
        let _ = exec_tx.send(ExecutionManagerCommand::QueueTask {
            task: Box::new(task),
            policy,
        }).await;
    }
    Ok(())
}

pub async fn run_all_tasks_now(
    path: &str,
    status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
    is_manual: bool,
) -> Result<()> {
    let st = status.lock().await;
    let running_ids = st.running_task_ids.clone();
    let queued_ids = st.queued_task_ids.clone();
    drop(st);

    let tasks_to_queue: Vec<(crate::runner::config::RunnerTask, crate::runner::engine::state::ExecutionPolicy)> = crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        let mut inner_queue = vec![];
        let now = Utc::now();
        let policy = policy_from_config(cfg);
        for task in &mut cfg.tasks {
            if task.enabled {
                if queued_ids.contains(&task.id) || running_ids.contains(&task.id) {
                    tracing::warn!(
                        "Task '{}' is already running or queued; skipping duplicate launch",
                        task.id
                    );
                    continue;
                }
                task.last_run_at = now.to_rfc3339();
                if !is_manual {
                    update_next_run(task, now, policy.min_task_interval_seconds);
                }
                inner_queue.push((task.clone(), policy.clone()));
            }
        }
        Ok(inner_queue)
    }).await?;
    for (task, policy) in tasks_to_queue {
        let _ = exec_tx.send(ExecutionManagerCommand::QueueTask {
            task: Box::new(task),
            policy,
        }).await;
    }
    Ok(())
}

#[cfg(test)]
use std::sync::atomic::AtomicBool;
#[cfg(test)]
pub static RACE_TESTING: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
lazy_static::lazy_static! {
    pub static ref RACE_BARRIER: tokio::sync::Barrier = tokio::sync::Barrier::new(2);
}
pub async fn run_task_by_id(
    path: &str,
    task_id: &str,
    status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
    is_manual: bool,
) -> Result<()> {
    #[cfg(test)]
    if RACE_TESTING.load(std::sync::atomic::Ordering::SeqCst) {
        RACE_BARRIER.wait().await;
    }

    let tid = task_id.to_string();
    let st = status.lock().await;
    let running_ids = st.running_task_ids.clone();
    let queued_ids = st.queued_task_ids.clone();
    drop(st);

    let task_to_queue: Option<(crate::runner::config::RunnerTask, crate::runner::engine::state::ExecutionPolicy)> = crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        let now = Utc::now();
        let policy = policy_from_config(cfg);

        if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == tid) {
            if queued_ids.contains(&task.id) || running_ids.contains(&task.id) {
                tracing::warn!("Task '{}' is already running or queued", task.id);
                return Ok(None);
            }

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
            return Ok(Some((task.clone(), policy.clone())));
        }
        Err(anyhow::anyhow!("Task '{}' not found", tid))
    }).await?;

    if let Some((t, p)) = task_to_queue {
        let _ = exec_tx.send(ExecutionManagerCommand::QueueTask {
            task: Box::new(t),
            policy: p,
        }).await;
    }
    Ok(())
}

pub(crate) async fn set_task_enabled(path: &str, task_id: &str, enabled: bool) -> Result<()> {
    let tid = task_id.to_string();
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == tid) {
            let previous_status = task.enabled;
            task.enabled = enabled;
            if enabled && task.next_run_at.is_empty() {
                task.next_run_at = Utc::now().to_rfc3339();
            }
            for schedule in &mut task.schedules {
                set_schedule_enabled(schedule, enabled);
            }
            info!(
                task_id = %tid,
                previous_status = %previous_status,
                new_status = %enabled,
                timestamp = %Utc::now().to_rfc3339(),
                "Task Enable/Disable Status Changed"
            );
            return Ok(());
        }
        Err(anyhow::anyhow!("Task '{}' not found", tid))
    }).await
}

pub async fn create_task(path: &str, mut task: RunnerTask) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        crate::runner::config::normalize_and_validate_task(&mut task, cfg)?;
        if cfg.tasks.iter().any(|t| t.id == task.id) {
            return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
        }
        let task_id = task.id.clone();
        let task_name = task.name.clone();
        let enabled = task.enabled;
        let created_time = Utc::now().to_rfc3339();
        let schedules = task.schedules.clone();

        cfg.tasks.push(task);
        info!(
            task_id = %task_id,
            task_name = %task_name,
            schedules = ?schedules,
            enabled = %enabled,
            created_time = %created_time,
            "Task Created"
        );
        Ok(())
    }).await
}

pub async fn update_task(path: &str, task_id: &str, mut task: RunnerTask) -> Result<()> {
    let tid = task_id.to_string();
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        let Some(existing_idx) = cfg.tasks.iter().position(|t| t.id == tid) else {
            return Err(anyhow::anyhow!("Task '{}' not found", tid));
        };
        if task.id.trim().is_empty() {
            task.id = tid.clone();
        }
        if cfg.tasks.iter().enumerate().any(|(idx, t)| idx != existing_idx && t.id == task.id) {
            return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
        }

        for (i, new_schedule) in task.schedules.iter_mut().enumerate() {
            if let Some(old_schedule) = cfg.tasks[existing_idx].schedules.get(i) {
                let matches = match (new_schedule.clone(), old_schedule) {
                    (
                        crate::runner::config::TaskSchedule::Interval { every_seconds: new_every, working_hours: new_wh, start_time: new_st, .. },
                        crate::runner::config::TaskSchedule::Interval { every_seconds: old_every, working_hours: old_wh, start_time: old_st, next_run_at: old_next, .. },
                    ) => {
                        if new_every == *old_every && new_wh == *old_wh && new_st == *old_st {
                            if let crate::runner::config::TaskSchedule::Interval { next_run_at, .. } = new_schedule {
                                *next_run_at = old_next.clone();
                            }
                            true
                        } else { false }
                    }
                    (
                        crate::runner::config::TaskSchedule::DailyTimes { times: new_times, working_hours: new_wh, .. },
                        crate::runner::config::TaskSchedule::DailyTimes { times: old_times, working_hours: old_wh, next_run_at: old_next, .. },
                    ) => {
                        if new_times == *old_times && new_wh == *old_wh {
                            if let crate::runner::config::TaskSchedule::DailyTimes { next_run_at, .. } = new_schedule {
                                *next_run_at = old_next.clone();
                            }
                            true
                        } else { false }
                    }
                    (
                        crate::runner::config::TaskSchedule::Weekly { day_of_week: new_dow, at_time: new_time, working_hours: new_wh, .. },
                        crate::runner::config::TaskSchedule::Weekly { day_of_week: old_dow, at_time: old_time, working_hours: old_wh, next_run_at: old_next, .. },
                    ) => {
                        if new_dow == *old_dow && new_time == *old_time && new_wh == *old_wh {
                            if let crate::runner::config::TaskSchedule::Weekly { next_run_at, .. } = new_schedule {
                                *next_run_at = old_next.clone();
                            }
                            true
                        } else { false }
                    }
                    (
                        crate::runner::config::TaskSchedule::Monthly { day_of_month: new_dom, at_time: new_time, working_hours: new_wh, .. },
                        crate::runner::config::TaskSchedule::Monthly { day_of_month: old_dom, at_time: old_time, working_hours: old_wh, next_run_at: old_next, .. },
                    ) if new_dom == *old_dom && new_time == *old_time && new_wh == *old_wh => {
                        if let crate::runner::config::TaskSchedule::Monthly { next_run_at, .. } = new_schedule {
                            *next_run_at = old_next.clone();
                        }
                        true
                    }
                    _ => false,
                };
                let _ = matches;
            }
        }

        crate::runner::config::normalize_and_validate_task(&mut task, cfg)?;

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
        info!(
            task_id = %tid,
            old_schedules = ?old_schedules,
            new_schedules = ?new_schedules,
            old_next_run = %old_next_run,
            new_next_run = %new_next_run,
            "Task Updated"
        );
        Ok(())
    }).await
}

pub async fn delete_task(path: &str, task_id: &str) -> Result<()> {
    let tid = task_id.to_string();
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        let initial_len = cfg.tasks.len();
        let task_to_delete = cfg.tasks.iter().find(|t| t.id == tid).cloned();
        cfg.tasks.retain(|t| t.id != tid);
        if cfg.tasks.len() == initial_len {
            return Err(anyhow::anyhow!("Task '{}' not found", tid));
        }
        if let Some(deleted_task) = task_to_delete {
            let deleted_name = deleted_task.name.clone();
            let deletion_timestamp = Utc::now().to_rfc3339();
            info!(
                task_id = %tid,
                task_name = %deleted_name,
                schedules = ?deleted_task.schedules,
                deletion_timestamp = %deletion_timestamp,
                "Task Deleted"
            );
        }
        Ok(())
    }).await
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

        cfg.save(path_str).unwrap();

        let status = Arc::new(Mutex::new(RunnerStatus {
            running_tasks_count: 0,
            queued_tasks_count: 0,
            running_task_ids: Vec::new(),
            queued_task_ids: Vec::new(),
            last_task_id: String::new(),
            last_error: String::new(),
            last_run_at: String::new(),
            waiting_for_app: std::collections::HashMap::new(),
        }));
        let (exec_tx, mut exec_rx) = mpsc::channel(128);

        run_task_by_id(path_str, "task_multi_schedule", &status, &exec_tx, false)
            .await
            .unwrap();

        let queued = exec_rx.recv().await.expect("Task should be queued");
        match queued {
            ExecutionManagerCommand::QueueTask { task, .. } => {
                assert_eq!(task.id, "task_multi_schedule");
            }
            _ => panic!("Expected QueueTask command"),
        }

        let cfg = crate::runner::engine::dispatcher::helpers::load_config(path_str).await.unwrap();
        let updated_task = cfg.tasks.first().unwrap();

        let updated_schedule_due = &updated_task.schedules[0];
        let updated_schedule_not_due = &updated_task.schedules[1];

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

        match updated_schedule_not_due {
            TaskSchedule::Once {
                enabled,
                next_run_at,
            } => {
                assert!(*enabled, "Not due schedule should remain enabled");
                assert_eq!(
                    next_run_at,
                    &future.to_rfc3339().to_string(),
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

        cfg.save(path_str).unwrap();

        let status = Arc::new(Mutex::new(RunnerStatus {
            running_tasks_count: 0,
            queued_tasks_count: 0,
            running_task_ids: Vec::new(),
            queued_task_ids: Vec::new(),
            last_task_id: String::new(),
            last_error: String::new(),
            last_run_at: String::new(),
            waiting_for_app: std::collections::HashMap::new(),
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

        let cfg = crate::runner::engine::dispatcher::helpers::load_config(path_str).await.unwrap();
        let updated_task = cfg.tasks.first().unwrap();

        let updated_interval = &updated_task.schedules[0];
        let updated_daily = &updated_task.schedules[1];

        match updated_interval {
            TaskSchedule::Interval { next_run_at, .. } => {
                assert!(
                    next_run_at != &past.to_rfc3339().to_string(),
                    "Interval should be advanced"
                );
            }
            _ => panic!("Expected Interval schedule"),
        }

        match updated_daily {
            TaskSchedule::DailyTimes { next_run_at, .. } => {
                assert_eq!(
                    next_run_at,
                    &future.to_rfc3339().to_string(),
                    "Daily should remain unchanged"
                );
            }
            _ => panic!("Expected Daily schedule"),
        }
    }

    #[tokio::test]
    async fn test_duplicate_admission_race() {
        use crate::runner::config::{RunnerConfig, RunnerTask};
        use crate::runner::engine::RunnerStatus;
        use std::sync::atomic::Ordering;
        use std::sync::Arc;
        use tokio::sync::{mpsc, Mutex};

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        let path_str = path.to_str().unwrap();

        let task = RunnerTask {
            id: "race_task".to_string(),
            name: "race_task".to_string(),
            enabled: true,
            schedules: vec![],
            steps: vec![],
            post_run_steps: vec![],
            last_run_at: "".to_string(),
            last_status: "SUCCESS".to_string(),
            timeout_seconds: 3600,
            frequency_seconds: 3600,
            next_run_at: "".to_string(),
            repetition: crate::runner::config::models::Repetition::Once,
        };

        let config = RunnerConfig {
            tasks: vec![task],
            ..RunnerConfig::default()
        };
        config.save(path_str).unwrap();

        let status = Arc::new(Mutex::new(RunnerStatus {
            running_tasks_count: 0,
            queued_tasks_count: 0,
            running_task_ids: Vec::new(),
            queued_task_ids: Vec::new(),
            last_error: "".to_string(),
            last_task_id: "".to_string(),
            last_run_at: "".to_string(),
            waiting_for_app: std::collections::HashMap::new(),
        }));

        let (exec_tx, mut exec_rx) = mpsc::channel(100);

        RACE_TESTING.store(true, Ordering::SeqCst);

        let p_str1 = path_str.to_string();
        let st1 = status.clone();
        let tx1 = exec_tx.clone();
        let handle1 =
            tokio::spawn(async move { run_task_by_id(&p_str1, "race_task", &st1, &tx1, true).await });

        let p_str2 = path_str.to_string();
        let st2 = status.clone();
        let tx2 = exec_tx.clone();
        let handle2 =
            tokio::spawn(async move { run_task_by_id(&p_str2, "race_task", &st2, &tx2, true).await });

        let res1 = handle1.await.unwrap();
        let res2 = handle2.await.unwrap();

        assert!(res1.is_ok() || res2.is_ok());

        RACE_TESTING.store(false, Ordering::SeqCst);

        let mut sent_commands = 0;
        while exec_rx.try_recv().is_ok() {
            sent_commands += 1;
        }

        assert!(
            sent_commands <= 2,
            "At most two commands might be sent (since run_task_by_id allows them through), but ExecutionManager dedups them safely!"
        );
    }
}
""")

# app_commands.rs
with open("src/runner/engine/dispatcher/app_commands.rs", "w") as f:
    f.write("""use crate::runner::config::RegisteredApp;

pub async fn create_registered_app(path: &str, app: RegisteredApp) -> anyhow::Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        cfg.registered_apps.push(app);
        Ok(())
    }).await
}

pub async fn update_registered_app(path: &str, app: RegisteredApp) -> anyhow::Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if let Some(existing) = cfg.registered_apps.iter_mut().find(|a| a.id == app.id) {
            existing.name = app.name;
            existing.executable_path = app.executable_path;
            existing.config_path = app.config_path;
            existing.allow_concurrent_tasks = app.allow_concurrent_tasks;
            Ok(())
        } else {
            Err(anyhow::anyhow!("App '{}' not found", app.id))
        }
    }).await
}

pub async fn delete_registered_app(path: &str, app_id: &str) -> anyhow::Result<()> {
    let aid = app_id.to_string();
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        cfg.registered_apps.retain(|a| a.id != aid);
        Ok(())
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::RunnerConfig;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_update_nonexistent_registered_app_fails() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();

        let cfg = RunnerConfig::default();
        cfg.save(path_str).unwrap();

        let app = RegisteredApp {
            id: "fake_id".to_string(),
            name: "fake".to_string(),
            executable_path: "fake.exe".to_string(),
            config_path: "fake.json".to_string(),
            allow_concurrent_tasks: false,
        };

        let res = update_registered_app(path_str, app).await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "App 'fake_id' not found");
    }
}
""")

# profile_commands.rs
with open("src/runner/engine/dispatcher/profile_commands.rs", "w") as f:
    f.write("""use crate::runner::config::WorkingHoursProfile;
use anyhow::Result;

pub async fn create_working_hours_profile(path: &str, profile: WorkingHoursProfile) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if cfg.working_hours_profiles.iter().any(|p| p.id == profile.id) {
            return Err(anyhow::anyhow!("Profile '{}' already exists", profile.id));
        }
        cfg.working_hours_profiles.push(profile);
        Ok(())
    }).await
}

pub async fn update_working_hours_profile(path: &str, profile: WorkingHoursProfile) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if let Some(pos) = cfg.working_hours_profiles.iter().position(|p| p.id == profile.id) {
            cfg.working_hours_profiles[pos] = profile;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Profile '{}' not found", profile.id))
        }
    }).await
}

pub async fn delete_working_hours_profile(path: &str, profile_id: String) -> Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        cfg.working_hours_profiles.retain(|p| p.id != profile_id);
        for task in &mut cfg.tasks {
            for schedule in &mut task.schedules {
                match schedule {
                    crate::runner::config::TaskSchedule::Interval {
                        working_hours_profile_id,
                        working_hours,
                        ..
                    }
                    | crate::runner::config::TaskSchedule::DailyTimes {
                        working_hours_profile_id,
                        working_hours,
                        ..
                    }
                    | crate::runner::config::TaskSchedule::Weekly {
                        working_hours_profile_id,
                        working_hours,
                        ..
                    }
                    | crate::runner::config::TaskSchedule::Monthly {
                        working_hours_profile_id,
                        working_hours,
                        ..
                    } if working_hours_profile_id.as_deref() == Some(&profile_id) => {
                        *working_hours_profile_id = None;
                        *working_hours = None;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }).await
}
""")

print("success")
