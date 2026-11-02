use crate::tasker::csv_task::UserInfo;
use anyhow::{Context, Result};
use csv::StringRecord;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};
use walkdir::WalkDir;

/// Builds the durable lead result beside the ticket result and enriches each lead
/// with the assignee's primary team from the users export. No category fallback is
/// applied: an empty or unknown assignee produces an empty team.
pub fn generate_lead_result(
    download_dir: &Path,
    ticket_result_path: &Path,
    minutes_ago: i64,
    assignee_map: &HashMap<String, UserInfo>,
) -> Result<Option<PathBuf>> {
    let lookback_minutes = u64::try_from(minutes_ago).context("minutes_ago cannot be negative")?;
    let lookback_seconds = lookback_minutes
        .checked_mul(60)
        .context("minutes_ago is too large")?;
    let threshold = SystemTime::now()
        .checked_sub(Duration::from_secs(lookback_seconds))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut target_files = Vec::new();
    for entry in WalkDir::new(download_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("lead_report") || !name.ends_with(".csv") {
            continue;
        }
        if path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified >= threshold)
        {
            target_files.push(path.to_path_buf());
        }
    }

    if target_files.is_empty() {
        info!("No recent lead_report CSV files found; lead_result.csv was not changed.");
        return Ok(None);
    }

    target_files.sort_by(|a, b| {
        let modified = |path: &Path| {
            path.metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        };
        modified(b).cmp(&modified(a))
    });

    let mut expected_headers: Option<StringRecord> = None;
    let mut lead_id_idx = None;
    let mut assignee_idx = None;
    let mut team_idx = None;
    let mut seen_leads = HashSet::new();
    let mut output_records = Vec::new();

    for file_path in &target_files {
        let bytes = std::fs::read(file_path)
            .with_context(|| format!("Failed to read lead report {}", file_path.display()))?;
        let content = String::from_utf8_lossy(&bytes);
        let first_line = content.lines().next().unwrap_or("");
        let delimiter = if first_line.contains('\t') && !first_line.contains(',') {
            b'\t'
        } else {
            b','
        };
        let mut reader = crate::utils::build_csv_reader_builder()
            .delimiter(delimiter)
            .from_reader(content.as_bytes());

        let headers = reader.headers()?.clone();
        if let Some(expected) = &expected_headers {
            let same_layout = expected.len() == headers.len()
                && expected.iter().zip(headers.iter()).all(|(left, right)| {
                    left.trim()
                        .trim_start_matches('\u{feff}')
                        .eq_ignore_ascii_case(right.trim().trim_start_matches('\u{feff}'))
                });
            if !same_layout {
                anyhow::bail!(
                    "Lead report '{}' has a different header layout from the other lead files",
                    file_path.display()
                );
            }
        } else {
            for (index, header) in headers.iter().enumerate() {
                let normalized = header
                    .trim()
                    .trim_start_matches('\u{feff}')
                    .to_ascii_lowercase();
                match normalized.as_str() {
                    "lead id" => lead_id_idx = Some(index),
                    "assignee" => assignee_idx = Some(index),
                    "team" => team_idx = Some(index),
                    _ => {}
                }
            }
            if lead_id_idx.is_none() {
                anyhow::bail!(
                    "Lead report '{}' is missing the required 'Lead Id' column",
                    file_path.display()
                );
            }
            expected_headers = Some(headers);
        }

        for result in reader.records() {
            let record = result
                .with_context(|| format!("Failed to parse lead report {}", file_path.display()))?;
            let lead_id = lead_id_idx
                .and_then(|index| record.get(index))
                .unwrap_or("")
                .trim();
            if !lead_id.is_empty() && !seen_leads.insert(lead_id.to_string()) {
                continue;
            }

            let assignee = assignee_idx
                .and_then(|index| record.get(index))
                .unwrap_or("")
                .trim()
                .to_uppercase();
            let team = assignee_map
                .get(&assignee)
                .and_then(|user| user.first_position.as_deref())
                .unwrap_or("");

            let mut output = StringRecord::new();
            for (index, value) in record.iter().enumerate() {
                if Some(index) == team_idx {
                    output.push_field(team);
                } else {
                    output.push_field(value);
                }
            }
            if team_idx.is_none() {
                output.push_field(team);
            }
            output_records.push(output);
        }
    }

    let mut headers = expected_headers.context("No lead report headers were loaded")?;
    if team_idx.is_none() {
        headers.push_field("team");
    }

    let output_path = ticket_result_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("lead_result.csv");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&output_path)
        .with_context(|| format!("Failed to create {}", output_path.display()))?;
    file.write_all(b"\xEF\xBB\xBF")?;
    let mut writer = csv::WriterBuilder::new().from_writer(file);
    writer.write_record(&headers)?;
    for record in &output_records {
        writer.write_record(record)?;
    }
    writer.flush()?;

    let team_column_idx = team_idx.unwrap_or(headers.len() - 1);
    let unassigned = output_records
        .iter()
        .filter(|record| record.get(team_column_idx).unwrap_or("").trim().is_empty())
        .count();
    if unassigned > 0 {
        warn!(
            "Lead result contains {} records without a resolved assignee team.",
            unassigned
        );
    }
    info!(
        "Wrote {} enriched lead records to {}",
        output_records.len(),
        output_path.display()
    );
    Ok(Some(output_path))
}

