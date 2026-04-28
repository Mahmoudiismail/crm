use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{debug, info};

/// Download a CSV file from a signed URL.
/// - Extracts filename from the URL path (URL-decoded).
/// - Saves to the provided `target_dir` (creating it if missing).
/// - 60-second timeout.
pub async fn download_csv(
    client: &reqwest::Client,
    url: &str,
    report_key: &str,
    target_dir: &Path,
) -> Result<String> {
    let filename = extract_filename(url)?;
    let dest_path = target_dir.join(&filename);
    info!(
        "[{}] Downloading CSV: {} → {:?}",
        report_key, url, dest_path
    );

    // Ensure the directory exists
    tokio::fs::create_dir_all(target_dir)
        .await
        .with_context(|| format!("Failed to create download directory: {:?}", target_dir))?;

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

    let file = tokio::fs::File::create(&dest_path)
        .await
        .with_context(|| format!("Failed to create file: {:?}", dest_path))?;
    let mut writer = BufWriter::with_capacity(128 * 1024, file);

    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("[{}] Error reading stream", report_key))?;
        writer.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
    }

    writer.flush().await?;
    info!(
        "[{}] Download complete: {} ({} bytes)",
        report_key, filename, downloaded
    );
    Ok(filename)
}

/// Extract a human-readable filename from a URL, URL-decoding it and sanitizing to prevent path traversal.
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

    // Sanitize: remove any path traversal attempts.
    // Replace Windows-style backslashes first, then use standard Path::file_name
    // to handle OS-specific edge cases safely.
    let safe_name = name.replace('\\', "/");

    // Some path parsers might see C:cmd.exe as a file C:cmd.exe on linux but drive letter on windows.
    // To be perfectly safe, split by ':' as well to remove any drive letters entirely if present.
    let no_drive = safe_name.rsplit(':').next().unwrap_or(&safe_name);

    let final_name = Path::new(no_drive)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("download.csv");

    Ok(final_name.to_string())
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

    #[test]
    fn test_extract_filename_path_traversal() {
        // Unix style
        let url = "https://example.com/path/to/%2e%2e%2fetc%2fpasswd";
        assert_eq!(extract_filename(url).unwrap(), "passwd");

        // Windows style
        let url = "https://example.com/path/to/%2e%2e%5c%2e%2e%5ccmd.exe";
        assert_eq!(extract_filename(url).unwrap(), "cmd.exe");

        // Absolute path Windows
        let url = "https://example.com/path/to/C%3A%5CWindows%5CSystem32%5Ccmd.exe";
        assert_eq!(extract_filename(url).unwrap(), "cmd.exe");

        // Drive letter edge case
        let url = "https://example.com/path/to/C%3Acmd.exe";
        assert_eq!(extract_filename(url).unwrap(), "cmd.exe");

        // Edge case: just dot
        let url = "https://example.com/path/to/%2e";
        assert_eq!(extract_filename(url).unwrap(), "download.csv");

        // Edge case: just double dot
        let url = "https://example.com/path/to/%2e%2e";
        assert_eq!(extract_filename(url).unwrap(), "download.csv");
    }
}
