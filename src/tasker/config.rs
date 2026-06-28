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
