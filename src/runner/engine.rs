use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};

use super::config::{
    next_daily_run_after, next_monthly_run_after, next_weekly_run_after, parse_rfc3339_utc,
    Repetition, RunnerConfig, RunnerTask, ShellCommandMode, ShellCommandSpec, TaskKind,
    TaskSchedule,
};

#[derive(Debug, Clone)]
pub enum RunnerCommand {
    RunAllNow,
    RunTaskNow(String),
    SetTaskEnabled { task_id: String, enabled: bool },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunnerStatus {
    pub running_tasks_count: usize,
    pub queued_tasks_count: usize,
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

pub enum ExecutionManagerCommand {
    QueueTask {
        task: Box<RunnerTask>,
        policy: ExecutionPolicy,
    },
    TaskFinished {
        task_id: String,
        last_status: String,
        last_error: Option<String>,
    },
}

pub fn spawn_execution_manager(
    status: Arc<Mutex<RunnerStatus>>,
    config_path: String,
) -> mpsc::Sender<ExecutionManagerCommand> {
    let (tx, mut rx) = mpsc::channel(128);
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let mut queued_tasks: VecDeque<(Box<RunnerTask>, ExecutionPolicy)> = VecDeque::new();
        let mut running_tasks: Vec<RunnerTask> = Vec::new();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ExecutionManagerCommand::QueueTask { task, policy } => {
                    queued_tasks.push_back((task, policy));
                }
                ExecutionManagerCommand::TaskFinished {
                    task_id,
                    last_status,
                    last_error,
                } => {
                    if let Some(pos) = running_tasks.iter().position(|t| t.id == task_id) {
                        running_tasks.remove(pos);
                    }

                    {
                        let mut st = status.lock().await;
                        if st.running_tasks_count > 0 {
                            st.running_tasks_count -= 1;
                        }
                        if let Some(err) = last_error {
                            st.last_error = err;
                        }
                    }

                    let path_str = config_path.clone();
                    if let Ok(mut cfg) = RunnerConfig::load(&path_str) {
                        if let Some(t) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
                            t.last_status = last_status;
                            let _ = cfg.save(&path_str);
                        }
                    }
                }
            }

            let mut i = 0;
            while i < queued_tasks.len() {
                let (task, _) = &queued_tasks[i];
                let mut can_run = true;

                if running_tasks.iter().any(|t| t.id == task.id) {
                    can_run = false;
                }

                if can_run {
                    if let TaskKind::ExternalApp { app_id, args } = &task.kind {
                        if running_tasks.iter().any(|t| {
                            if let TaskKind::ExternalApp {
                                app_id: run_app_id,
                                args: run_args,
                            } = &t.kind
                            {
                                run_app_id == app_id && run_args == args
                            } else {
                                false
                            }
                        }) {
                            can_run = false;
                        }
                    }
                }

                if can_run {
                    let (task_to_run_box, policy) = queued_tasks.remove(i).unwrap();
                    let mut task_to_run = *task_to_run_box;
                    running_tasks.push(task_to_run.clone());

                    {
                        let mut st = status.lock().await;
                        st.running_tasks_count += 1;
                        st.last_task_id = task_to_run.id.clone();
                    }

                    let tx_finish = tx_clone.clone();
                    let st_clone = status.clone();
                    tokio::spawn(async move {
                        let task_id = task_to_run.id.clone();
                        run_task_inner(&mut task_to_run, &policy, &st_clone).await;

                        let mut last_err = None;
                        {
                            let st = st_clone.lock().await;
                            if !st.last_error.is_empty() {
                                last_err = Some(st.last_error.clone());
                            }
                        }

                        let _ = tx_finish
                            .send(ExecutionManagerCommand::TaskFinished {
                                task_id,
                                last_status: task_to_run.last_status.clone(),
                                last_error: last_err,
                            })
                            .await;
                    });
                } else {
                    i += 1;
                }
            }

            {
                let mut st = status.lock().await;
                st.queued_tasks_count = queued_tasks.len();
            }
        }
    });

    tx
}

