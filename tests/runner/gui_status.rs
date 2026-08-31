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

fn create_sync_task(id: &str, app_ids: Vec<&str>, lock_file: &std::path::Path) -> RunnerTask {
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
        timeout_seconds: 10,
    };

    let lock_str = lock_file.to_str().unwrap().replace("\\", "/");

    let mut actions = vec![
        #[cfg(target_family = "unix")]
        ActionSpec::ShellCommand(ShellCommandSpec {
            command: format!(
                "touch '{lock_str}.started' && while [ ! -f '{lock_str}.release' ]; do sleep 0.1; done"
            ),
            continue_on_error: true,
        }),
        #[cfg(target_family = "windows")]
        ActionSpec::ShellCommand(ShellCommandSpec {
            command: format!(
                "New-Item -ItemType File -Force -Path '{lock_str}.started'; while (-not (Test-Path '{lock_str}.release')) {{ Start-Sleep -Milliseconds 100 }}"
            ),
            continue_on_error: true,
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
    let sync_dir = tempfile::tempdir().unwrap();

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

    let lock_a1 = sync_dir.path().join("a1");
    let lock_b1 = sync_dir.path().join("b1");

    let t_a1 = create_sync_task("A1", vec!["App A"], &lock_a1);
    let t_b1 = create_sync_task("B1", vec!["App B"], &lock_b1);

    // 1. Start A1.
    queue_task(t_a1.clone());

    // Wait until A1 is started
    let started_a1 = sync_dir.path().join("a1.started");
    let mut a1_running = false;
    for _ in 0..1000 {
        if started_a1.exists() {
            a1_running = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(a1_running, "A1 did not start");

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
    let started_b1 = sync_dir.path().join("b1.started");
    let mut b1_running = false;
    for _ in 0..1000 {
        if started_b1.exists() {
            b1_running = true;
            break;
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

    // 6. Release B1.
    let _ = std::fs::write(sync_dir.path().join("b1.release"), "");

    // Wait for B1 to finish
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

    // 8. A1 remains RUNNING until it finishes.
    {
        let st = status.lock().await;
        assert!(
            st.running_task_ids.contains(&"A1".to_string()),
            "A1 should still be running"
        );
    }

    // 9. Release A1.
    let _ = std::fs::write(sync_dir.path().join("a1.release"), "");

    // Wait for A1 to finish
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
