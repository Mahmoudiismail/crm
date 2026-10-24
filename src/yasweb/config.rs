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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct YaswebConfig {
    pub url: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub headless: bool,
    #[serde(default)]
    pub keep_open: bool,
    #[serde(default = "default_timeout_minutes")]
    pub timeout_minutes: u64,
    /// Maximum number of browser automation operations running at once.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub reports: HashMap<String, ReportConfig>,
    #[serde(default = "default_stdout_log_level")]
    pub log_stdout_level: String,
    #[serde(default = "default_file_log_level")]
    pub log_file_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
}

fn default_timeout_minutes() -> u64 {
    60
}

fn default_concurrency() -> usize {
    6
}

fn default_stdout_log_level() -> String {
    "DEBUG".to_string()
}

fn default_file_log_level() -> String {
    "TRACE".to_string()
}

impl Default for YaswebConfig {
    fn default() -> Self {
        Self {
            url: "https://example.com/".to_string(),
            username: "".to_string(),
            password: None,
            headless: false,
            keep_open: false,
            timeout_minutes: 60,
            concurrency: 6,
            reports: HashMap::new(),
            log_stdout_level: "DEBUG".to_string(),
            log_file_level: "TRACE".to_string(),
            download_path: None,
        }
    }
}

impl std::fmt::Debug for YaswebConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("YaswebConfig")
            .field("url", &self.url)
            .field("username", &if self.username.is_empty() { "" } else { "***REDACTED***" })
            .field(
                "password",
                &self.password.as_ref().map(|_| "***REDACTED***"),
            )
            .field("headless", &self.headless)
            .field("keep_open", &self.keep_open)
            .field("timeout_minutes", &self.timeout_minutes)
            .field("concurrency", &self.concurrency)
            .field("reports", &self.reports)
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level)
            .field("download_path", &self.download_path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrency_is_configurable_and_defaults_to_six() {
        assert_eq!(YaswebConfig::default().concurrency, 6);
        let config: YaswebConfig = serde_json::from_str(
            r#"{"url":"https://example.test/","username":"test","concurrency":4,"reports":{}}"#,
        )
        .unwrap();
        assert_eq!(config.concurrency, 4);
        assert_eq!(serde_json::to_value(config).unwrap()["concurrency"], 4);
    }

    #[test]
    fn zero_concurrency_means_unlimited_and_is_valid() {
        let config = YaswebConfig {
            concurrency: 0,
            ..YaswebConfig::default()
        };
        assert!(config.validate().is_ok());
    }
}
