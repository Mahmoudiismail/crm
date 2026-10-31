use anyhow::{Context, Result};
use rust_xlsxwriter::Workbook;
use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn generate_leads_report(
    lead_result_file: &Path,
    exclude_branches: &[String],
) -> Result<Option<PathBuf>> {
    if !lead_result_file.is_file() {
        info!(
            "Lead result file does not exist; skipping Call Center leads attachment: {}",
            lead_result_file.display()
        );
        return Ok(None);
    }

    let file = File::open(lead_result_file)
        .with_context(|| format!("Failed to open lead result {}", lead_result_file.display()))?;
    let mut reader = crate::utils::build_csv_reader_from_reader(file);
    let headers = reader.headers()?.clone();

    let mut lead_id_idx = None;
    let mut branch_idx = None;
    let mut status_idx = None;
    let mut team_idx = None;
    for (index, header) in headers.iter().enumerate() {
        match header
            .trim()
            .trim_start_matches('\u{feff}')
            .to_ascii_lowercase()
            .as_str()
        {
            "lead id" => lead_id_idx = Some(index),
            "branch" => branch_idx = Some(index),
            "status" => status_idx = Some(index),
            "team" => team_idx = Some(index),
            _ => {}
        }
    }

    let lead_id_idx = lead_id_idx.context("Lead result is missing the 'Lead Id' column")?;
    let branch_idx = branch_idx.context("Lead result is missing the 'Branch' column")?;
    let status_idx = status_idx.context("Lead result is missing the 'Status' column")?;
    let team_idx = team_idx.context("Lead result is missing the 'team' column")?;

    let excluded_branches: HashSet<String> = exclude_branches
        .iter()
        .map(|branch| branch.trim().to_lowercase())
        .collect();

    let mut seen_leads = HashSet::new();
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let mut written_records = 0usize;

    for result in reader.records() {
        let record = result.with_context(|| {
            format!("Failed to parse lead result {}", lead_result_file.display())
        })?;
        let team = record.get(team_idx).unwrap_or("").trim();
        if !team.eq_ignore_ascii_case("Call Center") {
            continue;
        }

        let branch = record
            .get(branch_idx)
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let status = record
            .get(status_idx)
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if excluded_branches.contains(&branch) || (status != "new" && status != "follow-up") {
            continue;
        }

        let lead_id = record.get(lead_id_idx).unwrap_or("").trim();
        if !lead_id.is_empty() && !seen_leads.insert(lead_id.to_string()) {
            continue;
        }

        let row_index = (written_records + 1) as u32;
        for (column_index, value) in record.iter().enumerate() {
            worksheet.write_string(row_index, column_index as u16, value)?;
        }
        written_records += 1;
    }

    if written_records == 0 {
        info!("No eligible Call Center leads found in lead_result.csv.");
        return Ok(None);
    }

    for (column_index, header) in headers.iter().enumerate() {
        worksheet.write_string(0, column_index as u16, header)?;
    }

    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let output_path = std::env::temp_dir().join(format!(
        "Call_Center_Leads_{}_{}.xlsx",
        std::process::id(),
        unique_id
    ));
    workbook.save(&output_path)?;

    info!(
        "Generated Call Center leads attachment with {} records from {}",
        written_records,
        lead_result_file.display()
    );
    Ok(Some(output_path))
}

#[cfg(test)]
mod tests {
    use super::generate_leads_report;
    use calamine::Reader;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn attachment_contains_only_eligible_call_center_leads() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("lead_result.csv");
        let mut file = std::fs::File::create(&input).unwrap();
        writeln!(file, "Lead Id,Assignee,Branch,Status,team").unwrap();
        writeln!(file, "L1,alice,Main Branch,New,Call Center").unwrap();
        writeln!(file, "L2,bob,Main Branch,Follow-up,Other Team").unwrap();
        writeln!(file, "L3,,Main Branch,New,").unwrap();
        writeln!(file, "L4,alice,Main Branch,Closed,Call Center").unwrap();
        writeln!(file, "L5,alice,Excluded Branch,New,Call Center").unwrap();
        writeln!(file, "L6,alice,Main Branch,Follow-up,Call Center").unwrap();

        let output = generate_leads_report(&input, &["Excluded Branch".to_string()])
            .unwrap()
            .expect("eligible Call Center leads should create an attachment");
        let mut workbook = calamine::open_workbook_auto(&output).unwrap();
        let sheet = workbook.worksheet_range_at(0).unwrap().unwrap();
        let lead_ids = sheet
            .rows()
            .skip(1)
            .map(|row| row[0].to_string())
            .collect::<Vec<_>>();
        assert_eq!(lead_ids, vec!["L1", "L6"]);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn missing_lead_result_is_not_an_error() {
        let dir = tempdir().unwrap();
        assert!(generate_leads_report(&dir.path().join("missing.csv"), &[])
            .unwrap()
            .is_none());
    }
}
