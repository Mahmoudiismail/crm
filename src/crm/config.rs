use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tracing::{debug, info};

const SECRET_CONFIG_KEYS: [&str; 6] = [
    "password",
    "access_token",
    "access_token_expiry",
    "id_token",
    "refresh_token",
    "token_timestamp",
];

/// Application configuration for the CRM tool.
///
/// Serves as the structured mapping of `crm_config.json`.
/// Invariants:
/// - Secrets (passwords, tokens) are omitted during serialization if `remember_secrets` is false.
/// - Dynamic date variables (e.g., `eomonth`, `today`) are resolved to ISO format upon loading.
#[derive(Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_incomplete_reservation_limit")]
    pub incomplete_reservation_limit: u32,
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

    #[serde(default = "default_recent_download_window_seconds")]
    pub recent_download_window_seconds: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u64>,

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

fn default_true() -> bool {
    true
}

fn default_recent_download_window_seconds() -> u64 {
    30
}

fn default_incomplete_reservation_limit() -> u32 {
    10000
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            region: "ap-south-1".into(),
            user_pool_id: "ap-south-1_wjZE70ShT".into(),
            client_id: "i7g0t35boqicb1tdc4rgthk6".into(),
            username: "".into(),
            password: "".into(),
            no_verify_ssl: false,
            remember_secrets: true,
            email: "Mahmoud_iismail@rayacx.com".into(),
            from_date: "2025-01-01".into(),
            calls_from_date: "2026-02-01".into(),
            to_date: String::new(),
            download_csv: true,
            account_id: "233b5ff5-8aff-4445-815b-39d7916a1d46".into(),
            incomplete_reservation_limit: 10000,
            application_id: "83921976-97dd-4679-9b36-ee936ecf50d1".into(),
            app_timezone_plus_minutes: "180".into(),
            base_url: "https://crm.fakeeh.care/medi-crm/vault/v1/".into(),
            scheduled_time: "01:00".into(),
            custom_download_folder: None,
            recent_download_window_seconds: 30,
            retention_days: None,
            access_token: String::new(),
            access_token_expiry: String::new(),
            id_token: String::new(),
            refresh_token: String::new(),
            token_timestamp: String::new(),
            dynamic_to_date: false,
            dynamic_calls_from_date: false,
            log_stdout_level: "DEBUG".to_string(),
            log_file_level: "TRACE".to_string(),
        }
    }
}

impl AppConfig {
    /// Load config from file, merging with defaults for any missing keys.
    /// If the file does not exist, create it with defaults.
    pub fn load(path: &str) -> Result<Self> {
        let config_path = Path::new(path);
        let mut config: AppConfig =
            crate::utils::load_or_create_config(config_path, &AppConfig::default())?;

        // Migrate legacy JSON tokens to DPAPI before removing them. If JSON no longer
        // contains tokens, restore the token cache from DPAPI when one exists.
        let mut tokens = token_store::TokenCache::from_config(&config);
        if let Some(stored_tokens) = token_store::load(config_path)? {
            tokens.fill_missing_from(stored_tokens);
        }

        if config.remember_secrets {
            config.apply_tokens(tokens);
        } else {
            if tokens.has_tokens() {
                token_store::save(config_path, &tokens)
                    .context("Failed to protect CRM tokens with Windows DPAPI")?;
            } else {
                token_store::remove(config_path)
                    .context("Failed to remove stale CRM token cache")?;
            }
            config.apply_tokens(tokens);

            // Preserve unknown JSON fields while removing the password and all token fields.
            let raw = std::fs::read_to_string(config_path)
                .context("Failed to read CRM config while stripping persisted secrets")?;
            let mut value: Value = serde_json::from_str(&raw)
                .context("Failed to parse CRM config while stripping persisted secrets")?;
            let mut secrets_removed = false;
            if let Value::Object(ref mut map) = value {
                for key in &SECRET_CONFIG_KEYS {
                    secrets_removed |= map.remove(*key).is_some();
                }
            }
            if secrets_removed {
                let pretty = serde_json::to_string_pretty(&value)?;
                crate::utils::atomic_write(config_path, &pretty)
                    .context("Failed to persist CRM config without secrets")?;
            }
        }

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.base_url.trim().is_empty() {
            anyhow::bail!("base_url cannot be empty");
        }
        Ok(())
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

        // (Do NOT overwrite calls_from_date merely because from_date exists)
        // If from_date is present, fetcher.rs logic will prioritize from_date over calls_from_date.
        // If calls_from_date is completely missing we could optionally default it,
        // but it acts as a fallback when from_date is absent.
    }

