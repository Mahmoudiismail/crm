use crm_tool::crm::config::AppConfig;
use crm_tool::crm::fetcher::fetch_reports;
use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_crm_signed_url_retry_and_fresh_url() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    // Track request counts
    let fetch_attempt = Arc::new(AtomicUsize::new(0));
    let download_attempt = Arc::new(AtomicUsize::new(0));
    let fetch_attempt_clone = fetch_attempt.clone();
    let download_attempt_clone = download_attempt.clone();

    // Spawn mock server
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            // Read basic request
            let mut buf = [0; 1024];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                .await
                .unwrap();
            let req_str = String::from_utf8_lossy(&buf);

            if req_str.contains("GET /task/download-ticket-data") {
                let current_fetch = fetch_attempt_clone.fetch_add(1, Ordering::SeqCst);

                // Return a signed URL on both fetch attempts.
                // It asks for fresh URL when the first signed URL fails 3 times,
                // so the fetch endpoint should be hit exactly twice.
                let signed_url = format!("http://{}/signed-download-{}", addr, current_fetch);

                let resp_body = format!(r#"{{"data": {{"url": "{}"}}}}"#, signed_url);

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
                    resp_body.len(),
                    resp_body
                );
                let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
            } else if req_str.contains("GET /signed-download-") {
                download_attempt_clone.fetch_add(1, Ordering::SeqCst);

                // Always fail downloads to trigger retries and fresh URL fetch
                let response = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n";
                let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
            } else {
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
            }
        }
    });

    // Configure test fetch
    let config = AppConfig {
        base_url: base_url.clone(),
        access_token: "dummy-token".to_string(),
        from_date: "2026-01-01".to_string(),
        to_date: "2026-01-31".to_string(),
        download_csv: true,
        ..Default::default()
    };

    let config_mutex = Arc::new(Mutex::new(config));
    let client = Client::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let download_dir = temp_dir.path().to_path_buf();

    // Call fetch_reports
    let result = fetch_reports(
        config_mutex,
        &client,
        vec!["tickets".to_string()],
        &download_dir,
    )
    .await;

    // Verify expectations:
    // 1. Initial fetch -> success (signed url)
    // 2. Download attempt 1, 2, 3 -> fail
    // 3. Fresh fetch -> success (fresh signed url)
    // 4. Fresh download attempt 1, 2, 3 -> fail
    // Total fetch = 2, Total download = 6
    // The final result should be an error because the download ultimately failed.

    assert!(
        result.is_err(),
        "Expected fetch_reports to return an error when all download attempts fail"
    );

    let fetch_count = fetch_attempt.load(Ordering::SeqCst);
    let download_count = download_attempt.load(Ordering::SeqCst);

    assert_eq!(
        fetch_count, 2,
        "Expected exactly 2 requests to the fetch endpoint (original + fresh)"
    );
    assert_eq!(
        download_count, 6,
        "Expected exactly 6 download attempts (3 original + 3 fresh)"
    );
}
