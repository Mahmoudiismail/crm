use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};

use super::config::{
    next_daily_run_after, next_monthly_run_after, parse_rfc3339_utc, Repetition, ReportType,
    RunnerConfig, RunnerTask, ShellCommandMode, ShellCommandSpec, TaskKind, TaskSchedule,
};

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

#[derive(Clone)]
struct TaskLogger {
    inner: Arc<Mutex<TaskLoggerInner>>,
}

impl TaskLogger {
    fn new(task_id: &str, task_name: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TaskLoggerInner::new(task_id, task_name))),
        }
    }

    async fn log(&self, message: &str) {
        let mut inner = self.inner.lock().await;
        inner.log(message);
    }

    async fn log_bytes(&self, prefix: &str, bytes: &[u8]) {
        let mut inner = self.inner.lock().await;
        inner.log_bytes(prefix, bytes);
    }
}
struct TaskLoggerInner {
    file: Option<fs::File>,
    task_id: String,
}

impl TaskLoggerInner {
    fn new(task_id: &str, task_name: &str) -> Self {
        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let safe_task_name = task_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let filename = format!("{}_{}_{}.log", timestamp, safe_task_name, task_id);

        let log_dir = match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .map(|p| p.join("logs").join(&safe_task_name))
                .unwrap_or_else(|| std::path::PathBuf::from("logs").join(&safe_task_name)),
            Err(_) => std::path::PathBuf::from("logs").join(&safe_task_name),
        };

        if let Err(e) = fs::create_dir_all(&log_dir) {
            error!(
                "Failed to create log directory {}: {}",
                log_dir.display(),
                e
            );
            return Self {
                file: None,
                task_id: task_id.to_string(),
            };
        }

        let log_path = log_dir.join(filename);
        match fs::File::create(&log_path) {
            Ok(file) => {
                let mut logger = Self {
                    file: Some(file),
                    task_id: task_id.to_string(),
                };
                logger.log(&format!("Task ID: {}", task_id));
                logger.log(&format!("Task Name: {}", task_name));
                logger.log(&format!("Start Time: {}", now.to_rfc3339()));
                logger.log("--------------------------------------------------");
                logger
            }
            Err(e) => {
                error!("Failed to create log file {}: {}", log_path.display(), e);
                Self {
                    file: None,
                    task_id: task_id.to_string(),
                }
            }
        }
    }

    fn log(&mut self, message: &str) {
        let now = Utc::now().to_rfc3339();
        let line = format!("[{}] {}\n", now, message);
        if let Some(ref mut f) = self.file {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
        debug!("[{}] {}", self.task_id, message);
    }

    fn log_bytes(&mut self, prefix: &str, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            self.log(&format!("{}: {}", prefix, line));
        }
    }
}
struct ExecutionPolicy {
    crm_config_path: String,
    crm_executable_path: String,
    allow_shell_tasks: bool,
    shell_timeout_seconds: u64,
    post_run_timeout_seconds: u64,
    min_task_interval_seconds: u64,
}

#[derive(Clone)]
pub struct RunnerHandle {
    pub command_tx: mpsc::Sender<RunnerCommand>,
    pub status: Arc<Mutex<RunnerStatus>>,
    pub runner_config_path: String,
}

