use anyhow::{Context, Result};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::runner::config::{ActionSpec, ExecutionMode, RunnerTask, TaskStep};
use crate::runner::engine::application::run_external_app;
use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::shell::run_shell_command;
use crate::runner::engine::state::{ExecutionPolicy, RunnerStatus};

async fn execute_action(
    action: &ActionSpec,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
) -> Result<()> {
    match action {
        ActionSpec::ShellCommand(spec) => {
            if !policy.allow_shell_tasks {
                return Err(anyhow::anyhow!(
                    "shell_command tasks are disabled by runner config"
                ));
            }
            if let Err(e) = run_shell_command(logger, &spec.command, timeout_seconds).await {
                if !spec.continue_on_error {
                    return Err(anyhow::anyhow!("command failed: {}", e));
                }
            }
            Ok(())
        }
        ActionSpec::ExternalApp(spec) => {
            if let Some(app) = policy.registered_apps.iter().find(|a| a.id == spec.app_id) {
                run_external_app(logger, app, &spec.args, timeout_seconds).await
            } else {
                Err(anyhow::anyhow!(
                    "Registered app with ID '{}' not found in config",
                    spec.app_id
                ))
            }
        }
    }
}

async fn execute_step(
    step: &TaskStep,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
) -> Result<()> {
    match step.mode {
        ExecutionMode::Sequential => {
            for action in &step.actions {
                execute_action(action, logger, policy, timeout_seconds).await?;
            }
            Ok(())
        }
        ExecutionMode::Parallel => {
            let mut handles = Vec::new();
            for action in &step.actions {
                let action = action.clone();
                let logger = logger.clone();
                let policy = policy.clone();

                handles.push(tokio::spawn(async move {
                    let result = execute_action(&action, &logger, &policy, timeout_seconds).await;
                    (action, result)
                }));
            }

            let mut failures = Vec::new();
            for handle in handles {
                let (action, result) = handle.await.context("parallel action join failed")?;
                if let Err(e) = result {
                    let name = match action {
                        ActionSpec::ShellCommand(s) => s.command,
                        ActionSpec::ExternalApp(a) => format!("app: {}", a.app_id),
                    };
                    failures.push(format!("{}: {}", name, e));
                }
            }

            if failures.is_empty() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "parallel actions failed: {}",
                    failures.join("; ")
                ))
            }
        }
    }
}

async fn execute_pipeline(
    steps: &[TaskStep],
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
) -> Result<()> {
    for step in steps {
        execute_step(step, logger, policy, timeout_seconds).await?;
    }
    Ok(())
}

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

    let result = execute_pipeline(&task.steps, &logger, policy, effective_shell_timeout).await;

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
                        let mut st = status.lock().await;
                        st.last_error = format!("post-run error: {}", e);

                        let task_id = &task.id;
                        let err_msg = format!("post-run error: {}", e);

                        let next_run = task.next_run_at.clone();
                        error!(
                            task_id = %task_id,
                            error_type = "PostRunError",
                            error_message = %err_msg,
                            schedules = ?task.schedules,
                            next_run = %next_run,
                            "Task Execution Failed in Post Run Pipeline"
                        );
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::ShellCommandSpec;

    #[tokio::test]
    async fn test_sequential_step_continues_on_error() {
        let policy = ExecutionPolicy {
            allow_shell_tasks: true,
            shell_timeout_seconds: 5,
            post_run_timeout_seconds: 5,
            min_task_interval_seconds: 1,
            registered_apps: vec![],
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

        execute_step(&step, &TaskLogger::new("test", "test"), &policy, 5)
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
        };
        let step = TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: false,
            })],
        };

        assert!(
            execute_step(&step, &TaskLogger::new("test", "test"), &policy, 5)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_parallel_step_fails_only_non_continued_errors() {
        let policy = ExecutionPolicy {
            allow_shell_tasks: true,
            shell_timeout_seconds: 5,
            post_run_timeout_seconds: 5,
            min_task_interval_seconds: 1,
            registered_apps: vec![],
        };
        let ignored_step = TaskStep {
            name: None,
            mode: ExecutionMode::Parallel,
            actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: true,
            })],
        };

        execute_step(&ignored_step, &TaskLogger::new("test", "test"), &policy, 5)
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
        assert!(
            execute_step(&failed_step, &TaskLogger::new("test", "test"), &policy, 5)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_mixed_pipeline_execution_order() {
        let policy = ExecutionPolicy {
            allow_shell_tasks: true,
            shell_timeout_seconds: 5,
            post_run_timeout_seconds: 5,
            min_task_interval_seconds: 1,
            registered_apps: vec![],
        };
        let steps = vec![
            TaskStep {
                name: None,
                mode: ExecutionMode::Sequential,
                actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "echo 1".to_string(),
                    continue_on_error: false,
                })],
            },
            TaskStep {
                name: None,
                mode: ExecutionMode::Parallel,
                actions: vec![
                    ActionSpec::ShellCommand(ShellCommandSpec {
                        command: "echo 2".to_string(),
                        continue_on_error: false,
                    }),
                    ActionSpec::ShellCommand(ShellCommandSpec {
                        command: "echo 3".to_string(),
                        continue_on_error: false,
                    }),
                ],
            },
            TaskStep {
                name: None,
                mode: ExecutionMode::Sequential,
                actions: vec![ActionSpec::ShellCommand(ShellCommandSpec {
                    command: "echo 4".to_string(),
                    continue_on_error: false,
                })],
            },
        ];

        execute_pipeline(&steps, &TaskLogger::new("test", "test"), &policy, 5)
            .await
            .unwrap();
    }
}
