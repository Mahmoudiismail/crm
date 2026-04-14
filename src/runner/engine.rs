use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::crm;
use crate::crm::types::ReportType;

use super::config::{Repetition, RunnerConfig, RunnerTask, TaskKind};

#[derive(Debug, Clone)]
pub enum RunnerCommand {
    RunAllNow,
    RunTaskNow(String),
    RunAdhocCrm(ReportType),
    SetTaskEnabled { task_id: String, enabled: bool },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunnerStatus {
    pub currently_running: bool,
    pub last_error: String,
    pub last_task_id: String,
    pub last_run_at: String,
}

#[derive(Debug, Clone)]
struct ExecutionPolicy {
    crm_config_path: String,
    allow_shell_tasks: bool,
    shell_timeout_seconds: u64,
    min_task_interval_seconds: u64,
}

#[derive(Clone)]
pub struct RunnerHandle {
    pub command_tx: mpsc::Sender<RunnerCommand>,
    pub status: Arc<Mutex<RunnerStatus>>,
    pub runner_config_path: String,
}

pub fn start_scheduler(runner_config_path: String) -> RunnerHandle {
    let (tx, mut rx) = mpsc::channel::<RunnerCommand>(64);
    let status = Arc::new(Mutex::new(RunnerStatus {
        currently_running: false,
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
    }));

    let status_bg = status.clone();
    let path_bg = runner_config_path.clone();

    tokio::spawn(async move {
        info!("Runner scheduler started");
        loop {
            let poll = RunnerConfig::load(&path_bg)
                .map(|c| c.poll_interval_seconds)
                .unwrap_or(30);

            tokio::select! {
                maybe_cmd = rx.recv() => {
                    match maybe_cmd {
                        Some(cmd) => {
                            if let Err(e) = handle_command(&path_bg, cmd, &status_bg).await {
                                error!("Runner command failed: {:#}", e);
                                let mut st = status_bg.lock().await;
                                st.last_error = format!("{}", e);
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(poll.max(5))) => {
                    if let Err(e) = run_due_tasks(&path_bg, &status_bg).await {
                        error!("Due task cycle failed: {:#}", e);
                        let mut st = status_bg.lock().await;
                        st.last_error = format!("{}", e);
                    }
                }
            }
        }
    });

    RunnerHandle {
        command_tx: tx,
        status,
        runner_config_path,
    }
}

async fn handle_command(path: &str, cmd: RunnerCommand, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    match cmd {
        RunnerCommand::RunAllNow => run_all_tasks_now(path, status).await,
        RunnerCommand::RunTaskNow(task_id) => run_task_by_id(path, &task_id, status).await,
        RunnerCommand::RunAdhocCrm(report) => run_adhoc_crm(path, report, status).await,
        RunnerCommand::SetTaskEnabled { task_id, enabled } => {
            set_task_enabled(path, &task_id, enabled).await
        }
    }
}

pub async fn create_task(path: &str, mut task: RunnerTask) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    normalize_and_validate_task(&mut task, &cfg)?;

    if cfg.tasks.iter().any(|t| t.id == task.id) {
        return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
    }

    cfg.tasks.push(task);
    cfg.save(path)?;
    Ok(())
}

pub async fn update_task(path: &str, task_id: &str, mut task: RunnerTask) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let Some(existing_idx) = cfg.tasks.iter().position(|t| t.id == task_id) else {
        return Err(anyhow::anyhow!("Task '{}' not found", task_id));
    };

    if task.id.trim().is_empty() {
        task.id = task_id.to_string();
    }
    normalize_and_validate_task(&mut task, &cfg)?;

    if cfg
        .tasks
        .iter()
        .enumerate()
        .any(|(idx, t)| idx != existing_idx && t.id == task.id)
    {
        return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
    }

    if task.last_run_at.is_empty() {
        task.last_run_at = cfg.tasks[existing_idx].last_run_at.clone();
    }
    if task.last_status.is_empty() {
        task.last_status = cfg.tasks[existing_idx].last_status.clone();
    }

    cfg.tasks[existing_idx] = task;
    cfg.save(path)?;
    Ok(())
}

pub async fn delete_task(path: &str, task_id: &str) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let initial_len = cfg.tasks.len();
    cfg.tasks.retain(|t| t.id != task_id);
    if cfg.tasks.len() == initial_len {
        return Err(anyhow::anyhow!("Task '{}' not found", task_id));
    }
    cfg.save(path)?;
    Ok(())
}

pub async fn run_due_tasks(path: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    for task in &mut cfg.tasks {
        if task.due_now(now) {
            run_task(task, &policy, status).await;
            update_next_run(task, now, policy.min_task_interval_seconds);
        }
    }

    cfg.save(path)?;
    Ok(())
}

async fn run_all_tasks_now(path: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);
    for task in &mut cfg.tasks {
        if task.enabled {
            run_task(task, &policy, status).await;
            update_next_run(task, now, policy.min_task_interval_seconds);
        }
    }
    cfg.save(path)?;
    Ok(())
}

async fn run_task_by_id(path: &str, task_id: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        run_task(task, &policy, status).await;
        update_next_run(task, now, policy.min_task_interval_seconds);
        cfg.save(path)?;
        return Ok(());
    }

    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

