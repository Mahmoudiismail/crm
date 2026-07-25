use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::runner::config::ShellCommandMode;
use crate::runner::config::{RunnerTask, TaskKind};
use crate::runner::engine::application::{run_external_app, run_post_run_script};
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::shell::{run_shell_parallel, run_shell_sequential};
use crate::runner::engine::state::{ExecutionPolicy, RunnerStatus};

pub async fn run_task_inner(
    task: &mut RunnerTask,
    policy: &ExecutionPolicy,
    status: &Arc<Mutex<RunnerStatus>>,
) {
    let logger = TaskLogger::new(&task.id, &task.name);
    logger.log("Initializing task execution...").await;

    let start_time = Utc::now();
    let scheduled_time = task.next_run_at.clone();

    info!(
        task_id = %task.id,
        task_name = %task.name,
        scheduled_time = %scheduled_time,
        actual_start_time = %start_time.to_rfc3339(),
        "Task Execution Started"
    );

    {
        let mut st = status.lock().await;
        st.last_error.clear();
    }

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

    let result = match task.legacy_kind() {
        TaskKind::ShellCommand { mode, commands } => {
            if !policy.allow_shell_tasks {
                Err(anyhow::anyhow!(
                    "shell_command tasks are disabled by runner config"
                ))
            } else {
                match mode {
                    ShellCommandMode::Sequential => {
                        run_shell_sequential(&logger, &commands, effective_shell_timeout).await
                    }
                    ShellCommandMode::Parallel => {
                        run_shell_parallel(&logger, &commands, effective_shell_timeout).await
                    }
                }
            }
        }
        TaskKind::ExternalApp { app_id, args } => {
            if let Some(app) = policy.registered_apps.iter().find(|a| a.id == *app_id) {
                run_external_app(&logger, app, &args, effective_shell_timeout).await
            } else {
                Err(anyhow::anyhow!(
                    "Registered app with ID '{}' not found in config",
                    app_id
                ))
            }
        }
    };

    match result {
        Ok(_) => {
            if !task.legacy_post_run_app_id().trim().is_empty() {
                if let Some(app) = policy
                    .registered_apps
                    .iter()
                    .find(|a| a.id == task.legacy_post_run_app_id())
                {
                    match run_external_app(
                        &logger,
                        app,
                        &task.legacy_post_run_app_args(),
                        effective_post_run_timeout,
                    )
                    .await
                    {
                        Ok(_) => task.last_status = "ok".to_string(),
                        Err(e) => {
                            task.last_status = format!("post-run app error: {}", e);
                            let mut st = status.lock().await;
                            st.last_error = format!("post-run app error: {}", e);

                            let task_id = &task.id;
                            let err_msg = format!("post-run app error: {}", e);

                            let next_run = task.next_run_at.clone();
                            error!(
                                task_id = %task_id,
                                error_type = "PostRunError",
                                error_message = %err_msg,
                                schedules = ?task.schedules,
                                next_run = %next_run,
                                "Task Execution Failed in Post Run App"
                            );
                        }
                    }
                } else {
                    let err_msg = format!(
                        "Registered app with ID '{}' not found in config for post run",
                        task.legacy_post_run_app_id()
                    );
                    task.last_status = format!("post-run error: {}", err_msg);
                    let mut st = status.lock().await;
                    st.last_error = err_msg.clone();
                    error!(
                        task_id = %task.id,
                        error_type = "PostRunError",
                        error_message = %err_msg,
                        "Task Execution Failed in Post Run App"
                    );
                }
            } else if !task.legacy_post_run_script().trim().is_empty() {
                match run_post_run_script(
                    &logger,
                    &task.legacy_post_run_script(),
                    effective_post_run_timeout,
                )
                .await
                {
                    Ok(_) => task.last_status = "ok".to_string(),
                    Err(e) => {
                        task.last_status = format!("post-run script error: {}", e);
                        let mut st = status.lock().await;
                        st.last_error = format!("post-run script error: {}", e);

                        let task_id = &task.id;
                        let err_msg = format!("post-run script error: {}", e);

                        let next_run = task.next_run_at.clone();
                        error!(
                            task_id = %task_id,
                            error_type = "PostRunError",
                            error_message = %err_msg,
                            schedules = ?task.schedules,
                            next_run = %next_run,
                            "Task Execution Failed in Post Run"
                        );
                    }
                }
            } else {
                logger.log("Task completed successfully.").await;
                task.last_status = "ok".to_string();
            }
        }
        Err(e) => {
            logger.log(&format!("Task failed with error: {}", e)).await;
            task.last_status = format!("error: {}", e);
            let mut st = status.lock().await;
            st.last_error = format!("{}", e);

            let task_id = &task.id;
            let err_msg = format!("{}", e);

            let next_run = task.next_run_at.clone();
            error!(
                task_id = %task_id,
                error_type = "ExecutionError",
                error_message = %err_msg,
                schedules = ?task.schedules,
                next_run = %next_run,
                "Task Execution Failed"
            );
        }
    }

    let end_time = Utc::now();
    let duration = end_time
        .signed_duration_since(start_time)
        .num_milliseconds();
    let status_str = task.last_status.clone();

    info!(
        task_id = %task.id,
        duration_ms = %duration,
        status = %status_str,
        "Task Execution Ended"
    );

    let mut st = status.lock().await;
    st.last_run_at = Utc::now().to_rfc3339();
}
