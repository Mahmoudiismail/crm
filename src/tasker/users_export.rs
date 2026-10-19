use crate::tasker::config::UsersExportConfig;
use anyhow::{Context, Result};
use calamine::Reader;
use csv::{ReaderBuilder, WriterBuilder};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
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
        let key = username;
        if !key.is_empty() && !department.trim().is_empty() {
            mapping.insert(key, department.trim().to_string());
        }
    }

    if mapping.is_empty() {
        anyhow::bail!("No user-to-department mappings were found in Table2 (Sheet2 columns H:I)");
    }
    Ok(mapping)
}

fn recent_latest_user_csvs(download_dir: &Path, minutes_ago: u64) -> Result<Vec<PathBuf>> {
    let cutoff = SystemTime::now() - Duration::from_secs(minutes_ago.saturating_mul(60));
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(download_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default();
        if !name.to_ascii_lowercase().starts_with("users_")
            || !path
                .extension()
                .and_then(|v| v.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        {
            continue;
        }

        let metadata = entry.metadata().with_context(|| format!("Failed to read metadata for {:?}", path))?;
        let created = metadata.created().or_else(|_| metadata.modified());
        let modified = metadata.modified();
        match (created, modified) {
            (Ok(created), Ok(modified)) if created >= cutoff => {
                files.push((modified, path));
            }
            (Ok(_), Ok(_)) => {}
            (Err(err), _) | (_, Err(err)) => {
                warn!("Could not determine timestamps for {:?}: {}", path, err)
            }
        }
    }

    let Some(latest_modified) = files.iter().map(|(modified, _)| *modified).max() else {
        return Ok(Vec::new());
    };
    Ok(files
        .into_iter()
        .filter(|(modified, _)| *modified == latest_modified)
        .map(|(_, path)| path)
        .collect())
}

// Power Query imports the combined CSV binaries using code page 1252.
fn decode_windows_1252(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    const CP1252: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
        '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
        '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];
    let mut decoded = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x80..=0x9F => decoded.push(CP1252[(byte - 0x80) as usize]),
            _ => decoded.push(char::from(byte)),
        }
    }
    decoded
}

pub fn run(config: &UsersExportConfig) -> Result<()> {
    let download_dir = resolve(&config.download_path);
    let mapping_path = resolve(&config.mapping_workbook);
    let output_path = resolve(&config.output_file);
    let files = recent_latest_user_csvs(&download_dir, config.minutes_ago)?;

    if files.is_empty() {
        anyhow::bail!(
            "No users_*.csv files created within the last {} minute(s) in {:?}",
            config.minutes_ago,
            download_dir
        );
    }

    let mapping = load_department_mapping(&mapping_path)?;
    let mut combined_bytes = Vec::new();
    for path in &files {
        combined_bytes.extend_from_slice(
            &fs::read(path).with_context(|| format!("Failed to read users CSV {:?}", path))?,
        );
    }
    let combined_csv = decode_windows_1252(&combined_bytes);
    let mut reader = ReaderBuilder::new().from_reader(Cursor::new(combined_csv.as_bytes()));
    let headers = reader.headers()?.clone();
    let first_name_idx =
        find_column(&headers, "First Name").context("Required users CSV column 'First Name' is missing")?;
    let last_name_idx =
        find_column(&headers, "Last Name").context("Required users CSV column 'Last Name' is missing")?;
    let branches_idx =
        find_column(&headers, "Branches").context("Required users CSV column 'Branches' is missing")?;
    let teams_idx =
        find_column(&headers, "Teams").context("Required users CSV column 'Teams' is missing")?;

    let mut output_rows = Vec::new();
    for record in reader.records() {
        let record = record.context("Malformed record in combined users CSV data")?;
        let username = format!(
            "{} {}",
            record.get(first_name_idx).unwrap_or_default(),
            record.get(last_name_idx).unwrap_or_default()
        );
        let mapped_department = mapping.get(&username).cloned().unwrap_or_default();
        let teams = record.get(teams_idx).unwrap_or_default().to_string();
        let final_department = if mapped_department.is_empty() {
            teams.clone()
        } else {
            mapped_department.clone()
        };

        output_rows.push(csv::StringRecord::from(vec![
            final_department,
            username,
            record.get(branches_idx).unwrap_or_default().to_string(),
            teams,
            mapped_department,
        ]));
    }

    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create users CSV output directory {:?}", parent))?;
    }
    let mut writer = WriterBuilder::new()
        .from_path(&output_path)
        .with_context(|| format!("Failed to create users CSV {:?}", output_path))?;
    writer.write_record(OUTPUT_HEADERS)?;
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
    fn windows_1252_decodes_non_ascii_bytes() {
        assert_eq!(decode_windows_1252(&[b'A', 0x80, b'B']), "A€B");
        assert_eq!(decode_windows_1252(&[0xEF, 0xBB, 0xBF, b'A']), "A");
    }

    #[test]
    fn output_headers_match_the_users_workbook_query() {
        assert_eq!(OUTPUT_HEADERS.len(), 5);
        assert_eq!(OUTPUT_HEADERS[4], "NewColumn.UserDepartmentName / Team Name");
    }
}
