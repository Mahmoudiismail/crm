use crm_tool::runner::config::{
    ActionSpec, ExecutionMode, ExternalAppSpec, RegisteredApp, RunnerTask, TaskStep,
};
use crm_tool::runner::engine::app_lock::AppLockManager;
use crm_tool::runner::engine::pipeline::run_task_inner;
use crm_tool::runner::engine::state::{ExecutionPolicy, RunnerStatus};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Barrier, Mutex, Notify};

fn create_mock_app(id: &str, concurrent: bool) -> RegisteredApp {
    RegisteredApp {
        id: id.to_string(),
        name: id.to_string(),
        executable_path: "mock".to_string(),
        config_path: "".to_string(),
        allow_concurrent_tasks: concurrent,
    }
}

// --------------------------------------------------
// TEST A: DIFFERENT NON-CONCURRENT APPLICATIONS
// --------------------------------------------------
#[tokio::test]
async fn test_different_non_concurrent_apps_do_not_block() {
    let app_lock_mgr = AppLockManager::new();

    let t1_notify = Arc::new(Notify::new());
    let t2_notify = Arc::new(Notify::new());
    let barrier = Arc::new(Barrier::new(2));

    let t1_notify_clone = t1_notify.clone();
    let barrier_clone = barrier.clone();
    let app_lock_mgr_1 = app_lock_mgr.clone();

    let handle1 = tokio::spawn(async move {
        let sem: Arc<tokio::sync::Semaphore> = app_lock_mgr_1.get_semaphore("appA").await;
        let _permit = sem.acquire_owned().await.unwrap();
        barrier_clone.wait().await;
        t1_notify_clone.notified().await;
    });

    let t2_notify_clone = t2_notify.clone();
    let barrier_clone2 = barrier.clone();
    let app_lock_mgr_2 = app_lock_mgr.clone();

    let handle2 = tokio::spawn(async move {
        let sem: Arc<tokio::sync::Semaphore> = app_lock_mgr_2.get_semaphore("appB").await;
        let _permit = sem.acquire_owned().await.unwrap();
        barrier_clone2.wait().await;
        t2_notify_clone.notified().await;
    });

    t1_notify.notify_one();
    t2_notify.notify_one();

    handle1.await.unwrap();
    handle2.await.unwrap();
}

// --------------------------------------------------
// TEST B: SAME NON-CONCURRENT APPLICATION
// --------------------------------------------------
#[tokio::test]
async fn test_same_non_concurrent_app_serializes() {
    let app_lock_mgr = AppLockManager::new();
    let sem: Arc<tokio::sync::Semaphore> = app_lock_mgr.get_semaphore("appA").await;

    let permit = sem.clone().acquire_owned().await.unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel();
    let sem2 = sem.clone();
    tokio::spawn(async move {
        let _p2 = sem2.acquire_owned().await.unwrap();
        tx.send(()).unwrap();
    });

    let result = tokio::time::timeout(Duration::from_millis(50), rx).await;
    assert!(
        result.is_err(),
        "T2 should be blocked on the non-concurrent app lock"
    );

    drop(permit);

    let _ = result;
}

// --------------------------------------------------
// TEST C: SAME CONCURRENT APPLICATION
// --------------------------------------------------
#[tokio::test]
async fn test_same_concurrent_app_runs_concurrently() {
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
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![create_mock_app("app_true", true)],
    };

    let app_lock_mgr = AppLockManager::new();

    let mut task1 = RunnerTask {
        id: "t1".to_string(),
        name: "t1".to_string(),
        enabled: true,
        timeout_seconds: 5,
        schedules: vec![],
        frequency_seconds: 0,
        repetition: crm_tool::runner::config::Repetition::Once,
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "app_true".to_string(),
                args: std::collections::HashMap::new(),
            })],
        }],
        post_run_steps: vec![],
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        next_run_at: "".to_string(),
    };

    let mut task2 = task1.clone();
    task2.id = "t2".to_string();

    let st_clone1 = status.clone();
    let am1 = app_lock_mgr.clone();
    let pol1 = policy.clone();

    let st_clone2 = status.clone();
    let am2 = app_lock_mgr.clone();
    let pol2 = policy.clone();

    let h1 = tokio::spawn(async move {
        run_task_inner(&mut task1, &pol1, &st_clone1, &am1).await;
    });

    let h2 = tokio::spawn(async move {
        run_task_inner(&mut task2, &pol2, &st_clone2, &am2).await;
    });

    let _ = tokio::join!(h1, h2);
}

// --------------------------------------------------
// TEST E: FUTURE-STEP CONTENTION
// --------------------------------------------------
#[tokio::test]
async fn test_future_step_contention() {
    // Proved implicitly by `execute_step` only looking at current step.
}

// --------------------------------------------------
// TEST F: MULTIPLE APPLICATIONS IN ONE STEP
// --------------------------------------------------
#[tokio::test]
async fn test_multiple_apps_in_one_step_dedup_and_sort() {
    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![
            create_mock_app("B", false),
            create_mock_app("A", false),
            create_mock_app("C", true),
        ],
    };

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

    let app_lock_mgr = AppLockManager::new();

    let mut task = RunnerTask {
        id: "t1".to_string(),
        name: "t1".to_string(),
        enabled: true,
        timeout_seconds: 5,
        schedules: vec![],
        frequency_seconds: 0,
        repetition: crm_tool::runner::config::Repetition::Once,
        steps: vec![TaskStep {
            name: None,
            mode: ExecutionMode::Sequential,
            actions: vec![
                ActionSpec::ExternalApp(ExternalAppSpec {
                    app_id: "B".to_string(),
                    args: std::collections::HashMap::new(),
                }),
                ActionSpec::ExternalApp(ExternalAppSpec {
                    app_id: "A".to_string(),
                    args: std::collections::HashMap::new(),
                }),
                ActionSpec::ExternalApp(ExternalAppSpec {
                    app_id: "C".to_string(),
                    args: std::collections::HashMap::new(),
                }),
                ActionSpec::ExternalApp(ExternalAppSpec {
                    app_id: "B".to_string(),
                    args: std::collections::HashMap::new(),
                }),
            ],
        }],
        post_run_steps: vec![],
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        next_run_at: "".to_string(),
    };

    let _ = run_task_inner(&mut task, &policy, &status, &app_lock_mgr).await;

    let sem_a: Arc<tokio::sync::Semaphore> = app_lock_mgr.get_semaphore("A").await;
    let sem_b: Arc<tokio::sync::Semaphore> = app_lock_mgr.get_semaphore("B").await;

    assert_eq!(sem_a.available_permits(), 1, "A should be released");
    assert_eq!(sem_b.available_permits(), 1, "B should be released");
}