    /// Save config to file, optionally stripping secrets and null values.
    pub fn save(&self, path: &str) -> Result<()> {
        if !self.remember_secrets {
            // Persist tokens first. If DPAPI fails, do not overwrite the config file or lose
            // the only copy of an existing token. Passwords are deliberately never cached.
            token_store::save(std::path::Path::new(path), &token_store::TokenCache::from_config(self))
                .context("Failed to protect CRM tokens with Windows DPAPI")?;
        }

        let mut value = serde_json::to_value(self)?;

        if !self.remember_secrets {
            if let Value::Object(ref mut map) = value {
                for key in &SECRET_CONFIG_KEYS {
                    map.remove(*key);
                }
                debug!("Stripped password and token fields from config (remember_secrets=false)");
            }
        }

        if let Value::Object(ref mut map) = value {
            if self.dynamic_to_date {
                map.insert(
                    "to_date".to_string(),
                    serde_json::Value::String("".to_string()),
                );
            }
            if self.dynamic_calls_from_date {
                map.insert(
                    "calls_from_date".to_string(),
                    serde_json::Value::String("".to_string()),
                );
            }
        }

        // Strip null values
        strip_nulls(&mut value);

        let pretty = serde_json::to_string_pretty(&value)?;

        if let Ok(existing_content) = std::fs::read_to_string(path) {
            if existing_content == pretty {
                if self.remember_secrets {
                    token_store::remove(std::path::Path::new(path))
                        .context("Failed to remove stale DPAPI token cache")?;
                }
                debug!("Config unchanged, skipping file write");
                return Ok(());
            }
            if let (Ok(existing_val), Ok(new_val)) = (
                serde_json::from_str::<serde_json::Value>(&existing_content),
                serde_json::from_str::<serde_json::Value>(&pretty),
            ) {
                if existing_val == new_val {
                    if self.remember_secrets {
                        token_store::remove(std::path::Path::new(path))
                            .context("Failed to remove stale DPAPI token cache")?;
                    }
                    debug!("Config unchanged, skipping file write");
                    return Ok(());
                }
            }
        }

        crate::utils::atomic_write(std::path::Path::new(path), &pretty)
            .with_context(|| format!("Failed to write config to {}", path))?;

        if self.remember_secrets {
            // The JSON config is now the selected storage mode; remove any stale DPAPI copy.
            token_store::remove(std::path::Path::new(path))
                .context("Failed to remove stale DPAPI token cache")?;
        }

        info!("Config saved to {}", path);
        Ok(())
    }

    fn apply_tokens(&mut self, tokens: token_store::TokenCache) {
        self.access_token = tokens.access_token;
        self.access_token_expiry = tokens.access_token_expiry;
        self.id_token = tokens.id_token;
        self.refresh_token = tokens.refresh_token;
        self.token_timestamp = tokens.token_timestamp;
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

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("region", &self.region)
            .field("user_pool_id", &self.user_pool_id)
            .field("client_id", &self.client_id)
            .field("username", &self.username)
            .field("password", &"***REDACTED***")
            .field("no_verify_ssl", &self.no_verify_ssl)
            .field("remember_secrets", &self.remember_secrets)
            .field("email", &self.email)
            .field("from_date", &self.from_date)
            .field("calls_from_date", &self.calls_from_date)
            .field("to_date", &self.to_date)
            .field("download_csv", &self.download_csv)
            .field("account_id", &self.account_id)
            .field("application_id", &self.application_id)
            .field("app_timezone_plus_minutes", &self.app_timezone_plus_minutes)
            .field("base_url", &self.base_url)
            .field("scheduled_time", &self.scheduled_time)
            .field("custom_download_folder", &self.custom_download_folder)
            .field(
                "recent_download_window_seconds",
                &self.recent_download_window_seconds,
            )
            .field("retention_days", &self.retention_days)
            .field("access_token", &"***REDACTED***")
            .field("access_token_expiry", &self.access_token_expiry)
            .field("id_token", &"***REDACTED***")
            .field("refresh_token", &"***REDACTED***")
            .field("token_timestamp", &"***REDACTED***")
            .field("dynamic_to_date", &self.dynamic_to_date)
            .field("dynamic_calls_from_date", &self.dynamic_calls_from_date)
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level)
            .finish()
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
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let path_str = config_path.to_str().unwrap();

