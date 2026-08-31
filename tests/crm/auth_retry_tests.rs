use chrono::{TimeDelta, Utc};
use crm_tool::crm::auth::{ensure_authenticated, TokenProvider};
use crm_tool::crm::config::AppConfig;
use crm_tool::crm::fetcher::{execute_with_auth_retry, FetchContext};
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_proactive_expiration_buffer() {
    let mut config = AppConfig {
        access_token: "some_token".into(),
        access_token_expiry: (Utc::now() + TimeDelta::try_minutes(4).unwrap()).to_rfc3339(),
        ..AppConfig::default()
    };

    let client = reqwest::Client::new();

    let result = ensure_authenticated(&mut config, &client, false, false).await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("InitiateAuth failed")
            || err_str.contains("InitiateAuth request failed")
            || err_str.contains("failed to lookup address information")
            || err_str.contains("builder error")
    );
}

#[tokio::test]
async fn test_valid_token_bypasses_auth() {
    let mut config = AppConfig {
        access_token: "valid_token".into(),
        access_token_expiry: (Utc::now() + TimeDelta::try_minutes(10).unwrap()).to_rfc3339(),
        ..AppConfig::default()
    };

    let client = reqwest::Client::new();

    let result = ensure_authenticated(&mut config, &client, false, false).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "valid_token");
}

#[tokio::test]
async fn test_force_refresh_bypasses_cache() {
    let mut config = AppConfig {
        access_token: "valid_token".into(),
        access_token_expiry: (Utc::now() + TimeDelta::try_minutes(10).unwrap()).to_rfc3339(),
        ..AppConfig::default()
    };

    let client = reqwest::Client::new();

    let result = ensure_authenticated(&mut config, &client, false, true).await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("InitiateAuth failed")
            || err_str.contains("InitiateAuth request failed")
            || err_str.contains("failed to lookup address information")
            || err_str.contains("builder error")
    );
}

#[derive(Debug)]
struct MockTokenProvider;

#[async_trait::async_trait]
impl TokenProvider for MockTokenProvider {
    async fn get_token(&self, force_refresh: bool) -> Result<String, anyhow::Error> {
        if force_refresh {
            Ok("token_v2".to_string())
        } else {
            Ok("token_v1".to_string())
        }
    }
}

#[tokio::test]
async fn test_fetch_users_report_401_retry_success() {
    let server = MockServer::start();

    // 1st request with token_v1 gets 401
    let first_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/users/download-user-data")
            .header("authorization", "Bearer token_v1");
        then.status(401)
            .body(json!({"message": "Token expired"}).to_string());
    });

    // 2nd request with token_v2 gets 200
    let retry_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/users/download-user-data")
            .header("authorization", "Bearer token_v2");
        then.status(200).body("c3VjY2Vzcw=="); // "success" in base64
    });

    let context = FetchContext {
        token_provider: Arc::new(MockTokenProvider),
    };

    let client = reqwest::Client::new();
    let url = format!("{}/users/download-user-data", server.base_url());

    let token = "token_v1";

    let resp = execute_with_auth_retry(Some(&context), token, |current_token| {
        let url = url.clone();
        let client = client.clone();
        async move {
            client
                .get(&url)
                .header("authorization", format!("Bearer {}", current_token))
                .send()
                .await
        }
    })
    .await
    .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    let body = resp.text().await.unwrap();
    assert_eq!(body, "c3VjY2Vzcw==");

    first_mock.assert_calls(1);
    retry_mock.assert_calls(1);
}

#[derive(Debug)]
struct FailingTokenProvider;

#[async_trait::async_trait]
impl TokenProvider for FailingTokenProvider {
    async fn get_token(&self, force_refresh: bool) -> Result<String, anyhow::Error> {
        if force_refresh {
            Ok("token_v2".to_string())
        } else {
            Ok("token_v1".to_string())
        }
    }
}

#[tokio::test]
async fn test_fetch_users_report_401_retry_fails_once() {
    let server = MockServer::start();

    // 1st request with token_v1 gets 401
    let first_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/users/download-user-data")
            .header("authorization", "Bearer token_v1");
        then.status(401)
            .body(json!({"message": "Token expired"}).to_string());
    });

    // 2nd request with token_v2 ALSO gets 401
    let retry_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/users/download-user-data")
            .header("authorization", "Bearer token_v2");
        then.status(401)
            .body(json!({"message": "Token expired again"}).to_string());
    });

    let context = FetchContext {
        token_provider: Arc::new(FailingTokenProvider),
    };

    let client = reqwest::Client::new();
    let url = format!("{}/users/download-user-data", server.base_url());

    let token = "token_v1";

    let resp = execute_with_auth_retry(Some(&context), token, |current_token| {
        let url = url.clone();
        let client = client.clone();
        async move {
            client
                .get(&url)
                .header("authorization", format!("Bearer {}", current_token))
                .send()
                .await
        }
    })
    .await
    .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    first_mock.assert_calls(1);
    retry_mock.assert_calls(1);
}
