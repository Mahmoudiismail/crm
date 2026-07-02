use crate::tasker::config::CsvAnalysisConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::{error, info, warn};
use walkdir::WalkDir;

// --- Data Models ---

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub positions: Vec<String>,
    pub first_position: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssignmentSettings {
    #[serde(alias = "Category", alias = "category")]
    pub category: String,
    #[serde(alias = "Type", alias = "type", alias = "type_")]
    pub type_: String,
    #[serde(alias = "Subtype", alias = "subtype")]
    pub subtype: String,
    #[serde(alias = "Auto agent/team assignment")]
    pub auto_agent_team_assignment: Option<String>,
}

pub fn parse_created_at(val: &str) -> Option<NaiveDateTime> {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Try dd/mm/yyyy hh:mm:ss
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%d/%m/%Y %H:%M:%S") {
        return Some(dt);
    }
    // Try mm/dd/yyyy hh:mm:ss
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%m/%d/%Y %H:%M:%S") {
        return Some(dt);
    }
    // Try float
    if let Ok(excel_float) = trimmed.parse::<f64>() {
        let base_date = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
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

pub struct CsvAnalysisParams<'a> {
    pub users_file: &'a str,
    pub assignment_settings_file: &'a str,
    pub download_path: &'a str,
    pub output_file: &'a str,
    pub minutes_ago: i64,
    pub exclude_branches: &'a [String],
    pub exclude_categories: &'a [String],
    pub category_exceptions: Option<&'a Vec<crate::tasker::config::CategoryException>>,
}

impl<'a> From<&'a CsvAnalysisConfig> for CsvAnalysisParams<'a> {
    fn from(config: &'a CsvAnalysisConfig) -> Self {
        Self {
            users_file: &config.users_file,
            assignment_settings_file: &config.assignment_settings_file,
            download_path: &config.download_path,
            output_file: &config.output_file,
            minutes_ago: config.minutes_ago,
            exclude_branches: &config.exclude_branches,
            exclude_categories: &config.exclude_categories,
            category_exceptions: config.category_exceptions.as_ref(),
        }
    }
}

impl<'a> From<&'a crate::tasker::config::DashboardUpdaterConfig> for CsvAnalysisParams<'a> {
    fn from(config: &'a crate::tasker::config::DashboardUpdaterConfig) -> Self {
        Self {
            users_file: &config.users_file,
            assignment_settings_file: &config.assignment_settings_file,
            download_path: &config.download_path,
            output_file: &config.output_file,
            minutes_ago: config.minutes_ago,
            exclude_branches: &config.exclude_branches,
            exclude_categories: &config.exclude_categories,
            category_exceptions: config.category_exceptions.as_ref(),
        }
    }
}

