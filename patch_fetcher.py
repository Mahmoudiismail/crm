import re

with open('src/crm/fetcher.rs', 'r') as f:
    content = f.read()

# Add std::time::SystemTime to imports
content = content.replace("use std::fmt::Write;", "use std::fmt::Write;\nuse std::time::SystemTime;\nuse std::path::Path;")
content = content.replace("use std::sync::Arc;", "use std::sync::Arc;\nuse std::fs;")

# Update fetch_reports signature
old_sig = """pub async fn fetch_reports(
    config: &AppConfig,
    client: &reqwest::Client,
    token: &str,
    report_type: ReportType,
) -> Result<Value> {"""

new_sig = """pub async fn fetch_reports(
    config: &AppConfig,
    client: &reqwest::Client,
    token: &str,
    report_type: ReportType,
    download_dir: &Path,
) -> Result<Value> {"""

content = content.replace(old_sig, new_sig)

# Add recent file check function
recent_file_fn = """
fn has_recent_download(download_dir: &Path, prefix: &str) -> bool {
    let threshold = SystemTime::now() - std::time::Duration::from_secs(30);

    if let Ok(entries) = fs::read_dir(download_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(prefix) && name.ends_with(".csv") {
                            if let Ok(modified) = metadata.modified() {
                                if modified >= threshold {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
"""

content = content + recent_file_fn

with open('src/crm/fetcher.rs', 'w') as f:
    f.write(content)
