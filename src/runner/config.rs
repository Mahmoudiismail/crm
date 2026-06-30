#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct YaswebReportSpec {
    pub report_type: String,
    pub report_name: String,
    #[serde(default)]
    pub filters: std::collections::HashMap<String, String>,
}
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Utc};
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
    #[serde(default = "default_yasweb_config_path")]
    pub yasweb_config_path: String,
    #[serde(default = "default_yasweb_executable_path")]
    pub yasweb_executable_path: String,
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
    pub registered_apps: Vec<RegisteredApp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredApp {
    pub id: String,
    pub name: String,
    pub executable_path: String,
    pub config_path: String,
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
    #[serde(default)]
    pub post_run_script: String,
    #[serde(default)]
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
    CrmFetch {
        report: ReportType,
    },
    ShellCommand {
        #[serde(default)]
        mode: ShellCommandMode,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        commands: Vec<ShellCommandSpec>,
    },
    Yasweb {
        #[serde(default)]
        reports: Vec<YaswebReportSpec>,
    },
    ExternalApp {
        #[serde(default)]
        app_id: String,
        #[serde(default)]
        args: std::collections::HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingHours {
    pub start: String,
    pub end: String,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours: Option<std::collections::HashMap<String, WorkingHours>>,
    },
    DailyTimes {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        times: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_hours: Option<std::collections::HashMap<String, WorkingHours>>,
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
#[derive(Default)]
pub enum ReportType {
    #[default]
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
            yasweb_config_path: default_yasweb_config_path(),
            yasweb_executable_path: default_yasweb_executable_path(),
            allow_shell_tasks: default_allow_shell_tasks(),
            shell_timeout_seconds: default_shell_timeout(),
            post_run_timeout_seconds: default_post_run_timeout(),
            min_task_interval_seconds: default_min_task_interval(),
            registered_apps: Vec::new(),
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
                post_run_script: String::new(),
                timeout_seconds: 0,
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

pub fn is_within_working_hours(
    working_hours: &std::collections::HashMap<String, WorkingHours>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let now_local = now.with_timezone(&chrono::Local);
    let current_day_idx = now_local.weekday().num_days_from_monday();
    let current_time = now_local.time();

    let parse_day = |day: &str| -> Option<u32> {
        match day {
            "Mon" | "Monday" | "mon" => Some(0),
            "Tue" | "Tuesday" | "tue" => Some(1),
            "Wed" | "Wednesday" | "wed" => Some(2),
            "Thu" | "Thursday" | "thu" => Some(3),
            "Fri" | "Friday" | "fri" => Some(4),
            "Sat" | "Saturday" | "sat" => Some(5),
            "Sun" | "Sunday" | "sun" => Some(6),
            _ => None,
        }
    };

    let weekday_str = match now_local.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    };

    let mut matched_hours = None;

    for (days_str, hours) in working_hours {
        let parts: Vec<&str> = days_str.split('-').map(|s| s.trim()).collect();
        let is_match = if parts.len() == 2 {
            let start_day = parse_day(parts[0]);
            let end_day = parse_day(parts[1]);
            if let (Some(s), Some(e)) = (start_day, end_day) {
                if s <= e {
                    current_day_idx >= s && current_day_idx <= e
                } else {
                    current_day_idx >= s || current_day_idx <= e
                }
            } else {
                false
            }
        } else {
            days_str == weekday_str || parse_day(days_str) == Some(current_day_idx)
        };

        if is_match {
            matched_hours = Some(hours);
            break;
        }
    }

    if let Some(hours) = matched_hours {
        if let (Ok(start), Ok(end)) = (
            chrono::NaiveTime::parse_from_str(&hours.start, "%H:%M"),
            chrono::NaiveTime::parse_from_str(&hours.end, "%H:%M"),
        ) {
            if start <= end {
                return current_time >= start && current_time <= end;
            } else {
                return current_time >= start || current_time <= end;
            }
        }
        return false;
    }

    working_hours.is_empty()
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

pub fn next_daily_run_after(
    times: &[String],
    now: DateTime<Utc>,
    working_hours: Option<&std::collections::HashMap<String, WorkingHours>>,
) -> Result<String> {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let mut candidates = Vec::new();

    for raw in times {
        let time = NaiveTime::parse_from_str(raw.trim(), "%H:%M")
            .with_context(|| format!("Invalid daily time '{}'. Use HH:MM", raw))?;

        // Look ahead up to 14 days to find the next valid working day,
        // to avoid infinite loops if configuration is somehow completely disjoint,
        // though typically it would just look ahead a couple days at most.
        for day_offset in 0_i64..14 {
            let date = today + chrono::TimeDelta::days(day_offset);
            let local_dt = date.and_time(time);
            let candidate = Local
                .from_local_datetime(&local_dt)
                .earliest()
                .or_else(|| Local.from_local_datetime(&local_dt).latest())
                .with_context(|| format!("Local time '{}' could not be resolved", raw))?
                .with_timezone(&Utc);

            if candidate > now {
                if let Some(wh) = working_hours {
                    if is_working_day(wh, candidate) {
                        candidates.push(candidate);
                        break; // Only need the first valid future candidate for this specific time
                    }
                } else {
                    candidates.push(candidate);
                    break;
                }
            }
        }
    }

    candidates
        .into_iter()
        .min()
        .map(|dt| dt.to_rfc3339())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "daily_times schedule requires at least one HH:MM time that falls on a working day"
            )
        })
}

pub fn is_working_day(
    working_hours: &std::collections::HashMap<String, WorkingHours>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let now_local = now.with_timezone(&chrono::Local);
    let current_day_idx = now_local.weekday().num_days_from_monday();

    let parse_day = |day: &str| -> Option<u32> {
        match day {
            "Mon" | "Monday" | "mon" => Some(0),
            "Tue" | "Tuesday" | "tue" => Some(1),
            "Wed" | "Wednesday" | "wed" => Some(2),
            "Thu" | "Thursday" | "thu" => Some(3),
            "Fri" | "Friday" | "fri" => Some(4),
            "Sat" | "Saturday" | "sat" => Some(5),
            "Sun" | "Sunday" | "sun" => Some(6),
            _ => None,
        }
    };

    let weekday_str = match now_local.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    };

    for days_str in working_hours.keys() {
        let parts: Vec<&str> = days_str.split('-').map(|s| s.trim()).collect();
        let is_match = if parts.len() == 2 {
            let start_day = parse_day(parts[0]);
            let end_day = parse_day(parts[1]);
            if let (Some(s), Some(e)) = (start_day, end_day) {
                if s <= e {
                    current_day_idx >= s && current_day_idx <= e
                } else {
                    current_day_idx >= s || current_day_idx <= e
                }
            } else {
                false
            }
        } else {
            days_str == weekday_str || parse_day(days_str) == Some(current_day_idx)
        };

        if is_match {
            return true;
        }
    }

    working_hours.is_empty()
}

