use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveTime, TimeZone, Utc};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schedules: Vec<TaskSchedule>,
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
    CrmFetch {
        report: ReportType,
    },
    ShellCommand {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        groups: Vec<ShellCommandGroup>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskSchedule {
    Once {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        next_run_at: String,
    },
    Interval {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default = "default_frequency")]
        every_seconds: u64,
        #[serde(default)]
        next_run_at: String,
    },
    DailyTimes {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        times: Vec<String>,
        #[serde(default)]
        next_run_at: String,
    },
    Weekly {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        day_of_week: String,
        #[serde(default)]
        at_time: String,
        #[serde(default)]
        next_run_at: String,
    },
    Monthly {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default = "default_day")]
        day_of_month: u32,
        #[serde(default)]
        at_time: String,
        #[serde(default)]
        next_run_at: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mode: ShellCommandMode,
    #[serde(default)]
    pub commands: Vec<ShellCommandSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandSpec {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ShellCommandMode {
    #[default]
    Sequential,
    Parallel,
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
                schedules: Vec::new(),
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
        if !self.schedules.is_empty() {
            return self.schedules.iter().any(|schedule| schedule.due_now(now));
        }
        if self.next_run_at.is_empty() {
            return true;
        }
        DateTime::parse_from_rfc3339(&self.next_run_at)
            .map(|dt| dt.with_timezone(&Utc) <= now)
            .unwrap_or(true)
    }

    pub fn schedule_summary(&self) -> String {
        if self.schedules.is_empty() {
            return match self.repetition {
                Repetition::Once => {
                    if self.next_run_at.is_empty() {
                        "Once, immediately".to_string()
                    } else {
                        format!("Once at {}", human_datetime(&self.next_run_at))
                    }
                }
                Repetition::Repeat => format!("Every {}", human_duration(self.frequency_seconds)),
            };
        }

        self.schedules
            .iter()
            .map(TaskSchedule::summary)
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn next_run_summary(&self) -> String {
        let mut dates = Vec::new();
        if self.schedules.is_empty() {
            if !self.next_run_at.is_empty() {
                dates.push(self.next_run_at.as_str());
            }
        } else {
            for schedule in &self.schedules {
                if let Some(next) = schedule.next_run_at() {
                    dates.push(next);
                }
            }
        }

        dates
            .into_iter()
            .filter_map(|value| parse_rfc3339_utc(value).ok())
            .min()
            .map(|dt| human_datetime(&dt.to_rfc3339()))
            .unwrap_or_else(|| "Immediate".to_string())
    }
}

impl TaskSchedule {
    pub fn due_now(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled() {
            return false;
        }

        match self.next_run_at() {
            Some(next) if !next.is_empty() => {
                parse_rfc3339_utc(next).map(|dt| dt <= now).unwrap_or(true)
            }
            _ => true,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Self::Once { enabled, .. }
            | Self::Interval { enabled, .. }
            | Self::DailyTimes { enabled, .. }
            | Self::Weekly { enabled, .. }
            | Self::Monthly { enabled, .. } => *enabled,
        }
    }

    pub fn next_run_at(&self) -> Option<&str> {
        match self {
            Self::Once { next_run_at, .. }
            | Self::Interval { next_run_at, .. }
            | Self::DailyTimes { next_run_at, .. }
            | Self::Weekly { next_run_at, .. }
            | Self::Monthly { next_run_at, .. } => {
                if next_run_at.is_empty() {
                    None
                } else {
                    Some(next_run_at.as_str())
                }
            }
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::Once {
                enabled,
                next_run_at,
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                if next_run_at.is_empty() {
                    format!("Once, immediately{}", state)
                } else {
                    format!("Once at {}{}", human_datetime(next_run_at), state)
                }
            }
            Self::Interval {
                enabled,
                every_seconds,
                ..
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                format!("Every {}{}", human_duration(*every_seconds), state)
            }
            Self::DailyTimes { enabled, times, .. } => {
                let state = if *enabled { "" } else { " (disabled)" };
                if times.is_empty() {
                    format!("Daily, no times{}", state)
                } else {
                    format!("Daily at {} local{}", times.join(", "), state)
                }
            }
            Self::Weekly {
                enabled,
                day_of_week,
                at_time,
                ..
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                let time_str = if at_time.is_empty() {
                    "default".to_string()
                } else {
                    at_time.clone()
                };
                format!("Weekly on {} at {}{}", day_of_week, time_str, state)
            }
            Self::Monthly {
                enabled,
                day_of_month,
                at_time,
                ..
            } => {
                let state = if *enabled { "" } else { " (disabled)" };
                let time_str = if at_time.is_empty() {
                    "default".to_string()
                } else {
                    at_time.clone()
                };
                format!("Monthly on day {} at {}{}", day_of_month, time_str, state)
            }
        }
    }
}

pub fn human_datetime(value: &str) -> String {
    parse_rfc3339_utc(value)
        .map(|dt| {
            let local = dt.with_timezone(&Local);
            format!(
                "{} ({})",
                local.format("%b %-d, %Y %-I:%M %p local"),
                relative_time(dt, Utc::now())
            )
        })
        .unwrap_or_else(|_| value.to_string())
}

pub fn human_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0 seconds".to_string();
    }

    let units = [
        ("day", 86_400),
        ("hour", 3_600),
        ("minute", 60),
        ("second", 1),
    ];
    let mut remaining = seconds;
    let mut parts = Vec::new();

    for (name, unit_seconds) in units {
        let count = remaining / unit_seconds;
        if count > 0 {
            parts.push(format!(
                "{} {}{}",
                count,
                name,
                if count == 1 { "" } else { "s" }
            ));
            remaining %= unit_seconds;
        }
        if parts.len() == 2 {
            break;
        }
    }

    parts.join(" ")
}

pub fn relative_time(value: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = now.signed_duration_since(value).num_seconds();
    if seconds.abs() < 60 {
        return if seconds >= 0 {
            "just now".to_string()
        } else {
            "in less than 1 minute".to_string()
        };
    }

    let abs = seconds.unsigned_abs();
    let label = human_duration(abs);
    if seconds >= 0 {
        format!("{} ago", label)
    } else {
        format!("in {}", label)
    }
}

pub fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("Invalid RFC3339 timestamp '{}'", value))
}

pub fn next_daily_run_after(times: &[String], now: DateTime<Utc>) -> Result<String> {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let mut candidates = Vec::new();

    for raw in times {
        let time = NaiveTime::parse_from_str(raw.trim(), "%H:%M")
            .with_context(|| format!("Invalid daily time '{}'. Use HH:MM", raw))?;
        for day_offset in [0_i64, 1] {
            let date = today + chrono::TimeDelta::days(day_offset);
            let local_dt = date.and_time(time);
            let candidate = Local
                .from_local_datetime(&local_dt)
                .earliest()
                .or_else(|| Local.from_local_datetime(&local_dt).latest())
                .with_context(|| format!("Local time '{}' could not be resolved", raw))?
                .with_timezone(&Utc);
            if candidate > now {
                candidates.push(candidate);
            }
        }
    }

    candidates
        .into_iter()
        .min()
        .map(|dt| dt.to_rfc3339())
        .ok_or_else(|| anyhow::anyhow!("daily_times schedule requires at least one HH:MM time"))
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

fn default_day() -> u32 {
    1
}