pub fn generate_csv(params: &CsvAnalysisParams) -> Result<Option<std::path::PathBuf>> {
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
    let mut users_rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(users_content.as_bytes());

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
    let mut assign_rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(assignment_content.as_bytes());

    for result in assign_rdr.deserialize::<AssignmentSettings>() {
        match result {
            Ok(setting) => {
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
    let now = Local::now().naive_local();
    let threshold = now - Duration::minutes(params.minutes_ago);
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
                            let mod_time: DateTime<Local> = modified.into();
                            if mod_time.naive_local() >= threshold {
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

    // Prepare exclusion filters
    let exclude_branches: HashSet<String> = params
        .exclude_branches
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();
    let exclude_categories: HashSet<String> = params
        .exclude_categories
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();

    // Parse logic

    info!(
        "Processing ticket files and writing to output: {}",
        output_file_path.display()
    );
    let mut output_writer = WriterBuilder::new().from_path(&output_file_path)?;
    let mut all_records = Vec::new();
    let mut wrote_headers = false;
    let mut total_filtered_rows = 0;
    let mut total_deduped_rows = 0;
    let mut seen_tickets = HashSet::new();

    for file_path in target_files {
        info!("Processing file: {}", file_path.display());
        let file_bytes = std::fs::read(&file_path)?;
        let file_content = String::from_utf8_lossy(&file_bytes);
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file_content.as_bytes());

        let headers = rdr.headers()?.clone();

        let mut assignee_idx = None;
        let mut type_idx = None;
        let mut subtype_idx = None;
        let mut cat_idx = None;
        let mut created_idx = None;
        let mut ticket_id_idx = None;
        let mut branch_idx = None;

        for (i, h) in headers.iter().enumerate() {
            let h_trim = h.trim();
            if h_trim == "Assignee" {
                assignee_idx = Some(i);
            } else if h_trim == "Ticket Type" {
                type_idx = Some(i);
            } else if h_trim == "Ticket Sub-Type" {
                subtype_idx = Some(i);
            } else if h_trim == "Ticket Category" {
                cat_idx = Some(i);
            } else if h_trim == "Created At" {
                created_idx = Some(i);
            } else if h_trim == "Ticket Id" {
                ticket_id_idx = Some(i);
            } else if h_trim == "Branch" {
                branch_idx = Some(i);
            }
        }

        if !wrote_headers {
            let mut out_headers = headers.clone();
            out_headers.push_field("Day");
            out_headers.push_field("Month");
            out_headers.push_field("Position");
            out_headers.push_field("team");
            out_headers.push_field("Is Exception");
            output_writer.write_record(&out_headers)?;
            wrote_headers = true;
        }

        for result in rdr.records() {
            let mut record = result?;
            let mut is_exception_val = "No";

            // Clean
            // Optimize: Collect fields directly into an array or iterator and push_record
            let mut new_record = StringRecord::with_capacity(record.len() * 16, record.len());
            for (i, field) in record.iter().enumerate() {
                if Some(i) == assignee_idx {
                    new_record.push_field(field.trim());
                } else if Some(i) == type_idx || Some(i) == subtype_idx || Some(i) == cat_idx {
                    if field.contains('_') {
                        new_record.push_field(&field.replace('_', " "));
                    } else {
                        new_record.push_field(field);
                    }
                } else {
                    new_record.push_field(field);
                }
            }
            record = new_record;

            // Deduplicate
            let ticket_id_val = ticket_id_idx.and_then(|idx| record.get(idx)).unwrap_or("");

            // Avoid string clone if we've seen it by querying with string slice first.
            if seen_tickets.contains(ticket_id_val) {
                total_deduped_rows += 1;
                continue;
            }
            let ticket_id_val_owned = ticket_id_val.to_string();
            seen_tickets.insert(ticket_id_val_owned.clone());

            let branch_val = branch_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_lowercase();

            // Keys
            let t_type = type_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();
            let t_subtype = subtype_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();
            let t_cat = cat_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();
            let assignee = assignee_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();

            let team2 = assignment_map.get(&(t_cat, t_type, t_subtype)).cloned();

            let (position, team) = if let Some(user_info) = assignee_map.get(&assignee) {
                let pos = if user_info.positions.is_empty() {
                    None
                } else if let Some(t2) = &team2 {
                    if user_info.positions.contains(t2) {
                        Some(t2.clone())
                    } else {
                        user_info.first_position.clone()
                    }
                } else {
                    user_info.first_position.clone()
                };

                let tm = pos.clone().or(team2.clone());
                (pos, tm)
            } else {
                (None, team2.clone())
            };

            // Filters
            if exclude_branches.contains(&branch_val) {
                total_filtered_rows += 1;
                continue;
            }

            let cat_val = cat_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if exclude_categories.contains(&cat_val) {
                // Check if this matches a category exception
                let mut matches_exception = false;
                if let Some(exceptions) = params.category_exceptions {
                    for exc in exceptions {
                        if exc.category.trim().to_lowercase() == cat_val {
                            let branch_matches = exc.branch.as_ref().is_none_or(|b| {
                                b.trim().is_empty() || b.trim().to_lowercase() == branch_val
                            });
                            let team_matches = exc.team.as_ref().is_none_or(|t| {
                                if t.trim().is_empty() {
                                    true
                                } else {
                                    team.as_ref().is_some_and(|tm| {
                                        tm.trim().to_lowercase() == t.trim().to_lowercase()
                                    })
                                }
                            });

                            if branch_matches && team_matches {
                                matches_exception = true;
                                break;
                            }
                        }
                    }
                }

                if !matches_exception {
                    total_filtered_rows += 1;
                    continue;
                }

                is_exception_val = "Yes";
            }

            // Date helpers
            let created_at_val = created_idx.and_then(|idx| record.get(idx)).unwrap_or("");
            let parsed_dt = parse_created_at(created_at_val);

            let day_str = parsed_dt
                .map(|dt| dt.date().day().to_string())
                .unwrap_or_default();
            let month_str = parsed_dt
                .map(|dt| dt.format("%m-%Y").to_string())
                .unwrap_or_default();

            record.push_field(&day_str);
            record.push_field(&month_str);
            record.push_field(position.as_deref().unwrap_or(""));
            record.push_field(team.as_deref().unwrap_or(""));
            record.push_field(is_exception_val);

            all_records.push((ticket_id_val_owned, record));
        }
    }

    // Sort by ticket id
    all_records.sort_by(|a, b| {
        let a_num = a.0.parse::<u64>().unwrap_or(0);
        let b_num = b.0.parse::<u64>().unwrap_or(0);
        if a_num > 0 && b_num > 0 {
            a_num.cmp(&b_num)
        } else {
            a.0.cmp(&b.0)
        }
    });

    info!(
        "Writing {} joined records to output file (deduped: {}, filtered: {}).",
        all_records.len(),
        total_deduped_rows,
        total_filtered_rows
    );

    for (_, record) in &all_records {
        output_writer.write_record(record)?;
    }

    output_writer.flush()?;
    info!(
        "CSV generation completed successfully. Output written to {}",
        output_file_path.display()
    );

    Ok(Some(output_file_path))
}

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
            if let Err(e) = crate::tasker::email::process_emails(
                &output_file_path.to_string_lossy(),
                email_cfg,
                only_call_center,
                send_exceptions,
                &config.download_path,
                config.minutes_ago,
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
mod tests {
    use super::{parse_created_at, resolve_relative_to_base_dir};
    use crate::tasker::config::CsvAnalysisConfig;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_analysis_mapping() {
        let mut users_file = NamedTempFile::new().unwrap();
        writeln!(users_file, "cognito_username,Team Name").unwrap();
        writeln!(users_file, "alice,Team A").unwrap();

        let mut assignments_file = NamedTempFile::new().unwrap();
        writeln!(
            assignments_file,
            "Category,Type,Subtype,Auto agent/team assignment"
        )
        .unwrap();
        writeln!(assignments_file, "Cat1,Type1,Sub1,Team A").unwrap();

        let download_dir = tempfile::tempdir().unwrap();
        let mut ticket_file =
            File::create(download_dir.path().join("ticket_report_test.csv")).unwrap();
        writeln!(
            ticket_file,
            "Ticket Id,Branch Name,Category,Type,Subtype,Status,Creation Date,Assignee"
        )
        .unwrap();
        writeln!(
            ticket_file,
            "1001,Main Branch,Cat1,Type1,Sub1,Open,01/01/2026 12:00:00,alice"
        )
        .unwrap();

        let output_file = NamedTempFile::new().unwrap();

        let config = CsvAnalysisConfig {
            download_path: download_dir.path().to_str().unwrap().to_string(),
            users_file: users_file.path().to_str().unwrap().to_string(),
            assignment_settings_file: assignments_file.path().to_str().unwrap().to_string(),
            minutes_ago: 60 * 24 * 365 * 10, // Ensure it picks up
            exclude_branches: vec![],
            exclude_categories: vec![],
            category_exceptions: None,
            output_file: output_file.path().to_str().unwrap().to_string(),
            email_config: None,
        };

        // Run the task
        super::run(&config, false, false).unwrap();

        // Validate the output file was created and contains expected headers
        let output_content = std::fs::read_to_string(config.output_file).unwrap();
        assert!(output_content.contains("Ticket Id"));
        assert!(output_content.contains("1001"));
    }

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

    #[test]
    fn test_real_dataset_mapping() {
        let users_file = NamedTempFile::new().unwrap();
        let assignments_file = NamedTempFile::new().unwrap();
        let download_dir = tempfile::tempdir().unwrap();
        let output_file = NamedTempFile::new().unwrap();

        let client = reqwest::blocking::Client::new();

        let agents_csv = client
            .get("https://paste.c-net.org/FreddoLocate")
            .send()
            .unwrap()
            .text()
            .unwrap();
        std::fs::write(users_file.path(), agents_csv).unwrap();

        let assignment_csv = client
            .get("https://paste.c-net.org/HahahaBackpack")
            .send()
            .unwrap()
            .text()
            .unwrap();
        std::fs::write(assignments_file.path(), assignment_csv).unwrap();

        let ticket_csv = client
            .get("https://paste.c-net.org/CalmedBrochure")
            .send()
            .unwrap()
            .text()
            .unwrap();
        std::fs::write(download_dir.path().join("ticket_report1.csv"), ticket_csv).unwrap();

        let config = CsvAnalysisConfig {
            download_path: download_dir.path().to_str().unwrap().to_string(),
            users_file: users_file.path().to_str().unwrap().to_string(),
            assignment_settings_file: assignments_file.path().to_str().unwrap().to_string(),
            minutes_ago: 60 * 24 * 365 * 10,
            exclude_branches: vec![],
            exclude_categories: vec![],
            category_exceptions: None,
            output_file: output_file.path().to_str().unwrap().to_string(),
            email_config: None,
        };

        super::run(&config, false, false).unwrap();

        let out_content = std::fs::read_to_string(output_file.path()).unwrap();
        let mut rdr = csv::ReaderBuilder::new().from_reader(out_content.as_bytes());
        let count = rdr.records().count();
        assert!(count > 1000, "Should have mapped thousands of records");
    }

    #[test]
    fn test_csv_analysis_deduplication() {
        // Setup configuration
        let download_dir = tempfile::tempdir().unwrap();
        let output_file = NamedTempFile::new().unwrap();
        let users_file = NamedTempFile::new().unwrap();
        let assignments_file = NamedTempFile::new().unwrap();

        // Write empty mappings
        writeln!(users_file.as_file(), "cognito_username,Team Name").unwrap();
        writeln!(
            assignments_file.as_file(),
            "Category,Type,Subtype,Auto agent/team assignment"
        )
        .unwrap();

        // Create two CSV files with overlapping tickets in the download directory
        let file1_path = download_dir.path().join("ticket_report_1.csv");
        let mut file1 = std::fs::File::create(&file1_path).unwrap();
        writeln!(
            file1,
            "Ticket Id,Assignee,Ticket Type,Ticket Sub-Type,Ticket Category,Created At,Branch"
        )
        .unwrap();
        writeln!(file1, "1001,alice,T1,ST1,C1,2023-01-01 10:00:00,BranchA").unwrap();
        writeln!(file1, "1002,bob,T2,ST2,C2,2023-01-01 11:00:00,BranchB").unwrap();

        // Ensure file 2 is created slightly later so it has a different modification time if needed,
        // though our task processes them in modified date order or all together.
        std::thread::sleep(std::time::Duration::from_millis(100));
        let file2_path = download_dir.path().join("ticket_report_2.csv");
        let mut file2 = std::fs::File::create(&file2_path).unwrap();
        writeln!(
            file2,
            "Ticket Id,Assignee,Ticket Type,Ticket Sub-Type,Ticket Category,Created At,Branch"
        )
        .unwrap();
        writeln!(file2, "1002,bob,T2,ST2,C2,2023-01-01 11:00:00,BranchB").unwrap(); // Duplicate!
        writeln!(file2, "1003,charlie,T3,ST3,C3,2023-01-01 12:00:00,BranchC").unwrap();

        let config = CsvAnalysisConfig {
            download_path: download_dir.path().to_str().unwrap().to_string(),
            users_file: users_file.path().to_str().unwrap().to_string(),
            assignment_settings_file: assignments_file.path().to_str().unwrap().to_string(),
            minutes_ago: 60 * 24 * 365, // Last year
            exclude_branches: vec![],
            exclude_categories: vec![],
            category_exceptions: None,
            output_file: output_file.path().to_str().unwrap().to_string(),
            email_config: None,
        };

        super::run(&config, false, false).unwrap();

        let out_content = std::fs::read_to_string(output_file.path()).unwrap();
        let mut rdr = csv::ReaderBuilder::new().from_reader(out_content.as_bytes());

        let records: Vec<_> = rdr.records().map(|r| r.unwrap()).collect();

        // We expect exactly 3 unique tickets: 1001, 1002, 1003.
        // 1002 should not appear twice despite being in two different files.
        assert_eq!(
            records.len(),
            3,
            "Output should contain exactly 3 deduplicated records"
        );

        // Assert the ticket IDs are present exactly once.
        // Note: the task sorts records by Ticket ID.
        let ids: Vec<&str> = records.iter().map(|r| r.get(0).unwrap()).collect();
        assert_eq!(ids, vec!["1001", "1002", "1003"]);
    }
}
