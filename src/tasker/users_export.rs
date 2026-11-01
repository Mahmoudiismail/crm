use crate::tasker::config::UsersExportConfig;
use anyhow::{Context, Result};
use calamine::Reader;
use csv::{ReaderBuilder, WriterBuilder};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
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


fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn parse_team_overrides<R: Read>(reader: R) -> Result<HashMap<String, String>> {
    let mut csv_reader = ReaderBuilder::new().from_reader(reader);
    let headers = csv_reader.headers()?.clone();
    let email_idx = find_column(&headers, "Email")
        .context("User team overrides CSV is missing the required 'Email' column")?;
    let team_idx = find_column(&headers, "Team")
        .context("User team overrides CSV is missing the required 'Team' column")?;
    let mut overrides = HashMap::new();

    for (index, record) in csv_reader.records().enumerate() {
        let record = record.context("Malformed record in user team overrides CSV")?;
        let email = normalize_email(record.get(email_idx).unwrap_or_default());
        let team = record.get(team_idx).unwrap_or_default().trim().to_string();
        if email.is_empty() && team.is_empty() {
            continue;
        }
        if email.is_empty() || team.is_empty() {
            anyhow::bail!("Invalid user team override on CSV row {}: both Email and Team are required", index + 2);
        }
        if overrides.insert(email.clone(), team).is_some() {
            anyhow::bail!("Duplicate email '{}' in user team overrides CSV", email);
        }
    }
    Ok(overrides)
}

fn load_team_overrides(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        info!("User team overrides file {:?} not found; continuing without overrides", path);
        return Ok(HashMap::new());
    }
    let file = fs::File::open(path)
        .with_context(|| format!("Failed to open user team overrides CSV {:?}", path))?;
    parse_team_overrides(file)
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
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        if !name.to_ascii_lowercase().starts_with("users_")
            || !path
                .extension()
                .and_then(|v| v.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| format!("Failed to read metadata for {:?}", path))?;
        let created = metadata.created();
        let modified = metadata.modified();
        match (created, modified) {
            (Ok(created), Ok(modified)) if created.max(modified) >= cutoff => {
                // Downloads may overwrite an existing file on Windows, leaving its
                // creation time old even though the export was just refreshed.
                files.push((modified, path));
            }
            (Ok(_), Ok(_)) => {}
            (Err(_), Ok(modified)) if modified >= cutoff => {
                files.push((modified, path));
            }
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
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
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
    let overrides_path = resolve(&config.overrides_file);
    let overrides = load_team_overrides(&overrides_path)?;
    let files = recent_latest_user_csvs(&download_dir, config.minutes_ago)?;

    if files.is_empty() {
        let directory_status = if download_dir.is_dir() {
            "exists"
        } else if download_dir.exists() {
            "exists but is not a directory"
        } else {
            "does not exist"
        };
        let mut matching_files = Vec::new();
        if download_dir.is_dir() {
            for entry in walkdir::WalkDir::new(&download_dir)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_file())
            {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default();
                if name.to_ascii_lowercase().starts_with("users_")
                    && path
                        .extension()
                        .and_then(|v| v.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
                {
                    let age = entry
                        .metadata()
                        .ok()
                        .and_then(|metadata| metadata.modified().ok())
                        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                        .map(|duration| format!("{} minute(s) old", duration.as_secs() / 60))
                        .unwrap_or_else(|| "age unavailable".to_string());
                    matching_files.push(format!("{} ({})", path.display(), age));
                }
            }
        }
        warn!(
            "Users export cannot continue: no recent users_*.csv files. Search directory {:?} {}; allowed age {} minute(s); matching files found: [{}]. Stopping dependent tasks to avoid using stale data.",
            download_dir,
            directory_status,
            config.minutes_ago,
            matching_files.join("; ")
        );
        anyhow::bail!(
            "Users export stopped: no users_*.csv files within {} minute(s) in {:?} (directory {})",
            config.minutes_ago,
            download_dir,
            directory_status
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
    let first_name_idx = find_column(&headers, "First Name")
        .context("Required users CSV column 'First Name' is missing")?;
    let last_name_idx = find_column(&headers, "Last Name")
        .context("Required users CSV column 'Last Name' is missing")?;
    let branches_idx = find_column(&headers, "Branches")
        .context("Required users CSV column 'Branches' is missing")?;
    let teams_idx =
        find_column(&headers, "Teams").context("Required users CSV column 'Teams' is missing")?;
    let email_idx = if overrides.is_empty() {
        None
    } else {
        Some(find_column(&headers, "Email").context(
            "Required users CSV column 'Email' is missing while user team overrides are configured",
        )?)
    };

    let mut output_rows = Vec::new();
    let mut applied_overrides = 0usize;
    for record in reader.records() {
        let record = record.context("Malformed record in combined users CSV data")?;
        let username = format!(
            "{} {}",
            record.get(first_name_idx).unwrap_or_default(),
            record.get(last_name_idx).unwrap_or_default()
        );
        let mapped_department = mapping.get(&username).cloned().unwrap_or_default();
        let teams = record.get(teams_idx).unwrap_or_default().to_string();
        let override_department = email_idx
            .and_then(|idx| record.get(idx))
            .map(normalize_email)
            .and_then(|email| overrides.get(&email));
        if override_department.is_some() {
            applied_overrides += 1;
        }
        let final_department = override_department.cloned().unwrap_or_else(|| {
            if mapped_department.is_empty() {
                teams.clone()
            } else {
                mapped_department.clone()
            }
        });

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
        "Users export completed: {} source file(s), {} row(s), {} team override(s) applied, output {:?}",
        files.len(),
        output_rows.len(),
        applied_overrides,
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
    fn team_overrides_match_emails_case_insensitively() {
        let csv = "Email,Team\\n User@Example.com ,Team C\\n";
        let overrides = parse_team_overrides(Cursor::new(csv)).unwrap();
        assert_eq!(overrides.get("user@example.com").unwrap(), "Team C");
    }

    #[test]
    fn duplicate_override_emails_are_rejected_case_insensitively() {
        let csv = "Email,Team\\nuser@example.com,Team A\\nUSER@example.com,Team B\\n";
        let error = parse_team_overrides(Cursor::new(csv)).unwrap_err();
        assert!(error.to_string().contains("Duplicate email"));
    }

    #[test]
    fn incomplete_override_rows_are_rejected() {
        let csv = "Email,Team\\nuser@example.com,\\n";
        let error = parse_team_overrides(Cursor::new(csv)).unwrap_err();
        assert!(error.to_string().contains("both Email and Team are required"));
    }

    #[test]
    fn windows_1252_decodes_non_ascii_bytes() {
        assert_eq!(decode_windows_1252(&[b'A', 0x80, b'B']), "A€B");
        assert_eq!(decode_windows_1252(&[0xEF, 0xBB, 0xBF, b'A']), "A");
    }

    #[test]
    fn output_headers_match_the_users_workbook_query() {
        assert_eq!(OUTPUT_HEADERS.len(), 5);
        assert_eq!(
            OUTPUT_HEADERS[4],
            "NewColumn.UserDepartmentName / Team Name"
        );
    }
}
