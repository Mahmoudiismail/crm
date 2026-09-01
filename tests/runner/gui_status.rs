use crm_tool::runner::config::{
    ActionSpec, ExecutionMode, ExternalAppSpec, RegisteredApp, Repetition, RunnerTask, TaskStep,
};
use crm_tool::runner::engine::app_lock::AppLockManager;
use crm_tool::runner::engine::pipeline::run_task_inner;
use crm_tool::runner::engine::state::{ExecutionPolicy, RunnerStatus};
use std::sync::Arc;
use std::time::Duration;
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

// --------------------------------------------------
// TEST: GUI STATUS WAITING FOR NON-CONCURRENT APP
// --------------------------------------------------
#[tokio::test]
async fn test_gui_status_waiting_for_app_deterministic() {
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

    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 10,
        post_run_timeout_seconds: 10,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![create_mock_app("App B", false)],
    };

    let app_lock_mgr = AppLockManager::new();

    let mut task = RunnerTask {
        id: "t1".to_string(),
        name: "t1".to_string(),
        enabled: true,
        timeout_seconds: 5,
        schedules: vec![],
        frequency_seconds: 0,
        repetition: Repetition::Once,
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "App B".to_string(),
                args: std::collections::HashMap::new(),
            })],
        }],
        post_run_steps: vec![],
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        next_run_at: "".to_string(),
    };

    // To test "waiting_for_app", we must make sure another thread holds the lock first.
    let sem = app_lock_mgr.get_semaphore("App B").await;
    let permit = sem.clone().acquire_owned().await.unwrap();

    let st_clone = status.clone();
    let am = app_lock_mgr.clone();
    let pol = policy.clone();

    // Start execution
    let handle = tokio::spawn(async move { run_task_inner(&mut task, &pol, &st_clone, &am).await });

    // Wait until it marks itself as waiting
    for _ in 0..100 {
        let is_waiting = {
            let st = status.lock().await;
            st.waiting_for_app.contains_key("t1")
        };
        if is_waiting {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    {
        let st = status.lock().await;
        assert_eq!(
            st.waiting_for_app.get("t1").unwrap(),
            "App B",
            "GUI should correctly report that t1 is waiting for App B"
        );
    }

    // Now release the permit
    drop(permit);

    // Wait until it finishes
    let _ = handle.await.unwrap();

    // Verify it cleans up properly
    {
        let st = status.lock().await;
        assert!(
            !st.waiting_for_app.contains_key("t1"),
            "waiting_for_app state should be cleared after completion"
        );
    }
}
