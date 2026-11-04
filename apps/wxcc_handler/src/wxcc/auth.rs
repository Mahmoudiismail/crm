use crate::wxcc::config::WxccConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::BufReader, path::{Path, PathBuf}, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tokio_rustls::{rustls, TlsAcceptor};
use url::Url;

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
    let callback = parse_loopback_redirect_uri(&config.redirect_uri)?;
    let listener = TcpListener::bind(("127.0.0.1", callback.port)).await
        .with_context(|| format!("Could not listen on 127.0.0.1:{}; the callback port may already be in use", callback.port))?;
    let tls_acceptor = if callback.https { Some(create_tls_acceptor(&config, config_path)?) } else { None };

    let (url, state) = create_authorization_request(config_path, config.clone())?;
    config.pending_oauth_state = Some(state.clone());
    webbrowser::open(&url).context("Failed to open the default browser")?;

    println!("Waiting for Webex authorization in your browser...");
    let (code, returned_state, mut stream) = if let Some(acceptor) = tls_acceptor {
        let (tcp, _) = timeout(CALLBACK_TIMEOUT, listener.accept()).await
            .context("Timed out waiting for the Webex OAuth callback")?.context("Failed to accept the Webex OAuth callback")?;
        let tls = timeout(Duration::from_secs(10), acceptor.accept(tcp)).await
            .context("Timed out establishing the HTTPS callback connection")?.context("Failed to establish the HTTPS callback connection")?;
        read_callback(tls, &callback.path).await?
    } else {
        let (tcp, _) = timeout(CALLBACK_TIMEOUT, listener.accept()).await
            .context("Timed out waiting for the Webex OAuth callback")?.context("Failed to accept the Webex OAuth callback")?;
        read_callback(tcp, &callback.path).await?
    };
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

#[derive(Debug, PartialEq, Eq)]
struct LoopbackRedirect { https: bool, port: u16, path: String }

fn parse_loopback_redirect_uri(value: &str) -> Result<LoopbackRedirect> {
    let uri = Url::parse(value).context("redirect_uri must be a valid absolute URL")?;
    let https = match uri.scheme() { "http" => false, "https" => true, _ => anyhow::bail!("redirect_uri scheme must be http or https") };
    let host = uri.host_str().context("redirect_uri must contain a host")?;
    if host != "127.0.0.1" && !host.eq_ignore_ascii_case("localhost") {
        anyhow::bail!("Automatic OAuth login only supports localhost loopback redirect URIs");
    }
    let authority = value.split_once("://").map(|(_, rest)| rest.split('/').next().unwrap_or_default()).unwrap_or_default();
    if !authority.rsplit_once(':').is_some_and(|(_, p)| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())) {
        anyhow::bail!("redirect_uri must include an explicit callback port");
    }
    let port = uri.port().context("redirect_uri must include an explicit non-default callback port")?;
    if port == 0 { anyhow::bail!("redirect_uri callback port must be between 1 and 65535"); }
    if uri.query().is_some() || uri.fragment().is_some() || !uri.username().is_empty() || uri.password().is_some() {
        anyhow::bail!("redirect_uri must not contain credentials, query parameters, or a fragment");
    }
    let path = uri.path();
    if path == "/" || path.is_empty() { anyhow::bail!("redirect_uri must contain a callback path"); }
    Ok(LoopbackRedirect { https, port, path: path.to_string() })
}

