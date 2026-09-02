use chrono::Utc;
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::runner::config::{RunnerConfig, RunnerTask};
use crate::runner::engine::dispatcher::mod_handle_command;
use crate::runner::engine::dispatcher::schedule::schedule_is_due;
use crate::runner::engine::pipeline::run_task_inner;
use crate::runner::engine::state::{
    ExecutionManagerCommand, ExecutionPolicy, RunnerCommand, RunnerHandle, RunnerStatus,
}; // We'll rename handle_command in mod.rs to avoid conflict

use crate::runner::engine::app_lock::AppLockManager;

pub fn spawn_execution_manager(
    status: Arc<Mutex<RunnerStatus>>,
    config_path: String,
    app_lock_manager: AppLockManager,
) -> mpsc::Sender<ExecutionManagerCommand> {
    let (tx, mut rx) = mpsc::channel(128);
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let mut queued_tasks: VecDeque<(Box<RunnerTask>, ExecutionPolicy)> = VecDeque::new();
        let mut running_tasks: Vec<(RunnerTask, tokio::task::JoinHandle<()>)> = Vec::new();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ExecutionManagerCommand::QueueTask { task, policy } => {
                    let is_duplicate = queued_tasks.iter().any(|(t, _)| t.id == task.id);
                    if !is_duplicate {
                        let mut st = status.lock().await;
                        if !st.queued_task_ids.contains(&task.id) {
                            st.queued_task_ids.push(task.id.clone());
                        }
                        queued_tasks.push_back((task, policy));
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
                        if st.running_tasks_count > 0 {
                            st.running_tasks_count -= 1;
                        }
                        st.running_task_ids.retain(|id| id != &task_id);
                        st.waiting_for_app.remove(&task_id);
                        if let Some(err) = result.error {
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
                let (task, _policy) = &queued_tasks[i];
                let mut can_run = true;

                if running_tasks.iter().any(|(t, _)| t.id == task.id) {
                    can_run = false;
                }

                if can_run {
                    let (task_to_run_box, policy) =
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

                        let result =
                            run_task_inner(&mut task_to_run, &policy, &st_clone, &app_lock_mgr)
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

    let get_mod_time = |p: &str| -> Option<SystemTime> { fs::metadata(p).ok()?.modified().ok() };

    let mut last_modified = get_mod_time(&config_path).unwrap_or(SystemTime::now());
    let mut last_cleanup =
        Utc::now() - chrono::Duration::try_hours(24).unwrap_or(chrono::Duration::zero());

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
                    if let Ok(cfg) = RunnerConfig::load(&config_path_loop) {
                        let now = Utc::now();
                        for task in &cfg.tasks {
                            if !task.enabled {
                                continue;
                            }
                            for schedule in &task.schedules {
                                if schedule_is_due(schedule, now) {
                                    info!("Cron schedule triggered for task: {}", task.id);
                                    let _ = tx_clone.send(RunnerCommand::RunTaskNow {
                                        task_id: task.id.clone(),
                                        is_manual: false,
                                    }).await;
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

#[cfg(test)]
mod tests_queue {
    use super::*;
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
}
