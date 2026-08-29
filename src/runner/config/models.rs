use serde::{Deserialize, Serialize};

use crate::runner::config::defaults::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_gui_host")]
    pub gui_host: String,
    #[serde(default = "default_gui_port")]
    pub gui_port: u16,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
    #[serde(default = "default_allow_shell_tasks")]
    pub allow_shell_tasks: bool,
    #[serde(default = "default_shell_timeout")]
    pub shell_timeout_seconds: u64,
    #[serde(default = "default_post_run_timeout")]
    pub post_run_timeout_seconds: u64,
    #[serde(default = "default_min_task_interval")]
    pub min_task_interval_seconds: u64,
    #[serde(default)]
    pub tasks: Vec<RunnerTask>,
    #[serde(default)]
    pub working_hours_profiles: Vec<WorkingHoursProfile>,
    #[serde(default)]
    pub registered_apps: Vec<RegisteredApp>,
    #[serde(default = "default_stdout_log_level")]
    pub log_stdout_level: String,
    #[serde(default = "default_file_log_level")]
    pub log_file_level: String,
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredApp {
    pub id: String,
    pub name: String,
    pub executable_path: String,
    pub config_path: String,
    #[serde(default)]
    pub allow_concurrent_tasks: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub mode: ExecutionMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionSpec {
    ShellCommand(ShellCommandSpec),
    ExternalApp(ExternalAppSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAppSpec {
    pub app_id: String,
    #[serde(default)]
    pub args: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    from = "crate::runner::config::migration::RunnerTaskLegacy",
    into = "crate::runner::config::migration::RunnerTaskLegacy"
)]
pub struct RunnerTask {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub repetition: Repetition,
    pub frequency_seconds: u64,
    pub next_run_at: String,
    pub schedules: Vec<TaskSchedule>,
    pub steps: Vec<TaskStep>,
    pub post_run_steps: Vec<TaskStep>,

    pub last_run_at: String,
    pub last_status: String,
    pub timeout_seconds: u64,
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
    ShellCommand {
        #[serde(default)]
        mode: ShellCommandMode,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        commands: Vec<ShellCommandSpec>,
    },
    ExternalApp {
        #[serde(default)]
        app_id: String,
        #[serde(default)]
        args: std::collections::HashMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingHoursProfile {
    pub id: String,
    pub name: String,
    pub days: std::collections::HashMap<String, WorkingHours>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingHours {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours: Option<std::collections::HashMap<String, WorkingHours>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours_profile_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_time: Option<String>,
    },
    DailyTimes {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        times: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours: Option<std::collections::HashMap<String, WorkingHours>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours_profile_id: Option<String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours: Option<std::collections::HashMap<String, WorkingHours>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours_profile_id: Option<String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours: Option<std::collections::HashMap<String, WorkingHours>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours_profile_id: Option<String>,
        #[serde(default)]
        next_run_at: String,
    },
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

impl Default for TaskKind {
    fn default() -> Self {
        Self::ShellCommand {
            mode: ShellCommandMode::Sequential,
            commands: Vec::new(),
        }
    }
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            gui_host: default_gui_host(),
            gui_port: default_gui_port(),
            poll_interval_seconds: default_poll_interval(),
            allow_shell_tasks: default_allow_shell_tasks(),
            shell_timeout_seconds: default_shell_timeout(),
            post_run_timeout_seconds: default_post_run_timeout(),
            min_task_interval_seconds: default_min_task_interval(),
            registered_apps: Vec::new(),
            tasks: Vec::new(),
            working_hours_profiles: Vec::new(),
            log_stdout_level: default_stdout_log_level(),
            log_file_level: default_file_log_level(),
            log_retention_days: default_log_retention_days(),
        }
    }
}

impl std::fmt::Debug for RunnerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No secrets in RunnerConfig itself, but implement for completeness
        f.debug_struct("RunnerConfig")
            .field("gui_host", &self.gui_host)
            .field("gui_port", &self.gui_port)
            .field("poll_interval_seconds", &self.poll_interval_seconds)
            .field("allow_shell_tasks", &self.allow_shell_tasks)
            .field("shell_timeout_seconds", &self.shell_timeout_seconds)
            .field("post_run_timeout_seconds", &self.post_run_timeout_seconds)
            .field("min_task_interval_seconds", &self.min_task_interval_seconds)
            .field("tasks", &self.tasks)
            .field("working_hours_profiles", &self.working_hours_profiles)
            .field("registered_apps", &self.registered_apps)
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level)
            .field("log_retention_days", &self.log_retention_days)
            .finish()
    }
}
