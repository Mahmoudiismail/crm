use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_gui_host")]
    pub gui_host: String,
    #[serde(default = "default_gui_port")]
    pub gui_port: u16,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
    #[serde(default = "default_crm_config_path")]
    pub crm_config_path: String,
    #[serde(default = "default_crm_executable_path")]
    pub crm_executable_path: String,
    #[serde(default = "default_allow_shell_tasks")]
    pub allow_shell_tasks: bool,
    #[serde(default = "default_shell_timeout")]
    pub shell_timeout_seconds: u64,
    #[serde(default = "default_min_task_interval")]
    pub min_task_interval_seconds: u64,
    #[serde(default)]
    pub tasks: Vec<RunnerTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerTask {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub repetition: Repetition,
    #[serde(default = "default_frequency")]
    pub frequency_seconds: u64,
    #[serde(default)]
    pub next_run_at: String,
    #[serde(default)]
    pub kind: TaskKind,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default)]
    pub last_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Repetition {
    #[default]
    Once,
    Repeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskKind {
    CrmFetch { report: ReportType },
    ShellCommand { command: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    All,
    Tickets,
    Calls,
    Leads,
    None,
}

impl ReportType {
    pub fn as_arg(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Tickets => "tickets",
            Self::Calls => "calls",
            Self::Leads => "leads",
            Self::None => "none",
        }
    }
}

impl Default for ReportType {
    fn default() -> Self {
        Self::All
    }
}

impl Default for TaskKind {
    fn default() -> Self {
        Self::CrmFetch {
            report: ReportType::All,
        }
    }
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            gui_host: default_gui_host(),
            gui_port: default_gui_port(),
            poll_interval_seconds: default_poll_interval(),
            crm_config_path: default_crm_config_path(),
            crm_executable_path: default_crm_executable_path(),
            allow_shell_tasks: default_allow_shell_tasks(),
            shell_timeout_seconds: default_shell_timeout(),
            min_task_interval_seconds: default_min_task_interval(),
            tasks: vec![RunnerTask {
                id: "daily_all_reports".to_string(),
                name: "Daily CRM Fetch (All Reports)".to_string(),
                enabled: true,
                repetition: Repetition::Repeat,
                frequency_seconds: 24 * 60 * 60,
                next_run_at: String::new(),
                kind: TaskKind::CrmFetch {
                    report: ReportType::All,
                },
                last_run_at: String::new(),
                last_status: String::new(),
            }],
        }
    }
}

impl RunnerConfig {
    pub fn load(path: &str) -> Result<Self> {
        if !std::path::Path::new(path).exists() {
            let default = Self::default();
            default.save(path)?;
            return Ok(default);
        }

        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read runner config: {}", path))?;
        let cfg: Self = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse runner config: {}", path))?;
        Ok(cfg)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let pretty = serde_json::to_string_pretty(self)?;
        std::fs::write(path, pretty)
            .with_context(|| format!("Failed to write runner config: {}", path))?;
        Ok(())
    }
}

impl RunnerTask {
    pub fn due_now(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        if self.next_run_at.is_empty() {
            return true;
        }
        DateTime::parse_from_rfc3339(&self.next_run_at)
            .map(|dt| dt.with_timezone(&Utc) <= now)
            .unwrap_or(true)
    }
}

fn default_gui_host() -> String {
    "127.0.0.1".to_string()
}

fn default_gui_port() -> u16 {
    8787
}

fn default_poll_interval() -> u64 {
    30
}

fn default_crm_config_path() -> String {
    "config.json".to_string()
}

fn default_crm_executable_path() -> String {
    if cfg!(target_os = "windows") {
        "crm.exe".to_string()
    } else {
        "crm".to_string()
    }
}

fn default_allow_shell_tasks() -> bool {
    false
}

fn default_shell_timeout() -> u64 {
    300
}

fn default_min_task_interval() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

fn default_frequency() -> u64 {
    3600
}
