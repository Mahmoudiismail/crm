use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tracing::{debug, info};

/// All configuration fields — mirrors the JSON config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub user_pool_id: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub no_verify_ssl: bool,
    #[serde(default = "default_true")]
    pub remember_secrets: bool,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub from_date: String,
    #[serde(default)]
    pub calls_from_date: String,
    #[serde(default)]
    pub to_date: String,
    #[serde(default)]
    pub download_csv: bool,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub application_id: String,
    #[serde(default)]
    pub app_timezone_plus_minutes: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub scheduled_time: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_download_folder: Option<String>,

    // Token / auth cache
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub access_token_expiry: String,
    #[serde(default)]
    pub id_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub token_timestamp: String,

    #[serde(skip)]
    pub dynamic_to_date: bool,
    #[serde(skip)]
    pub dynamic_calls_from_date: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            region: "ap-south-1".into(),
            user_pool_id: "ap-south-1_wjZE70ShT".into(),
            client_id: "i7g0t35boqicb1tdc4rgthk6".into(),
            username: "".into(),
            password: "".into(),
            no_verify_ssl: true,
            remember_secrets: true,
            email: "Mahmoud_iismail@rayacx.com".into(),
            from_date: "2025-01-01".into(),
            calls_from_date: "2026-02-01".into(),
            to_date: String::new(),
            download_csv: true,
            account_id: "233b5ff5-8aff-4445-815b-39d7916a1d46".into(),
            application_id: "83921976-97dd-4679-9b36-ee936ecf50d1".into(),
            app_timezone_plus_minutes: "180".into(),
            base_url: "https://crm.fakeeh.care/medi-crm/vault/v1/".into(),
            scheduled_time: "01:00".into(),
            custom_download_folder: None,
            access_token: String::new(),
            access_token_expiry: String::new(),
            id_token: String::new(),
            refresh_token: String::new(),
            token_timestamp: String::new(),
            dynamic_to_date: false,
            dynamic_calls_from_date: false,
        }
    }
}

impl AppConfig {
    /// Load config from file, merging with defaults for any missing keys.
    /// If the file does not exist, create it with defaults.
    pub fn load(path: &str) -> Result<Self> {
        let defaults = AppConfig::default();
        let default_value = serde_json::to_value(&defaults)?;

        if !Path::new(path).exists() {
            info!("Config file not found, creating with defaults: {}", path);
            let cfg = defaults;
            cfg.save(path)?;
            return Ok(cfg);
        }

        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        let mut file_value: Value = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        let mut config_changed = false;
        // Merge defaults into the file value (file takes precedence)
        if let (Value::Object(ref mut file_map), Value::Object(ref default_map)) =
            (&mut file_value, &default_value)
        {
            for (k, v) in default_map {
                if !file_map.contains_key(k) {
                    debug!("Config key '{}' missing, using default", k);
                    file_map.insert(k.clone(), v.clone());
                    config_changed = true;
                }
            }
        }

        let cfg: AppConfig =
            serde_json::from_value(file_value).context("Failed to deserialize merged config")?;

        // Rule: always check config and fix it with each run
        if config_changed {
            info!("Updated configuration file with missing default fields.");
            // save it back, we can just call save since we fully deserialized it
            cfg.save(path)?;
        }

        Ok(cfg)
    }

    /// Finalize runtime-derived fields after loading config.
    pub fn finalize_runtime_fields(&mut self) {
        use crate::utils::to_iso_date;

        // Normalize existing date fields
        self.from_date = to_iso_date(&self.from_date);
        self.to_date = to_iso_date(&self.to_date);
        self.calls_from_date = to_iso_date(&self.calls_from_date);

        // Finalize to_date: if still empty, default to today
        if self.to_date.is_empty() {
            self.to_date = Local::now().format("%Y-%m-%d").to_string();
            self.dynamic_to_date = true;
            debug!("to_date defaulted to today (Local): {}", self.to_date);
        }

        // Finalize calls_from_date: if empty, fall back to from_date
        if self.calls_from_date.is_empty() {
            self.calls_from_date = self.from_date.clone();
            self.dynamic_calls_from_date = true;
            debug!(
                "calls_from_date defaulted to from_date: {}",
                self.calls_from_date
            );
        }
    }

