pub mod models;
pub use models::*;
pub mod reader;
use crate::tasker::config::CsvAnalysisConfig;
use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
pub use reader::*;

mod processing;
mod writer;

use std::collections::HashMap;
use tracing::{error, info, warn};
use walkdir::WalkDir;

pub fn parse_start_date(val: &str) -> Option<NaiveDateTime> {
    if let Some(dt) = crate::utils::parse_flexible_date(val) {
        return dt.and_hms_opt(0, 0, 0);
    }

    let trimmed = val.trim();
    if trimmed.is_empty() {
        return None;
    }

    // e.g. "1-May" -> "1-May-2026" (append current year)
    let with_year = format!("{}-{}", trimmed, Local::now().year());
    if let Ok(dt) = NaiveDate::parse_from_str(&with_year, "%d-%b-%Y") {
        return dt.and_hms_opt(0, 0, 0);
    }

    // try d-b format
    let with_year2 = format!("{}-{}", trimmed, Local::now().year());
    if let Ok(dt) = NaiveDate::parse_from_str(&with_year2, "%e-%b-%Y") {
        return dt.and_hms_opt(0, 0, 0);
    }

    None
}

pub fn parse_created_at(val: &str) -> Option<NaiveDateTime> {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Try flexible date formats
    if let Some(dt) = crate::utils::parse_flexible_date(trimmed) {
        return dt.and_hms_opt(0, 0, 0);
    }
    // Try dd/mm/yyyy hh:mm:ss
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%d/%m/%Y %H:%M:%S") {
        return Some(dt);
    }
    // Try mm/dd/yyyy hh:mm:ss
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%m/%d/%Y %H:%M:%S") {
        return Some(dt);
    }
    // Try YYYY-MM-DD HH:MM:SS
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    // Try DD MMM YY HH:MM AM/PM
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%d %b %y %I:%M %p") {
        return Some(dt);
    }
    // Try DD MMM YYYY HH:MM AM/PM
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%d %b %Y %I:%M %p") {
        return Some(dt);
    }
    // Try float
    if let Ok(excel_float) = trimmed.parse::<f64>() {
        let base_date = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap_or_default();
        let days = excel_float.trunc() as i64;
        let fraction = excel_float.fract();
        let seconds_in_day = 86400.0;
        let total_seconds = (fraction * seconds_in_day).round() as u32;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        if let Some(date) = base_date.checked_add_signed(Duration::days(days)) {
            if let Some(time) = NaiveTime::from_hms_opt(hours, minutes, seconds) {
                return Some(NaiveDateTime::new(date, time));
            }
        }
    }
    None
}

pub fn resolve_relative_to_base_dir(
    path: &str,
    base_dir: Option<&std::path::Path>,
) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }

    if let Some(dir) = base_dir {
        return dir.join(p);
    }

    p
}

pub fn resolve_relative_to_exe_dir(path: &str) -> std::path::PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()));
    resolve_relative_to_base_dir(path, exe_dir.as_deref())
}

