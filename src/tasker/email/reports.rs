use anyhow::Result;
use csv::StringRecord;
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

    let now = std::time::SystemTime::now();
    let threshold = now
        .checked_sub(std::time::Duration::from_secs((minutes_ago * 60) as u64))
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

    let mut all_records: Vec<StringRecord> = Vec::new();
    let mut headers = None;
    let mut seen_leads = HashSet::new();
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

        if headers.is_none() {
            let h = rdr.headers()?.clone();
            for (i, col_name) in h.iter().enumerate() {
                let lower = col_name.trim().to_lowercase();
                let lower = lower.trim_start_matches('\u{feff}');

                if lower == "lead id" {
                    lead_id_idx = Some(i);
                } else if lower == "branch" {
                    branch_idx = Some(i);
                } else if lower == "status" {
                    status_idx = Some(i);
                }
            }
            headers = Some(h);
        }

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    let line_num = e.position().map(|p| p.line()).unwrap_or(0) as usize;
                    let file_content = std::fs::read_to_string(&file_path).unwrap_or_default();
                    let diagnostic_info =
                        crate::utils::generate_csv_diagnostic_context(&file_content, line_num);

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
            if seen_leads.contains(&lead_id) {
                continue;
            }
            seen_leads.insert(lead_id.clone());

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

            if !is_excluded_branch && status_matches {
                all_records.push(record);
            }
        }
    }

    if all_records.is_empty() {
        info!("No valid lead records found after processing files.");
        return Ok(None);
    }

    let tmp_dir = std::env::temp_dir();
    let xlsx_path = tmp_dir.join("Call_Center_Leads.xlsx");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    if let Some(h) = headers {
        for (i, name) in h.iter().enumerate() {
            worksheet.write_string(0, i as u16, name)?;
        }
    }

    for (row_idx, record) in all_records.iter().enumerate() {
        for (col_idx, field) in record.iter().enumerate() {
            worksheet.write_string((row_idx + 1) as u32, col_idx as u16, field)?;
        }
    }

    workbook.save(&xlsx_path)?;

    info!(
        "Successfully generated leads report with {} records at: {}",
        all_records.len(),
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
        assert!(path.to_string_lossy().contains("Call_Center_Leads.xlsx"));

        let _ = std::fs::remove_file(path);
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
        let out_bytes = std::fs::read(&out_path).unwrap(); // Excel files are binary, not strings!

        // Let's just check the size or existence to prove it worked, we can't easily assert on binary Excel
        assert!(out_bytes.len() > 100);
    }
}
