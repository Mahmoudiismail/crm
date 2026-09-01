use crate::runner::config::RegisteredApp;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum RunnerCommand {
    RunAllNow {
        is_manual: bool,
    },
    RunTaskNow {
        task_id: String,
        is_manual: bool,
    },
    SetTaskEnabled {
        task_id: String,
        enabled: bool,
    },
    CreateWorkingHoursProfile {
        profile: crate::runner::config::WorkingHoursProfile,
    },
    UpdateWorkingHoursProfile {
        profile: crate::runner::config::WorkingHoursProfile,
    },
    DeleteWorkingHoursProfile {
        profile_id: String,
    },
    Shutdown,
}

use std::collections::HashMap;
use tokio::sync::Semaphore;

pub type AppLockManager = Arc<Mutex<HashMap<String, Arc<Semaphore>>>>;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunnerStatus {
    pub running_tasks_count: usize,
    pub queued_tasks_count: usize,
    pub running_task_ids: Vec<String>,
    pub queued_task_ids: Vec<String>,
    pub last_error: String,
    pub last_task_id: String,
    pub last_run_at: String,
    pub waiting_for_app: HashMap<String, String>, // task_id -> app_id
}

#[derive(Debug)]
pub enum ExecutionManagerCommand {
    QueueTask {
        task: Box<crate::runner::config::RunnerTask>,
        policy: ExecutionPolicy,
    },
    TaskFinished {
        task_id: String,
        last_status: String,
        last_error: Option<String>,
    },
    ShutdownExecManager,
}

#[derive(Clone, Debug)]
pub struct ExecutionPolicy {
    pub allow_shell_tasks: bool,
    pub shell_timeout_seconds: u64,
    pub post_run_timeout_seconds: u64,
    pub min_task_interval_seconds: u64,
    pub registered_apps: Vec<RegisteredApp>,
    pub log_retention_days: u64,
}

#[derive(Clone, Debug)]
pub struct RunnerHandle {
    pub command_tx: tokio::sync::mpsc::Sender<RunnerCommand>,
    pub exec_tx: tokio::sync::mpsc::Sender<ExecutionManagerCommand>,
    pub status: Arc<Mutex<RunnerStatus>>,
    pub runner_config_path: String,
}