pub fn next_weekly_run_after(
    day_of_week: &str,
    at_time: &str,
    now: DateTime<Utc>,
) -> Result<String> {
    let day_lower = day_of_week.trim().to_lowercase();
    let target_weekday = match day_lower.as_str() {
        "sunday" | "sun" | "0" => chrono::Weekday::Sun,
        "monday" | "mon" | "1" => chrono::Weekday::Mon,
        "tuesday" | "tue" | "2" => chrono::Weekday::Tue,
        "wednesday" | "wed" | "3" => chrono::Weekday::Wed,
        "thursday" | "thu" | "4" => chrono::Weekday::Thu,
        "friday" | "fri" | "5" => chrono::Weekday::Fri,
        "saturday" | "sat" | "6" => chrono::Weekday::Sat,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid day of week '{}'. Use monday-sunday (or mon-sun, 0-6)",
                day_of_week
            ))
        }
    };

    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let now_weekday = today.weekday();

    let time = if at_time.is_empty() {
        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    } else {
        NaiveTime::parse_from_str(at_time.trim(), "%H:%M")
            .with_context(|| format!("Invalid weekly time '{}'. Use HH:MM", at_time))?
    };

    let days_until_target = (target_weekday.num_days_from_monday() as i64
        - now_weekday.num_days_from_monday() as i64
        + 7)
        % 7;

    let target_date = today + chrono::TimeDelta::days(days_until_target);
    let local_dt = target_date.and_time(time);
    let candidate = match Local.from_local_datetime(&local_dt) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => {
            return Err(anyhow::anyhow!("Could not resolve weekly schedule time"))
        }
    }
    .with_timezone(&Utc);

    if candidate <= now {
        let next_week = today + chrono::TimeDelta::days(7);
        let local_dt = next_week.and_time(time);
        let next_dt = match Local.from_local_datetime(&local_dt) {
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(dt, _) => dt,
            chrono::LocalResult::None => {
                return Err(anyhow::anyhow!("Could not resolve weekly schedule time"))
            }
        }
        .with_timezone(&Utc);
        Ok(next_dt.to_rfc3339())
    } else {
        Ok(candidate.to_rfc3339())
    }
}

