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
            let force_standard_report_format = report.report_type == "Standard Report";
            let (default_start_fmt, default_end_fmt) = match report.report_type.as_str() {
                "Report Manager" => ("%d-%b-%Y".to_string(), "%d-%b-%Y".to_string()),
                "Standard Report" => (
                    "%d-%b-%Y 00:00".to_string(),
                    "%d-%b-%Y 23:59".to_string(),
                ),
                _ => ("%d-%m-%Y 00:00".to_string(), "%d-%m-%Y 23:59".to_string()),
            };

            if let Some(ref mut sk) = report.start_date_key {
                if force_standard_report_format || sk.format.is_empty() {
                    if sk.format != default_start_fmt {
                        sk.format = default_start_fmt.clone();
                        config_updated = true;
                    }
                }
            }
            if let Some(ref mut ek) = report.end_date_key {
                if force_standard_report_format || ek.format.is_empty() {
                    if ek.format != default_end_fmt {
                        ek.format = default_end_fmt.clone();
                        config_updated = true;
                    }
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
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub filter_options: HashMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date_key: Option<DateKeyConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date_key: Option<DateKeyConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DiscoveredReportFilters {
    pub names: Vec<String>,
    pub options: HashMap<String, Vec<String>>,
}

impl DiscoveredReportFilters {
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.options.is_empty()
    }
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
            .field(
                "username",
                &if self.username.is_empty() {
                    ""
                } else {
                    "***REDACTED***"
                },
            )
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
    fn report_config_accepts_legacy_files_without_filter_options() {
        let report: ReportConfig =
            serde_json::from_str(r#"{"report_type":"Report Manager","filters":{"Hospital":""}}"#)
                .unwrap();
        assert!(report.filter_options.is_empty());
    }

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

    fn report_with_date_keys(
        report_type: &str,
        start_format: &str,
        end_format: &str,
    ) -> YaswebConfig {
        let mut reports = HashMap::new();
        reports.insert(
            "Example".to_string(),
            ReportConfig {
                report_type: report_type.to_string(),
                filters: HashMap::new(),
                filter_options: HashMap::new(),
                start_date_key: Some(DateKeyConfig {
                    key: "FromDate".to_string(),
                    format: start_format.to_string(),
                }),
                end_date_key: Some(DateKeyConfig {
                    key: "ToDate".to_string(),
                    format: end_format.to_string(),
                }),
                download_path: None,
            },
        );
        YaswebConfig {
            reports,
            ..YaswebConfig::default()
        }
    }

    #[test]
    fn standard_report_defaults_to_full_day_abbreviated_month_format_for_both_keys() {
        let mut config = report_with_date_keys("Standard Report", "", "");
        assert!(config.finalize_runtime_fields());
        let report = &config.reports["Example"];
        assert_eq!(report.start_date_key.as_ref().unwrap().format, "%d-%b-%Y 00:00");
        assert_eq!(report.end_date_key.as_ref().unwrap().format, "%d-%b-%Y 23:59");
        assert!(!config.finalize_runtime_fields());
    }

    #[test]
    fn standard_report_overrides_custom_date_formats_with_full_day_boundaries() {
        let mut config = report_with_date_keys("Standard Report", "%d-%b-%Y", "%d-%b-%Y");
        assert!(config.finalize_runtime_fields());
        let report = &config.reports["Example"];
        assert_eq!(report.start_date_key.as_ref().unwrap().format, "%d-%b-%Y 00:00");
        assert_eq!(report.end_date_key.as_ref().unwrap().format, "%d-%b-%Y 23:59");
    }

    #[test]
    fn report_manager_keeps_abbreviated_month_date_format() {
        let mut config = report_with_date_keys("Report Manager", "", "");
        assert!(config.finalize_runtime_fields());
        let report = &config.reports["Example"];
        assert_eq!(report.start_date_key.as_ref().unwrap().format, "%d-%b-%Y");
        assert_eq!(report.end_date_key.as_ref().unwrap().format, "%d-%b-%Y");
    }

    #[test]
    fn explicit_date_formats_are_preserved_for_standard_report() {
        let mut config = report_with_date_keys("Standard Report", "dd-mm-yyyy", "yyyy/mm/dd");
        assert!(!config.finalize_runtime_fields());
        let report = &config.reports["Example"];
        assert_eq!(report.start_date_key.as_ref().unwrap().format, "dd-mm-yyyy");
        assert_eq!(report.end_date_key.as_ref().unwrap().format, "yyyy/mm/dd");
    }

    #[test]
    fn standard_report_uses_day_abbreviated_month_and_full_day_time_boundaries() {
        let mut config = report_with_date_keys("Standard Report", "", "");
        assert!(config.finalize_runtime_fields());
        let report = config.reports.get("Example").unwrap();
        assert_eq!(report.start_date_key.as_ref().unwrap().format, "%d-%b-%Y 00:00");
        assert_eq!(report.end_date_key.as_ref().unwrap().format, "%d-%b-%Y 23:59");
    }

    #[test]
    fn empty_report_filter_values_survive_config_round_trip() {
        let mut report = report_with_date_keys("Standard Report", "", "")
            .reports
            .remove("Example")
            .unwrap();
        report.filters.insert("Hospital".to_string(), String::new());
        let serialized = serde_json::to_string(&report).unwrap();
        let restored: ReportConfig = serde_json::from_str(&serialized).unwrap();
        assert!(restored.filters.contains_key("Hospital"));
        assert_eq!(restored.filters["Hospital"], "");
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
