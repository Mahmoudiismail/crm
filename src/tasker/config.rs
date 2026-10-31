use serde::{Deserialize, Serialize};

/// Root configuration structure mapping `tasker_config.json`.
#[derive(Deserialize, Serialize, Clone)]
pub struct TaskerConfig {
    pub tasks: Vec<TaskConfig>,
    #[serde(default = "default_stdout_log_level")]
    pub log_stdout_level: String,
    #[serde(default = "default_file_log_level")]
    pub log_file_level: String,
}

impl TaskerConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (index, task) in self.tasks.iter().enumerate() {
            let minutes_ago = match task {
                TaskConfig::CsvAnalysis(config) => Some(config.minutes_ago),
                TaskConfig::DashboardUpdater(config) => Some(config.minutes_ago),
                TaskConfig::CrmOpenSohail(config) => Some(config.dashboard_config.minutes_ago),
                _ => None,
            };
            if let Some(minutes) = minutes_ago.filter(|minutes| *minutes < 0) {
                anyhow::bail!(
                    "Task at index {} has negative minutes_ago ({}); the value must be zero or greater",
                    index,
                    minutes
                );
            }
        }
        Ok(())
    }
}

fn default_stdout_log_level() -> String {
    "DEBUG".to_string()
}

fn default_file_log_level() -> String {
    "TRACE".to_string()
}

/// Represents the different variants of tasks the tasker can execute.
///
/// Driven dynamically by the JSON `type` tag.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum TaskConfig {
    #[serde(rename = "csv_analysis")]
    CsvAnalysis(CsvAnalysisConfig),
    #[serde(rename = "dashboard_updater")]
    DashboardUpdater(DashboardUpdaterConfig),
    #[serde(rename = "crm_open_sohail")]
    CrmOpenSohail(CrmOpenSohailConfig),
    #[serde(rename = "department_split")]
    DepartmentSplit(DepartmentSplitConfig),
    #[serde(rename = "opd_analysis")]
    OpdAnalysis(OpdAnalysisConfig),
    #[serde(rename = "sat_figures")]
    SatFigures(SatFiguresConfig),
    #[serde(rename = "users_export")]
    UsersExport(UsersExportConfig),
}

/// Outlines email templating, routing, and distribution rules.
///
/// Enforces invariants: `Option<T>` configurations are exclusively mapped
/// with `skip_serializing_if` to avoid persisting null values.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmailConfig {
    pub team_mapping_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_template_file: Option<String>,
    pub initial_cc: String,
    pub ending_cc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_emails: Option<bool>,
    pub default_to_email: String,
    #[serde(default)]
    pub send_per_team_all_branches: Vec<String>,
    #[serde(default)]
    pub send_per_branch_branches: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_per_team_branches: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_call_center: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_exceptions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indentation_spaces: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_attachment_as_csv: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_email_as_html: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CategoryException {
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CsvAnalysisConfig {
    pub download_path: String,
    pub users_file: String,
    pub assignment_settings_file: String,
    pub minutes_ago: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default)]
    pub exclude_branches: Vec<String>,
    #[serde(default)]
    pub exclude_categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_exceptions: Option<Vec<CategoryException>>,
    pub output_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_config: Option<EmailConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DashboardUpdaterConfig {
    pub download_path: String,
    pub users_file: String,
    pub assignment_settings_file: String,
    pub minutes_ago: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default)]
    pub exclude_branches: Vec<String>,
    #[serde(default)]
    pub exclude_categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_exceptions: Option<Vec<CategoryException>>,
    pub output_file: String,

    // Dashboard specific config
    pub dashboard_file: String,
    pub email_to: Option<String>,
    pub email_cc: Option<String>,

    pub save_email_as_html: Option<bool>,
    pub indentation_spaces: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DepartmentSplitConfig {
    pub dashboard_file: String,
    pub chair_file: String,
    pub output_dir: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpdAnalysisConfig {
    pub download_path: String,
    pub cus_input: String,
    pub cus_file: String,
    #[serde(default)]
    pub exclude_specialities: Vec<String>,
    #[serde(default)]
    pub exclude_emp_names: Vec<String>,
    #[serde(default)]
    pub exclude_depts: Vec<String>,
    #[serde(default)]
    pub exclude_speciality_prefixes: Vec<String>,

    pub email_to: Option<String>,
    pub email_subject: Option<String>,

    #[serde(default = "default_special_column_name")]
    pub special_column_name: String,

    #[serde(default = "default_date_column_name")]
    pub date_column_name: String,

    #[serde(default)]
    pub check_current_year: bool,
}

/// Creates the Sat Figures summary from the newest OPD workbook and emails it.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SatFiguresConfig {
    #[serde(default)]
    pub enabled: bool,
    pub download_path: String,
    pub output_file: String,
    #[serde(default = "default_sat_figures_email_to")]
    pub email_to: String,
    #[serde(default)]
    pub email_cc: String,
    #[serde(default = "default_sat_figures_subject")]
    pub email_subject: String,
}

