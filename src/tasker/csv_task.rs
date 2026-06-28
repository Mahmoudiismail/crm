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

pub fn run(config: &CsvAnalysisConfig, only_call_center: bool) -> Result<()> {
    info!(
        "Starting csv_analysis task (only_call_center: {})...",
        only_call_center
    );

    // 1. Load users (Table11)
    info!("Loading users file from {}", config.users_file);
    let mut assignee_map: HashMap<String, UserInfo> = HashMap::new();
    let users_bytes = std::fs::read(&config.users_file)
        .with_context(|| format!("Failed to read users file: {}", config.users_file))?;
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
        config.assignment_settings_file
    );
    let mut assignment_map: HashMap<(String, String, String), String> = HashMap::new();
    let assignment_bytes = std::fs::read(&config.assignment_settings_file).with_context(|| {
        format!(
            "Failed to read assignment file: {}",
            config.assignment_settings_file
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
        config.download_path, config.minutes_ago
    );
    let now = Local::now().naive_local();
    let threshold = now - Duration::minutes(config.minutes_ago);
    let mut target_files = Vec::new();

    for entry in WalkDir::new(&config.download_path)
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
            config.minutes_ago
        );
        return Ok(());
    }
    info!("Found {} target ticket files.", target_files.len());

    // Prepare exclusion filters
    let exclude_branches: HashSet<String> = config
        .exclude_branches
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();
    let exclude_categories: HashSet<String> = config
        .exclude_categories
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();

    // Parse logic
    fn parse_created_at(val: &str) -> Option<NaiveDateTime> {
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

    info!(
        "Processing ticket files and writing to output: {}",
        config.output_file
    );
    let mut output_writer = WriterBuilder::new().from_path(&config.output_file)?;
    let mut all_records = Vec::new();
    let mut wrote_headers = false;
    let mut total_filtered_rows = 0;
    let mut total_deduped_rows = 0;

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
            output_writer.write_record(&out_headers)?;
            wrote_headers = true;
        }

        let mut seen_tickets = HashSet::new();

        for result in rdr.records() {
            let mut record = result?;

            // Clean
            if let Some(idx) = assignee_idx {
                if let Some(val) = record.get(idx) {
                    let cleaned = val.trim().to_string();
                    let mut new_record = StringRecord::new();
                    for (i, field) in record.iter().enumerate() {
                        if i == idx {
                            new_record.push_field(&cleaned);
                        } else {
                            new_record.push_field(field);
                        }
                    }
                    record = new_record;
                }
            }

            for idx in [type_idx, subtype_idx, cat_idx].into_iter().flatten() {
                if let Some(val) = record.get(idx) {
                    let cleaned = val.replace('_', " ");
                    let mut new_record = StringRecord::new();
                    for (i, field) in record.iter().enumerate() {
                        if i == idx {
                            new_record.push_field(&cleaned);
                        } else {
                            new_record.push_field(field);
                        }
                    }
                    record = new_record;
                }
            }

            // Deduplicate
            let ticket_id_val = ticket_id_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_string();
            if seen_tickets.contains(&ticket_id_val) {
                total_deduped_rows += 1;
                continue;
            }
            seen_tickets.insert(ticket_id_val.clone());

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
                if let Some(exceptions) = &config.category_exceptions {
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

            all_records.push((ticket_id_val, record));
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
        "csv_analysis task completed successfully. Output written to {}",
        config.output_file
    );

    if let Some(email_cfg) = &config.email_config {
        // Start email processing
        info!("Email config present, starting email processing...");
        if let Err(e) =
            crate::tasker::email::process_emails(&config.output_file, email_cfg, only_call_center)
        {
            error!("Error processing emails: {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::tasker::config::CsvAnalysisConfig;
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
        super::run(&config, false).unwrap();

        // Validate the output file was created and contains expected headers
        let output_content = std::fs::read_to_string(config.output_file).unwrap();
        assert!(output_content.contains("Ticket Id"));
        assert!(output_content.contains("1001"));
    }
}
