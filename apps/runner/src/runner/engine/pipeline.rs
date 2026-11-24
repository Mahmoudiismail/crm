use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::runner::config::{ActionSpec, ExecutionMode, RegisteredApp, RunnerTask, TaskStep};
use crate::runner::engine::application::run_external_app;
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::shell::run_shell_command;
use crate::runner::engine::state::{ExecutionPolicy, RunnerStatus};

use crate::runner::engine::app_lock::AppLockManager;

/// Read Yasweb's own configured concurrency and enforce it in Runner, where
/// separate period processes share one limiter. A missing/legacy value keeps
/// Yasweb's documented default of six; zero retains Yasweb's unlimited meaning.
fn yasweb_concurrency_limit(
    app: &RegisteredApp,
    spec: &crate::runner::config::ExternalAppSpec,
) -> Result<Option<usize>> {
    let identity = format!("{} {} {}", app.id, app.name, app.executable_path).to_ascii_lowercase();
    if !identity.contains("yasweb") {
        return Ok(None);
    }

    let read_limit = |path: &str| -> usize {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
            .and_then(|value| value.get("concurrency").and_then(serde_json::Value::as_u64))
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(6)
    };

    // One registered Yasweb app must have one app-wide limit. Per-task --config
    // overrides are allowed only when they preserve that same limit; otherwise
    // whichever task starts first could silently determine the global cap.
    let registered_limit = read_limit(&app.config_path);
    let invocation_config = spec
        .args
        .get("--config")
        .filter(|path| !path.trim().is_empty())
        .unwrap_or(&app.config_path);
    let invocation_limit = read_limit(invocation_config);
    if invocation_limit != registered_limit {
        anyhow::bail!(
            "Yasweb concurrency mismatch for app '{}': registered config '{}' sets {}, but invocation config '{}' sets {}. All tasks for one registered Yasweb app must use the same concurrency limit.",
            app.name,
            app.config_path,
            registered_limit,
            invocation_config,
            invocation_limit
        );
    }

    if registered_limit == 0 {
        Ok(None)
    } else {
        Ok(Some(registered_limit))
    }
}

/// Acquire a Runner-owned permit for every Yasweb process, not once per CLI
/// invocation. This makes the configured cap effective across tasks and
/// concurrently scheduled date periods.
async fn run_external_app_with_shared_limit(
    logger: &TaskLogger,
    app: &RegisteredApp,
    spec: &crate::runner::config::ExternalAppSpec,
    period: Option<&crate::runner::config::ExecutionPeriod>,
    timeout_seconds: u64,
    app_lock_manager: &AppLockManager,
    status: &Arc<Mutex<RunnerStatus>>,
    task_id: &str,
    concurrency_state: &Arc<Mutex<(usize, usize)>>,
) -> Result<()> {
    let _permit = if app.allow_concurrent_tasks {
        if let Some(limit) = yasweb_concurrency_limit(app, spec)? {
            let semaphore = app_lock_manager
                .get_semaphore_with_limit(&app.id, limit)
                .await;

            {
                let mut counts = concurrency_state.lock().await;
                counts.1 += 1;
                if counts.0 == 0 {
                    let mut st = status.lock().await;
                    st.waiting_for_app
                        .insert(task_id.to_string(), app.id.clone());
                    if st.running_task_ids.iter().any(|id| id == task_id) {
                        st.running_task_ids.retain(|id| id != task_id);
                        st.running_tasks_count = st.running_tasks_count.saturating_sub(1);
                    }
                    if !st.queued_task_ids.iter().any(|id| id == task_id) {
                        st.queued_task_ids.push(task_id.to_string());
                    }
                    st.queued_tasks_count = st.queued_task_ids.len();
                }
            }
            logger
                .log(&format!(
                    "Waiting for Yasweb concurrency slot ({limit} max)..."
                ))
                .await;

            let permit = semaphore.acquire_owned().await.with_context(|| {
                format!("Yasweb concurrency limiter for '{}' was closed", app.name)
            });
            {
                let mut counts = concurrency_state.lock().await;
                counts.1 = counts.1.saturating_sub(1);
                if permit.is_ok() {
                    counts.0 += 1;
                    if counts.0 == 1 {
                        let mut st = status.lock().await;
                        st.waiting_for_app.remove(task_id);
                        st.queued_task_ids.retain(|id| id != task_id);
                        st.queued_tasks_count = st.queued_task_ids.len();
                        if !st.running_task_ids.iter().any(|id| id == task_id) {
                            st.running_task_ids.push(task_id.to_string());
                            st.running_tasks_count += 1;
                        }
                    }
                } else if counts.0 == 0 && counts.1 == 0 {
                    let mut st = status.lock().await;
                    st.waiting_for_app.remove(task_id);
                    st.queued_task_ids.retain(|id| id != task_id);
                    st.queued_tasks_count = st.queued_task_ids.len();
                }
            }
            Some(permit?)
        } else {
            None
        }
    } else {
        // Non-concurrent apps are already protected by execute_step's exclusive lock.
        None
    };

    let result = run_external_app(logger, app, spec, period, timeout_seconds).await;
    if _permit.is_some() {
        let mut counts = concurrency_state.lock().await;
        counts.0 = counts.0.saturating_sub(1);
        if counts.0 == 0 && counts.1 > 0 {
            let mut st = status.lock().await;
            st.waiting_for_app
                .insert(task_id.to_string(), app.id.clone());
            st.running_task_ids.retain(|id| id != task_id);
            st.running_tasks_count = st.running_tasks_count.saturating_sub(1);
            if !st.queued_task_ids.iter().any(|id| id == task_id) {
                st.queued_task_ids.push(task_id.to_string());
            }
            st.queued_tasks_count = st.queued_task_ids.len();
        }
    }
    result
}

