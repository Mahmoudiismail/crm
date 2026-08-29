use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub positions: Vec<String>,
    pub first_position: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssignmentSettings {
    #[serde(alias = "Category", alias = "category")]
    pub category: String,
    #[serde(alias = "Type", alias = "type", alias = "type_")]
    pub type_: String,
    #[serde(alias = "Subtype", alias = "subtype")]
    pub subtype: String,
    #[serde(alias = "Auto agent/team assignment")]
    pub auto_agent_team_assignment: Option<String>,
}

#[derive(Debug)]
pub struct CsvAnalysisParams<'a> {
    pub users_file: &'a str,
    pub assignment_settings_file: &'a str,
    pub download_path: &'a str,
    pub output_file: &'a str,
    pub minutes_ago: i64,
    pub start_date: Option<&'a str>,
    pub exclude_branches: &'a [String],
    pub exclude_categories: &'a [String],
    pub category_exceptions: Option<&'a Vec<crate::tasker::config::CategoryException>>,
}

impl<'a> From<&'a crate::tasker::config::CsvAnalysisConfig> for CsvAnalysisParams<'a> {
    fn from(config: &'a crate::tasker::config::CsvAnalysisConfig) -> Self {
        Self {
            users_file: &config.users_file,
            assignment_settings_file: &config.assignment_settings_file,
            download_path: &config.download_path,
            output_file: &config.output_file,
            minutes_ago: config.minutes_ago,
            start_date: config.start_date.as_deref(),
            exclude_branches: &config.exclude_branches,
            exclude_categories: &config.exclude_categories,
            category_exceptions: config.category_exceptions.as_ref(),
        }
    }
}

impl<'a> From<&'a crate::tasker::config::DashboardUpdaterConfig> for CsvAnalysisParams<'a> {
    fn from(config: &'a crate::tasker::config::DashboardUpdaterConfig) -> Self {
        Self {
            users_file: &config.users_file,
            assignment_settings_file: &config.assignment_settings_file,
            download_path: &config.download_path,
            output_file: &config.output_file,
            minutes_ago: config.minutes_ago,
            start_date: config.start_date.as_deref(),
            exclude_branches: &config.exclude_branches,
            exclude_categories: &config.exclude_categories,
            category_exceptions: config.category_exceptions.as_ref(),
        }
    }
}