async fn run_adhoc_crm(path: &str, report: ReportType, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let cfg = RunnerConfig::load(path)?;
    let policy = policy_from_config(&cfg);
    let mut task = RunnerTask {
        id: "adhoc_crm".to_string(),
        name: "Adhoc CRM Run".to_string(),
        enabled: true,
        repetition: Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        skip_login: false,
        output: None,
        kind: TaskKind::CrmFetch { report },
        last_run_at: String::new(),
        last_status: String::new(),
    };

    run_task(&mut task, &policy, status).await;
    Ok(())
}

async fn set_task_enabled(path: &str, task_id: &str, enabled: bool) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        task.enabled = enabled;
        if enabled && task.next_run_at.is_empty() {
            task.next_run_at = Utc::now().to_rfc3339();
        }
        cfg.save(path)?;
        return Ok(());
    }
    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

fn normalize_and_validate_task(task: &mut RunnerTask, cfg: &RunnerConfig) -> Result<()> {
    task.id = task.id.trim().to_string();
    task.name = task.name.trim().to_string();
    task.next_run_at = task.next_run_at.trim().to_string();

    if task.id.is_empty() {
        return Err(anyhow::anyhow!("Task id is required"));
    }
    if !is_valid_task_id(&task.id) {
        return Err(anyhow::anyhow!(
            "Invalid task id '{}'. Use letters, numbers, '-' or '_'",
            task.id
        ));
    }
    if task.name.is_empty() {
        return Err(anyhow::anyhow!("Task name is required"));
    }

    if !task.next_run_at.is_empty() {
        DateTime::parse_from_rfc3339(&task.next_run_at)
            .with_context(|| format!("Invalid next_run_at timestamp '{}'. Use RFC3339", task.next_run_at))?;
    }

    if matches!(task.repetition, Repetition::Repeat) {
        task.frequency_seconds = task
            .frequency_seconds
            .max(cfg.min_task_interval_seconds.max(1));
    }

    if let Some(output) = task.output.as_mut() {
        let trimmed = output.trim().to_string();
        if trimmed.is_empty() {
            task.output = None;
        } else {
            *output = trimmed;
        }
    }

    match &mut task.kind {
        TaskKind::CrmFetch { .. } => {}
        TaskKind::ShellCommand { command } => {
            *command = command.trim().to_string();
            if command.is_empty() {
                return Err(anyhow::anyhow!("shell_command requires a non-empty command"));
            }
        }
    }

    Ok(())
}

fn is_valid_task_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn update_next_run(task: &mut RunnerTask, now: DateTime<Utc>, min_task_interval_seconds: u64) {
    task.last_run_at = now.to_rfc3339();
    match task.repetition {
        Repetition::Once => {
            task.enabled = false;
            task.next_run_at = String::new();
        }
        Repetition::Repeat => {
            let effective_frequency = task.frequency_seconds.max(min_task_interval_seconds.max(1));
            let next = now + chrono::TimeDelta::seconds(effective_frequency as i64);
            task.next_run_at = next.to_rfc3339();
        }
    }
}

fn policy_from_config(cfg: &RunnerConfig) -> ExecutionPolicy {
    ExecutionPolicy {
        crm_config_path: cfg.crm_config_path.clone(),
        allow_shell_tasks: cfg.allow_shell_tasks,
        shell_timeout_seconds: cfg.shell_timeout_seconds.max(1),
        min_task_interval_seconds: cfg.min_task_interval_seconds.max(1),
    }
}

async fn run_task(task: &mut RunnerTask, policy: &ExecutionPolicy, status: &Arc<Mutex<RunnerStatus>>) {
    {
        let mut st = status.lock().await;
        if st.currently_running {
            task.last_status = "skipped: another task is running".to_string();
            return;
        }
        st.currently_running = true;
        st.last_error.clear();
        st.last_task_id = task.id.clone();
    }

    let result = match &task.kind {
        TaskKind::CrmFetch { report } => {
            crm::run_once(&policy.crm_config_path, *report, task.skip_login, task.output.clone())
                .await
        }
        TaskKind::ShellCommand { command } => {
            if !policy.allow_shell_tasks {
                Err(anyhow::anyhow!("shell_command tasks are disabled by runner config"))
            } else {
                run_shell_command(command, policy.shell_timeout_seconds).await
            }
        }
    };

    match result {
        Ok(_) => task.last_status = "ok".to_string(),
        Err(e) => {
            task.last_status = format!("error: {}", e);
            let mut st = status.lock().await;
            st.last_error = format!("{}", e);
        }
    }

    let mut st = status.lock().await;
    st.last_run_at = Utc::now().to_rfc3339();
    st.currently_running = false;
}

async fn run_shell_command(command: &str, shell_timeout_seconds: u64) -> Result<()> {
    let output = tokio::time::timeout(
        Duration::from_secs(shell_timeout_seconds.max(1)),
        tokio::process::Command::new("bash")
            .arg("-lc")
            .arg(command)
            .output(),
    )
    .await
    .with_context(|| {
        format!(
            "Command timed out after {}s: {}",
            shell_timeout_seconds, command
        )
    })??;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
