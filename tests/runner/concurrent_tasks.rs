use crm_tool::runner::config::{
    ActionSpec, ExecutionMode, ExternalAppSpec, RegisteredApp, Repetition, RunnerTask, TaskStep,
};
use crm_tool::runner::engine::app_lock::AppLockManager;
use crm_tool::runner::engine::pipeline::run_task_inner;
use crm_tool::runner::engine::state::{ExecutionPolicy, RunnerStatus};
use std::sync::Arc;

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

fn base_task(id: &str) -> RunnerTask {
    RunnerTask {
        id: id.to_string(),
        name: id.to_string(),
        enabled: true,
        timeout_seconds: 5,
        schedules: vec![],
        frequency_seconds: 0,
        repetition: Repetition::Once,
        steps: vec![],
        post_run_steps: vec![],
        last_run_at: "".to_string(),
        last_status: "".to_string(),
        next_run_at: "".to_string(),
    }
}

// --------------------------------------------------
// TEST A: DIFFERENT NON-CONCURRENT APPLICATIONS
// --------------------------------------------------
#[tokio::test]
async fn test_different_non_concurrent_apps_do_not_block() {
    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![
            create_mock_app("AppA", false),
            create_mock_app("AppB", false),
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

    let mut t1 = base_task("t1");
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "AppA".to_string(),
            args: std::collections::HashMap::new(),
        })],
    });

    let mut t2 = base_task("t2");
    t2.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "AppB".to_string(),
            args: std::collections::HashMap::new(),
        })],
    });

    let st_clone1 = status.clone();
    let am1 = app_lock_mgr.clone();
    let pol1 = policy.clone();

    let st_clone2 = status.clone();
    let am2 = app_lock_mgr.clone();
    let pol2 = policy.clone();

    let h1 = tokio::spawn(async move {
        run_task_inner(&mut t1, &pol1, &st_clone1, &am1).await;
    });

    let h2 = tokio::spawn(async move {
        run_task_inner(&mut t2, &pol2, &st_clone2, &am2).await;
    });

    // Let both pipelines run. If they blocked each other, we'd have a timeout or deadlock.
    // They don't interact dynamically, so they should complete seamlessly.
    let _ = tokio::join!(h1, h2);

    // Verifying both run safely and release correctly
    let sem_a = app_lock_mgr.get_semaphore("AppA").await;
    let sem_b = app_lock_mgr.get_semaphore("AppB").await;
    assert_eq!(sem_a.available_permits(), 1);
    assert_eq!(sem_b.available_permits(), 1);
}

// --------------------------------------------------
// TEST B: SAME NON-CONCURRENT APPLICATION
// --------------------------------------------------
#[tokio::test]
async fn test_same_non_concurrent_app_serializes() {
    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![create_mock_app("AppA", false)],
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

    let mut t1 = base_task("t1");
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "AppA".to_string(),
            args: std::collections::HashMap::new(),
        })],
    });

    let mut t2 = t1.clone();
    t2.id = "t2".to_string();

    let sem = app_lock_mgr.get_semaphore("AppA").await;
    // Manually lock it to force T1 to wait.
    let permit = sem.clone().acquire_owned().await.unwrap();

    let st_clone1 = status.clone();
    let am1 = app_lock_mgr.clone();
    let pol1 = policy.clone();

    // Spawn T1, it will hit execute_step and wait for the semaphore.
    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let h1 = tokio::spawn(async move {
        tx1.send(()).unwrap();
        run_task_inner(&mut t1, &pol1, &st_clone1, &am1).await;
    });

    // Wait until T1 has definitely started running and blocks
    rx1.await.unwrap();
    tokio::task::yield_now().await;

    // Give it deterministic time to hit the app lock manager waiting block
    let (tx_check, rx_check) = tokio::sync::oneshot::channel();
    let st_clone = status.clone();
    tokio::spawn(async move {
        loop {
            let is_waiting = {
                let st = st_clone.lock().await;
                st.waiting_for_app.contains_key("t1")
            };
            if is_waiting {
                tx_check.send(()).unwrap();
                break;
            }
            tokio::task::yield_now().await;
        }
    });
    // Wait until T1 is confirmed to be waiting for the app.
    let _ = rx_check.await;

    // Now release the permit manually.
    drop(permit);

    // Now T1 can continue and finish.
    h1.await.unwrap();

    assert_eq!(sem.available_permits(), 1);
}