        // Write a partial config with only a few fields
        let partial_json = r#"{
            "region": "us-east-1",
            "download_csv": false
        }"#;
        std::fs::write(&config_path, partial_json).unwrap();

        let config = AppConfig::load(path_str).expect("Failed to load partial config");

        // Overridden values
        assert_eq!(config.region, "us-east-1");
        assert!(!config.download_csv);

        // Default values should be filled in
        assert_eq!(config.username, "");
        assert!(!config.no_verify_ssl);
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

    #[cfg(windows)]
    #[test]
    fn test_load_migrates_legacy_tokens_to_dpapi_and_removes_json_secrets() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let config = AppConfig {
            remember_secrets: false,
            password: "password-secret".to_string(),
            access_token: "access-secret".to_string(),
            access_token_expiry: "expiry-secret".to_string(),
            id_token: "id-secret".to_string(),
            refresh_token: "refresh-secret".to_string(),
            token_timestamp: "timestamp-secret".to_string(),
            ..AppConfig::default()
        };
        let mut serialized = serde_json::to_value(&config).unwrap();
        serialized["future_option"] = serde_json::Value::String("preserve-me".to_string());
        std::fs::write(&config_path, serde_json::to_string(&serialized).unwrap()).unwrap();

        let loaded = AppConfig::load(config_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.password, "password-secret");
        assert_eq!(loaded.access_token, "access-secret");
        assert_eq!(loaded.refresh_token, "refresh-secret");

        let persisted: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(persisted["future_option"], "preserve-me");
        for key in SECRET_CONFIG_KEYS {
            assert!(persisted.get(key).is_none(), "secret key {key} persisted");
        }

        let encrypted = std::fs::read_to_string(token_store::cache_path(&config_path)).unwrap();
        assert!(!encrypted.contains("access-secret"));
        assert!(!encrypted.contains("password-secret"));

        let loaded_again = AppConfig::load(config_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded_again.access_token, "access-secret");
        assert_eq!(loaded_again.id_token, "id-secret");
        assert_eq!(loaded_again.refresh_token, "refresh-secret");
        assert!(loaded_again.password.is_empty());
    }

    #[test]
    fn test_save_strips_secrets_when_remember_secrets_is_false() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("save_config.json");
        let path_str = config_path.to_str().unwrap();

        let config = AppConfig {
            remember_secrets: false,
            password: "my_super_secret_password".to_string(),
            ..AppConfig::default()
        };

        config.save(path_str).expect("Failed to save config");

        let parsed_json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path_str).unwrap()).unwrap();
        for key in SECRET_CONFIG_KEYS {
            assert!(!parsed_json.as_object().unwrap().contains_key(key));
        }
        assert!(parsed_json.get("region").is_some());
    }

    #[cfg(windows)]
    #[test]
    fn test_save_stores_only_tokens_in_dpapi_when_remember_secrets_is_false() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("save_config.json");
        let config = AppConfig {
            remember_secrets: false,
            password: "must-not-be-stored".to_string(),
            access_token: "token-only-secret".to_string(),
            refresh_token: "refresh-only-secret".to_string(),
            ..AppConfig::default()
        };

        config.save(config_path.to_str().unwrap()).unwrap();

        let json = std::fs::read_to_string(&config_path).unwrap();
        assert!(!json.contains("token-only-secret"));
        assert!(!json.contains("must-not-be-stored"));
        let encrypted = std::fs::read_to_string(token_store::cache_path(&config_path)).unwrap();
        assert!(!encrypted.contains("token-only-secret"));
        assert!(!encrypted.contains("must-not-be-stored"));

        let tokens = token_store::load(&config_path).unwrap().unwrap();
        assert_eq!(tokens.access_token, "token-only-secret");
        assert_eq!(tokens.refresh_token, "refresh-only-secret");
    }

    #[test]
    fn test_finalize_runtime_fields() {
        let mut config = AppConfig {
            from_date: "01-May-2026".to_string(),
            to_date: "".to_string(),
            calls_from_date: "01-Apr-2026".to_string(),
            ..AppConfig::default()
        };

        config.finalize_runtime_fields();

        assert_eq!(config.from_date, "2026-05-01");
        // to_date should default to today
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(config.to_date, today);
        assert!(config.dynamic_to_date);

        // calls_from_date should NOT fall back to from_date
        assert_eq!(config.calls_from_date, "2026-04-01");
        assert!(!config.dynamic_calls_from_date);
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_default_config_tls_verification_enabled() {
        let config = AppConfig::default();
        assert!(
            !config.no_verify_ssl,
            "TLS verification should be enabled by default"
        );
    }

    #[test]
    fn test_missing_no_verify_ssl_defaults_to_false() {
        let json = r#"{
            "region": "us-east-1"
        }"#;
        let config: AppConfig = serde_json::from_str(json).expect("Failed to parse JSON");
        assert!(
            !config.no_verify_ssl,
            "Missing no_verify_ssl should default to false"
        );
    }

    #[test]
    fn test_explicit_false_no_verify_ssl_is_false() {
        let json = r#"{
            "no_verify_ssl": false
        }"#;
        let config: AppConfig = serde_json::from_str(json).expect("Failed to parse JSON");
        assert!(
            !config.no_verify_ssl,
            "Explicit false no_verify_ssl should be false"
        );
    }

    #[test]
    fn test_explicit_true_no_verify_ssl_is_true() {
        let json = r#"{
            "no_verify_ssl": true
        }"#;
        let config: AppConfig = serde_json::from_str(json).expect("Failed to parse JSON");
        assert!(
            config.no_verify_ssl,
            "Explicit true no_verify_ssl should be true"
        );
    }
}