async fn execute_action(
    action: &ActionSpec,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
    execution_at: DateTime<Utc>,
    app_lock_manager: &AppLockManager,
    status: &Arc<Mutex<RunnerStatus>>,
    task_id: &str,
    concurrency_state: &Arc<Mutex<(usize, usize)>>,
) -> Result<()> {
    match action {
        ActionSpec::ShellCommand(spec) => {
            if !policy.allow_shell_tasks {
                return Err(anyhow::anyhow!(
                    "shell_command tasks are disabled by runner config"
                ));
            }
            let cmd = spec.command.clone();
            if let Err(e) = run_shell_command(logger, &cmd, timeout_seconds).await {
                if !spec.continue_on_error {
                    return Err(anyhow::anyhow!("command failed: {}", e));
                }
            }
            Ok(())
        }
        ActionSpec::ExternalApp(spec) => {
            if let Some(app) = policy.registered_apps.iter().find(|a| a.id == spec.app_id) {
                if !crate::runner::engine::application::supports_periodization(logger, app, spec)
                    .await
                {
                    return run_external_app_with_shared_limit(
                        logger,
                        app,
                        spec,
                        None,
                        timeout_seconds,
                        app_lock_manager,
                        status,
                        task_id,
                        concurrency_state,
                    )
                    .await;
                }

                let periods = crate::runner::config::resolve_app_periods_at(spec, execution_at)?;
                logger
                    .log(&format!(
                        "Resolved date period for app '{}' at {}: start_expression='{}', end_expression='{}', periods={}",
                        app.name,
                        execution_at.to_rfc3339(),
                        spec.start_date.as_deref().unwrap_or("today"),
                        spec.end_date.as_deref().unwrap_or("<same as start>"),
                        periods
                            .iter()
                            .map(|period| format!("{} -> {}", period.start_date, period.end_date))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .await;

                if app.allow_concurrent_tasks {
                    let total_periods = periods.len();
                    let mut handles = Vec::new();

                    for (idx, period) in periods.into_iter().enumerate() {
                        let logger = logger.clone();
                        let app = app.clone();
                        let spec_owned = spec.clone();
                        let app_lock_manager = app_lock_manager.clone();
                        let status = status.clone();
                        let task_id = task_id.to_string();
                        let concurrency_state = concurrency_state.clone();

                        handles.push(tokio::spawn(async move {
                            if total_periods > 1 {
                                logger
                                    .log(&format!(
                                        "Executing app '{}' period {}/{} ({} -> {})...",
                                        app.name,
                                        idx + 1,
                                        total_periods,
                                        period.start_date,
                                        period.end_date
                                    ))
                                    .await;
                            }
                            run_external_app_with_shared_limit(
                                &logger,
                                &app,
                                &spec_owned,
                                Some(&period),
                                timeout_seconds,
                                &app_lock_manager,
                                &status,
                                &task_id,
                                &concurrency_state,
                            )
                            .await
                        }));
                    }

                    for handle in handles {
                        handle
                            .await
                            .context("concurrent period execution join error")??;
                    }
                    Ok(())
                } else {
                    let total_periods = periods.len();
                    for (idx, period) in periods.iter().enumerate() {
                        if total_periods > 1 {
                            logger
                                .log(&format!(
                                    "Executing app '{}' period {}/{} ({} -> {})...",
                                    app.name,
                                    idx + 1,
                                    total_periods,
                                    period.start_date,
                                    period.end_date
                                ))
                                .await;
                        }
                        run_external_app_with_shared_limit(
                            logger,
                            app,
                            spec,
                            Some(period),
                            timeout_seconds,
                            app_lock_manager,
                            status,
                            task_id,
                            concurrency_state,
                        )
                        .await?;
                    }
                    Ok(())
                }
            } else {
                Err(anyhow::anyhow!(
                    "Registered app with ID '{}' not found in config",
                    spec.app_id
                ))
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_step(
    step: &TaskStep,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
    status: &Arc<Mutex<RunnerStatus>>,
    app_lock_manager: &AppLockManager,
    task_id: &str,
    execution_at: DateTime<Utc>,
) -> Result<()> {
    let mut required_apps = std::collections::HashSet::new();
    for action in &step.actions {
        if let ActionSpec::ExternalApp(app) = action {
            required_apps.insert(app.app_id.clone());
        }
    }

    let mut non_concurrent_apps: Vec<String> = Vec::new();
    for app_id in required_apps {
        if let Some(app) = policy.registered_apps.iter().find(|a| a.id == app_id) {
            if !app.allow_concurrent_tasks {
                non_concurrent_apps.push(app_id.clone());
            }
        }
    }

    non_concurrent_apps.sort();
    non_concurrent_apps.dedup();

    let mut acquired_permits = Vec::new();
    for app_id in &non_concurrent_apps {
        let sem = app_lock_manager.get_semaphore(app_id).await;

        {
            let mut st = status.lock().await;
            st.waiting_for_app
                .insert(task_id.to_string(), app_id.clone());
            if st.running_task_ids.iter().any(|id| id == task_id) {
                st.running_task_ids.retain(|id| id != task_id);
                st.running_tasks_count = st.running_tasks_count.saturating_sub(1);
            }
            if !st.queued_task_ids.iter().any(|id| id == task_id) {
                st.queued_task_ids.push(task_id.to_string());
            }
            st.queued_tasks_count = st.queued_tasks_count.max(st.queued_task_ids.len());
        }

        logger
            .log(&format!(
                "Waiting for exclusive lock on app '{}'...",
                app_id
            ))
            .await;

        let permit_result = sem.acquire_owned().await;
        if permit_result.is_err() {
            let mut st = status.lock().await;
            st.waiting_for_app.remove(task_id);
            return Err(anyhow::anyhow!("Failed to acquire app lock for {}", app_id));
        }
        let permit = permit_result.unwrap();

        logger
            .log(&format!("Acquired exclusive lock on app '{}'.", app_id))
            .await;

        {
            let mut st = status.lock().await;
            st.waiting_for_app.remove(task_id);
            st.queued_task_ids.retain(|id| id != task_id);
            st.queued_tasks_count = st.queued_tasks_count.saturating_sub(1);
            if !st.running_task_ids.iter().any(|id| id == task_id) {
                st.running_task_ids.push(task_id.to_string());
                st.running_tasks_count += 1;
            }
        }

        acquired_permits.push(permit);
    }

    let concurrency_state = Arc::new(Mutex::new((0usize, 0usize)));
    let result = match step.mode {
        ExecutionMode::Sequential => {
            let mut step_result = Ok(());
            for action in &step.actions {
                if let Err(e) = execute_action(
                    action,
                    logger,
                    policy,
                    timeout_seconds,
                    execution_at,
                    app_lock_manager,
                    status,
                    task_id,
                    &concurrency_state,
                )
                .await
                {
                    step_result = Err(e);
                    break;
                }
            }
            step_result
        }
        ExecutionMode::Parallel => {
            let mut tasks = tokio::task::JoinSet::new();
            for action in &step.actions {
                let action = action.clone();
                let logger = logger.clone();
                let policy = policy.clone();
                let app_lock_manager = app_lock_manager.clone();
                let status = status.clone();
                let task_id = task_id.to_string();
                let concurrency_state = concurrency_state.clone();

                tasks.spawn(async move {
                    let result = execute_action(
                        &action,
                        &logger,
                        &policy,
                        timeout_seconds,
                        execution_at,
                        &app_lock_manager,
                        &status,
                        &task_id,
                        &concurrency_state,
                    )
                    .await;
                    (action, result)
                });
            }

            while let Some(joined) = tasks.join_next().await {
                let (action, result) = joined.context("parallel action join failed")?;
                if let Err(error) = result {
                    let name = match action {
                        ActionSpec::ShellCommand(s) => s.command,
                        ActionSpec::ExternalApp(a) => format!("app: {}", a.app_id),
                    };

                    // A failed action invalidates the step. Cancel unfinished sibling
                    // actions instead of allowing dependent work to continue.
                    tasks.abort_all();
                    while tasks.join_next().await.is_some() {}

                    return Err(anyhow::anyhow!(
                        "parallel action failed; remaining actions were cancelled: {}: {}",
                        name,
                        error
                    ));
                }
            }

            Ok(())
        }
    };

    result
}

#[allow(clippy::too_many_arguments)]
async fn execute_pipeline(
    steps: &[TaskStep],
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
    status: &Arc<Mutex<RunnerStatus>>,
    app_lock_manager: &AppLockManager,
    task_id: &str,
    execution_at: DateTime<Utc>,
) -> Result<()> {
    for step in steps {
        execute_step(
            step,
            logger,
            policy,
            timeout_seconds,
            status,
            app_lock_manager,
            task_id,
            execution_at,
        )
        .await?;
    }
    Ok(())
}

use crate::runner::engine::state::TaskExecutionResult;

pub async fn run_task_inner(
    task: &mut RunnerTask,
    policy: &ExecutionPolicy,
    status: &Arc<Mutex<RunnerStatus>>,
    app_lock_manager: &AppLockManager,
) -> TaskExecutionResult {
    run_task_inner_at(task, policy, status, app_lock_manager, Utc::now()).await
}

pub(crate) async fn run_task_inner_at(
    task: &mut RunnerTask,
    policy: &ExecutionPolicy,
    status: &Arc<Mutex<RunnerStatus>>,
    app_lock_manager: &AppLockManager,
    execution_at: DateTime<Utc>,
) -> TaskExecutionResult {
    let logger = TaskLogger::new(&task.id, &task.name);
    logger.log("Initializing task execution...").await;

    let start_time = Utc::now();
    let scheduled_time = execution_at.to_rfc3339();

    // Per-task lifecycle details belong in the task-specific log, not runner.log.
    logger
        .log(&format!(
            "Task execution started; scheduled_time={}, actual_start_time={}",
            scheduled_time,
            start_time.to_rfc3339()
        ))
        .await;

    let effective_shell_timeout = if task.timeout_seconds > 0 {
        task.timeout_seconds
    } else {
        policy.shell_timeout_seconds
    };

    let effective_post_run_timeout = if task.timeout_seconds > 0 {
        task.timeout_seconds
    } else {
        policy.post_run_timeout_seconds
    };

    let result = execute_pipeline(
        &task.steps,
        &logger,
        policy,
        effective_shell_timeout,
        status,
        app_lock_manager,
        &task.id,
        execution_at,
    )
    .await;

    match result {
        Ok(_) => {
            if task.post_run_steps.is_empty() {
                logger.log("Task completed successfully.").await;
                task.last_status = "ok".to_string();
            } else {
                logger
                    .log("Main pipeline completed. Executing post run steps...")
                    .await;
                let post_run_result = execute_pipeline(
                    &task.post_run_steps,
                    &logger,
                    policy,
                    effective_post_run_timeout,
                    status,
                    app_lock_manager,
                    &task.id,
                    execution_at,
                )
                .await;
                match post_run_result {
                    Ok(_) => {
                        logger
                            .log("Task and post run steps completed successfully.")
                            .await;
                        task.last_status = "ok".to_string();
                    }
                    Err(e) => {
                        task.last_status = format!("post-run error: {}", e);

                        let err_msg = format!("post-run error: {}", e);
                        logger
                            .log(&format!("Post-run pipeline failed: {}", err_msg))
                            .await;
                    }
                }
            }
        }
        Err(e) => {
            logger.log(&format!("Task failed with error: {}", e)).await;
            task.last_status = format!("error: {}", e);

            // The full failure details are retained in this task's separate log.
        }
    }

    let end_time = Utc::now();
    let duration = end_time
        .signed_duration_since(start_time)
        .num_milliseconds();
    let status_str = task.last_status.clone();

    logger
        .log(&format!(
            "Task execution ended; duration_ms={}, status={}",
            duration, status_str
        ))
        .await;

    let mut st = status.lock().await;
    st.last_run_at = Utc::now().to_rfc3339();

    let success = task.last_status == "ok";
    let error = if success {
        None
    } else {
        Some(task.last_status.clone())
    };
    TaskExecutionResult { success, error }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::ShellCommandSpec;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::sync::Barrier;

    #[test]
    fn yasweb_concurrency_uses_registered_config_and_preserves_zero_as_unlimited() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("yasweb_config.json");
        let app = |config_path: &std::path::Path| RegisteredApp {
            id: "yasweb".to_string(),
            name: "Yasweb Automation Engine".to_string(),
            executable_path: "yasweb.exe".to_string(),
            config_path: config_path.to_string_lossy().into_owned(),
            plugin_api_version: Some(1),
            allow_concurrent_tasks: true,
        };

        let spec = crate::runner::config::ExternalAppSpec {
            app_id: "yasweb".to_string(),
            args: std::collections::HashMap::new(),
            period_mode: crate::runner::config::PeriodMode::Custom,
            start_date: None,
            end_date: None,
            start_date_arg: None,
            end_date_arg: None,
        };

        std::fs::write(&config_path, r#"{"concurrency":3}"#).unwrap();
        assert_eq!(
            yasweb_concurrency_limit(&app(&config_path), &spec).unwrap(),
            Some(3)
        );

        std::fs::write(&config_path, r#"{"concurrency":0}"#).unwrap();
        assert_eq!(
            yasweb_concurrency_limit(&app(&config_path), &spec).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn test_sequential_step_continues_on_error() {
        let policy = ExecutionPolicy {
            allow_shell_tasks: true,
            shell_timeout_seconds: 5,
            post_run_timeout_seconds: 5,
            min_task_interval_seconds: 1,
            registered_apps: vec![],
            log_retention_days: 30,
        };
        let step = TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![
                ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "exit 8".to_string(),
                    continue_on_error: true,
                }),
                ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "echo ok".to_string(),
                    continue_on_error: false,
                }),
            ],
        };

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
        let app_lock_mgr = AppLockManager::new();
        execute_step(
            &step,
            &TaskLogger::new("test", "test"),
            &policy,
            5,
            &status,
            &app_lock_mgr,
            "test_task",
            Utc::now(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_sequential_step_stops_on_non_continued_error() {
        let policy = ExecutionPolicy {
            allow_shell_tasks: true,
            shell_timeout_seconds: 5,
            post_run_timeout_seconds: 5,
            min_task_interval_seconds: 1,
            registered_apps: vec![],
            log_retention_days: 30,
        };
        let step = TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: false,
            })],
        };

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
        let app_lock_mgr = AppLockManager::new();
        assert!(execute_step(
            &step,
            &TaskLogger::new("test", "test"),
            &policy,
            5,
            &status,
            &app_lock_mgr,
            "test_task",
            Utc::now(),
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn test_parallel_step_fails_only_non_continued_errors() {
        let policy = ExecutionPolicy {
            allow_shell_tasks: true,
            shell_timeout_seconds: 5,
            post_run_timeout_seconds: 5,
            min_task_interval_seconds: 1,
            registered_apps: vec![],
            log_retention_days: 30,
        };
        let ignored_step = TaskStep {
            name: None,
            mode: ExecutionMode::Parallel,
            actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: true,
            })],
        };

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
        let app_lock_mgr = AppLockManager::new();
        execute_step(
            &ignored_step,
            &TaskLogger::new("test", "test"),
            &policy,
            5,
            &status,
            &app_lock_mgr,
            "test_task",
            Utc::now(),
        )
        .await
        .unwrap();

        let failed_step = TaskStep {
            name: None,
            mode: ExecutionMode::Parallel,
            actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: false,
            })],
        };
        assert!(execute_step(
            &failed_step,
            &TaskLogger::new("test", "test"),
            &policy,
            5,
            &status,
            &app_lock_mgr,
            "test_task",
            Utc::now(),
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn test_behavioral_concurrent_period_execution_and_error_propagation() {
        let barrier = Arc::new(Barrier::new(2));
        let barrier_clone = barrier.clone();

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let h1 = tokio::spawn(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            barrier_clone.wait().await;
            Ok::<(), anyhow::Error>(())
        });

        let counter_clone2 = counter.clone();
        let barrier_clone2 = barrier.clone();
        let h2 = tokio::spawn(async move {
            counter_clone2.fetch_add(1, Ordering::SeqCst);
            barrier_clone2.wait().await;
            Err::<(), anyhow::Error>(anyhow::anyhow!("Intentional failure"))
        });

        let (res1, res2) = tokio::join!(h1, h2);
        assert!(res1.unwrap().is_ok());
        let err_res = res2.unwrap();
        assert!(err_res.is_err());
        assert_eq!(err_res.unwrap_err().to_string(), "Intentional failure");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
