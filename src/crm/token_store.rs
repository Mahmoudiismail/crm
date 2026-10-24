//! Windows DPAPI-backed persistence for CRM authentication tokens.
//!
//! Only token values and their timing metadata are stored here. Passwords are never
//! included in this payload.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DPAPI_ENTROPY: &[u8] = b"crm_tool.token_cache.v1";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct TokenCache {
    pub access_token: String,
    pub access_token_expiry: String,
    pub id_token: String,
    pub refresh_token: String,
    pub token_timestamp: String,
}

impl TokenCache {
    pub fn from_config(config: &super::config::AppConfig) -> Self {
        Self {
            access_token: config.access_token.clone(),
            access_token_expiry: config.access_token_expiry.clone(),
            id_token: config.id_token.clone(),
            refresh_token: config.refresh_token.clone(),
            token_timestamp: config.token_timestamp.clone(),
        }
    }

    pub fn has_tokens(&self) -> bool {
        !self.access_token.is_empty() || !self.id_token.is_empty() || !self.refresh_token.is_empty()
    }

    pub fn fill_missing_from(&mut self, stored: Self) {
        if self.access_token.is_empty() {
            self.access_token = stored.access_token;
        }
        if self.access_token_expiry.is_empty() {
            self.access_token_expiry = stored.access_token_expiry;
        }
        if self.id_token.is_empty() {
            self.id_token = stored.id_token;
        }
        if self.refresh_token.is_empty() {
            self.refresh_token = stored.refresh_token;
        }
        if self.token_timestamp.is_empty() {
            self.token_timestamp = stored.token_timestamp;
        }
    }
}

pub(super) fn cache_path(config_path: &Path) -> PathBuf {
    let filename = config_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.json".to_string());
    config_path.with_file_name(format!("{filename}.tokens.dpapi"))
}

#[cfg(windows)]
pub(super) fn load(config_path: &Path) -> Result<Option<TokenCache>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use windows_dpapi::{decrypt_data, Scope};

    let path = cache_path(config_path);
    if !path.exists() {
        return Ok(None);
    }

    let encoded = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read DPAPI token cache at {}", path.display()))?;
    let encrypted = STANDARD
        .decode(encoded.trim())
        .context("DPAPI token cache is not valid Base64")?;
    let plaintext = decrypt_data(&encrypted, Scope::User, Some(DPAPI_ENTROPY))
        .context("Failed to decrypt CRM token cache for the current Windows user")?;
    let tokens = serde_json::from_slice(&plaintext)
        .context("Decrypted CRM token cache contains invalid data")?;
    Ok(Some(tokens))
}

#[cfg(not(windows))]
pub(super) fn load(config_path: &Path) -> Result<Option<TokenCache>> {
    let path = cache_path(config_path);
    if path.exists() {
        anyhow::bail!("CRM token cache requires Windows DPAPI");
    }
    Ok(None)
}

#[cfg(windows)]
pub(super) fn save(config_path: &Path, tokens: &TokenCache) -> Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use windows_dpapi::{encrypt_data, Scope};

    let path = cache_path(config_path);
    if !tokens.has_tokens() {
        return remove(config_path);
    }

    let plaintext = serde_json::to_vec(tokens)?;
    let encrypted = encrypt_data(&plaintext, Scope::User, Some(DPAPI_ENTROPY))
        .context("CryptProtectData failed for CRM token cache")?;
    let encoded = STANDARD.encode(encrypted);
    crate::utils::atomic_write(&path, &encoded)
        .with_context(|| format!("Failed to persist DPAPI token cache at {}", path.display()))
}

#[cfg(not(windows))]
pub(super) fn save(config_path: &Path, tokens: &TokenCache) -> Result<()> {
    if tokens.has_tokens() {
        anyhow::bail!("CRM token persistence with remember_secrets=false requires Windows DPAPI");
    }
    remove(config_path)
}

pub(super) fn remove(config_path: &Path) -> Result<()> {
    let path = cache_path(config_path);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("Failed to remove DPAPI token cache at {}", path.display())),
    }
}
