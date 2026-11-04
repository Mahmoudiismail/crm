use crate::wxcc::config::WxccConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use std::{collections::HashMap, path::Path, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

const AUTHORIZE_ENDPOINT: &str = "https://webexapis.com/v1/authorize";
const TOKEN_ENDPOINT: &str = "https://webexapis.com/v1/access_token";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(180);
const CALLBACK_REQUEST_LIMIT: usize = 8192;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    refresh_token_expires_in: Option<i64>,
}

pub fn authorization_url(config: &WxccConfig) -> Result<(String, String)> {
    config.validate(true)?;
    let state = Alphanumeric.sample_string(&mut rand::rng(), 40);
    let scopes = config.scopes.join(" ");
    let url = format!(
        "{AUTHORIZE_ENDPOINT}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&config.redirect_uri),
        urlencoding::encode(&scopes),
        urlencoding::encode(&state),
    );
    Ok((url, state))
}

pub fn create_authorization_request(
    config_path: &Path,
    mut config: WxccConfig,
) -> Result<(String, String)> {
    let (url, state) = authorization_url(&config)?;
    config.pending_oauth_state = Some(state.clone());
    config.save(config_path)?;
    Ok((url, state))
}

pub async fn login(config_path: &Path, mut config: WxccConfig) -> Result<()> {
    config.validate(true)?;
    let (port, callback_path) = parse_loopback_redirect_uri(&config.redirect_uri)?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| {
            format!("Could not listen on 127.0.0.1:{port}; the callback port may already be in use")
        })?;

    let (url, state) = create_authorization_request(config_path, config.clone())?;
    config.pending_oauth_state = Some(state.clone());
    webbrowser::open(&url).context("Failed to open the default browser")?;

    println!("Waiting for Webex authorization in your browser...");
    let (code, returned_state, mut stream) = wait_for_callback(listener, &callback_path).await?;
    if returned_state != state {
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("OAuth state mismatch; authorization response was not accepted");
    }

    match exchange_code(config_path, config, &code, &returned_state).await {
        Ok(()) => {
            send_callback_response(&mut stream, true).await?;
            println!("Webex authorization completed. Tokens were saved locally.");
            Ok(())
        }
        Err(err) => {
            let _ = send_callback_response(&mut stream, false).await;
            Err(err)
        }
    }
}

fn parse_loopback_redirect_uri(redirect_uri: &str) -> Result<(u16, String)> {
    let uri = redirect_uri
        .strip_prefix("http://")
        .context("Automatic OAuth login requires an http:// loopback redirect_uri")?;
    let (authority, path) = uri.split_once('/').context(
        "redirect_uri must include a callback path, e.g. http://127.0.0.1:8765/callback",
    )?;
    let (host, port) = authority
        .rsplit_once(':')
        .context("redirect_uri must include an explicit callback port")?;
    if host != "127.0.0.1" && !host.eq_ignore_ascii_case("localhost") {
        anyhow::bail!("Automatic OAuth login only supports localhost loopback redirect URIs");
    }
    let port = port
        .parse::<u16>()
        .context("redirect_uri contains an invalid callback port")?;
    if port == 0 {
        anyhow::bail!("redirect_uri callback port must be between 1 and 65535");
    }
    if path.is_empty() || path.contains('?') || path.contains('#') {
        anyhow::bail!("redirect_uri must contain a plain callback path without query or fragment");
    }
    Ok((port, format!("/{path}")))
}

async fn wait_for_callback(
    listener: TcpListener,
    expected_path: &str,
) -> Result<(String, String, TcpStream)> {
    let (mut stream, _) = timeout(CALLBACK_TIMEOUT, listener.accept())
        .await
        .context("Timed out waiting for the Webex OAuth callback")?
        .context("Failed to accept the Webex OAuth callback")?;

    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    loop {
        let read = timeout(Duration::from_secs(10), stream.read(&mut chunk))
            .await
            .context("Timed out reading the OAuth callback request")?
            .context("Failed to read the OAuth callback request")?;
        if read == 0 {
            anyhow::bail!("Webex closed the OAuth callback connection unexpectedly");
        }
        request.extend_from_slice(&chunk[..read]);
        if request.len() > CALLBACK_REQUEST_LIMIT {
            anyhow::bail!("OAuth callback request exceeded the allowed size");
        }
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let request =
        String::from_utf8(request).context("OAuth callback request was not valid UTF-8")?;
    let request_line = request
        .lines()
        .next()
        .context("OAuth callback had no HTTP request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    if method != "GET" {
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("OAuth callback must use HTTP GET");
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != expected_path {
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("OAuth callback path did not match the configured redirect_uri");
    }

    let parameters = parse_query_parameters(query)?;
    if let Some(error) = parameters.get("error") {
        let description = parameters
            .get("error_description")
            .map(String::as_str)
            .unwrap_or("No additional details were provided");
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("Webex authorization failed: {error}: {description}");
    }
    let code = parameters
        .get("code")
        .filter(|value| !value.is_empty())
        .context("Webex callback did not contain an authorization code")?
        .clone();
    let state = parameters
        .get("state")
        .filter(|value| !value.is_empty())
        .context("Webex callback did not contain OAuth state")?
        .clone();
    Ok((code, state, stream))
}

fn parse_query_parameters(query: &str) -> Result<HashMap<String, String>> {
    let mut parameters = HashMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = urlencoding::decode(key)
            .context("OAuth callback contained an invalid query parameter name")?
            .into_owned();
        let value = urlencoding::decode(value)
            .context("OAuth callback contained an invalid query parameter value")?
            .into_owned();
        if parameters.insert(key.clone(), value).is_some() {
            anyhow::bail!("OAuth callback contained a duplicate {key} parameter");
        }
    }
    Ok(parameters)
}

async fn send_callback_response(stream: &mut TcpStream, success: bool) -> Result<()> {
    let (status, body) = if success {
        ("200 OK", "<!doctype html><html><body><h2>Webex authorization completed</h2><p>You can close this browser tab and return to WXCC Handler.</p></body></html>")
    } else {
        ("400 Bad Request", "<!doctype html><html><body><h2>Webex authorization failed</h2><p>Return to WXCC Handler and check the displayed error.</p></body></html>")
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .context("Failed to send the OAuth callback response")
}

pub async fn exchange_code(
    config_path: &Path,
    mut config: WxccConfig,
    code: &str,
    state: &str,
) -> Result<()> {
    config.validate(true)?;
    if code.trim().is_empty() {
        anyhow::bail!("Authorization code cannot be empty");
    }
    let expected = config
        .pending_oauth_state
        .as_deref()
        .context("No pending OAuth state was saved. Generate a new authorization URL first.")?;
    if expected != state {
        anyhow::bail!("OAuth state mismatch; authorization response was not accepted");
    }
    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", config.redirect_uri.as_str()),
        ])
        .send()
        .await
        .context("Failed to exchange Webex authorization code")?
        .error_for_status()
        .context("Webex rejected the authorization-code exchange")?;
    let tokens: TokenResponse = response
        .json()
        .await
        .context("Webex returned an invalid token response")?;
    apply_tokens(&mut config, tokens);
    config.pending_oauth_state = None;
    config.save(config_path)
}