pub fn generate_csv(params: &CsvAnalysisParams<'_>) -> Result<Option<std::path::PathBuf>> {
    info!(
        "Starting CSV Generation (minutes_ago: {})",
        params.minutes_ago
    );

    let users_file_path = resolve_relative_to_exe_dir(params.users_file);
    let assignment_settings_path = resolve_relative_to_exe_dir(params.assignment_settings_file);
    let download_dir_path = resolve_relative_to_exe_dir(params.download_path);
    let output_file_path = resolve_relative_to_exe_dir(params.output_file);

    // 1. Load users (Table11)
    info!("Loading users file from {}", users_file_path.display());
    let mut assignee_map: HashMap<String, UserInfo> = HashMap::new();
    let users_bytes = std::fs::read(&users_file_path)
        .with_context(|| format!("Failed to read users file: {}", users_file_path.display()))?;
    let users_content = String::from_utf8_lossy(&users_bytes);
    let mut users_rdr = crate::utils::build_csv_reader_from_reader(users_content.as_bytes());

    let headers = users_rdr.headers()?.clone();
    let mut cognito_idx = None;
    let mut team_idx = None;

    for (i, h) in headers.iter().enumerate() {
        if h.contains("cognito_username") {
            cognito_idx = Some(i);
        } else if h == "UserDepartmentName / Team Name" {
            team_idx = Some(i);
        }
    }

    if let (Some(c_idx), Some(t_idx)) = (cognito_idx, team_idx) {
        for result in users_rdr.records() {
            let record = result?;
            tracing::trace!("Processing user record: {:?}", record);
            if let (Some(cognito), Some(team_str)) = (record.get(c_idx), record.get(t_idx)) {
                let cognito = cognito.trim();
                if cognito.is_empty() {
                    continue;
                }
                let positions: Vec<String> = if team_str.trim().is_empty() {
                    Vec::new()
                } else {
                    team_str.split(',').map(|s| s.trim().to_string()).collect()
                };
                let first_position = positions.first().cloned();

                assignee_map.insert(
                    cognito.to_uppercase(),
                    UserInfo {
                        positions,
                        first_position,
                    },
                );
            }
        }
        info!("Loaded {} user mappings.", assignee_map.len());
    } else {
        warn!("Could not find required columns in users file (cognito_username, UserDepartmentName / Team Name).");
    }

    // 2. Load assignment settings
    info!(
        "Loading assignment settings from {}",
        assignment_settings_path.display()
    );
    let mut assignment_map: HashMap<(String, String, String), String> = HashMap::new();
    let assignment_bytes = std::fs::read(&assignment_settings_path).with_context(|| {
        format!(
            "Failed to read assignment file: {}",
            assignment_settings_path.display()
        )
    })?;
    let assignment_content = String::from_utf8_lossy(&assignment_bytes);
    let mut assign_rdr = crate::utils::build_csv_reader_from_reader(assignment_content.as_bytes());

    for result in assign_rdr.deserialize::<AssignmentSettings>() {
        match result {
            Ok(setting) => {
                tracing::trace!("Processing assignment setting: {:?}", setting);
                if let Some(team2) = setting.auto_agent_team_assignment {
                    let key = (
                        setting.category.trim().to_uppercase(),
                        setting.type_.trim().to_uppercase(),
                        setting.subtype.trim().to_uppercase(),
                    );
                    assignment_map.insert(key, team2.trim().to_string());
                }
            }
            Err(_) => {
                // Keep trying to parse, but maybe log or ignore
            }
        }
    }
    info!("Loaded {} assignment settings.", assignment_map.len());

    // 3. Find target tickets CSVs
    info!(
        "Scanning for target ticket CSVs in {} (modified in last {} min)",
        download_dir_path.display(),
        params.minutes_ago
    );
    let now = std::time::SystemTime::now();
    let threshold = now
        .checked_sub(std::time::Duration::from_secs(
            (params.minutes_ago * 60) as u64,
        ))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let mut target_files = Vec::new();

    for entry in WalkDir::new(&download_dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("ticket_report") && name.ends_with(".csv") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified >= threshold {
                                target_files.push(path.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
    }

    if target_files.is_empty() {
        info!(
            "No target files found modified in the last {} minutes.",
            params.minutes_ago
        );
        return Ok(None);
    }

    // Sort files with modification date newer first
    target_files.sort_by(|a, b| {
        let meta_a = a
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let meta_b = b
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        meta_b.cmp(&meta_a)
    });

    info!("Found {} target ticket files.", target_files.len());

    // Parse start_date if provided (logging only)
    if let Some(sd_str) = params.start_date {
        if let Some(sd) = crate::tasker::csv_task::parse_created_at(sd_str) {
            info!("Filtering records with created_at >= {:?}", sd);
        }
    }

    let process_result = crate::tasker::csv_task::processing::process_files(
        target_files,
        params,
        &assignee_map,
        &assignment_map,
    )?;

    crate::tasker::csv_task::writer::write_processed_records(
        &output_file_path,
        process_result.headers,
        &process_result.records,
        process_result.total_deduped_rows,
        process_result.total_filtered_rows,
    )
}

/// Executes the primary CSV parsing, merging, and filtering task.
///
/// This function finds the most recently downloaded CRM ticket and user exports,
/// validates their CSV structure strictly (failing on malformed data), left-joins
/// the datasets based on assignee names, applies configured inclusion/exclusion filters,
/// and outputs a finalized `results.csv`.
///
/// If `email_config` is defined, it seamlessly triggers `tasker::email::run` to distribute
/// the resulting data to targeted teams.
///
/// # Invariants
/// - Does not modify or drop trailing columns. Missing or malformed CSV rows yield a descriptive error.
/// - The "Created At" column is explicitly parsed and formatted to generate a supplementary "Month" (`MMM-yyyy`) column.
pub fn run(
    config: &CsvAnalysisConfig,
    only_call_center: bool,

    send_exceptions: bool,
) -> Result<()> {
    info!(
        "Starting CsvAnalysis task (only_call_center: {}, send_exceptions: {}). Config: {:?}",
        only_call_center, send_exceptions, config
    );

    let params = CsvAnalysisParams::from(config);
    let output_file_path_opt = generate_csv(&params)?;

    if let Some(output_file_path) = output_file_path_opt {
        if let Some(email_cfg) = &config.email_config {
            // Start email processing
            info!("Email config present, starting email processing...");
            // Provide sensible defaults for missing arguments based on global context (since csv_task might not have download_dir yet)
            // or we look into how we should pass it. Let's pass standard defaults or look into config.

            if let Err(e) = crate::tasker::email::process_emails(
                &output_file_path.to_string_lossy(),
                email_cfg,
                only_call_center,
                send_exceptions,
                &config.download_path,
                config.minutes_ago,
                config.category_exceptions.as_deref(),
                &config.exclude_branches,
            ) {
                error!("Error processing emails: {}", e);
            }
        }
    } else {
        info!("No new data found, skipping email processing.");
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::{parse_created_at, resolve_relative_to_base_dir};
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn test_parse_created_at() {
        // dd/mm/yyyy
        assert_eq!(
            parse_created_at("01/02/2026 12:00:00"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap()
            ))
        );

        // mm/dd/yyyy
        assert_eq!(
            parse_created_at("02/15/2026 14:30:00"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),
                NaiveTime::from_hms_opt(14, 30, 0).unwrap()
            ))
        );

        // Excel float
        assert_eq!(
            parse_created_at("44562.5"), // Roughly sometime in 2022
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap()
            ))
        );

        // DD MMM YY HH:MM AM/PM
        assert_eq!(
            parse_created_at("21 Feb 26 11:58 PM"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2026, 2, 21).unwrap(),
                NaiveTime::from_hms_opt(23, 58, 0).unwrap()
            ))
        );

        // DD MMM YYYY HH:MM AM/PM
        assert_eq!(
            parse_created_at("01 Jan 2026 12:00 AM"),
            Some(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            ))
        );

        // Invalid
        assert_eq!(parse_created_at(""), None);
        assert_eq!(parse_created_at("invalid"), None);
    }

    #[test]
    fn test_resolve_relative_to_base_dir() {
        let base = std::path::PathBuf::from("/base/dir");
        let rel_path = resolve_relative_to_base_dir("file.txt", Some(&base));
        assert_eq!(rel_path, std::path::PathBuf::from("/base/dir/file.txt"));

        let abs_path = resolve_relative_to_base_dir("/absolute/path", Some(&base));
        assert_eq!(abs_path, std::path::PathBuf::from("/absolute/path"));

        let rel_path_no_base = resolve_relative_to_base_dir("file.txt", None);
        assert_eq!(rel_path_no_base, std::path::PathBuf::from("file.txt"));
    }
}