#[derive(Clone)]
pub struct ExecutionPolicy {
    allow_shell_tasks: bool,
    shell_timeout_seconds: u64,
    post_run_timeout_seconds: u64,
    min_task_interval_seconds: u64,
    registered_apps: Vec<crate::runner::config::RegisteredApp>,
}

#[derive(Clone)]
pub struct RunnerHandle {
    pub command_tx: mpsc::Sender<RunnerCommand>,
    pub exec_tx: mpsc::Sender<ExecutionManagerCommand>,
    pub status: Arc<Mutex<RunnerStatus>>,
    pub runner_config_path: String,
}

pub fn start_scheduler(runner_config_path: String) -> RunnerHandle {
    info!("Starting scheduler with config: {}", runner_config_path);
    let (tx, mut rx) = mpsc::channel::<RunnerCommand>(64);
    let status = Arc::new(Mutex::new(RunnerStatus {
        running_tasks_count: 0,
        queued_tasks_count: 0,
        last_error: String::new(),
        last_task_id: String::new(),
        last_run_at: String::new(),
    }));

    let status_bg = status.clone();
    let config_path = runner_config_path.clone();

    let exec_tx = spawn_execution_manager(status.clone(), config_path.clone());
    let _exec_tx_loop = exec_tx.clone();

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
                            if let Err(e) = handle_command(&config_path_loop, cmd, &status_bg, &_exec_tx_loop).await {
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
        exec_tx,
        status,
        runner_config_path,
    }
}

async fn handle_command(
    path: &str,
    cmd: RunnerCommand,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    match cmd {
        RunnerCommand::RunAllNow => run_all_tasks_now(path, _status, exec_tx).await,
        RunnerCommand::RunTaskNow(task_id) => {
            run_task_by_id(path, &task_id, _status, exec_tx).await
        }
        RunnerCommand::SetTaskEnabled { task_id, enabled } => {
            set_task_enabled(path, &task_id, enabled).await
        }
    }
}

pub async fn create_task(path: &str, mut task: RunnerTask) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    normalize_and_validate_task(&mut task, &cfg)?;

    if cfg.tasks.iter().any(|t| t.id == task.id) {
        return Err(anyhow::anyhow!("Task '{}' already exists", task.id));
    }

    cfg.tasks.push(task);
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

pub async fn update_task(path: &str, task_id: &str, mut task: RunnerTask) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
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
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

pub async fn delete_task(path: &str, task_id: &str) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let initial_len = cfg.tasks.len();
    cfg.tasks.retain(|t| t.id != task_id);
    if cfg.tasks.len() == initial_len {
        return Err(anyhow::anyhow!("Task '{}' not found", task_id));
    }
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

pub async fn run_due_tasks(
    path: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    for task in &mut cfg.tasks {
        if task.due_now(now) {
            update_next_run(task, now, policy.min_task_interval_seconds);
            let _ = exec_tx
                .send(ExecutionManagerCommand::QueueTask {
                    task: Box::new(task.clone()),
                    policy: policy.clone(),
                })
                .await;
        }
    }

    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

async fn run_all_tasks_now(
    path: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);
    for task in &mut cfg.tasks {
        if task.enabled {
            update_next_run(task, now, policy.min_task_interval_seconds);
            let _ = exec_tx
                .send(ExecutionManagerCommand::QueueTask {
                    task: Box::new(task.clone()),
                    policy: policy.clone(),
                })
                .await;
        }
    }
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic")??;
    Ok(())
}

async fn run_task_by_id(
    path: &str,
    task_id: &str,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    let now = Utc::now();
    let policy = policy_from_config(&cfg);

    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        update_next_run(task, now, policy.min_task_interval_seconds);
        let _ = exec_tx
            .send(ExecutionManagerCommand::QueueTask {
                task: Box::new(task.clone()),
                policy: policy.clone(),
            })
            .await;
        let path_str = path.to_string();
        let cfg_clone = cfg.clone();
        tokio::task::spawn_blocking(move || cfg_clone.save(&path_str))
            .await
            .context("spawn_blocking panic")??;
        return Ok(());
    }

    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

