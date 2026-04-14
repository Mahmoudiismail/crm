use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::crm::{auth, config::AppConfig, downloader, fetcher};
use crate::crm::types::ReportType;

use super::config::{Repetition, RunnerConfig, RunnerTask, TaskKind};

#[derive(Debug, Clone)]
pub enum RunnerCommand {
    RunAllNow,
    RunTaskNow(String),
    RunAdhocCrm(ReportType),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RunnerStatus {
    pub currently_running: bool,
    pub last_error: String,
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
    }
}

pub async fn run_due_tasks(path: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let now = Utc::now();

    for task in &mut cfg.tasks {
        if task.due_now(now) {
            run_task(task, &cfg.crm_config_path, status).await;
            update_next_run(task, now);
        }
    }

    cfg.save(path)?;
    Ok(())
}

async fn run_all_tasks_now(path: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let now = Utc::now();
    for task in &mut cfg.tasks {
        if task.enabled {
            run_task(task, &cfg.crm_config_path, status).await;
            update_next_run(task, now);
        }
    }
    cfg.save(path)?;
    Ok(())
}

async fn run_task_by_id(path: &str, task_id: &str, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let mut cfg = RunnerConfig::load(path)?;
    let now = Utc::now();

    if let Some(task) = cfg.tasks.iter_mut().find(|t| t.id == task_id) {
        run_task(task, &cfg.crm_config_path, status).await;
        update_next_run(task, now);
        cfg.save(path)?;
        return Ok(());
    }

    Err(anyhow::anyhow!("Task '{}' not found", task_id))
}

async fn run_adhoc_crm(path: &str, report: ReportType, status: &Arc<Mutex<RunnerStatus>>) -> Result<()> {
    let cfg = RunnerConfig::load(path)?;
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

    run_task(&mut task, &cfg.crm_config_path, status).await;
    Ok(())
}

fn update_next_run(task: &mut RunnerTask, now: DateTime<Utc>) {
    task.last_run_at = now.to_rfc3339();
    match task.repetition {
        Repetition::Once => {
            task.enabled = false;
            task.next_run_at = String::new();
        }
        Repetition::Repeat => {
            let next = now + chrono::TimeDelta::seconds(task.frequency_seconds as i64);
            task.next_run_at = next.to_rfc3339();
        }
    }
}

async fn run_task(task: &mut RunnerTask, crm_config_path: &str, status: &Arc<Mutex<RunnerStatus>>) {
    {
        let mut st = status.lock().await;
        if st.currently_running {
            task.last_status = "skipped: another task is running".to_string();
            return;
        }
        st.currently_running = true;
        st.last_error.clear();
    }

    let result = match &task.kind {
        TaskKind::CrmFetch { report } => run_crm_fetch(crm_config_path, *report, task.skip_login, task.output.clone()).await,
        TaskKind::ShellCommand { command } => run_shell_command(command).await,
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
    st.currently_running = false;
}

async fn run_shell_command(command: &str) -> Result<()> {
    let output = tokio::process::Command::new("bash")
        .arg("-lc")
        .arg(command)
        .output()
        .await
        .with_context(|| format!("Failed to run command: {}", command))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

async fn run_crm_fetch(
    crm_config_path: &str,
    report: ReportType,
    skip_login: bool,
    output: Option<String>,
) -> Result<()> {
    let mut config = AppConfig::load(crm_config_path)?;
    config.finalize_runtime_fields();

    let client = build_client(&config)?;

    let token = auth::ensure_authenticated(&mut config, &client, skip_login).await?;
    config.save(crm_config_path)?;

    let results = fetcher::fetch_reports(&config, &client, &token, report).await?;

    if config.download_csv {
        let urls = fetcher::extract_urls(&results);
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let download_dir = exe_dir.join("download");

        for (key, url) in &urls {
            if let Err(e) = downloader::download_csv(&client, url, key, &download_dir).await {
                error!("Download failed for {}: {:#}", key, e);
            }
        }
    }

    if let Some(output_path) = output {
        let pretty = serde_json::to_string_pretty(&results)?;
        std::fs::write(output_path, pretty)?;
    }

    config.save(crm_config_path)?;
    Ok(())
}

fn build_client(config: &AppConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if config.no_verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }
    Ok(builder.build()?)
}
