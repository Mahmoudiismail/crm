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
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmailConfig {
    pub team_mapping_file: String,
    pub body_template_file: Option<String>,
    pub initial_cc: String,
    pub ending_cc: String,
    pub send_emails: Option<bool>,
    pub default_to_email: String,
    pub send_per_team_branches: Vec<String>,
    pub send_per_branch_branches: Vec<String>,
    pub send_call_center: Option<bool>,
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
    pub exclude_branches: Vec<String>,
    pub exclude_categories: Vec<String>,
    pub category_exceptions: Option<Vec<CategoryException>>,
    pub output_file: String,
    pub email_config: Option<EmailConfig>,
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

        match &config.tasks[0] {
            TaskConfig::CsvAnalysis(csv) => {
                assert_eq!(csv.download_path, "./downloads");
                assert_eq!(csv.minutes_ago, 15);
                assert_eq!(csv.exclude_branches, vec!["B1"]);
            }
        }
    }
}
