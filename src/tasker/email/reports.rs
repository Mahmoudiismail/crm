use anyhow::{Context, Result};
use rust_xlsxwriter::Workbook;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{error, info};
use walkdir::WalkDir;

pub fn generate_leads_report(
    download_dir: &str,
    minutes_ago: i64,
    exclude_branches: &[String],
) -> Result<Option<PathBuf>> {
    let download_dir_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(download_dir);
    let exclude_branches_lower: HashSet<String> = exclude_branches
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();

    let lookback_minutes = u64::try_from(minutes_ago).context("minutes_ago cannot be negative")?;
    let lookback_seconds = lookback_minutes
        .checked_mul(60)
        .context("minutes_ago is too large")?;
    let now = std::time::SystemTime::now();
    let threshold = now
        .checked_sub(std::time::Duration::from_secs(lookback_seconds))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    let mut target_files = Vec::new();

    for entry in WalkDir::new(&download_dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("lead_report") && name.ends_with(".csv") {
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
            "No lead_report CSV files found within the last {} minutes.",
            minutes_ago
        );
        return Ok(None);
    }
    info!(
        "Found {} lead_report files for processing.",
        target_files.len()
    );

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

    let mut headers: Option<csv::StringRecord> = None;
    let mut seen_leads = HashSet::new();
    let mut written_records = 0usize;
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let mut lead_id_idx = None;
    let mut branch_idx = None;
    let mut status_idx = None;

    for file_path in target_files {
        let file_bytes = std::fs::read(&file_path)?;
        let file_content = String::from_utf8_lossy(&file_bytes);

        let first_line = file_content.lines().next().unwrap_or("");
        let delimiter = if first_line.contains('\t') && !first_line.contains(',') {
            b'\t'
        } else {
            b','
        };

        let mut rdr = crate::utils::build_csv_reader_builder()
            .delimiter(delimiter)
            .from_reader(file_content.as_bytes());

        let current_headers = rdr.headers()?.clone();
        if let Some(expected_headers) = &headers {
            let headers_match = expected_headers.len() == current_headers.len()
                && expected_headers
                    .iter()
                    .zip(current_headers.iter())
                    .all(|(expected, actual)| {
                        let expected = expected.trim().trim_start_matches('\u{feff}');
                        let actual = actual.trim().trim_start_matches('\u{feff}');
                        expected.eq_ignore_ascii_case(actual)
                    });
            if !headers_match {
                anyhow::bail!(
                    "Lead report '{}' has a different header layout from the first input file",
                    file_path.display()
                );
            }
        } else {
            for (i, col_name) in current_headers.iter().enumerate() {
                let normalized = col_name
                    .trim()
                    .trim_start_matches('\u{feff}')
                    .to_lowercase();

                if normalized == "lead id" {
                    lead_id_idx = Some(i);
                } else if normalized == "branch" {
                    branch_idx = Some(i);
                } else if normalized == "status" {
                    status_idx = Some(i);
                }
            }

            if lead_id_idx.is_none() || branch_idx.is_none() || status_idx.is_none() {
                anyhow::bail!(
                    "Lead report '{}' must contain Lead Id, Branch, and Status columns",
                    file_path.display()
                );
            }
            headers = Some(current_headers);
        }

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    let line_num = e.position().map(|p| p.line()).unwrap_or(0) as usize;
                    let diagnostic_info = crate::utils::generate_csv_diagnostic_context_from_file(
                        &file_path, line_num,
                    );

                    error!(
                        "CSV parsing error in file {:?} at line {}: {}\nDiagnostic Context (±20 lines):\n{}",
                        file_path, line_num, e, diagnostic_info
                    );
                    anyhow::bail!("Failed to parse lead report CSV: {}", e);
                }
            };

            let lead_id = lead_id_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_string();

            let branch = branch_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let status = status_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let is_excluded_branch = exclude_branches_lower.contains(&branch);
            let status_matches = status == "new" || status == "follow-up";
            if is_excluded_branch || !status_matches {
                continue;
            }

            // Excluded or unsupported rows must not reserve an ID and suppress
            // a later valid record with the same Lead Id.
            if !lead_id.is_empty() && !seen_leads.insert(lead_id) {
                continue;
            }

            let row_idx = (written_records + 1) as u32;
            for (col_idx, field) in record.iter().enumerate() {
                worksheet.write_string(row_idx, col_idx as u16, field)?;
            }
            written_records += 1;
        }
    }

    if written_records == 0 {
        info!("No valid lead records found after processing files.");
        return Ok(None);
    }

    let tmp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0); // Fallback to 0 if time went backwards
    let xlsx_path = tmp_dir.join(format!("Call_Center_Leads_{}.xlsx", unique_id));
    if let Some(h) = headers {
        for (i, name) in h.iter().enumerate() {
            worksheet.write_string(0, i as u16, name)?;
        }
    }

    workbook.save(&xlsx_path)?;

    info!(
        "Successfully generated leads report with {} records at: {}",
        written_records,
        xlsx_path.display()
    );
    Ok(Some(xlsx_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn negative_lookback_is_rejected() {
        let dir = tempdir().unwrap();
        let error = generate_leads_report(dir.path().to_str().unwrap(), -1, &[]).unwrap_err();
        assert!(error.to_string().contains("minutes_ago cannot be negative"));
    }

    #[test]
    fn test_call_center_leads_attachment_logic() {
        let download_dir = tempdir().unwrap();

        // Create a mock lead report (tab separated like the real one)
        let lead_report_path = download_dir.path().join("lead_report_test.csv");
        let mut lead_file = File::create(&lead_report_path).unwrap();
        // Use tabs as observed in the real dataset
        writeln!(lead_file, "Lead Id\tBranch\tStatus").unwrap();
        writeln!(lead_file, "L1\tMain Branch\tNew").unwrap();
        writeln!(lead_file, "L2\tMain Branch\tFollow-up").unwrap();
        writeln!(lead_file, "L3\tExcluded Branch\tNew").unwrap();

        let exclude_branches = vec!["Excluded Branch".to_string()];

        let result =
            generate_leads_report(download_dir.path().to_str().unwrap(), 60, &exclude_branches)
                .unwrap();

        assert!(result.is_some(), "Leads report should be generated");
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("Call_Center_Leads_"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn excluded_duplicate_does_not_suppress_valid_lead() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("lead_report_duplicates.csv");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Lead Id,Branch,Status").unwrap();
        writeln!(file, "L1,Excluded Branch,New").unwrap();
        writeln!(file, "L1,Main Branch,New").unwrap();

        let excluded = vec!["Excluded Branch".to_string()];
        let result = generate_leads_report(dir.path().to_str().unwrap(), 10, &excluded)
            .unwrap()
            .expect("valid duplicate should be included");

        use calamine::Reader;
        let mut workbook = calamine::open_workbook_auto(&result).unwrap();
        let range = workbook.worksheet_range_at(0).unwrap().unwrap();
        let lead_ids = range
            .rows()
            .skip(1)
            .map(|row| row[0].to_string())
            .collect::<Vec<_>>();
        assert_eq!(lead_ids, vec!["L1"]);

        let _ = std::fs::remove_file(result);
    }

    #[test]
    fn test_leads_report_parsing_delimiter_and_bom() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("lead_report_123.csv");
        let mut file = std::fs::File::create(&file_path).unwrap();

        // Add BOM \ufeff at the start, and use commas
        // also add a tab inside a value to test delimiter logic
        file.write_all(b"\xef\xbb\xbf\"Lead Id\",\"Branch\",\"Status\",\"Remarks\"\n")
            .unwrap();
        file.write_all(b"\"1\",\"Jeddah\",\"new\",\"Some\tTab\"\n")
            .unwrap();
        file.write_all(b"\"2\",\"Jeddah\",\"follow-up\",\"\"\n")
            .unwrap();
        file.write_all(b"\"3\",\"Riyadh\",\"closed\",\"\"\n")
            .unwrap();
        file.write_all(b"\"4\",\"Jeddah\",\"follow up\",\"\"\n")
            .unwrap(); // We no longer match 'follow up', only 'follow-up' and 'new'

        // We set minutes_ago high enough to include the newly created file.
        let exclude_branches = vec!["Riyadh".to_string()];

        let path =
            generate_leads_report(dir.path().to_str().unwrap(), 10, &exclude_branches).unwrap();

        assert!(path.is_some());

        let out_path = path.unwrap();
        assert!(out_path.exists(), "The leads output file must exist");
        let out_bytes = std::fs::read(&out_path).unwrap();
        assert!(out_bytes.len() > 100);

        use calamine::Reader;

        let mut workbook = calamine::open_workbook_auto(&out_path).unwrap();
        let range = workbook.worksheet_range_at(0).unwrap().unwrap();
        let lead_ids = range
            .rows()
            .skip(1)
            .map(|row| row[0].to_string())
            .collect::<Vec<_>>();
        assert_eq!(lead_ids, vec!["1", "2"]);

        let _ = std::fs::remove_file(out_path);
    }
}
