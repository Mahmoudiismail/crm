use crate::tasker::config::UsersExportConfig;
use anyhow::{Context, Result};
use calamine::{DataType, Reader};
use csv::{ReaderBuilder, WriterBuilder};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

const OUTPUT_HEADERS: [&str; 5] = [
    "UserDepartmentName / Team Name",
    "cognito_username\n(Filled by THB)",
    "Branches",
    "Teams",
    "NewColumn.UserDepartmentName / Team Name",
];

fn resolve(path: &str) -> PathBuf {
    crate::tasker::csv_task::resolve_relative_to_exe_dir(path)
}

fn normalized_header(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn find_column(headers: &csv::StringRecord, expected: &str) -> Option<usize> {
    let expected = normalized_header(expected);
    headers
        .iter()
        .position(|header| normalized_header(header) == expected)
}

fn load_department_mapping(path: &Path) -> Result<HashMap<String, String>> {
    let mut workbook = calamine::open_workbook_auto(path)
        .with_context(|| format!("Failed to open users mapping workbook {:?}", path))?;
    let sheet = workbook
        .worksheet_range("Sheet2")
        .context("The users mapping workbook does not contain Sheet2")?;
    let mut mapping = HashMap::new();

    // Table2 is the manually maintained mapping in columns H:I.
    for row in sheet.rows().skip(1) {
        let username = row.get(7).map(ToString::to_string).unwrap_or_default();
        let department = row.get(8).map(ToString::to_string).unwrap_or_default();
        let key = normalized_header(&username);
        if !key.is_empty() && !department.trim().is_empty() {
            mapping.insert(key, department.trim().to_string());
        }
    }

    if mapping.is_empty() {
        anyhow::bail!("No user-to-department mappings were found in Table2 (Sheet2 columns H:I)");
    }
    Ok(mapping)
}

fn recent_user_csvs(download_dir: &Path, minutes_ago: u64) -> Result<Vec<PathBuf>> {
    let cutoff = SystemTime::now() - Duration::from_secs(minutes_ago.saturating_mul(60));
    let entries = fs::read_dir(download_dir)
        .with_context(|| format!("Failed to read users download folder {:?}", download_dir))?;
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
        if !name.to_ascii_lowercase().starts_with("users_")
            || !path.extension().and_then(|v| v.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        {
            continue;
        }

        let metadata = entry.metadata()?;
        let created = metadata.created().or_else(|_| metadata.modified());
        match created {
            Ok(timestamp) if timestamp >= cutoff => files.push((timestamp, path)),
            Ok(_) => {}
            Err(err) => warn!("Could not determine creation time for {:?}: {}", path, err),
        }
    }

    files.sort_by_key(|(created, _)| *created);
    Ok(files.into_iter().map(|(_, path)| path).collect())
}

pub fn run(config: &UsersExportConfig) -> Result<()> {
    let download_dir = resolve(&config.download_path);
    let mapping_path = resolve(&config.mapping_workbook);
    let output_path = resolve(&config.output_file);
    let files = recent_user_csvs(&download_dir, config.minutes_ago)?;

    if files.is_empty() {
        anyhow::bail!(
            "No users_*.csv files were created within the last {} minute(s) in {:?}",
            config.minutes_ago,
            download_dir
        );
    }

    let mapping = load_department_mapping(&mapping_path)?;
    let output_headers = csv::StringRecord::from(OUTPUT_HEADERS);
    let mut output_rows: Vec<csv::StringRecord> = Vec::new();

    for path in &files {
        let mut reader = ReaderBuilder::new()
            .from_path(path)
            .with_context(|| format!("Failed to read users CSV {:?}", path))?;
        let headers = reader.headers()?.clone();
        let team_idx = find_column(&headers, "UserDepartmentName / Team Name")
            .context("Required users CSV column 'UserDepartmentName / Team Name' is missing")?;
        let username_idx = find_column(&headers, "cognito_username (Filled by THB)")
            .context("Required users CSV column 'cognito_username (Filled by THB)' is missing")?;
        let branches_idx = find_column(&headers, "Branches")
            .context("Required users CSV column 'Branches' is missing")?;
        let teams_idx = find_column(&headers, "Teams")
            .context("Required users CSV column 'Teams' is missing")?;

        for record in reader.records() {
            let record = record.with_context(|| format!("Malformed record in {:?}", path))?;
            let username = record.get(username_idx).unwrap_or_default();
            let department = mapping
                .get(&normalized_header(username))
                .cloned()
                .unwrap_or_default();
            output_rows.push(csv::StringRecord::from(vec![
                record.get(team_idx).unwrap_or_default().to_string(),
                username.to_string(),
                record.get(branches_idx).unwrap_or_default().to_string(),
                record.get(teams_idx).unwrap_or_default().to_string(),
                department,
            ]));
        }
    }

    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create users CSV output directory {:?}", parent))?;
    }
    let mut writer = WriterBuilder::new()
        .from_path(&output_path)
        .with_context(|| format!("Failed to create users CSV {:?}", output_path))?;
    writer.write_record(&output_headers)?;
    for record in &output_rows {
        writer.write_record(record)?;
    }
    writer.flush()?;

    info!(
        "Users export completed: {} source file(s), {} row(s), output {:?}",
        files.len(),
        output_rows.len(),
        output_path
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_normalization_handles_line_breaks_and_case() {
        assert_eq!(
            normalized_header("cognito_username\n(Filled by THB)"),
            "cognito_username (filled by thb)"
        );
    }

    #[test]
    fn mapping_keys_ignore_case_and_extra_whitespace() {
        assert_eq!(normalized_header("  Mahmoud  Ibrahim\nIsmail "), "mahmoud ibrahim ismail");
    }

    #[test]
    fn output_headers_match_the_users_workbook_query() {
        assert_eq!(OUTPUT_HEADERS.len(), 5);
        assert_eq!(OUTPUT_HEADERS[4], "NewColumn.UserDepartmentName / Team Name");
    }
}
