use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Download a CSV file from a signed URL.
/// - Extracts filename from the URL path (URL-decoded).
/// - Streams to the current working directory.
/// - 60-second timeout.
pub async fn download_csv(client: &reqwest::Client, url: &str, report_key: &str) -> Result<String> {
    let filename = extract_filename(url)?;
    info!("[{}] Downloading CSV: {} → {}", report_key, url, filename);

    let resp = client
        .get(url)
        .header("accept-encoding", "identity")
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .with_context(|| format!("Failed to GET {}", url))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("[{}] Download HTTP {}: {}", report_key, status, body);
    }

    let content_length = resp.content_length();
    debug!("[{}] Content-Length: {:?}", report_key, content_length);

    let dest = Path::new(&filename);
    let mut file = tokio::fs::File::create(&dest)
        .await
        .with_context(|| format!("Failed to create file: {}", filename))?;

    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("[{}] Error reading stream", report_key))?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
    }

    file.flush().await?;
    info!(
        "[{}] Download complete: {} ({} bytes)",
        report_key, filename, downloaded
    );
    Ok(filename)
}

/// Extract a human-readable filename from a URL, URL-decoding it.
fn extract_filename(url: &str) -> Result<String> {
    // Parse the URL path component
    let path = if let Some(qmark) = url.find('?') {
        &url[..qmark]
    } else {
        url
    };

    let filename = path.rsplit('/').next().unwrap_or("download.csv");

    let decoded = urlencoding::decode(filename)
        .with_context(|| format!("Failed to URL-decode filename: {}", filename))?;

    let name = decoded.to_string();
    if name.is_empty() {
        return Ok("download.csv".into());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename() {
        let url = "https://crm.fakeeh.care/crm-files/reports/Ticket%20Report%202026.csv?X-Amz-Signature=abc123";
        let name = extract_filename(url).unwrap();
        assert_eq!(name, "Ticket Report 2026.csv");
    }

    #[test]
    fn test_extract_filename_no_query() {
        let url = "https://example.com/path/to/file.csv";
        let name = extract_filename(url).unwrap();
        assert_eq!(name, "file.csv");
    }
}
