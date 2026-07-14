use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{debug, error, info};
use base64::{engine::general_purpose::STANDARD as Base64Standard, Engine as _};

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

/// Process a raw Base64 payload, decode it, validate it as CSV, and save it.
pub async fn process_base64_payload(
    payload: &str,
    report_key: &str,
    target_dir: &Path,
) -> Result<String> {
    info!("[{}] Processing Base64 payload ({} chars)", report_key, payload.len());
    let decode_start = SystemTime::now();

    // Clean up potential surrounding quotes or whitespace from json encoding
    let clean_payload = payload.trim().trim_matches('"');

    if clean_payload.is_empty() {
        anyhow::bail!("[{}] Payload is empty after cleaning", report_key);
    }

    let decoded = Base64Standard.decode(clean_payload)
        .with_context(|| format!("[{}] Failed to decode Base64 payload", report_key))?;

    if let Ok(duration) = decode_start.elapsed() {
        info!("[{}] Base64 decode completed in {:?}", report_key, duration);
    }

    // Check for BOM (Byte Order Mark) and remove it if present.
    // The UTF-8 BOM is EF BB BF.
    let utf8_content = if decoded.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(decoded[3..].to_vec())
    } else {
        String::from_utf8(decoded)
    }.with_context(|| format!("[{}] Decoded payload is not valid UTF-8", report_key))?;

    // Validate CSV and extract stats
    let mut row_count = 0;
    let mut column_count = 0;

    {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true) // assume at least headers exist
            .flexible(true)
            .from_reader(utf8_content.as_bytes());

        if let Ok(headers) = rdr.headers() {
            column_count = headers.len();
        }

        for result in rdr.records() {
            if result.is_err() {
                error!("[{}] CSV Validation error at row {}", report_key, row_count + 1);
            }
            row_count += 1;
        }
    }

    info!("[{}] CSV Validated: {} rows, {} columns", report_key, row_count, column_count);

    // Ensure the directory exists
    tokio::fs::create_dir_all(target_dir)
        .await
        .with_context(|| format!("Failed to create download directory: {:?}", target_dir))?;

    // Create filename user_report_<timestamp>.csv
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let filename = format!("{}_{}.csv", report_key, timestamp);
    let dest_path = target_dir.join(&filename);

    info!("[{}] Saving CSV to {:?}", report_key, dest_path);

    tokio::fs::write(&dest_path, utf8_content)
        .await
        .with_context(|| format!("[{}] Failed to write CSV file to {:?}", report_key, dest_path))?;

    let metadata = tokio::fs::metadata(&dest_path).await?;
    info!("[{}] Output file saved successfully. Size: {} bytes", report_key, metadata.len());

    Ok(filename)
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

    #[tokio::test]
    async fn test_process_base64_payload_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_dir = temp_dir.path();

        let csv_data = "id,name\n1,Jules\n2,Smith\n";
        let base64_payload = Base64Standard.encode(csv_data.as_bytes());

        let result = process_base64_payload(&base64_payload, "test_report", target_dir).await;
        assert!(result.is_ok());

        let filename = result.unwrap();
        assert!(filename.starts_with("test_report_"));
        assert!(filename.ends_with(".csv"));

        let file_path = target_dir.join(filename);
        assert!(file_path.exists());

        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, csv_data);
    }

    #[tokio::test]
    async fn test_process_base64_payload_bom_removal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_dir = temp_dir.path();

        let mut csv_bytes = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        csv_bytes.extend_from_slice(b"id,name\n1,Jules\n2,Smith\n");
        let base64_payload = Base64Standard.encode(&csv_bytes);

        let result = process_base64_payload(&base64_payload, "bom_report", target_dir).await;
        assert!(result.is_ok());

        let file_path = target_dir.join(result.unwrap());
        let content_bytes = std::fs::read(file_path).unwrap();

        // BOM should be stripped in the final output
        assert!(!content_bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert_eq!(String::from_utf8(content_bytes).unwrap(), "id,name\n1,Jules\n2,Smith\n");
    }

    #[tokio::test]
    async fn test_process_base64_payload_empty_and_invalid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_dir = temp_dir.path();

        // Empty
        let res = process_base64_payload("", "empty_rep", target_dir).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Payload is empty"));

        // Invalid Base64
        let res = process_base64_payload("!@#$%", "invalid_rep", target_dir).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Failed to decode Base64"));
    }
}