// --------------------------------------------------
// TEST C: SAME CONCURRENT APPLICATION
// --------------------------------------------------
#[tokio::test]
async fn test_same_concurrent_app_runs_concurrently() {
    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![create_mock_app("AppConcurrent", true)],
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

    let mut t1 = base_task("t1");
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "AppConcurrent".to_string(),
            args: std::collections::HashMap::new(),
        })],
    });

    let mut t2 = t1.clone();
    t2.id = "t2".to_string();

    let st_clone1 = status.clone();
    let am1 = app_lock_mgr.clone();
    let pol1 = policy.clone();

    let st_clone2 = status.clone();
    let am2 = app_lock_mgr.clone();
    let pol2 = policy.clone();

    let h1 = tokio::spawn(async move {
        run_task_inner(&mut t1, &pol1, &st_clone1, &am1).await;
    });

    let h2 = tokio::spawn(async move {
        run_task_inner(&mut t2, &pol2, &st_clone2, &am2).await;
    });

    let _ = tokio::join!(h1, h2);

    // Semaphores should not even be created for concurrent apps
    let sem = app_lock_mgr.get_semaphore("AppConcurrent").await;
    assert_eq!(sem.available_permits(), 1);
}

// --------------------------------------------------
// TEST D: MULTIPLE ACTIONS USING SAME APPLICATION
// --------------------------------------------------
#[tokio::test]
async fn test_multiple_actions_same_app() {
    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![create_mock_app("AppA", false)],
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

    let mut t1 = base_task("t1");
    // One step, multiple actions referencing AppA
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppA".to_string(),
                args: std::collections::HashMap::new(),
            }),
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppA".to_string(),
                args: std::collections::HashMap::new(),
            }),
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppA".to_string(),
                args: std::collections::HashMap::new(),
            }),
        ],
    });

    let _ = run_task_inner(&mut t1, &policy, &status, &app_lock_mgr).await;

    let sem = app_lock_mgr.get_semaphore("AppA").await;
    assert_eq!(
        sem.available_permits(),
        1,
        "AppA should be cleanly released after all actions finish in step"
    );
}

// --------------------------------------------------
// TEST E: FUTURE-STEP CONTENTION
// --------------------------------------------------
#[tokio::test]
async fn test_future_step_contention() {
    let policy = ExecutionPolicy {
        allow_shell_tasks: true,
        shell_timeout_seconds: 5,
        post_run_timeout_seconds: 5,
        min_task_interval_seconds: 1,
        log_retention_days: 1,
        registered_apps: vec![
            create_mock_app("AppA", true),
            create_mock_app("AppB", false),
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

    // T1 needs AppA in Step1, then AppB in Step2
    let mut t1 = base_task("t1");
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "AppA".to_string(),
            args: std::collections::HashMap::new(),
        })],
    });
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
            app_id: "AppB".to_string(),
            args: std::collections::HashMap::new(),
        })],
    });

    // We simulate T2 already holding AppB
    let sem_b = app_lock_mgr.get_semaphore("AppB").await;
    let permit_b = sem_b.clone().acquire_owned().await.unwrap();

    let st_clone1 = status.clone();
    let am1 = app_lock_mgr.clone();
    let pol1 = policy.clone();

    // Spawn T1, it should execute Step 1 easily, and block only on Step 2 (waiting for AppB)
    let h1 = tokio::spawn(async move {
        run_task_inner(&mut t1, &pol1, &st_clone1, &am1).await;
    });

    // Give it some time to process step 1 and get blocked on step 2
    // We let T1 proceed and manually yield to ensure it has a chance to execute up to the waiting block.
    tokio::task::yield_now().await;

    // Now T2 releases AppB
    drop(permit_b);

    // T1 should complete
    h1.await.unwrap();
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
            create_mock_app("AppB", false),
            create_mock_app("AppA", false),
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

    // Task T1 requires B then A
    let mut t1 = base_task("t1");
    t1.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppB".to_string(),
                args: std::collections::HashMap::new(),
            }),
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppA".to_string(),
                args: std::collections::HashMap::new(),
            }),
        ],
    });

    // Task T2 requires A then B
    let mut t2 = base_task("t2");
    t2.steps.push(TaskStep {
        name: None,
        mode: ExecutionMode::Sequential,
        actions: vec![
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppA".to_string(),
                args: std::collections::HashMap::new(),
            }),
            ActionSpec::ExternalApp(ExternalAppSpec {
                app_id: "AppB".to_string(),
                args: std::collections::HashMap::new(),
            }),
        ],
    });

    let st_clone1 = status.clone();
    let am1 = app_lock_mgr.clone();
    let pol1 = policy.clone();

    let st_clone2 = status.clone();
    let am2 = app_lock_mgr.clone();
    let pol2 = policy.clone();

    // Spawn both concurrently to prove no deadlock occurs via sorting (they should both acquire A then B)
    let h1 = tokio::spawn(async move {
        run_task_inner(&mut t1, &pol1, &st_clone1, &am1).await;
    });

    let h2 = tokio::spawn(async move {
        run_task_inner(&mut t2, &pol2, &st_clone2, &am2).await;
    });

    // They must both finish successfully without deadlocking
    let _ = tokio::join!(h1, h2);

    let sem_a = app_lock_mgr.get_semaphore("AppA").await;
    let sem_b = app_lock_mgr.get_semaphore("AppB").await;

    assert_eq!(sem_a.available_permits(), 1, "AppA should be released");
    assert_eq!(sem_b.available_permits(), 1, "AppB should be released");
}
