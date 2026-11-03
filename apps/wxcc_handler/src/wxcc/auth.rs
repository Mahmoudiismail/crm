use crate::wxcc::config::WxccConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use std::path::Path;

const AUTHORIZE_ENDPOINT: &str = "https://webexapis.com/v1/authorize";
const TOKEN_ENDPOINT: &str = "https://webexapis.com/v1/access_token";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

pub fn authorization_url(config: &WxccConfig) -> Result<(String, String)> {
    config.validate(true)?;
    let state = Alphanumeric.sample_string(&mut rand::rng(), 40);
    let scopes = config.scopes.join(" ");
    let url = format!(
        "{AUTHORIZE_ENDPOINT}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        urlencoding::encode(&config.client_id), urlencoding::encode(&config.redirect_uri),
        urlencoding::encode(&scopes), urlencoding::encode(&state),
    );
    Ok((url, state))
}

pub async fn exchange_code(config_path: &Path, mut config: WxccConfig, code: &str, state: &str) -> Result<()> {
    config.validate(true)?;
    if code.trim().is_empty() { anyhow::bail!("Authorization code cannot be empty"); }
    let expected = config.pending_oauth_state.as_deref()
        .context("No pending OAuth state was saved. Generate a new authorization URL first.")?;
    if expected != state { anyhow::bail!("OAuth state mismatch; authorization response was not accepted"); }

    let response = reqwest::Client::new().post(TOKEN_ENDPOINT).form(&[
        ("grant_type", "authorization_code"), ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()), ("code", code),
        ("redirect_uri", config.redirect_uri.as_str()),
    ]).send().await.context("Failed to exchange Webex authorization code")?
        .error_for_status().context("Webex rejected the authorization-code exchange")?;
    let tokens: TokenResponse = response.json().await.context("Webex returned an invalid token response")?;
    apply_tokens(&mut config, tokens);
    config.pending_oauth_state = None;
    config.save(config_path)
}

pub async fn refresh_token(config_path: &Path, mut config: WxccConfig) -> Result<()> {
    config.validate(true)?;
    let refresh = config.refresh_token.as_deref().filter(|v| !v.is_empty())
        .context("No refresh_token is stored; authorize the integration first")?;
    let response = reqwest::Client::new().post(TOKEN_ENDPOINT).form(&[
        ("grant_type", "refresh_token"), ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()), ("refresh_token", refresh),
    ]).send().await.context("Failed to refresh the Webex access token")?
        .error_for_status().context("Webex rejected the refresh-token request")?;
    let tokens: TokenResponse = response.json().await.context("Webex returned an invalid refresh response")?;
    apply_tokens(&mut config, tokens);
    config.save(config_path)
}

fn apply_tokens(config: &mut WxccConfig, tokens: TokenResponse) {
    config.access_token = Some(tokens.access_token);
    if let Some(refresh) = tokens.refresh_token { config.refresh_token = Some(refresh); }
    config.access_token_expires_at = Some(Utc::now().timestamp() + tokens.expires_in);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authorization_url_encodes_scopes_and_includes_state() {
        let config = WxccConfig {
            client_id: "test-client".into(), client_secret: "test-secret".into(),
            redirect_uri: "https://localhost/callback".into(),
            scopes: vec!["cjp:config_read".into(), "cjp:user".into()], ..Default::default()
        };
        let (url, state) = authorization_url(&config).unwrap();
        assert!(url.starts_with(AUTHORIZE_ENDPOINT));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=cjp%3Aconfig_read%20cjp%3Auser"));
        assert!(url.contains(&format!("state={state}")));
        assert_eq!(state.len(), 40);
    }
    #[test]
    fn token_expiry_and_refresh_rotation_are_saved_in_memory() {
        let mut config = WxccConfig::default();
        apply_tokens(&mut config, TokenResponse { access_token: "access".into(), refresh_token: Some("refresh".into()), expires_in: 3600 });
        assert_eq!(config.access_token.as_deref(), Some("access"));
        assert_eq!(config.refresh_token.as_deref(), Some("refresh"));
        assert!(config.access_token_expires_at.unwrap() > Utc::now().timestamp());
    }
}