#[cfg(test)]
mod tests {
    use super::generate_lead_result;
    use crate::tasker::csv_task::UserInfo;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn lead_result_maps_team_from_assignee_and_leaves_missing_assignees_blank() {
        let dir = tempdir().unwrap();
        let download_dir = dir.path().join("downloads");
        std::fs::create_dir_all(&download_dir).unwrap();
        let ticket_result = dir.path().join("ticket_result.csv");
        let lead_input = download_dir.join("lead_report_test.csv");
        let mut file = std::fs::File::create(&lead_input).unwrap();
        writeln!(file, "Lead Id,Assignee,Lead Type,Branch,Status").unwrap();
        writeln!(file, "L1,alice,Inquiry,Main,New").unwrap();
        writeln!(file, "L2,bob,Inquiry,Main,New").unwrap();
        writeln!(file, "L3,,Inquiry,Main,New").unwrap();
        writeln!(file, "L4,unknown,Inquiry,Main,New").unwrap();

        let users = HashMap::from([
            (
                "ALICE".to_string(),
                UserInfo {
                    positions: vec!["Call Center".to_string()],
                    first_position: Some("Call Center".to_string()),
                },
            ),
            (
                "BOB".to_string(),
                UserInfo {
                    positions: vec!["Admissions".to_string()],
                    first_position: Some("Admissions".to_string()),
                },
            ),
        ]);

        let output = generate_lead_result(&download_dir, &ticket_result, 60, &users)
            .unwrap()
            .expect("lead result should be generated");
        assert_eq!(output, dir.path().join("lead_result.csv"));

        let mut reader =
            crate::utils::build_csv_reader_from_reader(std::fs::File::open(&output).unwrap());
        let headers = reader.headers().unwrap().clone();
        let team_idx = headers
            .iter()
            .position(|header| header.eq_ignore_ascii_case("team"))
            .unwrap();
        let rows = reader
            .records()
            .map(Result::unwrap)
            .map(|record| {
                (
                    record.get(0).unwrap().to_string(),
                    record.get(team_idx).unwrap_or("").to_string(),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(rows.get("L1").map(String::as_str), Some("Call Center"));
        assert_eq!(rows.get("L2").map(String::as_str), Some("Admissions"));
        assert_eq!(rows.get("L3").map(String::as_str), Some(""));
        assert_eq!(rows.get("L4").map(String::as_str), Some(""));
    }

    #[test]
    fn existing_team_column_is_replaced_not_duplicated() {
        let dir = tempdir().unwrap();
        let download_dir = dir.path().join("downloads");
        std::fs::create_dir_all(&download_dir).unwrap();
        let ticket_result = dir.path().join("ticket_result.csv");
        let lead_input = download_dir.join("lead_report_test.csv");
        std::fs::write(
            &lead_input,
            "Lead Id,Assignee,team,Status\nL1,alice,Old Team,New\n",
        )
        .unwrap();
        let users = HashMap::from([(
            "ALICE".to_string(),
            UserInfo {
                positions: vec!["Call Center".to_string()],
                first_position: Some("Call Center".to_string()),
            },
        )]);

        let output = generate_lead_result(&download_dir, &ticket_result, 60, &users)
            .unwrap()
            .unwrap();
        let mut reader =
            crate::utils::build_csv_reader_from_reader(std::fs::File::open(output).unwrap());
        let headers = reader.headers().unwrap().clone();
        assert_eq!(
            headers
                .iter()
                .filter(|header| header.eq_ignore_ascii_case("team"))
                .count(),
            1
        );
        let record = reader.records().next().unwrap().unwrap();
        let team_idx = headers
            .iter()
            .position(|header| header.eq_ignore_ascii_case("team"))
            .unwrap();
        assert_eq!(record.get(team_idx), Some("Call Center"));
    }
}
