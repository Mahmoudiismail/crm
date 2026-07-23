use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone, PartialEq, Default)]
pub struct DateKeyConfig {
    pub key: String,
    pub format: String,
}

impl<'de> Deserialize<'de> for DateKeyConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DateKeyConfigOpt {
            String(String),
            Struct { key: String, format: String },
        }

        let opt = DateKeyConfigOpt::deserialize(deserializer)?;
        match opt {
            DateKeyConfigOpt::String(s) => Ok(DateKeyConfig {
                key: s,
                format: "".to_string(),
            }),
            DateKeyConfigOpt::Struct { key, format } => Ok(DateKeyConfig { key, format }),
        }
    }
}

impl YaswebConfig {
    /// Finalize runtime-derived fields, such as healing empty date formats.
    /// Returns true if the configuration was modified.
    pub fn finalize_runtime_fields(&mut self) -> bool {
        let mut config_updated = false;

        for report in self.reports.values_mut() {
            let (default_start_fmt, default_end_fmt) = if report.report_type == "Report Manager" {
                ("%d-%b-%Y".to_string(), "%d-%b-%Y".to_string())
            } else {
                ("%d-%m-%Y 00:00".to_string(), "%d-%m-%Y 23:59".to_string())
            };

            if let Some(ref mut sk) = report.start_date_key {
                if sk.format.is_empty() {
                    sk.format = default_start_fmt.clone();
                    config_updated = true;
                }
            }
            if let Some(ref mut ek) = report.end_date_key {
                if ek.format.is_empty() {
                    ek.format = default_end_fmt.clone();
                    config_updated = true;
                }
            }
        }

        config_updated
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.url.trim().is_empty() {
            anyhow::bail!("url cannot be empty");
        }
        if self.concurrency == 0 {
            anyhow::bail!("concurrency must be greater than 0");
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportConfig {
    pub report_type: String,
    #[serde(default)]
    pub filters: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date_key: Option<DateKeyConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date_key: Option<DateKeyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YaswebConfig {
    pub url: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub headless: bool,
    #[serde(default)]
    pub keep_open: bool,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub reports: HashMap<String, ReportConfig>,
    #[serde(default = "default_stdout_log_level")]
    pub log_stdout_level: String,
    #[serde(default = "default_file_log_level")]
    pub log_file_level: String,
}

fn default_stdout_log_level() -> String {
    "DEBUG".to_string()
}

fn default_file_log_level() -> String {
    "TRACE".to_string()
}

fn default_concurrency() -> usize {
    6
}

impl Default for YaswebConfig {
    fn default() -> Self {
        Self {
            url: "https://example.com/".to_string(),
            username: "".to_string(),
            password: None,
            headless: false,
            keep_open: false,
            concurrency: 6,
            reports: HashMap::new(),
            log_stdout_level: "DEBUG".to_string(),
            log_file_level: "TRACE".to_string(),
        }
    }
}