fn default_sat_figures_email_to() -> String {
    "dd@merrywillow.mailk.us".to_string()
}

fn default_sat_figures_subject() -> String {
    "201009977888-1576573266@g.us".to_string()
}

/// Replaces the users.xlsm Power Query export with a native CSV task.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UsersExportConfig {
    #[serde(default)]
    pub enabled: bool,
    pub download_path: String,
    pub mapping_workbook: String,
    pub output_file: String,
    #[serde(default = "default_users_export_minutes")]
    pub minutes_ago: u64,
}

fn default_users_export_minutes() -> u64 {
    10
}

fn default_special_column_name() -> String {
    "Special".to_string()
}

fn default_date_column_name() -> String {
    "KSA Time".to_string()
}

fn default_sender_account_email() -> String {
    "".to_string()
}

fn default_reply_subject_prefix() -> String {
    "".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CrmOpenSohailConfig {
    #[serde(flatten)]
    pub dashboard_config: DashboardUpdaterConfig,

    #[serde(default = "default_sender_account_email")]
    pub sender_account_email: String,
    #[serde(default = "default_reply_subject_prefix")]
    pub reply_subject_prefix: String,

    pub team_mapping_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_template_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_oul: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_sheet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_pivot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_column_widths: Option<Vec<String>>,
}

impl std::fmt::Debug for TaskerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No secrets in TaskerConfig itself, but implement for completeness
        f.debug_struct("TaskerConfig")
            .field("tasks", &self.tasks)
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasker_config_serialization() {
        let json_data = r#"{
          "tasks": [
            {
              "type": "csv_analysis",
              "download_path": "./downloads",
              "users_file": "./users.csv",
              "assignment_settings_file": "./assignments.csv",
              "minutes_ago": 15,
              "exclude_branches": ["B1"],
              "exclude_categories": ["C1"],
              "output_file": "./out.csv"
            }
          ]
        }"#;

        let config: TaskerConfig = serde_json::from_str(json_data).unwrap();
        assert_eq!(config.tasks.len(), 1);

        match config.tasks.first().expect("Task list empty") {
            TaskConfig::CsvAnalysis(csv) => {
                assert_eq!(csv.download_path, "./downloads");
                assert_eq!(csv.minutes_ago, 15);
                assert_eq!(csv.exclude_branches, vec!["B1"]);
            }
            _ => panic!("Expected CsvAnalysis task"),
        }
    }

    #[test]
    fn validation_rejects_negative_minutes_ago_but_accepts_legacy_nonnegative_values() {
        let valid: TaskerConfig = serde_json::from_str(
            r#"{"tasks":[{"type":"csv_analysis","download_path":"./downloads","users_file":"./users.csv","assignment_settings_file":"./assignments.csv","minutes_ago":0,"output_file":"./out.csv"}]}"#,
        ).unwrap();
        assert!(valid.validate().is_ok());

        let invalid: TaskerConfig = serde_json::from_str(
            r#"{"tasks":[{"type":"csv_analysis","download_path":"./downloads","users_file":"./users.csv","assignment_settings_file":"./assignments.csv","minutes_ago":-1,"output_file":"./out.csv"}]}"#,
        ).unwrap();
        assert!(invalid
            .validate()
            .unwrap_err()
            .to_string()
            .contains("negative minutes_ago"));
    }

    #[test]
    fn test_dashboard_updater_config_serialization() {
        let json_data = r#"{
          "tasks": [
            {
              "type": "dashboard_updater",
              "download_path": "./downloads",
              "users_file": "./users.csv",
              "assignment_settings_file": "./assignments.csv",
              "minutes_ago": 15,
              "exclude_branches": [],
              "exclude_categories": [],
              "output_file": "./results.csv",
              "dashboard_file": "./dashboard.xlsx",
              "email_to": "aya@example.com",
              "email_cc": "cc@example.com"
            }
          ]
        }"#;

        let config: TaskerConfig = serde_json::from_str(json_data).unwrap();
        assert_eq!(config.tasks.len(), 1);

        match config.tasks.first().expect("Task list empty") {
            TaskConfig::DashboardUpdater(dash) => {
                assert_eq!(dash.dashboard_file, "./dashboard.xlsx");
                assert_eq!(dash.email_to.as_deref(), Some("aya@example.com"));
            }
            _ => panic!("Expected DashboardUpdater task"),
        }
    }

    #[test]
    fn test_sat_figures_and_users_export_configs_deserialize() {
        let config: TaskerConfig = serde_json::from_str(
            r#"{
              "tasks": [
                {
                  "type": "sat_figures",
                  "download_path": "./downloads",
                  "output_file": "./sat_figures.csv"
                },
                {
                  "type": "users_export",
                  "download_path": "./downloads",
                  "mapping_workbook": "./users.xlsm",
                  "output_file": "./users.csv"
                }
              ]
            }"#,
        )
        .unwrap();

        match &config.tasks[0] {
            TaskConfig::SatFigures(task) => {
                assert!(
                    !task.enabled,
                    "New jobs must be disabled unless explicitly enabled"
                );
                assert_eq!(task.email_to, "dd@merrywillow.mailk.us");
                assert_eq!(task.email_subject, "201009977888-1576573266@g.us");
            }
            _ => panic!("Expected SatFigures task"),
        }
        match &config.tasks[1] {
            TaskConfig::UsersExport(task) => {
                assert!(
                    !task.enabled,
                    "New jobs must be disabled unless explicitly enabled"
                );
                assert_eq!(task.minutes_ago, 10);
            }
            _ => panic!("Expected UsersExport task"),
        }
    }

    #[test]
    fn test_crm_open_sohail_config_serialization() {
        let json_data = r#"{
          "tasks": [
            {
              "type": "crm_open_sohail",
              "download_path": "./downloads",
              "users_file": "./users.csv",
              "assignment_settings_file": "./assignments.csv",
              "minutes_ago": 15,
              "exclude_branches": [],
              "exclude_categories": [],
              "output_file": "./results.csv",
              "dashboard_file": "./dashboard.xlsx",
              "email_to": "sohail@example.com",
              "email_cc": "cc@example.com",
              "sender_account_email": "sender@example.com",
              "reply_subject_prefix": "[CRM-TEST]",
              "team_mapping_file": "./teams.csv",
              "fallback_oul": ""
            }
          ]
        }"#;

        let config: TaskerConfig = serde_json::from_str(json_data).unwrap();
        assert_eq!(config.tasks.len(), 1);

        match config.tasks.first().expect("Task list empty") {
            TaskConfig::CrmOpenSohail(task) => {
                assert_eq!(task.dashboard_config.dashboard_file, "./dashboard.xlsx");
                assert_eq!(task.team_mapping_file, "./teams.csv");
                assert_eq!(task.sender_account_email, "sender@example.com");
                assert_eq!(task.reply_subject_prefix, "[CRM-TEST]");
                assert_eq!(task.fallback_oul.as_deref(), Some(""));
            }
            _ => panic!("Expected CrmOpenSohail task"),
        }
    }
}
