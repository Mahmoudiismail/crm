use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportConfig {
    pub report_type: String,
    #[serde(default)]
    pub filters: HashMap<String, String>,
    #[serde(default)]
    pub start_date_key: Option<String>,
    #[serde(default)]
    pub end_date_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YaswebConfig {
    pub url: String,
    pub username: String,
    pub password: Option<String>,
    #[serde(default)]
    pub headless: bool,
    #[serde(default)]
    pub keep_open: bool,
    #[serde(default)]
    pub reports: HashMap<String, ReportConfig>,
}

impl Default for YaswebConfig {
    fn default() -> Self {
        Self {
            url: "https://example.com/".to_string(),
            username: "username".to_string(),
            password: Some("password".to_string()),
            headless: false,
            keep_open: false,
            reports: HashMap::new(),
        }
    }
}