pub fn next_monthly_run_after(
    day_of_month: u32,
    at_time: &str,
    now: DateTime<Utc>,
) -> Result<String> {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let current_year = today.year();
    let current_month = today.month();

    let time = if at_time.is_empty() {
        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    } else {
        NaiveTime::parse_from_str(at_time.trim(), "%H:%M")
            .with_context(|| format!("Invalid monthly time '{}'. Use HH:MM", at_time))?
    };

    for month_offset in 0..12 {
        let target_month = current_month + month_offset;
        let (year, month) = if target_month > 12 {
            (current_year + 1, target_month - 12)
        } else {
            (current_year, target_month)
        };

        let day = day_of_month.min(days_in_month(year, month));
        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .with_context(|| format!("Invalid date for month {}-{}", year, month))?;

        let local_dt = date.and_time(time);
        let candidate = match Local.from_local_datetime(&local_dt) {
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(dt, _) => dt,
            chrono::LocalResult::None => continue,
        }
        .with_timezone(&Utc);

        if candidate > now {
            return Ok(candidate.to_rfc3339());
        }
    }

    Err(anyhow::anyhow!(
        "Could not find a valid monthly schedule date"
    ))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
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

fn default_yasweb_config_path() -> String {
    "yasweb_config.json".to_string()
}

fn default_yasweb_executable_path() -> String {
    if cfg!(target_os = "windows") {
        "yasweb.exe".to_string()
    } else {
        "yasweb".to_string()
    }
}

fn default_allow_shell_tasks() -> bool {
    false
}

fn default_shell_timeout() -> u64 {
    900
}

fn default_post_run_timeout() -> u64 {
    900
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn human_duration_uses_largest_units() {
        assert_eq!(human_duration(3_660), "1 hour 1 minute");
        assert_eq!(human_duration(86_400), "1 day");
    }

    #[test]
    fn test_interval_schedule_persistence() {
        // Create a task with an interval schedule
        let task = RunnerTask {
            id: "test_interval".to_string(),
            name: "Test Interval Task".to_string(),
            enabled: true,
            repetition: Repetition::Repeat,
            frequency_seconds: 3600,
            next_run_at: String::new(),
            schedules: vec![TaskSchedule::Interval {
                enabled: true,
                every_seconds: 7200,
                next_run_at: Utc::now().to_rfc3339(),
                working_hours: None,
            }],
            kind: TaskKind::CrmFetch {
                report: ReportType::All,
            },
            last_run_at: String::new(),
            last_status: String::new(),
            post_run_script: String::new(),
            timeout_seconds: 0,
        };

        let cfg = RunnerConfig {
            tasks: vec![task],
            ..Default::default()
        };

        // Save to temp file
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_interval_config.json");
        cfg.save(&path.to_string_lossy()).unwrap();

        // Load back
        let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();

        // Cleanup
        let _ = fs::remove_file(&path);

        // Verify
        assert_eq!(loaded.tasks.len(), 1);
        let loaded_task = &loaded.tasks[0];
        assert_eq!(loaded_task.id, "test_interval");
        assert_eq!(loaded_task.name, "Test Interval Task");
        assert!(loaded_task.enabled);
        assert!(matches!(loaded_task.kind, TaskKind::CrmFetch { .. }));

        // Verify interval schedule
        assert_eq!(loaded_task.schedules.len(), 1);
        match &loaded_task.schedules[0] {
            TaskSchedule::Interval {
                every_seconds,
                enabled,
                ..
            } => {
                assert_eq!(*every_seconds, 7200);
                assert!(*enabled);
            }
            _ => panic!("Expected Interval schedule"),
        }
    }

    #[test]
    fn test_shell_command_persistence() {
        // Create a task with shell commands
        let task = RunnerTask {
            id: "test_shell".to_string(),
            name: "Test Shell Task".to_string(),
            enabled: true,
            repetition: Repetition::Once,
            frequency_seconds: 0,
            next_run_at: String::new(),
            schedules: vec![],
            kind: TaskKind::ShellCommand {
                mode: ShellCommandMode::Parallel,
                commands: vec![
                    ShellCommandSpec {
                        command: "tar -czf backup.tar.gz /data".to_string(),
                        continue_on_error: false,
                    },
                    ShellCommandSpec {
                        command: "echo Backup complete".to_string(),
                        continue_on_error: true,
                    },
                ],
            },
            last_run_at: String::new(),
            last_status: String::new(),
            post_run_script: String::new(),
            timeout_seconds: 0,
        };

        let cfg = RunnerConfig {
            tasks: vec![task],
            ..Default::default()
        };

        // Save to temp file
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_shell_config.json");
        cfg.save(&path.to_string_lossy()).unwrap();

        // Load back
        let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();

        // Cleanup
        let _ = fs::remove_file(&path);

        // Verify
        assert_eq!(loaded.tasks.len(), 1);
        let loaded_task = &loaded.tasks[0];
        assert_eq!(loaded_task.id, "test_shell");
        assert_eq!(loaded_task.name, "Test Shell Task");

        // Verify shell command kind
        match &loaded_task.kind {
            TaskKind::ShellCommand { mode, commands } => {
                assert_eq!(*mode, ShellCommandMode::Parallel);
                assert_eq!(commands.len(), 2);
                assert_eq!(commands[0].command, "tar -czf backup.tar.gz /data");
                assert!(!commands[0].continue_on_error);
                assert!(commands[1].continue_on_error);
            }
            _ => panic!("Expected ShellCommand kind"),
        }
    }

    #[test]
    fn test_mixed_tasks_persistence() {
        // Create multiple tasks with different kinds and schedules
        let tasks = vec![
            RunnerTask {
                id: "crm_task".to_string(),
                name: "CRM Fetch".to_string(),
                enabled: true,
                repetition: Repetition::Repeat,
                frequency_seconds: 86400,
                next_run_at: String::new(),
                schedules: vec![TaskSchedule::Interval {
                    enabled: true,
                    every_seconds: 86400,
                    next_run_at: Utc::now().to_rfc3339(),
                    working_hours: None,
                }],
                kind: TaskKind::CrmFetch {
                    report: ReportType::Tickets,
                },
                last_run_at: String::new(),
                last_status: String::new(),
                post_run_script: String::new(),
                timeout_seconds: 0,
            },
            RunnerTask {
                id: "shell_task".to_string(),
                name: "Shell Commands".to_string(),
                enabled: false,
                repetition: Repetition::Once,
                frequency_seconds: 0,
                next_run_at: String::new(),
                schedules: vec![TaskSchedule::Once {
                    enabled: true,
                    next_run_at: (Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
                }],
                kind: TaskKind::ShellCommand {
                    mode: ShellCommandMode::Sequential,
                    commands: vec![ShellCommandSpec {
                        command: "echo Hello World".to_string(),
                        continue_on_error: false,
                    }],
                },
                last_run_at: String::new(),
                last_status: String::new(),
                post_run_script: String::new(),
                timeout_seconds: 0,
            },
        ];

        let cfg = RunnerConfig {
            tasks: tasks.clone(),
            ..Default::default()
        };

        // Save and load
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("mixed_config.json");
        cfg.save(&path.to_string_lossy()).unwrap();
        let loaded = RunnerConfig::load(&path.to_string_lossy()).unwrap();

        // Cleanup
        let _ = fs::remove_file(&path);

        // Verify both tasks
        assert_eq!(loaded.tasks.len(), 2);

        // Verify first task (CRM with interval)
        let crm_task = &loaded.tasks[0];
        assert_eq!(crm_task.id, "crm_task");
        assert!(
            matches!(crm_task.kind, TaskKind::CrmFetch { report } if report == ReportType::Tickets)
        );
        assert!(!crm_task.schedules.is_empty());
        match &crm_task.schedules[0] {
            TaskSchedule::Interval { every_seconds, .. } => {
                assert_eq!(*every_seconds, 86400);
            }
            _ => panic!("Expected Interval schedule"),
        }

        // Verify second task (Shell with once)
        let shell_task = &loaded.tasks[1];
        assert_eq!(shell_task.id, "shell_task");
        assert!(!shell_task.enabled);
        match &shell_task.kind {
            TaskKind::ShellCommand { mode, commands } => {
                assert_eq!(*mode, ShellCommandMode::Sequential);
                assert_eq!(commands.len(), 1);
                assert_eq!(commands[0].command, "echo Hello World");
            }
            _ => panic!("Expected ShellCommand kind"),
        }
        match &shell_task.schedules[0] {
            TaskSchedule::Once { .. } => {}
            _ => panic!("Expected Once schedule"),
        }
    }

    #[test]
    fn test_next_daily_run_after() {
        use chrono::TimeZone;

        // Base date: 2024-01-01 12:00:00 UTC
        let base_now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // 1. Same day, future time
        let times = vec!["15:00".to_string()];
        let res = next_daily_run_after(&times, base_now, None).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        // It should be strictly after base_now
        assert!(dt.with_timezone(&Utc) > base_now);
        // Note: Timezone logic means we can't definitively check the day without knowing local tz,
        // but it should be valid RFC3339 output.

        // 2. Same day, past time (should wrap to next day)
        let times = vec!["10:00".to_string()];
        let res = next_daily_run_after(&times, base_now, None).unwrap();
        let dt2 = DateTime::parse_from_rfc3339(&res).unwrap();
        assert!(dt2.with_timezone(&Utc) > base_now);
        // It should be approximately 22 hours later (24 hours minus 2 hours)

        // 3. Multiple times, picks the earliest valid one
        let times = vec![
            "10:00".to_string(),
            "15:00".to_string(),
            "18:00".to_string(),
        ];
        let res = next_daily_run_after(&times, base_now, None).unwrap();
        let dt3 = DateTime::parse_from_rfc3339(&res).unwrap();
        assert!(dt3.with_timezone(&Utc) > base_now);
        // It should pick "15:00" as it's the earliest future time
        assert!(dt3 <= dt); // It should be equal to the 15:00 single time case

        // 4. Empty times array
        let empty_times: Vec<String> = vec![];
        assert!(next_daily_run_after(&empty_times, base_now, None).is_err());

        // 5. Invalid time formats
        let invalid_times = vec!["25:00".to_string()];
        assert!(next_daily_run_after(&invalid_times, base_now, None).is_err());

        let invalid_format = vec!["3 PM".to_string()];
        assert!(next_daily_run_after(&invalid_format, base_now, None).is_err());
    }

    #[test]
    fn test_next_weekly_run_after() {
        use chrono::TimeZone;

        // Base date: Monday 2024-01-01 12:00:00 UTC
        let base_now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // 1. Same day, future time
        let res = next_weekly_run_after("Monday", "15:00", base_now).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).year(), 2024);
        assert_eq!(dt.with_timezone(&Utc).month(), 1);
        assert_eq!(dt.with_timezone(&Utc).day(), 1);
        // Note: The function uses Local internally, so we should be careful with exact hour comparison
        // unless we know the local timezone. But we can at least check it's after base_now.
        assert!(dt.with_timezone(&Utc) > base_now);

        // 2. Same day, past time (should wrap to next week)
        let res = next_weekly_run_after("Monday", "10:00", base_now).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert!(dt.with_timezone(&Utc) > base_now);
        // It should be 7 days later
        let diff = dt.with_timezone(&Utc) - base_now;
        assert!(diff.num_days() >= 6); // roughly a week

        // 3. Different day formats
        assert!(next_weekly_run_after("mon", "10:00", base_now).is_ok());
        assert!(next_weekly_run_after("1", "10:00", base_now).is_ok()); // 1 is Monday
        assert!(next_weekly_run_after("monday", "10:00", base_now).is_ok());

        // 4. Later in the week
        let res = next_weekly_run_after("Wednesday", "12:00", base_now).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).weekday(), chrono::Weekday::Wed);

        // 5. Earlier in the week (should wrap)
        let res = next_weekly_run_after("Sunday", "12:00", base_now).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).weekday(), chrono::Weekday::Sun);
        assert!(dt.with_timezone(&Utc) > base_now);

        // 6. Invalid inputs
        assert!(next_weekly_run_after("InvalidDay", "12:00", base_now).is_err());
        assert!(next_weekly_run_after("Monday", "25:00", base_now).is_err());
        assert!(next_weekly_run_after("Monday", "not-a-time", base_now).is_err());

        // 7. Empty time (should default to 00:00)
        let res = next_weekly_run_after("Tuesday", "", base_now).unwrap();
        let dt = DateTime::parse_from_rfc3339(&res).unwrap();
        assert_eq!(dt.with_timezone(&Utc).weekday(), chrono::Weekday::Tue);
    }

    #[test]
    fn test_relative_time() {
        use chrono::TimeZone;
        let now = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

        // 1. just now (past, < 60s)
        assert_eq!(relative_time(now, now), "just now");
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(59), now),
            "just now"
        );

        // 2. in less than 1 minute (future, < 60s)
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(1), now),
            "in less than 1 minute"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(59), now),
            "in less than 1 minute"
        );

        // 3. past, >= 60s
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(60), now),
            "1 minute ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(119), now),
            "1 minute 59 seconds ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(120), now),
            "2 minutes ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(3600), now),
            "1 hour ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(7200), now),
            "2 hours ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(86400), now),
            "1 day ago"
        );
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(172800), now),
            "2 days ago"
        );

        // 4. future, >= 60s
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(60), now),
            "in 1 minute"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(119), now),
            "in 1 minute 59 seconds"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(120), now),
            "in 2 minutes"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(3600), now),
            "in 1 hour"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(7200), now),
            "in 2 hours"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(86400), now),
            "in 1 day"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(172800), now),
            "in 2 days"
        );

        // 5. compound values (human_duration returns up to two units)
        assert_eq!(
            relative_time(now - chrono::TimeDelta::seconds(3600 + 120), now),
            "1 hour 2 minutes ago"
        );
        assert_eq!(
            relative_time(now + chrono::TimeDelta::seconds(86400 + 7200), now),
            "in 1 day 2 hours"
        );
    }

    #[test]
    fn test_is_within_working_hours() {
        use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
        use std::collections::HashMap;

        let mut working_hours = HashMap::new();
        working_hours.insert(
            "Monday".to_string(),
            WorkingHours {
                start: "09:00".to_string(),
                end: "17:00".to_string(),
            },
        );
        working_hours.insert(
            "Friday".to_string(),
            WorkingHours {
                start: "10:00".to_string(),
                end: "15:00".to_string(),
            },
        );

        // Note: tests depend on local timezone if we construct UTC then to Local, so it's easier to mock "now"
        // Actually `is_within_working_hours` converts input Utc to Local inside the function.
        // To strictly test it, let's create a known local datetime, convert to UTC, and pass it in.

        // Let's create a Monday 10:00 AM local time.
        // 2026-06-15 is a Monday.
        let date_mon = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();

        let time_10am = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let dt_mon_10am = Local
            .from_local_datetime(&NaiveDateTime::new(date_mon, time_10am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        let time_8am = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let dt_mon_8am = Local
            .from_local_datetime(&NaiveDateTime::new(date_mon, time_8am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        let time_6pm = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        let dt_mon_6pm = Local
            .from_local_datetime(&NaiveDateTime::new(date_mon, time_6pm))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert!(is_within_working_hours(&working_hours, dt_mon_10am)); // Inside hours
        assert!(!is_within_working_hours(&working_hours, dt_mon_8am)); // Before hours
        assert!(!is_within_working_hours(&working_hours, dt_mon_6pm)); // After hours

        // 2026-06-16 is a Tuesday (not in working hours map)
        let date_tue = NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();
        let dt_tue_10am = Local
            .from_local_datetime(&NaiveDateTime::new(date_tue, time_10am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert!(!is_within_working_hours(&working_hours, dt_tue_10am)); // Tuesday is not configured

        // Friday
        let date_fri = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let dt_fri_10am = Local
            .from_local_datetime(&NaiveDateTime::new(date_fri, time_10am))
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let dt_fri_8am = Local
            .from_local_datetime(&NaiveDateTime::new(date_fri, time_8am))
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert!(is_within_working_hours(&working_hours, dt_fri_10am)); // 10:00 is exactly start (inclusive)
        assert!(!is_within_working_hours(&working_hours, dt_fri_8am)); // 08:00 is before 10:00
    }
}