pub async fn refresh_token(config_path: &Path, mut config: WxccConfig) -> Result<()> {
    config.validate(true)?;
    let refresh = config
        .refresh_token
        .as_deref()
        .filter(|v| !v.is_empty())
        .context("No refresh_token is stored; authorize the integration first")?;
    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("refresh_token", refresh),
        ])
        .send()
        .await
        .context("Failed to refresh the Webex access token")?
        .error_for_status()
        .context("Webex rejected the refresh-token request")?;
    let tokens: TokenResponse = response
        .json()
        .await
        .context("Webex returned an invalid refresh response")?;
    apply_tokens(&mut config, tokens);
    config.save(config_path)
}

fn apply_tokens(config: &mut WxccConfig, tokens: TokenResponse) {
    config.access_token = Some(tokens.access_token);
    if let Some(refresh) = tokens.refresh_token {
        config.refresh_token = Some(refresh);
    }
    let now = Utc::now().timestamp();
    config.access_token_expires_at = Some(now + tokens.expires_in);
    if let Some(expires_in) = tokens.refresh_token_expires_in {
        config.refresh_token_expires_at = Some(now + expires_in);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_url_encodes_scopes_and_includes_state() {
        let config = WxccConfig {
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
            redirect_uri: "http://127.0.0.1:8765/callback".into(),
            scopes: vec!["cjp:config_read".into(), "cjp:user".into()],
            ..Default::default()
        };
        let (url, state) = authorization_url(&config).unwrap();
        assert!(url.starts_with(AUTHORIZE_ENDPOINT));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=cjp%3Aconfig_read%20cjp%3Auser"));
        assert!(url.contains(&format!("state={state}")));
        assert_eq!(state.len(), 40);
    }

    #[test]
    fn parses_only_loopback_http_redirect_uri_with_explicit_port_and_path() {
        assert_eq!(
            parse_loopback_redirect_uri("http://127.0.0.1:8765/callback").unwrap(),
            (8765, "/callback".to_string())
        );
        assert_eq!(
            parse_loopback_redirect_uri("http://localhost:8765/callback").unwrap(),
            (8765, "/callback".to_string())
        );
        assert!(parse_loopback_redirect_uri("https://127.0.0.1:8765/callback").is_err());
        assert!(parse_loopback_redirect_uri("http://0.0.0.0:8765/callback").is_err());
        assert!(parse_loopback_redirect_uri("http://127.0.0.1/callback").is_err());
        assert!(parse_loopback_redirect_uri("http://127.0.0.1:8765/callback?x=1").is_err());
    }

    #[test]
    fn decodes_callback_query_and_rejects_duplicate_parameters() {
        let parameters = parse_query_parameters("code=abc%2B123&state=state-value").unwrap();
        assert_eq!(parameters.get("code").map(String::as_str), Some("abc+123"));
        assert_eq!(
            parameters.get("state").map(String::as_str),
            Some("state-value")
        );
        assert!(parse_query_parameters("code=one&code=two").is_err());
    }

    #[test]
    fn token_expiry_and_refresh_rotation_are_saved_in_memory() {
        let mut config = WxccConfig::default();
        apply_tokens(
            &mut config,
            TokenResponse {
                access_token: "access".into(),
                refresh_token: Some("refresh".into()),
                expires_in: 3600,
                refresh_token_expires_in: Some(86400),
            },
        );
        assert_eq!(config.access_token.as_deref(), Some("access"));
        assert_eq!(config.refresh_token.as_deref(), Some("refresh"));
        assert!(config.access_token_expires_at.unwrap() > Utc::now().timestamp());
        assert!(config.refresh_token_expires_at.unwrap() > config.access_token_expires_at.unwrap());
    }
}
