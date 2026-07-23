use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskerConfig {
    pub tasks: Vec<TaskConfig>,
    #[serde(default = "default_stdout_log_level")]
    pub log_stdout_level: String,
    #[serde(default = "default_file_log_level")]
    pub log_file_level: String,
}

impl TaskerConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

fn default_stdout_log_level() -> String {
    "DEBUG".to_string()
}

fn default_file_log_level() -> String {
    "TRACE".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum TaskConfig {
    #[serde(rename = "csv_analysis")]
    CsvAnalysis(CsvAnalysisConfig),
    #[serde(rename = "dashboard_updater")]
    DashboardUpdater(DashboardUpdaterConfig),
    #[serde(rename = "crm_open_sohail")]
    CrmOpenSohail(CrmOpenSohailConfig),
}

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
pub struct CrmOpenSohailConfig {
    #[serde(flatten)]
    pub dashboard_config: DashboardUpdaterConfig,

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
              "team_mapping_file": "./teams.csv",
              "fallback_oul": "N/A"
            }
          ]
        }"#;

        let config: TaskerConfig = serde_json::from_str(json_data).unwrap();
        assert_eq!(config.tasks.len(), 1);

        match config.tasks.first().expect("Task list empty") {
            TaskConfig::CrmOpenSohail(task) => {
                assert_eq!(task.dashboard_config.dashboard_file, "./dashboard.xlsx");
                assert_eq!(task.team_mapping_file, "./teams.csv");
                assert_eq!(task.fallback_oul.as_deref(), Some("N/A"));
            }
            _ => panic!("Expected CrmOpenSohail task"),
        }
    }
}
