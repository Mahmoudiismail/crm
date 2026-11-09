use serde::{Deserialize, Serialize};

use crate::runner::config::defaults::*;

fn option_string_is_none_or_empty(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|value| value.trim().is_empty())
}

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
    pub task_groups: Vec<TaskGroup>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub filters: Vec<TaskFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskFilter {
    NameContains { value: String },
    Enabled { value: bool },
    LastFailed,
}

impl TaskGroup {
    /// Explicit task IDs and saved filters are combined with OR semantics.
    pub fn matches(&self, task: &RunnerTask) -> bool {
        if self.task_ids.iter().any(|id| id == &task.id) {
            return true;
        }

        self.filters.iter().any(|filter| match filter {
            TaskFilter::NameContains { value } => {
                !value.trim().is_empty()
                    && task.name.to_lowercase().contains(&value.trim().to_lowercase())
            }
            TaskFilter::Enabled { value } => task.enabled == *value,
            TaskFilter::LastFailed => {
                let status = task.last_status.to_lowercase();
                status.contains("fail") || status.contains("error")
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredApp {
    pub id: String,
    pub name: String,
    pub executable_path: String,
    pub config_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_api_version: Option<u32>,
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
    #[serde(default)]
    pub period_mode: PeriodMode,
    #[serde(default, skip_serializing_if = "option_string_is_none_or_empty")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "option_string_is_none_or_empty")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date_arg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date_arg: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PeriodMode {
    #[default]
    Custom,
    Daily,
    Monthly,
    Quarterly,
    AMonth,
    AQuarter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPeriod {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
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
            task_groups: Vec::new(),
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
            .field("task_groups", &self.task_groups)
            .field("working_hours_profiles", &self.working_hours_profiles)
            .field("registered_apps", &self.registered_apps)
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level)
            .field("log_retention_days", &self.log_retention_days)
            .finish()
    }
}


#[cfg(test)]
mod task_group_tests {
    use super::*;

    fn task(id: &str, name: &str, enabled: bool, last_status: &str) -> RunnerTask {
        RunnerTask {
            id: id.to_string(),
            name: name.to_string(),
            enabled,
            repetition: Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            schedules: Vec::new(),
            steps: Vec::new(),
            post_run_steps: Vec::new(),
            last_run_at: String::new(),
            last_status: last_status.to_string(),
            timeout_seconds: 0,
        }
    }

    #[test]
    fn task_group_matches_explicit_ids_or_any_filter() {
        let group = TaskGroup {
            id: "g".to_string(),
            name: "Group".to_string(),
            task_ids: vec!["manual".to_string()],
            filters: vec![
                TaskFilter::NameContains { value: "crm".to_string() },
                TaskFilter::LastFailed,
            ],
        };

        assert!(group.matches(&task("manual", "Other", true, "SUCCESS")));
        assert!(group.matches(&task("named", "CRM Daily", true, "SUCCESS")));
        assert!(group.matches(&task("failed", "Other", false, "FAILED")));
        assert!(!group.matches(&task("unmatched", "Other", false, "SUCCESS")));
    }

    #[test]
    fn task_group_name_filter_is_case_insensitive_and_blank_filter_is_ignored() {
        let group = TaskGroup {
            id: "g".to_string(),
            name: "Group".to_string(),
            task_ids: Vec::new(),
            filters: vec![TaskFilter::NameContains { value: "  ".to_string() }],
        };
        assert!(!group.matches(&task("one", "Any name", true, "")));

        let group = TaskGroup {
            filters: vec![TaskFilter::NameContains { value: "cRm".to_string() }],
            ..group
        };
        assert!(group.matches(&task("two", "Daily CRM Report", true, "")));
    }

    #[test]
    fn old_runner_config_without_task_groups_still_deserializes() {
        let config: RunnerConfig = serde_json::from_str(r#"{"tasks":[]}"#).unwrap();
        assert!(config.task_groups.is_empty());
    }
}

#[cfg(test)]
mod external_app_date_serialization_tests {
    use super::{ExternalAppSpec, PeriodMode};
    use std::collections::HashMap;

    fn spec(start_date: Option<&str>, end_date: Option<&str>) -> ExternalAppSpec {
        ExternalAppSpec {
            app_id: "test".to_string(),
            args: HashMap::new(),
            period_mode: PeriodMode::Custom,
            start_date: start_date.map(str::to_string),
            end_date: end_date.map(str::to_string),
            start_date_arg: None,
            end_date_arg: None,
        }
    }

    #[test]
    fn omits_empty_period_dates_from_json() {
        let json = serde_json::to_value(spec(Some(""), Some("   "))).unwrap();
        assert!(json.get("start_date").is_none());
        assert!(json.get("end_date").is_none());
    }

    #[test]
    fn preserves_configured_period_dates_in_json() {
        let json = serde_json::to_value(spec(Some("2026-10-01"), None)).unwrap();
        assert_eq!(json["start_date"], "2026-10-01");
        assert!(json.get("end_date").is_none());
    }
}