fn create_tls_acceptor(config: &WxccConfig, config_path: &Path) -> Result<TlsAcceptor> {
    let cert = config.callback_tls_cert_path.as_deref().filter(|v| !v.trim().is_empty())
        .context("HTTPS redirect_uri requires callback_tls_cert_path in wxcc_config.json")?;
    let key = config.callback_tls_key_path.as_deref().filter(|v| !v.trim().is_empty())
        .context("HTTPS redirect_uri requires callback_tls_key_path in wxcc_config.json")?;
    let resolve = |v: &str| {
        let p = PathBuf::from(v);
        if p.is_absolute() { p } else { config_path.parent().unwrap_or_else(|| Path::new(".")).join(p) }
    };
    let cert_path = resolve(cert);
    let key_path = resolve(key);
    let mut cert_reader = BufReader::new(File::open(&cert_path).with_context(|| format!("Failed to open callback TLS certificate: {cert_path:?}"))?);
    let certs = rustls_pemfile::certs(&mut cert_reader).collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to parse callback TLS certificate PEM")?;
    if certs.is_empty() { anyhow::bail!("callback_tls_cert_path does not contain a PEM certificate"); }
    let mut key_reader = BufReader::new(File::open(&key_path).with_context(|| format!("Failed to open callback TLS private key: {key_path:?}"))?);
    let key = rustls_pemfile::private_key(&mut key_reader).context("Failed to parse callback TLS private key PEM")?
        .context("callback_tls_key_path does not contain a supported PEM private key")?;
    let server = rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key)
        .context("Callback TLS certificate and private key do not match or are invalid")?;
    Ok(TlsAcceptor::from(Arc::new(server)))
}

async fn read_callback<S>(mut stream: S, expected_path: &str) -> Result<(String, String, S)>
where S: AsyncRead + AsyncWrite + Unpin {
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    loop {
        let read = timeout(Duration::from_secs(10), stream.read(&mut chunk)).await
            .context("Timed out reading the OAuth callback request")?.context("Failed to read the OAuth callback request")?;
        if read == 0 { anyhow::bail!("Webex closed the OAuth callback connection unexpectedly"); }
        request.extend_from_slice(&chunk[..read]);
        if request.len() > CALLBACK_REQUEST_LIMIT { anyhow::bail!("OAuth callback request exceeded the allowed size"); }
        if request.windows(4).any(|w| w == b"\r\n\r\n") { break; }
    }
    let request = String::from_utf8(request).context("OAuth callback request was not valid UTF-8")?;
    let line = request.lines().next().context("OAuth callback had no HTTP request line")?;
    let mut parts = line.split_whitespace();
    if parts.next().unwrap_or_default() != "GET" {
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("OAuth callback must use HTTP GET");
    }
    let target = parts.next().unwrap_or_default();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != expected_path {
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("OAuth callback path did not match the configured redirect_uri");
    }
    let params = parse_query_parameters(query)?;
    if let Some(error) = params.get("error") {
        let description = params.get("error_description").map(String::as_str).unwrap_or("No additional details were provided");
        send_callback_response(&mut stream, false).await?;
        anyhow::bail!("Webex authorization failed: {error}: {description}");
    }
    let code = params.get("code").filter(|v| !v.is_empty()).context("Webex callback did not contain an authorization code")?.clone();
    let state = params.get("state").filter(|v| !v.is_empty()).context("Webex callback did not contain OAuth state")?.clone();
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

async fn send_callback_response<S>(stream: &mut S, success: bool) -> Result<()>
where S: AsyncWrite + Unpin {
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
    fn parses_http_and_https_loopback_redirect_uris_from_config() {
        assert_eq!(parse_loopback_redirect_uri("http://127.0.0.1:8765/callback").unwrap(),
            LoopbackRedirect { https: false, port: 8765, path: "/callback".into() });
        assert_eq!(parse_loopback_redirect_uri("https://localhost:9443/oauth/callback").unwrap(),
            LoopbackRedirect { https: true, port: 9443, path: "/oauth/callback".into() });
        assert!(parse_loopback_redirect_uri("http://0.0.0.0:8765/callback").is_err());
        assert!(parse_loopback_redirect_uri("http://127.0.0.1/callback").is_err());
        assert!(parse_loopback_redirect_uri("http://127.0.0.1:8765/callback?x=1").is_err());
        assert!(parse_loopback_redirect_uri("ftp://127.0.0.1:8765/callback").is_err());
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
