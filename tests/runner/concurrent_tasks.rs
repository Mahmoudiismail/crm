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

#[test]
fn test_registered_app_deserialize_defaults() {
    let json_str = r#"{
        "id": "app_old",
        "name": "Old App",
        "executable_path": "path.exe",
        "config_path": ""
    }"#;

    let app: RegisteredApp = serde_json::from_str(json_str).unwrap();
    assert_eq!(app.id, "app_old");
    assert!(!app.allow_concurrent_tasks);

    let json_str_true = r#"{
        "id": "app_new",
        "name": "New App",
        "executable_path": "path.exe",
        "config_path": "",
        "allow_concurrent_tasks": true
    }"#;

    let app2: RegisteredApp = serde_json::from_str(json_str_true).unwrap();
    assert!(app2.allow_concurrent_tasks);

    let json_str_false = r#"{
        "id": "app_new_false",
        "name": "New App False",
        "executable_path": "path.exe",
        "config_path": "",
        "allow_concurrent_tasks": false
    }"#;

    let app3: RegisteredApp = serde_json::from_str(json_str_false).unwrap();
    assert!(!app3.allow_concurrent_tasks);
}

fn create_long_task(id: &str, app_ids: Vec<&str>, sync_file: &std::path::Path) -> RunnerTask {
    let mut task = RunnerTask {
        id: id.to_string(),
        name: "Test Task".to_string(),
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

    let sync_file_str = sync_file.to_str().unwrap();

    // We add a deterministic file-polling shell command to hold up the execution pipeline.
    // The task will not finish until the test explicitly creates the sync_file.
    // Then we add the external apps so they register as dependencies.
    let mut actions = vec![
        #[cfg(target_family = "unix")]
        ActionSpec::ShellCommand(ShellCommandSpec {
            command: format!("while [ ! -f '{sync_file_str}' ]; do sleep 0.1; done"),
            continue_on_error: true,
        }),
        #[cfg(target_family = "windows")]
        ActionSpec::ShellCommand(ShellCommandSpec {
            command: format!("powershell -Command \"while (-not (Test-Path '{sync_file_str}')) {{ Start-Sleep -Milliseconds 100 }}\""),
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

fn create_mock_app(id: &str, concurrent: bool) -> RegisteredApp {
    RegisteredApp {
        id: id.to_string(),
        name: id.to_string(),
        executable_path: "exe".to_string(),
        config_path: "cfg".to_string(),
        allow_concurrent_tasks: concurrent,
    }
}

async fn wait_for_state(
    status: &Arc<Mutex<RunnerStatus>>,
    expected_running: usize,
    expected_queued: usize,
) {
    for _ in 0..1000 {
        let done = {
            let st = status.lock().await;
            st.running_tasks_count == expected_running && st.queued_tasks_count == expected_queued
        };
        if done {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn test_execution_manager_concurrency_policy() {
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
    let sync_dir = tempfile::tempdir().unwrap();

    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 10,
        post_run_timeout_seconds: 10,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![
            create_mock_app("app_false", false),
            create_mock_app("app_true", true),
            create_mock_app("app_y", false),
            create_mock_app("app_z", true),
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

    // Helper macro to flush queue checks and unblock tasks
    macro_rules! reset {
        ($($files:expr),*) => {
            $(
                let _ = std::fs::write(&$files, "");
            )*
            let _ = exec_tx
                .send(ExecutionManagerCommand::ShutdownExecManager)
                .await;
            wait_for_state(&status, 0, 0).await;
            {
                let mut st = status.lock().await;
                st.running_tasks_count = 0;
                st.queued_tasks_count = 0;
            }
        };
    }

    // TEST 5: Same task duplicate blocked
    let sync1 = sync_dir.path().join("task_1_a.lock");
    let sync2 = sync_dir.path().join("task_1_b.lock");
    queue_task(create_long_task("task_1", vec!["app_true"], &sync1));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task("task_1", vec!["app_true"], &sync2));
    wait_for_state(&status, 1, 1).await;

    {
        let st = status.lock().await;
        assert_eq!(
            st.running_tasks_count, 1,
            "Duplicate task should not run concurrently"
        );
        assert_eq!(st.queued_tasks_count, 1, "Duplicate task should be queued");
    }
    reset!(sync1, sync2);
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
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

    // TEST 6: Same app, false -> NO LONGER blocked in the execution manager queue
    // (It now starts and acquires a dynamic lock inside the execution pipeline,
    //  so both tasks enter 'running' state from the manager's perspective,
    //  even though the 2nd task is sleeping on the semaphore inside pipeline.rs)
    let sync1 = sync_dir.path().join("t_false_1.lock");
    let sync2 = sync_dir.path().join("t_false_2.lock");
    queue_task(create_long_task("t_false_1", vec!["app_false"], &sync1));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task("t_false_2", vec!["app_false"], &sync2));
    wait_for_state(&status, 2, 0).await;

    {
        let st = status.lock().await;
        assert_eq!(
            st.running_tasks_count, 2,
            "Same app_false -> 2nd task enters running state to wait on dynamic lock"
        );
        assert_eq!(st.queued_tasks_count, 0, "No tasks queued in manager");
    }
    reset!(sync1, sync2);
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
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

    // TEST 7: Same app, true -> allowed
    let sync1 = sync_dir.path().join("t_true_1.lock");
    let sync2 = sync_dir.path().join("t_true_2.lock");
    queue_task(create_long_task("t_true_1", vec!["app_true"], &sync1));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task("t_true_2", vec!["app_true"], &sync2));
    wait_for_state(&status, 2, 0).await;

    {
        let st = status.lock().await;
        assert_eq!(
            st.running_tasks_count, 2,
            "Same app_true -> 2nd task allowed"
        );
        assert_eq!(st.queued_tasks_count, 0, "No tasks queued");
    }
    reset!(sync1, sync2);
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
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

    // TEST 8: Different apps -> allowed
    let sync1 = sync_dir.path().join("t_diff_1.lock");
    let sync2 = sync_dir.path().join("t_diff_2.lock");
    queue_task(create_long_task("t_diff_1", vec!["app_false"], &sync1));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task("t_diff_2", vec!["app_y"], &sync2));
    wait_for_state(&status, 2, 0).await;

    {
        let st = status.lock().await;
        assert_eq!(st.running_tasks_count, 2, "Different apps -> both allowed");
        assert_eq!(st.queued_tasks_count, 0, "No tasks queued");
    }
    reset!(sync1, sync2);
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
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

    // TEST 9: Multiple app dependencies, false -> allowed (waits internally at execution)
    let sync1 = sync_dir.path().join("t_multi_1.lock");
    let sync2 = sync_dir.path().join("t_multi_2.lock");
    queue_task(create_long_task(
        "t_multi_1",
        vec!["app_true", "app_false"],
        &sync1,
    ));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task(
        "t_multi_2",
        vec!["app_false", "app_z"],
        &sync2,
    ));
    wait_for_state(&status, 2, 0).await;

    {
        let st = status.lock().await;
        assert_eq!(
            st.running_tasks_count, 2,
            "Multi app enters running state concurrently"
        );
        assert_eq!(st.queued_tasks_count, 0, "No tasks queued");
    }
    reset!(sync1, sync2);
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
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

    // TEST 10: Multiple app dependencies, all allowed -> allowed
    let sync1 = sync_dir.path().join("t_multi_t1.lock");
    let sync2 = sync_dir.path().join("t_multi_t2.lock");
    queue_task(create_long_task(
        "t_multi_t1",
        vec!["app_true", "app_z"],
        &sync1,
    ));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task(
        "t_multi_t2",
        vec!["app_z", "app_true"],
        &sync2,
    ));
    wait_for_state(&status, 2, 0).await;

    {
        let st = status.lock().await;
        assert_eq!(
            st.running_tasks_count, 2,
            "Multi app with all true -> allowed"
        );
        assert_eq!(st.queued_tasks_count, 0, "No tasks queued");
    }
    reset!(sync1, sync2);
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
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

    // TEST 11: No ExternalApp -> existing behavior
    let sync1 = sync_dir.path().join("t_noapp_1.lock");
    let sync2 = sync_dir.path().join("t_noapp_2.lock");
    queue_task(create_long_task("t_noapp_1", vec![], &sync1));
    wait_for_state(&status, 1, 0).await;
    queue_task(create_long_task("t_noapp_2", vec![], &sync2));
    wait_for_state(&status, 2, 0).await;

    {
        let st = status.lock().await;
        assert_eq!(st.running_tasks_count, 2, "No app -> allowed");
        assert_eq!(st.queued_tasks_count, 0, "No tasks queued");
    }

    let _ = std::fs::write(&sync1, "");
    let _ = std::fs::write(&sync2, "");
}

// TEST 12: Race condition - Deterministic test avoiding arbitrary sleep where possible.
// Both tasks are submitted quickly. They should both enter the running state (since they are different tasks)
// but lock dynamically on the same application under the hood.
#[tokio::test]
async fn test_execution_manager_race_safety() {
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
    let sync_dir = tempfile::tempdir().unwrap();
    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 10,
        post_run_timeout_seconds: 10,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![create_mock_app("app_false", false)],
    };

    let sync1 = sync_dir.path().join("race_1.lock");
    let sync2 = sync_dir.path().join("race_2.lock");
    let t1 = create_long_task("race_1", vec!["app_false"], &sync1);
    let t2 = create_long_task("race_2", vec!["app_false"], &sync2);

    // Send both tasks as fast as possible to the channel queue to simulate a race condition.
    // The ExecutionManager channel receiver processes them in order.
    let _ = exec_tx
        .send(ExecutionManagerCommand::QueueTask {
            task: Box::new(t1),
            policy: policy.clone(),
        })
        .await;
    let _ = exec_tx
        .send(ExecutionManagerCommand::QueueTask {
            task: Box::new(t2),
            policy: policy.clone(),
        })
        .await;

    // Deterministically wait for the execution manager to process the queue
    for _ in 0..100 {
        let done = {
            let st = status.lock().await;
            st.running_tasks_count == 2 && st.queued_tasks_count == 0
        };
        if done {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let st = status.lock().await;
    assert_eq!(
        st.running_tasks_count, 2,
        "Both tasks should acquire execution permission and enter running state"
    );
    assert_eq!(st.queued_tasks_count, 0, "No tasks are queued safely");

    let _ = std::fs::write(&sync1, "");
    let _ = std::fs::write(&sync2, "");
}