pub fn start_scheduler(runner_config_path: String) -> RunnerHandle {
    info!("Starting scheduler with config: {}", runner_config_path);
    let (tx, mut rx) = mpsc::channel::<RunnerCommand>(64);
    let status = Arc::new(Mutex::new(RunnerStatus {
        currently_running: false,
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
    }));

    let status_bg = status.clone();
    let config_path = runner_config_path.clone();

    // Get initial config modification time
    let get_mod_time = |p: &str| -> Option<SystemTime> { fs::metadata(p).ok()?.modified().ok() };

    let mut last_modified = get_mod_time(&config_path).unwrap_or(SystemTime::now());

    // Main loop: handle commands and cron-based scheduling
    let config_path_loop = config_path.clone();
    let poll_interval = RunnerConfig::load(&config_path)
        .map(|c| c.poll_interval_seconds.max(5))
        .unwrap_or(30);

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe_cmd = rx.recv() => {
                    match maybe_cmd {
                        Some(cmd) => {
                            if let Err(e) = handle_command(&config_path_loop, cmd, &status_bg).await {
                                error!("Runner command failed: {:#}", e);
                                let mut st = status_bg.lock().await;
                                st.last_error = format!("{}", e);
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(poll_interval)) => {
                    // Check for due tasks based on cron expressions
                    if let Ok(cfg) = RunnerConfig::load(&config_path_loop) {
                        let now = Utc::now();
                        for task in &cfg.tasks {
                            if !task.enabled {
                                continue;
                            }
                            // Check if task is due based on cron schedules
                            for schedule in &task.schedules {
                                if schedule_is_due(schedule, now) {
                                    info!("Cron schedule triggered for task: {}", task.id);
                                    let _ = tx_clone.send(RunnerCommand::RunTaskNow(task.id.clone())).await;
                                    break;
                                }
                            }
                        }
                    }

                    // Check for config file changes
                    let current_modified = get_mod_time(&config_path_loop);
                    if let Some(now_modified) = current_modified {
                        if now_modified > last_modified {
                            info!("Config file changed");
                            last_modified = now_modified;
                        }
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

async fn handle_command(
    path: &str,
    cmd: RunnerCommand,
    status: &Arc<Mutex<RunnerStatus>>,
) -> Result<()> {
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

async fn run_task_by_id(
    path: &str,
    task_id: &str,
    status: &Arc<Mutex<RunnerStatus>>,
) -> Result<()> {
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

async fn run_adhoc_crm(
    path: &str,
    report: ReportType,
    status: &Arc<Mutex<RunnerStatus>>,
) -> Result<()> {
    let cfg = RunnerConfig::load(path)?;
    let policy = policy_from_config(&cfg);
    let mut task = RunnerTask {
        id: "adhoc_crm".to_string(),
        name: "Adhoc CRM Run".to_string(),
        enabled: true,
        repetition: Repetition::Once,
        frequency_seconds: 0,
        next_run_at: String::new(),
        schedules: Vec::new(),
        kind: TaskKind::CrmFetch { report },
        last_run_at: String::new(),
        last_status: String::new(),
        post_run_script: String::new(),
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
        for schedule in &mut task.schedules {
            set_schedule_enabled(schedule, enabled);
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
        parse_rfc3339_utc(&task.next_run_at).with_context(|| {
            format!(
                "Invalid next_run_at timestamp '{}'. Use RFC3339",
                task.next_run_at
            )
        })?;
    }

    if matches!(task.repetition, Repetition::Repeat) {
        task.frequency_seconds = task
            .frequency_seconds
            .max(cfg.min_task_interval_seconds.max(1));
    }

    normalize_and_validate_schedules(task, cfg.min_task_interval_seconds.max(1))?;

    match &mut task.kind {
        TaskKind::CrmFetch { .. } => {}
        TaskKind::ShellCommand { commands, .. } => {
            commands.retain(|c| !c.command.trim().is_empty());
            for c in commands.iter_mut() {
                c.command = c.command.trim().to_string();
            }
            if commands.is_empty() {
                return Err(anyhow::anyhow!(
                    "shell_command requires at least one non-empty command"
                ));
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
    if !task.schedules.is_empty() {
        for schedule in &mut task.schedules {
            if schedule.due_now(now) {
                advance_schedule(schedule, now, min_task_interval_seconds);
            }
        }
        return;
    }

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
        crm_executable_path: cfg.crm_executable_path.clone(),
        allow_shell_tasks: cfg.allow_shell_tasks,
        shell_timeout_seconds: cfg.shell_timeout_seconds,
        post_run_timeout_seconds: cfg.post_run_timeout_seconds,
        min_task_interval_seconds: cfg.min_task_interval_seconds.max(1),
    }
}

async fn run_task(
    task: &mut RunnerTask,
    policy: &ExecutionPolicy,
    status: &Arc<Mutex<RunnerStatus>>,
) {
    let logger = TaskLogger::new(&task.id, &task.name);
    logger.log("Initializing task execution...").await;

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
            run_crm_command(
                &logger,
                &policy.crm_executable_path,
                &policy.crm_config_path,
                *report,
                policy.shell_timeout_seconds,
            )
            .await
        }
        TaskKind::ShellCommand { mode, commands } => {
            if !policy.allow_shell_tasks {
                Err(anyhow::anyhow!(
                    "shell_command tasks are disabled by runner config"
                ))
            } else {
                match mode {
                    ShellCommandMode::Sequential => {
                        run_shell_sequential(&logger, commands, policy.shell_timeout_seconds).await
                    }
                    ShellCommandMode::Parallel => {
                        run_shell_parallel(&logger, commands, policy.shell_timeout_seconds).await
                    }
                }
            }
        }
    };

    match result {
        Ok(_) => {
            if !task.post_run_script.trim().is_empty() {
                match run_post_run_script(
                    &logger,
                    &task.post_run_script,
                    policy.post_run_timeout_seconds,
                )
                .await
                {
                    Ok(_) => task.last_status = "ok".to_string(),
                    Err(e) => {
                        task.last_status = format!("post-run script error: {}", e);
                        let mut st = status.lock().await;
                        st.last_error = format!("post-run script error: {}", e);
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
        }
    }

    let mut st = status.lock().await;
    st.last_run_at = Utc::now().to_rfc3339();
    st.currently_running = false;
}

async fn run_crm_command(
    logger: &TaskLogger,
    executable_path: &str,
    config_path: &str,
    report: ReportType,
    timeout_seconds: u64,
) -> Result<()> {
    let resolved_executable = resolve_crm_executable(executable_path);
    let resolved_config_path = resolve_relative_to_exe_dir(config_path);

    let mut command = tokio::process::Command::new(&resolved_executable);
    command.arg("--config").arg(&resolved_config_path);
    command.arg("--report").arg(report.as_arg());
    logger
        .log(&format!("Executing CRM command: {:?}", command))
        .await;

    let output = tokio::time::timeout(
        Duration::from_secs(timeout_seconds.max(1)),
        command.output(),
    )
    .await
    .with_context(|| {
        format!(
            "crm command timed out after {}s ({})",
            timeout_seconds,
            resolved_executable.display()
        )
    })??;
    logger.log_bytes("STDOUT", &output.stdout).await;
    logger.log_bytes("STDERR", &output.stderr).await;
    if !output.status.success() {
        let stdout_excerpt = excerpt_utf8(&output.stdout);
        let stderr_excerpt = excerpt_utf8(&output.stderr);
        return Err(anyhow::anyhow!(
            "crm command failed ({}) with status {:?}; stderr: {}; stdout: {}",
            resolved_executable.display(),
            output.status.code(),
            stderr_excerpt,
            stdout_excerpt
        ));
    }

    Ok(())
}

fn resolve_crm_executable(configured: &str) -> std::path::PathBuf {
    let configured = configured.trim();
    let configured_name = if configured.is_empty() {
        default_crm_binary_name().to_string()
    } else {
        configured.to_string()
    };

    let configured_path = std::path::PathBuf::from(&configured_name);
    if configured_path.is_absolute() {
        return configured_path;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let sibling = exe_dir.join(&configured_name);
            if sibling.exists() {
                return sibling;
            }

            if configured.is_empty() {
                let default_sibling = exe_dir.join(default_crm_binary_name());
                if default_sibling.exists() {
                    return default_sibling;
                }
            }
        }
    }

    configured_path
}

fn resolve_relative_to_exe_dir(path: &str) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            return exe_dir.join(p);
        }
    }

    p
}

fn default_crm_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "crm.exe"
    } else {
        "crm"
    }
}

fn excerpt_utf8(bytes: &[u8]) -> String {
    const MAX: usize = 400;
    let text = String::from_utf8_lossy(bytes).replace(['\n', '\r'], " ");
    if text.len() > MAX {
        format!("{}...", &text[..MAX])
    } else if text.is_empty() {
        "<empty>".to_string()
    } else {
        text
    }
}

async fn run_post_run_script(
    logger: &TaskLogger,
    script_path: &str,
    timeout_seconds: u64,
) -> Result<()> {
    let path = std::path::Path::new(script_path);
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let mut command = match ext.as_str() {
        "txt" | "vbs" => {
            let mut cmd = tokio::process::Command::new("cscript.exe");
            cmd.arg("//NoLogo").arg(script_path);
            cmd
        }
        "bat" | "cmd" => {
            let mut cmd = tokio::process::Command::new("cmd.exe");
            cmd.arg("/c").arg(script_path);
            cmd
        }
        "ps1" => {
            let mut cmd = tokio::process::Command::new("powershell.exe");
            cmd.arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-File")
                .arg(script_path);
            cmd
        }
        _ => tokio::process::Command::new(script_path),
    };
    logger
        .log(&format!("Executing post-run script: {:?}", command))
        .await;

    let output = if timeout_seconds == 0 {
        command.output().await?
    } else {
        tokio::time::timeout(Duration::from_secs(timeout_seconds), command.output())
            .await
            .with_context(|| {
                format!(
                    "post-run script timed out after {}s: {}",
                    timeout_seconds, script_path
                )
            })??
    };

    logger.log_bytes("STDOUT", &output.stdout).await;
    logger.log_bytes("STDERR", &output.stderr).await;
    if !output.status.success() {
        let stdout_excerpt = excerpt_utf8(&output.stdout);
        let stderr_excerpt = excerpt_utf8(&output.stderr);
        return Err(anyhow::anyhow!(
            "script failed ({:?}): stderr: {}; stdout: {}",
            output.status.code(),
            stderr_excerpt,
            stdout_excerpt
        ));
    }

    Ok(())
}

async fn run_shell_command(
    logger: &TaskLogger,
    command: &str,
    shell_timeout_seconds: u64,
) -> Result<()> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.arg("/c").arg(command);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-lc").arg(command);
        c
    };

    logger
        .log(&format!("Executing shell command: {}", command))
        .await;

    let output = if shell_timeout_seconds == 0 {
        cmd.output().await?
    } else {
        tokio::time::timeout(Duration::from_secs(shell_timeout_seconds), cmd.output())
            .await
            .with_context(|| {
                format!(
                    "Command timed out after {}s: {}",
                    shell_timeout_seconds, command
                )
            })??
    };

    logger.log_bytes("STDOUT", &output.stdout).await;
    logger.log_bytes("STDERR", &output.stderr).await;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn normalize_and_validate_schedules(
    task: &mut RunnerTask,
    min_task_interval_seconds: u64,
) -> Result<()> {
    for schedule in &mut task.schedules {
        match schedule {
            TaskSchedule::Once { next_run_at, .. } => {
                *next_run_at = next_run_at.trim().to_string();
                if !next_run_at.is_empty() {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!("Invalid once schedule '{}'. Use RFC3339", next_run_at)
                    })?;
                }
            }
            TaskSchedule::Interval {
                every_seconds,
                next_run_at,
                ..
            } => {
                *every_seconds = (*every_seconds).max(min_task_interval_seconds);
                *next_run_at = next_run_at.trim().to_string();
                // Always compute next_run_at based on interval to ensure it reflects the current schedule
                let next = Utc::now() + chrono::TimeDelta::seconds(*every_seconds as i64);
                *next_run_at = next.to_rfc3339();
            }
            TaskSchedule::DailyTimes {
                times, next_run_at, ..
            } => {
                times.retain(|time| !time.trim().is_empty());
                for time in times.iter_mut() {
                    *time = time.trim().to_string();
                    chrono::NaiveTime::parse_from_str(time, "%H:%M")
                        .with_context(|| format!("Invalid daily time '{}'. Use HH:MM", time))?;
                }
                if times.is_empty() {
                    return Err(anyhow::anyhow!(
                        "daily_times schedule requires at least one HH:MM time"
                    ));
                }
                *next_run_at = next_run_at.trim().to_string();
                if next_run_at.is_empty() {
                    *next_run_at = next_daily_run_after(times, Utc::now())?;
                } else {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!(
                            "Invalid daily_times next_run_at '{}'. Use RFC3339",
                            next_run_at
                        )
                    })?;
                }
            }
            TaskSchedule::Weekly {
                day_of_week,
                at_time,
                next_run_at,
                ..
            } => {
                *day_of_week = day_of_week.trim().to_string();
                *at_time = at_time.trim().to_string();
                if !at_time.is_empty() {
                    chrono::NaiveTime::parse_from_str(at_time, "%H:%M")
                        .with_context(|| format!("Invalid weekly time '{}'. Use HH:MM", at_time))?;
                }
                *next_run_at = next_run_at.trim().to_string();
            }
            TaskSchedule::Monthly {
                day_of_month,
                at_time,
                next_run_at,
                ..
            } => {
                *day_of_month = (*day_of_month).clamp(1, 31);
                *at_time = at_time.trim().to_string();
                if !at_time.is_empty() {
                    chrono::NaiveTime::parse_from_str(at_time, "%H:%M").with_context(|| {
                        format!("Invalid monthly time '{}'. Use HH:MM", at_time)
                    })?;
                }
                // Compute next_run_at
                *next_run_at = next_monthly_run_after(*day_of_month, at_time, Utc::now())?;
            }
        }
    }

    Ok(())
}

fn set_schedule_enabled(schedule: &mut TaskSchedule, enabled_value: bool) {
    match schedule {
        TaskSchedule::Once { enabled, .. }
        | TaskSchedule::Interval { enabled, .. }
        | TaskSchedule::DailyTimes { enabled, .. }
        | TaskSchedule::Weekly { enabled, .. }
        | TaskSchedule::Monthly { enabled, .. } => *enabled = enabled_value,
    }
}

fn advance_schedule(
    schedule: &mut TaskSchedule,
    now: DateTime<Utc>,
    min_task_interval_seconds: u64,
) {
    match schedule {
        TaskSchedule::Once {
            enabled,
            next_run_at,
        } => {
            *enabled = false;
            next_run_at.clear();
        }
        TaskSchedule::Interval {
            every_seconds,
            next_run_at,
            working_hours,
            ..
        } => {
            let effective_frequency = (*every_seconds).max(min_task_interval_seconds.max(1));
            let next = now + chrono::TimeDelta::seconds(effective_frequency as i64);

            if let Some(_wh) = working_hours {
                // If the naturally computed next run is outside working hours, `schedule_is_due`
                // will hold the task back until working hours open, at which point it fires immediately.
            }

            *every_seconds = effective_frequency;
            *next_run_at = next.to_rfc3339();
        }
        TaskSchedule::DailyTimes {
            times, next_run_at, ..
        } => match next_daily_run_after(times, now) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
        TaskSchedule::Weekly { next_run_at, .. } => {
            *next_run_at = (now + chrono::TimeDelta::days(7)).to_rfc3339();
        }
        TaskSchedule::Monthly { next_run_at, .. } => {
            let next = now + chrono::TimeDelta::days(30);
            *next_run_at = next.to_rfc3339();
        }
    }
}

async fn run_shell_sequential(
    logger: &TaskLogger,
    commands: &[ShellCommandSpec],
    shell_timeout_seconds: u64,
) -> Result<()> {
    for spec in commands {
        if let Err(e) = run_shell_command(logger, &spec.command, shell_timeout_seconds).await {
            if !spec.continue_on_error {
                return Err(anyhow::anyhow!("command failed: {}", e));
            }
        }
    }
    Ok(())
}

async fn run_shell_parallel(
    logger: &TaskLogger,
    commands: &[ShellCommandSpec],
    shell_timeout_seconds: u64,
) -> Result<()> {
    let handles = commands
        .iter()
        .map(|spec| {
            let spec = spec.clone();
            let l = logger.clone();
            tokio::spawn(async move {
                let result = run_shell_command(&l, &spec.command, shell_timeout_seconds).await;
                (spec, result)
            })
        })
        .collect::<Vec<_>>();

    let mut failures = Vec::new();
    for handle in handles {
        let (spec, result) = handle
            .await
            .context("parallel shell command task join failed")?;
        if let Err(e) = result {
            if !spec.continue_on_error {
                failures.push(format!("{}: {}", spec.command, e));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "parallel commands failed: {}",
            failures.join("; ")
        ))
    }
}

fn schedule_is_due(schedule: &TaskSchedule, now: DateTime<Utc>) -> bool {
    if !schedule.enabled() {
        return false;
    }

    match schedule {
        TaskSchedule::Once { next_run_at, .. } => {
            if next_run_at.is_empty() {
                // Run immediately
                true
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(scheduled_time) => now >= scheduled_time,
                    Err(_) => false,
                }
            }
        }
        TaskSchedule::Interval {
            next_run_at,
            working_hours,
            ..
        } => {
            let is_due = if next_run_at.is_empty() {
                true
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            };

            if is_due {
                if let Some(wh) = working_hours {
                    crate::runner::config::is_within_working_hours(wh, now)
                } else {
                    true
                }
            } else {
                false
            }
        }
        TaskSchedule::DailyTimes { next_run_at, .. } => {
            if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            }
        }
        TaskSchedule::Weekly { next_run_at, .. } => {
            if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            }
        }
        TaskSchedule::Monthly { next_run_at, .. } => {
            if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::{human_duration, next_daily_run_after};

    #[test]
    fn legacy_repeat_task_is_due_without_next_run() {
        let task = RunnerTask {
            id: "legacy".to_string(),
            name: "Legacy".to_string(),
            enabled: true,
            repetition: Repetition::Repeat,
            frequency_seconds: 60,
            next_run_at: String::new(),
            schedules: Vec::new(),
            kind: TaskKind::CrmFetch {
                report: ReportType::All,
            },
            last_run_at: String::new(),
            last_status: String::new(),
            post_run_script: String::new(),
        };

        assert!(task.due_now(Utc::now()));
    }

    #[test]
    fn human_duration_uses_largest_units() {
        assert_eq!(human_duration(3_660), "1 hour 1 minute");
        assert_eq!(human_duration(86_400), "1 day");
    }

    #[test]
    fn daily_local_schedule_gets_future_next_run() {
        let now = Utc::now();
        let next = next_daily_run_after(&["00:00".to_string(), "23:59".to_string()], now).unwrap();
        let next = parse_rfc3339_utc(&next).unwrap();
        assert!(next > now);
    }

    #[tokio::test]
    async fn sequential_continues_when_command_allows_error() {
        let commands = vec![
            ShellCommandSpec {
                command: "exit 8".to_string(),
                continue_on_error: true,
            },
            ShellCommandSpec {
                command: "echo ok".to_string(),
                continue_on_error: false,
            },
        ];

        run_shell_sequential(&TaskLogger::new("test", "test"), &commands, 5)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn sequential_stops_on_non_continued_error() {
        let commands = vec![ShellCommandSpec {
            command: "exit 8".to_string(),
            continue_on_error: false,
        }];

        assert!(
            run_shell_sequential(&TaskLogger::new("test", "test"), &commands, 5)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn parallel_fails_only_non_continued_errors() {
        let ignored = vec![ShellCommandSpec {
            command: "exit 8".to_string(),
            continue_on_error: true,
        }];
        run_shell_parallel(&TaskLogger::new("test", "test"), &ignored, 5)
            .await
            .unwrap();

        let failed = vec![ShellCommandSpec {
            command: "exit 8".to_string(),
            continue_on_error: false,
        }];
        assert!(
            run_shell_parallel(&TaskLogger::new("test", "test"), &failed, 5)
                .await
                .is_err()
        );
    }
}