    /// Save config to file, optionally stripping secrets and null values.
    pub fn save(&self, path: &str) -> Result<()> {
        let mut value = serde_json::to_value(self)?;

        if !self.remember_secrets {
            if let Value::Object(ref mut map) = value {
                let secret_keys = [
                    "password",
                    "access_token",
                    "access_token_expiry",
                    "id_token",
                    "refresh_token",
                    "token_timestamp",
                ];
                for key in &secret_keys {
                    map.remove(*key);
                }
                debug!("Stripped secret fields from config (remember_secrets=false)");
            }
        }

        if let Value::Object(ref mut map) = value {
            if self.dynamic_to_date {
                map.remove("to_date");
            }
            if self.dynamic_calls_from_date {
                map.remove("calls_from_date");
            }
        }

        // Strip null values
        strip_nulls(&mut value);

        let pretty = serde_json::to_string_pretty(&value)?;
        std::fs::write(path, pretty)
            .with_context(|| format!("Failed to write config to {}", path))?;
        info!("Config saved to {}", path);
        Ok(())
    }
}

/// Recursively remove null values from a serde_json::Value.
fn strip_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            for (_, v) in map.iter_mut() {
                strip_nulls(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_nulls(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_creates_default_file_if_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("missing_config.json");
        let path_str = config_path.to_str().unwrap();

        // Ensure file does not exist
        assert!(!config_path.exists());

        // Load config, which should create the file with defaults
        let config = AppConfig::load(path_str).expect("Failed to load config");

        // Verify default values
        assert_eq!(config.region, "ap-south-1");
        assert_eq!(config.username, "");

        // Verify the file was actually created
        assert!(config_path.exists());

        // Verify we can parse the created file back and it matches
        let loaded_again = AppConfig::load(path_str).expect("Failed to load newly created config");
        assert_eq!(config.region, loaded_again.region);
    }

    #[test]
    fn test_load_merges_partial_config() {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write a partial config with only a few fields
        let partial_json = r#"{
            "region": "us-east-1",
            "download_csv": false
        }"#;
        temp_file.write_all(partial_json.as_bytes()).unwrap();
        let path_str = temp_file.path().to_str().unwrap();

        let config = AppConfig::load(path_str).expect("Failed to load partial config");

        // Overridden values
        assert_eq!(config.region, "us-east-1");
        assert!(!config.download_csv);

        // Default values should be filled in
        assert_eq!(config.username, "");
        assert!(config.no_verify_ssl);
    }

    #[test]
    fn test_load_returns_error_on_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write invalid JSON
        let invalid_json = r#"{ "region": "us-east-1", "#;
        temp_file.write_all(invalid_json.as_bytes()).unwrap();
        let path_str = temp_file.path().to_str().unwrap();

        let result = AppConfig::load(path_str);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse config file"));
    }

    #[test]
    fn test_save_strips_secrets_when_remember_secrets_is_false() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("save_config.json");
        let path_str = config_path.to_str().unwrap();

        let config = AppConfig {
            remember_secrets: false,
            password: "my_super_secret_password".to_string(),
            access_token: "some_token".to_string(),
            ..AppConfig::default()
        };

        config.save(path_str).expect("Failed to save config");

        // Read the raw JSON
        let raw_json = std::fs::read_to_string(path_str).unwrap();
        let parsed_json: serde_json::Value = serde_json::from_str(&raw_json).unwrap();

        // Verify secrets are missing
        if let serde_json::Value::Object(map) = parsed_json {
            assert!(!map.contains_key("password"), "Password should be stripped");
            assert!(
                !map.contains_key("access_token"),
                "Access token should be stripped"
            );
            assert!(
                map.contains_key("region"),
                "Non-secret fields should remain"
            );
        } else {
            panic!("Expected JSON object");
        }
    }
}
