use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::runner::config::{RunnerConfig, RunnerTask};
use crate::runner::engine::dispatcher::mod_handle_command;
use crate::runner::engine::dispatcher::schedule::schedule_is_due;
use crate::runner::engine::state::{
    ExecutionManagerCommand, ExecutionPolicy, RunnerCommand, RunnerHandle, RunnerStatus,
};

use crate::runner::engine::app_lock::AppLockManager;

pub fn spawn_execution_manager(
    status: Arc<Mutex<RunnerStatus>>,
    config_path: String,
    app_lock_manager: AppLockManager,
) -> mpsc::Sender<ExecutionManagerCommand> {
    let (tx, mut rx) = mpsc::channel(128);
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let mut queued_tasks: VecDeque<(Box<RunnerTask>, ExecutionPolicy, DateTime<Utc>)> =
            VecDeque::new();
        let mut running_tasks: Vec<(RunnerTask, tokio::task::JoinHandle<()>)> = Vec::new();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ExecutionManagerCommand::QueueTask {
                    task,
                    policy,
                    scheduled_at,
                } => {
                    let mut st = status.lock().await;
                    let is_queued = queued_tasks.iter().any(|(t, _, _)| t.id == task.id);
                    let is_running = running_tasks.iter().any(|(t, _)| t.id == task.id);

                    if is_queued || is_running {
                        tracing::warn!(
                            "Task '{}' is already running or queued; skipping duplicate launch",
                            task.id
                        );
                    } else {
                        if !st.queued_task_ids.contains(&task.id) {
                            st.queued_task_ids.push(task.id.clone());
                        }
                        queued_tasks.push_back((task, policy, scheduled_at));
                    }
                }
                ExecutionManagerCommand::TaskFinished {
                    task_id,
                    last_status,
                    result,
                } => {
                    if let Some(pos) = running_tasks.iter().position(|(t, _)| t.id == task_id) {
                        running_tasks.remove(pos);
                    }

                    {
                        let mut st = status.lock().await;
                        let was_running = st.running_task_ids.iter().any(|id| id == &task_id);
                        st.running_task_ids.retain(|id| id != &task_id);
                        if was_running && st.running_tasks_count > 0 {
                            st.running_tasks_count -= 1;
                        }
                        st.queued_task_ids.retain(|id| id != &task_id);
                        st.waiting_for_app.remove(&task_id);
                        if let Some(err) = result.error {
                            st.last_error = err;
                        }
                    }

                    let path_str = config_path.clone();
                    tokio::spawn(async move {
                        let _ = crate::runner::engine::dispatcher::helpers::modify_config(
                            &path_str,
                            move |cfg| {
                                if let Some(t) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
                                    t.last_status = last_status;
                                }
                                Ok(())
                            },
                        )
                        .await;
                    });
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
                let (task, _policy, _scheduled_at) = &queued_tasks[i];
                let mut can_run = true;

                if running_tasks.iter().any(|(t, _)| t.id == task.id) {
                    can_run = false;
                }

                if can_run {
                    let (task_to_run_box, policy, scheduled_at) =
                        queued_tasks.remove(i).expect("Queue index out of bounds");
                    let task_to_run = *task_to_run_box;
                    {
                        let mut st = status.lock().await;
                        st.running_tasks_count += 1;
                        if !st.running_task_ids.contains(&task_to_run.id) {
                            st.running_task_ids.push(task_to_run.id.clone());
                        }
                        st.queued_task_ids.retain(|id| id != &task_to_run.id);
                        st.last_task_id = task_to_run.id.clone();
                    }

                    let tx_finish = tx_clone.clone();
                    let st_clone = status.clone();
                    let app_lock_mgr = app_lock_manager.clone();
                    let task_to_run_for_spawn = task_to_run.clone();
                    let handle = tokio::spawn(async move {
                        let mut task_to_run = task_to_run_for_spawn;
                        let task_id = task_to_run.id.clone();

                        let result = crate::runner::engine::pipeline::run_task_inner_at(
                            &mut task_to_run,
                            &policy,
                            &st_clone,
                            &app_lock_mgr,
                            scheduled_at,
                        )
                        .await;

                        let _ = tx_finish
                            .send(ExecutionManagerCommand::TaskFinished {
                                task_id,
                                last_status: task_to_run.last_status.clone(),
                                result,
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
                st.queued_tasks_count = queued_tasks.len() + st.waiting_for_app.len();
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
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
        waiting_for_app: std::collections::HashMap::new(),
    }));

    let status_bg = status.clone();
    let config_path = runner_config_path.clone();

    let app_lock_manager = AppLockManager::new();
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone(), app_lock_manager);
    let _exec_tx_loop = exec_tx.clone();

    let mut last_modified = fs::metadata(&config_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .unwrap_or(SystemTime::now());
    let mut last_cleanup =
        Utc::now() - chrono::Duration::try_hours(24).unwrap_or(chrono::Duration::zero());

    let config_path_loop = config_path.clone();
    let poll_interval = {
        let path_str = config_path.clone();
        let cfg = std::thread::spawn(move || RunnerConfig::load(&path_str))
            .join()
            .unwrap();
        cfg.map(|c| c.poll_interval_seconds.max(5)).unwrap_or(30)
    };

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe_cmd = rx.recv() => {
                    match maybe_cmd {
                        Some(cmd) => {
                            let is_shutdown = matches!(cmd, RunnerCommand::Shutdown);
                            if let Err(e) = mod_handle_command(&config_path_loop, cmd, &status_bg, &_exec_tx_loop).await {
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
                    if let Ok(cfg) = crate::runner::engine::dispatcher::helpers::load_config(&config_path_loop).await {
                        let now = Utc::now();
                        for task in &cfg.tasks {
                            if !task.enabled {
                                continue;
                            }
                            for schedule in &task.schedules {
                                if schedule_is_due(schedule, now) {
                                    match tx_clone.send(RunnerCommand::RunTaskNow {
                                        task_id: task.id.clone(),
                                        is_manual: false,
                                    }).await {
                                        Ok(()) => {
                                            info!("Cron schedule triggered for task: {}", task.id)
                                        }
                                        Err(error) => error!(
                                            "Failed to dispatch scheduled task {}: {}",
                                            task.id,
                                            error
                                        ),
                                    }
                                    break;
                                }
                            }
                        }
                    }

                    if let Ok(cfg) = crate::runner::engine::dispatcher::helpers::load_config(&config_path_loop).await {
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

                    let current_modified = tokio::fs::metadata(&config_path_loop)
                        .await
                        .ok()
                        .and_then(|metadata| metadata.modified().ok());
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

#[cfg(test)]
mod tests_queue {
    use super::*;
    use crate::runner::config::{Repetition, TaskSchedule};
    use crate::runner::engine::dispatcher::run_task_by_id;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_runner_queue_capacity() {
        let (tx, _rx) = mpsc::channel::<RunnerCommand>(64);

        for i in 0..64 {
            let res = tx.try_send(RunnerCommand::RunTaskNow {
                task_id: format!("task_{}", i),
                is_manual: false,
            });
            assert!(res.is_ok(), "Should send up to capacity");
        }

        let res = tx.try_send(RunnerCommand::RunAllNow { is_manual: false });
        assert!(res.is_err(), "Should fail when capacity reached");

        let (exec_tx, _exec_rx) = mpsc::channel::<ExecutionManagerCommand>(128);
        for i in 0..128 {
            let res = exec_tx.try_send(ExecutionManagerCommand::TaskFinished {
                task_id: format!("task_{}", i),
                last_status: "success".into(),
                result: crate::runner::engine::state::TaskExecutionResult {
                    success: true,
                    error: None,
                },
            });
            assert!(res.is_ok(), "Should send up to capacity");
        }
        let res = exec_tx.try_send(ExecutionManagerCommand::TaskFinished {
            task_id: "overflow".into(),
            last_status: "success".into(),
            result: crate::runner::engine::state::TaskExecutionResult {
                success: true,
                error: None,
            },
        });
        assert!(res.is_err(), "Should fail when capacity reached");
    }

    #[tokio::test]
    async fn test_real_duplicate_admission_race() {
        let temp_dir = tempdir().unwrap();
        let config_file = temp_dir.path().join("runner.json");
        let config_path = config_file.to_str().unwrap().to_string();

        let mut cfg = RunnerConfig {
            allow_shell_tasks: true,
            ..RunnerConfig::default()
        };
        let marker = temp_dir
            .path()
            .join(format!(
                "runner_race_execution_{}_{}.txt",
                std::process::id(),
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            ))
            .to_string_lossy()
            .into_owned();
        let task = RunnerTask {
            id: "race_task".to_string(),
            name: "Race Task".to_string(),
            enabled: true,
            schedules: vec![],
            repetition: Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            steps: vec![crate::runner::config::TaskStep {
                name: None,
                mode: crate::runner::config::ExecutionMode::Sequential,
                actions: vec![crate::runner::config::ActionSpec::ShellCommand(
                    crate::runner::config::ShellCommandSpec {
                        command: if cfg!(target_os = "windows") {
                            format!(
                                r#"cmd.exe /c "echo run >> {} & ping -n 3 127.0.0.1 > nul""#,
                                marker
                            )
                        } else {
                            format!("bash -c \"echo run >> {}; sleep 2\"", marker)
                        },
                        continue_on_error: false,
                    },
                )],
            }],
            post_run_steps: vec![],
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 3600,
        };
        cfg.tasks.push(task);
        cfg.save(&config_path).unwrap();

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

        let app_lock_manager = AppLockManager::new();
        let exec_tx =
            spawn_execution_manager(status.clone(), config_path.clone(), app_lock_manager);

        let barrier = Arc::new(tokio::sync::Barrier::new(2));

        let path1 = config_path.clone();
        let status1 = status.clone();
        let exec_tx1 = exec_tx.clone();
        let barrier1 = barrier.clone();

        let h1 = tokio::spawn(async move {
            barrier1.wait().await;
            run_task_by_id(&path1, "race_task", &status1, &exec_tx1, true).await
        });

        let path2 = config_path.clone();
        let status2 = status.clone();
        let exec_tx2 = exec_tx.clone();
        let barrier2 = barrier.clone();

        let h2 = tokio::spawn(async move {
            barrier2.wait().await;
            run_task_by_id(&path2, "race_task", &status2, &exec_tx2, true).await
        });

        let (r1, r2) = tokio::join!(h1, h2);
        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());

        // Keep the first process alive long enough for both concurrent admission
        // requests to reach the execution manager, then verify actual executions.
        tokio::time::sleep(Duration::from_secs(5)).await;

        let marker_path = std::path::PathBuf::from(&marker);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let executions = loop {
            match tokio::fs::read_to_string(&marker_path).await {
                Ok(contents) => break contents,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && tokio::time::Instant::now() < deadline =>
                {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let task_status = RunnerConfig::load(&config_path)
                        .ok()
                        .and_then(|cfg| {
                            cfg.tasks
                                .into_iter()
                                .find(|task| task.id == "race_task")
                                .map(|task| task.last_status)
                        })
                        .unwrap_or_else(|| "task status unavailable".to_string());
                    panic!(
                        "task did not write its execution marker; last task status: {task_status}"
                    );
                }
                Err(error) => panic!("failed to read task execution marker: {error}"),
            }
        };
        let _ = tokio::fs::remove_file(&marker_path).await;

        assert_eq!(
            executions.lines().count(),
            1,
            "Concurrent requests for the same task must result in exactly one execution"
        );
    }

    #[tokio::test]
    async fn test_scheduler_executes_task_scheduled_ten_seconds_ahead() {
        let temp_dir = tempdir().unwrap();
        let config_file = temp_dir.path().join("runner.json");
        let config_path = config_file.to_str().unwrap().to_string();
        let marker_path = temp_dir.path().join("scheduled-task-ran.txt");
        let marker = marker_path.to_string_lossy().into_owned();

        let task = RunnerTask {
            id: "scheduled_10s_task".to_string(),
            name: "Scheduled 10-second task".to_string(),
            enabled: true,
            schedules: vec![TaskSchedule::Once {
                enabled: true,
                next_run_at: (Utc::now() + chrono::Duration::seconds(10)).to_rfc3339(),
            }],
            repetition: Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            steps: vec![crate::runner::config::TaskStep {
                name: None,
                mode: crate::runner::config::ExecutionMode::Sequential,
                actions: vec![crate::runner::config::ActionSpec::ShellCommand(
                    crate::runner::config::ShellCommandSpec {
                        command: if cfg!(target_os = "windows") {
                            format!(
                                r#"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -NonInteractive -Command "Set-Content -LiteralPath '{}' -Value ran""#,
                                marker.replace('\'', "''")
                            )
                        } else {
                            format!(
                                "bash -c \"printf ran > '{}'\"",
                                marker.replace('\'', "'\\''")
                            )
                        },
                        continue_on_error: false,
                    },
                )],
            }],
            post_run_steps: vec![],
            last_run_at: String::new(),
            last_status: String::new(),
            timeout_seconds: 60,
        };

        let cfg = RunnerConfig {
            allow_shell_tasks: true,
            poll_interval_seconds: 5,
            tasks: vec![task],
            ..RunnerConfig::default()
        };
        cfg.save(&config_path).unwrap();

        let handle = start_scheduler(config_path);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(35);
        let marker_contents = loop {
            match tokio::fs::read_to_string(&marker_path).await {
                Ok(contents) => break Some(contents),
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && tokio::time::Instant::now() < deadline =>
                {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break None,
                Err(error) => panic!("failed to read scheduled-task marker: {error}"),
            }
        };

        // Allow the task's completion bookkeeping to finish before shutting down.
        if marker_contents.is_some() {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let _ = handle.command_tx.send(RunnerCommand::Shutdown).await;

        let contents = marker_contents.unwrap_or_else(|| {
            panic!(
                "scheduler did not execute the task scheduled 10 seconds ahead within 35 seconds"
            )
        });
        assert_eq!(contents.trim(), "ran");
    }
}
