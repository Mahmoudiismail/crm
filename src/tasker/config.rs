use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskerConfig {
    pub tasks: Vec<TaskConfig>,
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
    pub body_template_file: Option<String>,
    pub initial_cc: String,
    pub ending_cc: String,
    pub send_emails: Option<bool>,
    pub default_to_email: String,
    pub send_per_team_all_branches: Vec<String>,
    pub send_per_branch_branches: Vec<String>,
    pub send_per_team_branches: Option<Vec<String>>,
    pub send_call_center: Option<bool>,
    pub send_exceptions: Option<bool>,
    pub indentation_spaces: Option<u32>,
    pub save_attachment_as_csv: Option<bool>,
    pub save_email_as_html: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CategoryException {
    pub category: String,
    pub branch: Option<String>,
    pub team: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CsvAnalysisConfig {
    pub download_path: String,
    pub users_file: String,
    pub assignment_settings_file: String,
    pub minutes_ago: i64,
    pub start_date: Option<String>,
    pub exclude_branches: Vec<String>,
    pub exclude_categories: Vec<String>,
    pub category_exceptions: Option<Vec<CategoryException>>,
    pub output_file: String,
    pub email_config: Option<EmailConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DashboardUpdaterConfig {
    pub download_path: String,
    pub users_file: String,
    pub assignment_settings_file: String,
    pub minutes_ago: i64,
    pub start_date: Option<String>,
    pub exclude_branches: Vec<String>,
    pub exclude_categories: Vec<String>,
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
    pub body_template_file: Option<String>,
    pub subject_template: Option<String>,
    pub branch_filter: Option<Vec<String>>,
    pub month_filter: Option<Vec<String>>,
    pub fallback_oul: Option<String>,
    pub dashboard_sheet_name: Option<String>,
    pub dashboard_pivot_name: Option<String>,
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
