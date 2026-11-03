use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WxccConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub access_token_expires_at: Option<i64>,
    pub pending_oauth_state: Option<String>,
    pub log_stdout_level: String,
    pub log_file_level: String,
}

impl Default for WxccConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(), client_secret: String::new(), redirect_uri: String::new(),
            scopes: Vec::new(), access_token: None, refresh_token: None,
            access_token_expires_at: None, pending_oauth_state: None,
            log_stdout_level: "INFO".to_string(), log_file_level: "DEBUG".to_string(),
        }
    }
}

impl std::fmt::Debug for WxccConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WxccConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_uri", &self.redirect_uri)
            .field("scopes", &self.scopes)
            .field("access_token", &self.access_token.as_ref().map(|_| "[REDACTED]"))
            .field("refresh_token", &self.refresh_token.as_ref().map(|_| "[REDACTED]"))
            .field("access_token_expires_at", &self.access_token_expires_at)
            .field("pending_oauth_state", &self.pending_oauth_state.as_ref().map(|_| "[REDACTED]"))
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level).finish()
    }
}

impl WxccConfig {
    pub fn load(path: &str) -> Result<Self> {
        let path = Path::new(path);
        crm_tool::utils::load_or_create_config(path, &Self::default())
            .with_context(|| format!("Failed to load or create config at {path:?}"))
    }

    pub fn validate(&self, require_oauth: bool) -> Result<()> {
        for (name, level) in [("log_stdout_level", self.log_stdout_level.as_str()), ("log_file_level", self.log_file_level.as_str())] {
            if !["trace", "debug", "info", "warn", "error"].contains(&level.to_ascii_lowercase().as_str()) {
                anyhow::bail!("Invalid {name}: {level}");
            }
        }
        if require_oauth {
            if self.client_id.trim().is_empty() { anyhow::bail!("client_id must be configured before OAuth authorization"); }
            if self.client_secret.trim().is_empty() { anyhow::bail!("client_secret must be configured before OAuth authorization"); }
            if self.redirect_uri.trim().is_empty() { anyhow::bail!("redirect_uri must match the URI registered in the Webex integration"); }
            if self.scopes.is_empty() { anyhow::bail!("At least one Webex OAuth scope must be configured"); }
        }
        Ok(())
    }

    pub fn has_access_token(&self) -> bool {
        self.access_token.as_ref().is_some_and(|token| !token.is_empty()) && self.access_token_expires_at.is_some()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let serialized = serde_json::to_string_pretty(self).context("Failed to serialize WXCC Handler configuration")?;
        crm_tool::utils::atomic_write(path, &serialized)
            .with_context(|| format!("Failed to save WXCC Handler config to {path:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_deserialize_and_validate_without_oauth() {
        let config: WxccConfig = serde_json::from_str("{}").unwrap();
        assert!(config.validate(false).is_ok());
        assert!(config.client_id.is_empty());
        assert!(config.access_token.is_none());
    }
    #[test]
    fn debug_output_redacts_client_secret_and_tokens() {
        let config = WxccConfig { client_secret: "secret-value".into(), access_token: Some("access-value".into()), refresh_token: Some("refresh-value".into()), ..Default::default() };
        let output = format!("{config:?}");
        assert!(!output.contains("secret-value"));
        assert!(!output.contains("access-value"));
        assert!(!output.contains("refresh-value"));
    }
    #[test]
    fn oauth_validation_requires_client_id() {
        assert!(WxccConfig::default().validate(true).unwrap_err().to_string().contains("client_id"));
    }
}
