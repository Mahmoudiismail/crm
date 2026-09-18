use crm_tool::crm::auth::{initiate_auth, respond_to_auth_challenge};
use crm_tool::crm::build_client;
use crm_tool::crm::config::AppConfig;

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

#[tokio::test]
async fn test_tls_client_behavior_enabled() {
    let mut config = AppConfig::default();
    config.no_verify_ssl = false;
    let client = build_client(&config).expect("Failed to build client");

    let res = client.get("https://self-signed.badssl.com/").send().await;
    assert!(
        res.is_err(),
        "Client with TLS verification ENABLED should reject self-signed certs"
    );

    let err_str = format!("{:?}", res.err().unwrap());
    assert!(
        err_str.contains("UnknownIssuer")
            || err_str.contains("certificate")
            || err_str.contains("cert")
            || err_str.contains("tls")
            || err_str.contains("handshake"),
        "Error should be TLS/certificate related, got: {}",
        err_str
    );
}

#[tokio::test]
async fn test_tls_client_behavior_disabled_opt_out() {
    let mut config = AppConfig::default();
    config.no_verify_ssl = true;
    let client = build_client(&config).expect("Failed to build client");

    let res = client.get("https://self-signed.badssl.com/").send().await;
    assert!(
        res.is_ok(),
        "Client with TLS verification DISABLED should accept self-signed certs. Error: {:?}",
        res.err()
    );
}

#[tokio::test]
async fn test_initiate_auth_secrets_not_logged_or_returned() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let secret_sentinel = "ACCESS_TOKEN_SENTINEL_INITIATE";

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0; 1024];
            let _ = socket.read(&mut buf).await;

            let json_resp = serde_json::json!({
                "message": "Auth challenge failed",
                "sensitive_token": secret_sentinel
            });
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\n\r\n{}",
                json_resp.to_string().len(),
                json_resp
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    let client = reqwest::Client::new();
    let result = initiate_auth(&client, &url, "client123", "user123", "00aabb").await;

    assert!(result.is_err());

    let err_str = result.unwrap_err().to_string();
    assert!(
        !err_str.contains(secret_sentinel),
        "Result::Err should NOT contain the secret sentinel! Found in: {}",
        err_str
    );
}

#[tokio::test]
async fn test_respond_to_auth_challenge_secrets_not_logged_or_returned() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let secret_sentinel = "ACCESS_TOKEN_SENTINEL_RESPOND";

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0; 1024];
            let _ = socket.read(&mut buf).await;

            let json_resp = serde_json::json!({
                "message": "Auth challenge failed again",
                "sensitive_token": secret_sentinel
            });
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\n\r\n{}",
                json_resp.to_string().len(),
                json_resp
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    let client = reqwest::Client::new();
    let result = respond_to_auth_challenge(
        &client,
        &url,
        "client123",
        "user123",
        "secret123",
        "sig123",
        "time123",
    )
    .await;

    assert!(result.is_err());

    let err_str = result.unwrap_err().to_string();
    assert!(
        !err_str.contains(secret_sentinel),
        "Result::Err should NOT contain the secret sentinel! Found in: {}",
        err_str
    );
}

#[tokio::test]
async fn test_initiate_auth_secrets_not_logged_or_returned_success() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let secret_sentinel = "ACCESS_TOKEN_SENTINEL_INITIATE_SUCCESS";

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0; 1024];
            let _ = socket.read(&mut buf).await;

            let json_resp = serde_json::json!({
                "ChallengeName": "PASSWORD_VERIFIER",
                "ChallengeParameters": {
                    "SRP_B": "b_value",
                    "SALT": "salt_value",
                    "SECRET_BLOCK": "secret_block",
                    "USER_ID_FOR_SRP": "user_id"
                },
                "sensitive_token": secret_sentinel
            });
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_resp.to_string().len(),
                json_resp
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    let client = reqwest::Client::new();
    let result = initiate_auth(&client, &url, "client123", "user123", "00aabb").await;

    assert!(result.is_ok(), "initiate_auth failed: {:?}", result.err());
}

#[tokio::test]
async fn test_respond_to_auth_challenge_secrets_not_logged_or_returned_success() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    let secret_sentinel = "ACCESS_TOKEN_SENTINEL_RESPOND_SUCCESS";

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0; 1024];
            let _ = socket.read(&mut buf).await;

            let json_resp = serde_json::json!({
                "AuthenticationResult": {
                    "AccessToken": secret_sentinel,
                    "IdToken": "id",
                    "RefreshToken": "refresh",
                    "ExpiresIn": 3600
                }
            });
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_resp.to_string().len(),
                json_resp
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    let client = reqwest::Client::new();
    let result = respond_to_auth_challenge(
        &client,
        &url,
        "client123",
        "user123",
        "secret123",
        "sig123",
        "time123",
    )
    .await;

    assert!(
        result.is_ok(),
        "respond_to_auth_challenge failed: {:?}",
        result.err()
    );
    let auth = result.unwrap();
    assert_eq!(
        auth.access_token, secret_sentinel,
        "Token parsed correctly from mock"
    );
}