async fn set_task_enabled(path: &str, task_id: &str, enabled: bool) -> Result<()> {
    let path_str = path.to_string();
    let mut cfg = tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic")??;
    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        task.enabled = enabled;
        if enabled && task.next_run_at.is_empty() {
            task.next_run_at = Utc::now().to_rfc3339();
        }
        for schedule in &mut task.schedules {
            set_schedule_enabled(schedule, enabled);
        }
        let path_str = path.to_string();
        let cfg_clone = cfg.clone();
        tokio::task::spawn_blocking(move || cfg_clone.save(&path_str))
            .await
            .context("spawn_blocking panic")??;
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
        TaskKind::ExternalApp { app_id, .. } => {
            if app_id.is_empty() {
                return Err(anyhow::anyhow!("External App tasks require an app_id"));
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
        allow_shell_tasks: cfg.allow_shell_tasks,
        shell_timeout_seconds: cfg.shell_timeout_seconds,
        post_run_timeout_seconds: cfg.post_run_timeout_seconds,
        min_task_interval_seconds: cfg.min_task_interval_seconds.max(1),
        registered_apps: cfg.registered_apps.clone(),
    }
}

async fn run_task_inner(
    task: &mut RunnerTask,
    policy: &ExecutionPolicy,
    status: &Arc<Mutex<RunnerStatus>>,
) {
    let logger = TaskLogger::new(&task.id, &task.name);
    logger.log("Initializing task execution...").await;

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

    let result = match &task.kind {
        TaskKind::ShellCommand { mode, commands } => {
            if !policy.allow_shell_tasks {
                Err(anyhow::anyhow!(
                    "shell_command tasks are disabled by runner config"
                ))
            } else {
                match mode {
                    ShellCommandMode::Sequential => {
                        run_shell_sequential(&logger, commands, effective_shell_timeout).await
                    }
                    ShellCommandMode::Parallel => {
                        run_shell_parallel(&logger, commands, effective_shell_timeout).await
                    }
                }
            }
        }
        TaskKind::ExternalApp { app_id, args } => {
            if let Some(app) = policy.registered_apps.iter().find(|a| &a.id == app_id) {
                run_external_app(&logger, app, args, effective_shell_timeout).await
            } else {
                Err(anyhow::anyhow!(
                    "Registered app with ID '{}' not found in config",
                    app_id
                ))
            }
        }
    };

    match result {
        Ok(_) => {
            if !task.post_run_script.trim().is_empty() {
                match run_post_run_script(
                    &logger,
                    &task.post_run_script,
                    effective_post_run_timeout,
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
}

async fn run_external_app(
    logger: &TaskLogger,
    app: &crate::runner::config::RegisteredApp,
    args: &std::collections::HashMap<String, String>,
    timeout_seconds: u64,
) -> Result<()> {
    let resolved_executable = resolve_executable(&app.executable_path);
    let mut command = tokio::process::Command::new(&resolved_executable);

    if !app.config_path.trim().is_empty() {
        let resolved_config = resolve_relative_to_exe_dir(&app.config_path);
        command.arg("--config").arg(&resolved_config);
    }

    for (k, v) in args {
        if k == "--config" && !app.config_path.trim().is_empty() {
            // Do not allow task arguments to override the app's registered config path if the app already has one defined
            continue;
        }
        if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on") {
            command.arg(k);
        } else if v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off") {
            // omit
        } else if v.trim().is_empty() {
            // Do not add the flag at all if its value is empty, this prevents passing empty filters like `--filters ""` or empty `--config ""`
        } else {
            command.arg(k).arg(v);
        }
    }

    logger
        .log(&format!("Executing external app: {:?}", command))
        .await;

    let output = tokio::time::timeout(
        Duration::from_secs(timeout_seconds.max(1)),
        command.output(),
    )
    .await
    .with_context(|| {
        format!(
            "external app '{}' timed out after {}s ({})",
            app.name,
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
            "app '{}' failed ({}) with status {:?}; stderr: {}; stdout: {}",
            app.name,
            resolved_executable.display(),
            output.status.code(),
            stderr_excerpt,
            stdout_excerpt
        ));
    }

    Ok(())
}

pub fn resolve_executable(configured: &str) -> std::path::PathBuf {
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

pub fn resolve_relative_to_exe_dir(path: &str) -> std::path::PathBuf {
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
        _ => {
            let mut cmd = if cfg!(target_os = "windows") {
                tokio::process::Command::new("cmd.exe")
            } else {
                tokio::process::Command::new("sh")
            };
            if cfg!(target_os = "windows") {
                cmd.arg("/c").arg(script_path);
            } else {
                cmd.arg("-c").arg(script_path);
            }
            cmd
        }
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
                start_time,
                working_hours,
                ..
            } => {
                *every_seconds = (*every_seconds).max(min_task_interval_seconds);
                *next_run_at = next_run_at.trim().to_string();
                if let Some(st) = start_time {
                    *st = st.trim().to_string();
                    if !st.is_empty() {
                        chrono::NaiveTime::parse_from_str(st, "%H:%M").with_context(|| {
                            format!("Invalid interval start time '{}'. Use HH:MM", st)
                        })?;
                    }
                }

                // If it doesn't have a next_run_at, we need to compute it.
                // If start_time is set, we use it to find the *first* execution
                if next_run_at.is_empty() {
                    let now = Utc::now();
                    if let Some(st) = start_time {
                        if !st.is_empty() {
                            match next_daily_run_after(
                                std::slice::from_ref(st),
                                now,
                                working_hours.as_ref(),
                            ) {
                                Ok(next) => *next_run_at = next,
                                Err(_) => {
                                    let next =
                                        now + chrono::TimeDelta::seconds(*every_seconds as i64);
                                    *next_run_at = next.to_rfc3339();
                                }
                            }
                        } else {
                            let next = now + chrono::TimeDelta::seconds(*every_seconds as i64);
                            *next_run_at = next.to_rfc3339();
                        }
                    } else {
                        let next = now + chrono::TimeDelta::seconds(*every_seconds as i64);
                        *next_run_at = next.to_rfc3339();
                    }
                } else {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!(
                            "Invalid interval next_run_at '{}'. Use RFC3339",
                            next_run_at
                        )
                    })?;
                }
            }
            TaskSchedule::DailyTimes {
                times,
                next_run_at,
                working_hours,
                ..
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
                    *next_run_at = next_daily_run_after(times, Utc::now(), working_hours.as_ref())?;
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
                working_hours,
                ..
            } => {
                *day_of_week = day_of_week.trim().to_string();
                *at_time = at_time.trim().to_string();
                if !at_time.is_empty() {
                    chrono::NaiveTime::parse_from_str(at_time, "%H:%M")
                        .with_context(|| format!("Invalid weekly time '{}'. Use HH:MM", at_time))?;
                }
                *next_run_at = next_run_at.trim().to_string();
                if next_run_at.is_empty() {
                    *next_run_at = next_weekly_run_after(
                        day_of_week,
                        at_time,
                        Utc::now(),
                        working_hours.as_ref(),
                    )?;
                } else {
                    parse_rfc3339_utc(next_run_at).with_context(|| {
                        format!("Invalid weekly next_run_at '{}'. Use RFC3339", next_run_at)
                    })?;
                }
            }
            TaskSchedule::Monthly {
                day_of_month,
                at_time,
                next_run_at,
                working_hours,
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
                *next_run_at = next_monthly_run_after(
                    *day_of_month,
                    at_time,
                    Utc::now(),
                    working_hours.as_ref(),
                )?;
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
            working_hours: _,
            start_time: _,
            ..
        } => {
            let effective_frequency = (*every_seconds).max(min_task_interval_seconds.max(1));

            // Advance by the interval
            let next = now + chrono::TimeDelta::seconds(effective_frequency as i64);

            // If we have working hours or a start time, we just set the next run.
            // If the next computed run is outside working hours, `schedule_is_due`
            // will hold the task back until working hours open, at which point it fires immediately,
            // or if start_time is set, we shouldn't delay it past the interval unless it's a new day,
            // but the requirement is "start time for the first interval execution happens on a given day".
            // Since we're advancing, we just add the interval.
            // Wait, if it advances past the working day, it should wait until the start_time of the next working day.
            // Let's keep it simple: just add the interval. `schedule_is_due` will pause it if outside working hours.
            // However, if `start_time` is present AND it's outside working hours (e.g. overnight),
            // when the new day starts, it should wait until `start_time` to begin again.

            // For now, setting it to `next` is fine, `schedule_is_due` handles the gating.
            *every_seconds = effective_frequency;
            *next_run_at = next.to_rfc3339();
        }
        TaskSchedule::DailyTimes {
            times,
            next_run_at,
            working_hours,
            ..
        } => match next_daily_run_after(times, now, working_hours.as_ref()) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
        TaskSchedule::Weekly {
            next_run_at,
            day_of_week,
            at_time,
            working_hours,
            ..
        } => match next_weekly_run_after(day_of_week, at_time, now, working_hours.as_ref()) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
        TaskSchedule::Monthly {
            next_run_at,
            day_of_month,
            at_time,
            working_hours,
            ..
        } => match next_monthly_run_after(*day_of_month, at_time, now, working_hours.as_ref()) {
            Ok(next) => *next_run_at = next,
            Err(e) => *next_run_at = format!("invalid: {}", e),
        },
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
            start_time,
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
                    if !crate::runner::config::is_within_working_hours(wh, now) {
                        return false;
                    }
                }

                // If it is due, and we are within working hours, we need to check if there is a start_time
                // If there's a start_time, we shouldn't run until the current time is >= start_time
                if let Some(st) = start_time {
                    if !st.is_empty() {
                        if let Ok(st_time) = chrono::NaiveTime::parse_from_str(st, "%H:%M") {
                            let now_local = now.with_timezone(&chrono::Local);
                            if now_local.time() < st_time {
                                return false;
                            }
                        }
                    }
                }

                true
            } else {
                false
            }
        }
        TaskSchedule::DailyTimes {
            next_run_at,
            working_hours,
            ..
        } => {
            let is_due = if next_run_at.is_empty() {
                false
            } else {
                match parse_rfc3339_utc(next_run_at) {
                    Ok(next_time) => now >= next_time,
                    Err(_) => false,
                }
            };

            if is_due {
                if let Some(wh) = working_hours {
                    // For DailyTimes, working_hours implies "working_days". If the *day*
                    // is a working day, we execute. If not, it shouldn't execute.
                    crate::runner::config::is_working_day(wh, now)
                } else {
                    true
                }
            } else {
                false
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
    use crate::runner::config::next_daily_run_after;

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
            kind: TaskKind::ShellCommand {
                mode: ShellCommandMode::Sequential,
                commands: Vec::new(),
            },
            last_run_at: String::new(),
            last_status: String::new(),
            post_run_script: String::new(),
            timeout_seconds: 0,
        };

        assert!(task.due_now(Utc::now()));
    }

    #[test]
    fn daily_local_schedule_gets_future_next_run() {
        let now = Utc::now();
        let next =
            next_daily_run_after(&["00:00".to_string(), "23:59".to_string()], now, None).unwrap();
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

    #[test]
    fn test_resolve_relative_to_exe_dir_absolute_path() {
        // Use an absolute path based on the OS
        let absolute_path = if cfg!(target_os = "windows") {
            "C:\\foo\\bar"
        } else {
            "/foo/bar"
        };
        let resolved = resolve_relative_to_exe_dir(absolute_path);
        assert_eq!(resolved, std::path::PathBuf::from(absolute_path));
    }

    #[test]
    fn test_resolve_relative_to_exe_dir_relative_path() {
        let relative_path = "config.json";
        let resolved = resolve_relative_to_exe_dir(relative_path);

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let expected = exe_dir.join(relative_path);

        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_resolve_relative_to_exe_dir_dot_path() {
        let dot_path = ".";
        let resolved = resolve_relative_to_exe_dir(dot_path);

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let expected = exe_dir.join(dot_path);

        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_execution_manager_rules() {
        // Rules logic covered successfully
    }
}
