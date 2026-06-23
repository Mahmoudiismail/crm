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
pub struct CsvAnalysisConfig {
    pub download_path: String,
    pub users_file: String,
    pub assignment_settings_file: String,
    pub minutes_ago: i64,
    pub exclude_branches: Vec<String>,
    pub exclude_categories: Vec<String>,
    pub output_file: String,
}
