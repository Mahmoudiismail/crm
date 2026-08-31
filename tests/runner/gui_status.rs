use crm_tool::runner::config::{
    ActionSpec, ExecutionMode, ExternalAppSpec, RegisteredApp, RunnerTask, ShellCommandSpec,
    TaskStep,
};
use crm_tool::runner::engine::dispatcher::spawn_execution_manager;
use crm_tool::runner::engine::state::{ExecutionManagerCommand, ExecutionPolicy, RunnerStatus};
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::Mutex;

fn create_mock_app(id: &str, concurrent: bool) -> RegisteredApp {
    RegisteredApp {
        id: id.to_string(),
        name: id.to_string(),
        executable_path: "exe".to_string(),
        config_path: "cfg".to_string(),
        allow_concurrent_tasks: concurrent,
    }
}

fn create_sync_task(id: &str, app_ids: Vec<&str>) -> RunnerTask {
    let mut task = RunnerTask {
        id: id.to_string(),
        name: format!("Task {}", id),
        enabled: true,
        repetition: Default::default(),
        frequency_seconds: 0,
        next_run_at: "".to_string(),
        schedules: vec![],
        steps: vec![],
        post_run_steps: vec![],
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        timeout_seconds: 60,
    };

    let mut actions = vec![
        #[cfg(target_family = "unix")]
        ActionSpec::ShellCommand(ShellCommandSpec {
            command: "sleep 2".to_string(),
            continue_on_error: false,
        }),
        #[cfg(target_family = "windows")]
        ActionSpec::ShellCommand(ShellCommandSpec {
            command: "powershell -Command \"Start-Sleep -Seconds 2\"".to_string(),
            continue_on_error: false,
        }),
    ];

    for app_id in app_ids {
        actions.push(ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: app_id.to_string(),
            args: Default::default(),
        }));
    }

    task.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions,
    });
    task
}

#[tokio::test]
async fn test_gui_status_concurrent_tasks() {
    let status = Arc::new(Mutex::new(RunnerStatus {
        app_locks: std::collections::HashMap::new(),
        running_tasks_count: 0,
        queued_tasks_count: 0,
        running_task_ids: Vec::new(),
        queued_task_ids: Vec::new(),
        last_error: "".to_string(),
        last_task_id: "".to_string(),
        last_run_at: "".to_string(),
    }));

    let temp_file = NamedTempFile::new().unwrap();
    let config_path = temp_file.path().to_str().unwrap().to_string();
    let _sync_dir = tempfile::tempdir().unwrap();

    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 10,
        post_run_timeout_seconds: 10,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![
            create_mock_app("App A", true),
            create_mock_app("App B", false),
        ],
    };

    let queue_task = |task: RunnerTask| {
        let tx = exec_tx.clone();
        let p = policy.clone();
        tokio::spawn(async move {
            let _ = tx
                .send(ExecutionManagerCommand::QueueTask {
                    task: Box::new(task),
                    policy: p,
                })
                .await;
        })
    };

    let t_a1 = create_sync_task("A1", vec!["App A"]);
    let t_b1 = create_sync_task("B1", vec!["App B"]);

    // 1. Start A1.
    queue_task(t_a1.clone());

    // Wait until A1 is started
    let mut a1_running = false;
    for _ in 0..1000 {
        {
            let st = status.lock().await;
            if st.running_task_ids.contains(&"A1".to_string()) {
                a1_running = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    if !a1_running {
        let st = status.lock().await;
        panic!("A1 did not start. Last error: {}", st.last_error);
    }

    // 2. A1 remains running.
    {
        let st = status.lock().await;
        assert!(
            st.running_task_ids.contains(&"A1".to_string()),
            "A1 is not in running_task_ids"
        );
        assert!(
            !st.queued_task_ids.contains(&"A1".to_string()),
            "A1 should not be queued"
        );
    }

    // 3. Start B1 while A1 is still running.
    queue_task(t_b1.clone());

    // Wait until B1 is started
    let mut b1_running = false;
    for _ in 0..1000 {
        {
            let st = status.lock().await;
            if st.running_task_ids.contains(&"B1".to_string()) {
                b1_running = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(b1_running, "B1 did not start concurrently with A1");

    // 4. B1 is reported as RUNNING, not WAITING.
    // 5. A1 is still reported as RUNNING.
    {
        let st = status.lock().await;
        assert!(
            st.running_task_ids.contains(&"A1".to_string()),
            "A1 is not in running_task_ids"
        );
        assert!(
            st.running_task_ids.contains(&"B1".to_string()),
            "B1 is not in running_task_ids"
        );
        assert!(
            !st.queued_task_ids.contains(&"B1".to_string()),
            "B1 should not be queued"
        );
    }

    // 6. Wait for B1 to finish (since it sleeps for 2 seconds)
    for _ in 0..1000 {
        let st = status.lock().await;
        if !st.running_task_ids.contains(&"B1".to_string()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // 7. B1 becomes SUCCESS.
    {
        let st = status.lock().await;
        assert!(
            !st.running_task_ids.contains(&"B1".to_string()),
            "B1 should have finished"
        );
    }

    // 8. Wait for A1 to finish (since it sleeps for 2 seconds)
    for _ in 0..1000 {
        let st = status.lock().await;
        if !st.running_task_ids.contains(&"A1".to_string()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    {
        let st = status.lock().await;
        assert!(
            !st.running_task_ids.contains(&"A1".to_string()),
            "A1 should have finished"
        );
    }
}
