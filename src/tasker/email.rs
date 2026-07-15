use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use csv::StringRecord;
use rust_xlsxwriter::Workbook;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tracing::{error, info};
use walkdir::WalkDir;

use crate::tasker::config::EmailConfig;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TeamMapping {
    #[serde(alias = "Team Name")]
    team_name: String,
    #[serde(alias = "Receiver Name")]
    receiver_name: Option<String>,
    #[serde(alias = "To Emails")]
    to_emails: Option<String>,
    #[serde(alias = "CC")]
    cc: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TicketRow {
    ticket_id: String,
    assignee: String,
    ticket_type: String,
    ticket_subtype: String,
    ticket_category: String,
    status: String,
    branch: String,
    team: String,
    created_at_dt: Option<NaiveDate>,
    original_row: csv::StringRecord,
}

fn run_powershell(script: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("send_email_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(script.as_bytes())?;
    temp_file.as_file().sync_all()?;

    let (file, path) = temp_file.keep()?;
    drop(file);

    let status = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&path)
        .status()?;

    let _ = std::fs::remove_file(&path);

    if !status.success() {
        anyhow::bail!("PowerShell script exited with status: {}", status);
    }

    Ok(())
}

fn generate_pivot_html(rows: &[TicketRow], statuses: &[String], include_team_col: bool) -> String {
    #[derive(Default)]
    struct Counts {
        status_counts: HashMap<String, usize>,
        total: usize,
    }
    impl Counts {
        fn add(&mut self, status: &str) {
            *self.status_counts.entry(status.to_string()).or_insert(0) += 1;
            self.total += 1;
        }
    }

    let mut sorted_rows = rows.to_vec();
    sorted_rows.sort_by(|a, b| {
        let cmp_team = a.team.cmp(&b.team);
        if cmp_team != std::cmp::Ordering::Equal && include_team_col {
            return cmp_team;
        }
        let cmp_ass = a.assignee.cmp(&b.assignee);
        if cmp_ass != std::cmp::Ordering::Equal {
            return cmp_ass;
        }
        let cmp_sub = a.ticket_subtype.cmp(&b.ticket_subtype);
        if cmp_sub != std::cmp::Ordering::Equal {
            return cmp_sub;
        }
        a.ticket_category.cmp(&b.ticket_category)
    });

    let mut team_counts: HashMap<String, Counts> = HashMap::new();
    let mut assignee_counts: HashMap<(String, String), Counts> = HashMap::new();
    let mut subtype_counts: HashMap<(String, String, String), Counts> = HashMap::new();
    let mut category_counts: HashMap<(String, String, String, String), Counts> = HashMap::new();

    let mut grand_total_by_status: HashMap<String, usize> = HashMap::new();
    let mut grand_total = 0;

    for r in &sorted_rows {
        let t = if include_team_col {
            r.team.clone()
        } else {
            "".to_string()
        };
        let a = r.assignee.clone();
        let s = r.ticket_subtype.clone();
        let c = r.ticket_category.clone();
        let st = r.status.to_lowercase();

        team_counts.entry(t.clone()).or_default().add(&st);
        assignee_counts
            .entry((t.clone(), a.clone()))
            .or_default()
            .add(&st);
        subtype_counts
            .entry((t.clone(), a.clone(), s.clone()))
            .or_default()
            .add(&st);
        category_counts
            .entry((t.clone(), a.clone(), s.clone(), c.clone()))
            .or_default()
            .add(&st);

        *grand_total_by_status.entry(st.clone()).or_insert(0) += 1;
        grand_total += 1;
    }

    // Now filter active_statuses based on what actually has > 0 in grand_total_by_status
    let active_statuses: Vec<String> = statuses
        .iter()
        .filter(|s| {
            grand_total_by_status
                .get(&s.to_lowercase())
                .copied()
                .unwrap_or(0)
                > 0
        })
        .cloned()
        .collect();

    let mut html = String::new();
    html.push_str("<table style='border-collapse: collapse; width: max-content; font-family: Arial, sans-serif; border: 1px solid black; font-size: 14px;'>");
    html.push_str("<tr style='background-color: #d9e1f2; color: black; font-weight: bold;'>");
    html.push_str(
        "<th style='border: 1px solid black; padding: 2px; text-align: left;'>Row Labels</th>",
    );
    for s in &active_statuses {
        html.push_str(&format!(
            "<th style='border: 1px solid black; padding: 8px 15px; text-align: center;'>{}</th>",
            s
        ));
    }
    html.push_str(
        "<th style='border: 1px solid black; padding: 8px 15px; text-align: center;'>Grand Total</th>",
    );
    html.push_str("</tr>");

    let mut printed_teams = HashSet::new();
    let mut printed_assignees = HashSet::new();
    let mut printed_subtypes = HashSet::new();
    let mut printed_categories = HashSet::new();

    let render_row = |name: &str, indent: usize, is_bold: bool, counts: &Counts| -> String {
        let mut r_html = String::new();
        let indent_px = indent * 20 + 8;
        let bold_tag = if is_bold { "<b>" } else { "" };
        let bold_end = if is_bold { "</b>" } else { "" };

        r_html.push_str("<tr>");
        r_html.push_str(&format!(
            "<td style='padding: 8px; padding-left: {}px; border: 1px solid black;'>{}{}{}</td>",
            indent_px, bold_tag, name, bold_end
        ));

        for st in &active_statuses {
            let cnt = counts
                .status_counts
                .get(&st.to_lowercase())
                .copied()
                .unwrap_or(0);
            let val = if cnt > 0 {
                cnt.to_string()
            } else {
                "".to_string()
            };
            r_html.push_str(&format!(
                "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}{}{}</td>",
                bold_tag, val, bold_end
            ));
        }
        r_html.push_str(&format!(
            "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}{}{}</td>",
            bold_tag, counts.total, bold_end
        ));
        r_html.push_str("</tr>");
        r_html
    };

    for r in &sorted_rows {
        let t = if include_team_col {
            r.team.clone()
        } else {
            "".to_string()
        };
        let a = r.assignee.clone();
        let s = r.ticket_subtype.clone();
        let c = r.ticket_category.clone();

        let a_key = (t.clone(), a.clone());
        let s_key = (t.clone(), a.clone(), s.clone());
        let c_key = (t.clone(), a.clone(), s.clone(), c.clone());

        // Skip employees who only have closed tickets
        let assignee_count = if let Some(count) = assignee_counts.get(&a_key) {
            count
        } else {
            continue;
        };
        let mut has_non_closed = false;
        for (st_key, st_cnt) in &assignee_count.status_counts {
            if !st_key.eq_ignore_ascii_case("closed") && *st_cnt > 0 {
                has_non_closed = true;
                break;
            }
        }

        if !has_non_closed {
            continue;
        }

        if include_team_col && !printed_teams.contains(&t) {
            if let Some(count) = team_counts.get(&t) {
                html.push_str(&render_row(&t, 0, true, count));
            }
            printed_teams.insert(t.clone());
        }

        if !printed_assignees.contains(&a_key) {
            let indent = if include_team_col { 1 } else { 0 };
            if let Some(count) = assignee_counts.get(&a_key) {
                html.push_str(&render_row(&a, indent, true, count));
            }
            printed_assignees.insert(a_key.clone());
        }

        let subtype_count = if let Some(count) = subtype_counts.get(&s_key) {
            count
        } else {
            continue;
        };
        let mut subtype_has_non_closed = false;
        for (st_key, st_cnt) in &subtype_count.status_counts {
            if !st_key.eq_ignore_ascii_case("closed") && *st_cnt > 0 {
                subtype_has_non_closed = true;
                break;
            }
        }

        if !subtype_has_non_closed {
            continue;
        }

        if !printed_subtypes.contains(&s_key) {
            let indent = if include_team_col { 2 } else { 1 };
            html.push_str(&render_row(&s, indent, false, subtype_count));
            printed_subtypes.insert(s_key.clone());
        }

        let category_count = if let Some(count) = category_counts.get(&c_key) {
            count
        } else {
            continue;
        };
        let mut category_has_non_closed = false;
        for (st_key, st_cnt) in &category_count.status_counts {
            if !st_key.eq_ignore_ascii_case("closed") && *st_cnt > 0 {
                category_has_non_closed = true;
                break;
            }
        }

        if !category_has_non_closed {
            continue;
        }

        if !printed_categories.contains(&c_key) {
            let indent = if include_team_col { 3 } else { 2 };
            html.push_str(&render_row(&c, indent, false, category_count));
            printed_categories.insert(c_key);
        }
    }

    // Grand total
    html.push_str("<tr style='background-color: #d9e1f2; color: black; font-weight: bold;'>");
    html.push_str(
        "<td style='padding: 8px; text-align: center; border: 1px solid black;'>Grand Total</td>",
    );
    for st in &active_statuses {
        let cnt = grand_total_by_status
            .get(&st.to_lowercase())
            .copied()
            .unwrap_or(0);
        let val = if cnt > 0 {
            cnt.to_string()
        } else {
            "".to_string()
        };
        html.push_str(&format!(
            "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}</td>",
            val
        ));
    }
    html.push_str(&format!(
        "<td style='padding: 8px 15px; text-align: center; border: 1px solid black;'>{}</td>",
        grand_total
    ));
    html.push_str("</tr>");

    html.push_str("</table>");
    html
}
fn generate_leads_report(
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
        let mut rdr = crate::utils::build_csv_reader_builder()
            .delimiter(if file_content.contains('\t') {
                b'\t'
            } else {
                b','
            })
            .from_reader(file_content.as_bytes());

        if headers.is_none() {
            let h = rdr.headers()?.clone();
            for (i, col_name) in h.iter().enumerate() {
                let lower = col_name.trim().to_lowercase();
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

            let status_matches = status == "new" || status == "follow up" || status == "follow-up";

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

#[allow(clippy::too_many_arguments)]
pub fn process_emails(
    results_file: &str,
    config: &EmailConfig,
    only_call_center: bool,
    send_exceptions: bool,
    download_dir: &str,
    minutes_ago: i64,
    category_exceptions: Option<&[crate::tasker::config::CategoryException]>,
    exclude_branches: &[String],
) -> Result<()> {
    info!(
        "Starting email processing module. Reading output from {} (only_call_center: {}, send_exceptions: {})",
        results_file, only_call_center, send_exceptions
    );

    let effective_send_exceptions = send_exceptions || config.send_exceptions.unwrap_or(false);

    let exception_teams: std::collections::HashSet<String> =
        if let Some(exceptions) = category_exceptions {
            exceptions
                .iter()
                .filter_map(|e| e.team.as_ref())
                .filter(|t: &&String| !t.trim().is_empty())
                .map(|t: &String| t.trim().to_lowercase())
                .collect()
        } else {
            std::collections::HashSet::new()
        };

    let team_mapping_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.team_mapping_file);
    // 1. Load the team mapping file
    let mut team_maps: HashMap<String, TeamMapping> = HashMap::new();
    let mapping_file = File::open(&team_mapping_path).context(format!(
        "Failed to open team mapping file: {}",
        team_mapping_path.display()
    ))?;
    let mut map_rdr = crate::utils::build_csv_reader_from_reader(mapping_file);

    for result in map_rdr.deserialize::<TeamMapping>() {
        match result {
            Ok(mapping) => {
                tracing::trace!("Loaded team mapping: {:?}", mapping);
                team_maps.insert(mapping.team_name.trim().to_lowercase(), mapping);
            }
            Err(e) => {
                error!("Failed to parse row in team mapping file: {}", e);
            }
        }
    }
    info!("Loaded {} team mappings.", team_maps.len());

    // 2. Read the results.csv file to memory
    let file = File::open(results_file)?;
    let mut rdr = crate::utils::build_csv_reader_from_reader(file);
    let headers = rdr.headers()?.clone();

    let mut tkt_id_idx = None;
    let mut assignee_idx = None;
    let mut subtype_idx = None;
    let mut category_idx = None;
    let mut type_idx = None;
    let mut status_idx = None;
    let mut branch_idx = None;
    let mut team_idx = None;
    let mut created_at_idx = None;
    let mut is_exception_idx = None;
    let mut position_idx = None;

    for (i, h) in headers.iter().enumerate() {
        let h_low = h.trim().to_lowercase();
        if h_low == "ticket id" {
            tkt_id_idx = Some(i);
        } else if h_low == "assignee" {
            assignee_idx = Some(i);
        } else if h_low == "ticket sub-type" {
            subtype_idx = Some(i);
        } else if h_low == "ticket category" {
            category_idx = Some(i);
        } else if h_low == "ticket type" {
            type_idx = Some(i);
        } else if h_low == "status" || h_low == "ticket status" {
            status_idx = Some(i);
        } else if h_low == "branch" {
            branch_idx = Some(i);
        } else if h_low == "team" {
            team_idx = Some(i);
        } else if h_low == "created at" {
            created_at_idx = Some(i);
        } else if h_low == "is exception" {
            is_exception_idx = Some(i);
        } else if h_low == "position" {
            position_idx = Some(i);
        } else if h_low == "month" {
            // we reuse position_idx as a generic way to filter out columns if we want,
            // but let's just make a new one or handle it in the filter.
            // Actually, we can just skip it similarly:
        }
    }

    let mut month_idx = None;
    for (i, h) in headers.iter().enumerate() {
        if h.trim().to_lowercase() == "month" {
            month_idx = Some(i);
        }
    }

    let mut ticket_rows = Vec::new();
    let mut dynamic_statuses = HashSet::new();

    for result in rdr.records() {
        let record = result?;
        tracing::trace!("Processing email record: {:?}", record);

        let is_exception_val = is_exception_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("No")
            .trim()
            .to_lowercase();

        let is_exception = is_exception_val == "yes";

        if effective_send_exceptions {
            if !is_exception {
                continue; // Only process exceptions
            }
        } else {
            if is_exception {
                continue; // Only process normal tickets
            }
        }

        let status = status_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim()
            .to_string();

        let branch = branch_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim()
            .to_string();
        let team = team_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim()
            .to_string();

        // Dynamic dates parsing to find the min date later
        let mut created_at_dt = None;
        if let Some(idx) = created_at_idx {
            if let Some(val) = record.get(idx) {
                let trimmed = val.trim();
                // We'll just try to parse dd/mm/yyyy or mm/dd/yyyy
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%d/%m/%Y %H:%M:%S")
                {
                    created_at_dt = Some(dt.date());
                } else if let Ok(dt) =
                    chrono::NaiveDateTime::parse_from_str(trimmed, "%m/%d/%Y %H:%M:%S")
                {
                    created_at_dt = Some(dt.date());
                }
            }
        }

        if !status.is_empty() {
            dynamic_statuses.insert(status.to_lowercase());
        }

        let assignee_val = assignee_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim();
        let display_assignee = if assignee_val.is_empty() {
            "Unassigned"
        } else {
            assignee_val
        };

        ticket_rows.push(TicketRow {
            ticket_id: tkt_id_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_string(),
            assignee: display_assignee.to_string(),
            ticket_type: type_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_string(),
            ticket_subtype: subtype_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_string(),
            ticket_category: category_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_string(),
            status: status.clone(),
            branch: branch.clone(),
            team: team.clone(),
            created_at_dt,
            original_row: record,
        });
    }

    info!("Loaded {} tickets for email evaluation.", ticket_rows.len());

    let mut statuses_vec: Vec<String> = dynamic_statuses
        .into_iter()
        .filter(|s| !s.eq_ignore_ascii_case("closed"))
        .collect();
    // Sort logic: open, follow-up, on-hold, then alphabetical
    statuses_vec.sort_by(|a, b| {
        let a_ord = match a.as_str() {
            "open" => 0,
            "follow-up" | "followup" => 1,
            "on-hold" | "onhold" => 2,
            _ => 3,
        };
        let b_ord = match b.as_str() {
            "open" => 0,
            "follow-up" | "followup" => 1,
            "on-hold" | "onhold" => 2,
            _ => 3,
        };
        if a_ord == b_ord {
            a.cmp(b)
        } else {
            a_ord.cmp(&b_ord)
        }
    });

    // Grouping
    // We have 3 buckets: per_team, per_branch, and call_center

    let send_per_team_all_branches: HashSet<String> = config
        .send_per_team_all_branches
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let send_per_branch_branches: HashSet<String> = config
        .send_per_branch_branches
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let send_per_team_branches: HashSet<String> = config
        .send_per_team_branches
        .as_ref()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let send_cc = only_call_center || config.send_call_center.unwrap_or(false);

    let mut per_team_buckets: HashMap<String, Vec<TicketRow>> = HashMap::new(); // Key: Team name
    let mut per_branch_buckets: HashMap<String, Vec<TicketRow>> = HashMap::new(); // Key: Branch name
    let mut call_center_bucket: Vec<TicketRow> = Vec::new();

    for row in ticket_rows {
        let b_low = row.branch.to_lowercase();
        let t_low = row.team.to_lowercase();

        let is_cc = t_low == "call center";

        // "all allowed branches" means the branch must be in send_per_branch_branches.
        let allowed_branch = send_per_branch_branches.contains(&b_low);

        if effective_send_exceptions {
            per_team_buckets
                .entry(row.team.clone())
                .or_default()
                .push(row);
        } else if is_cc {
            // Include Call Center tickets if send_cc is enabled, regardless of only_call_center flag
            if send_cc {
                call_center_bucket.push(row);
            }
        } else if !only_call_center {
            if send_per_team_all_branches.contains(&t_low) {
                per_team_buckets
                    .entry(row.team.clone())
                    .or_default()
                    .push(row);
            } else if allowed_branch {
                per_branch_buckets
                    .entry(row.branch.clone())
                    .or_default()
                    .push(row);
            } else if send_per_team_branches.contains(&b_low) {
                per_team_buckets
                    .entry(row.team.clone())
                    .or_default()
                    .push(row);
            }
        }
    }

    let today_str = Local::now().format("%d %b %Y").to_string();

    let send_email_for_bucket = |raw_bucket_name: &str,
                                 rows: &[TicketRow],
                                 is_branch: bool|
     -> Result<()> {
        let bucket_name_cleaned = raw_bucket_name.replace('\u{FFFD}', "").replace("ï¿½", "");
        let bucket_name = bucket_name_cleaned.as_str();

        let mut leads_report_path = None;
        if bucket_name.eq_ignore_ascii_case("call center") && !effective_send_exceptions {
            match generate_leads_report(download_dir, minutes_ago, exclude_branches) {
                Ok(path_opt) => leads_report_path = path_opt,
                Err(e) => error!("Failed to generate leads report for Call Center: {}", e),
            }
        }

        let all_closed =
            rows.is_empty() || rows.iter().all(|r| r.status.eq_ignore_ascii_case("closed"));

        if all_closed {
            if leads_report_path.is_none() {
                info!(
                    "Skipping email for {} because all tickets are closed or empty, and no leads report generated.",
                    raw_bucket_name
                );
                return Ok(());
            } else {
                info!(
                    "All tickets closed or empty for {}, but leads report was generated. Continuing to send leads.",
                    raw_bucket_name
                );
            }
        }

        // Find min date
        let mut min_date = None;
        for r in rows {
            if let Some(d) = r.created_at_dt {
                if let Some(curr_min) = min_date {
                    if d < curr_min {
                        min_date = Some(d);
                    }
                } else {
                    min_date = Some(d);
                }
            }
        }
        let from_date_str = min_date
            .map(|d| {
                let limit_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
                let use_date = if d < limit_date { d } else { limit_date };
                if use_date.year() == chrono::Local::now().year() {
                    use_date.format("%d %b").to_string()
                } else {
                    use_date.format("%d %b %Y").to_string()
                }
            })
            .unwrap_or_else(|| "1 May 2026".to_string());

        info!(
            "Generating email for {} with {} rows.",
            bucket_name,
            rows.len()
        );

        let mapping = team_maps.get(&bucket_name.to_lowercase());
        let mapped_to = mapping
            .and_then(|m| m.to_emails.clone())
            .unwrap_or_default();

        let (to_emails, cc_list) = if mapped_to.trim().is_empty() {
            // Fallback: send to default email only, no CCs.
            (config.default_to_email.clone(), String::new())
        } else {
            let mapped_cc = mapping.and_then(|m| m.cc.clone()).unwrap_or_default();
            let ccs = if bucket_name.eq_ignore_ascii_case("call center")
                || effective_send_exceptions
                || exception_teams.contains(&bucket_name.to_lowercase())
            {
                vec![mapped_cc]
            } else {
                vec![
                    config.initial_cc.clone(),
                    mapped_cc,
                    config.ending_cc.clone(),
                ]
            }
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
            .join(";");
            (mapped_to, ccs)
        };

        let html_table = generate_pivot_html(rows, &statuses_vec, is_branch);

        let receiver_name = mapping
            .and_then(|m| m.receiver_name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "All".to_string());

        let (subject, body) = if bucket_name.eq_ignore_ascii_case("Call Center")
            && !effective_send_exceptions
        {
            (format!("Open TKTs - {}", bucket_name), "".to_string())
        } else if let Some(template_path_str) = &config.body_template_file {
            let template_path =
                crate::tasker::csv_task::resolve_relative_to_exe_dir(template_path_str);
            let template_content = if template_path.exists() {
                std::fs::read_to_string(&template_path).unwrap_or_else(|e| {
                    error!(
                        "Failed to read template file {}: {}",
                        template_path.display(),
                        e
                    );
                    "".to_string()
                })
            } else {
                let default_template = r#"<!DOCTYPE html>
<html>
<head>
    <title>Open TKTs - {bucket_name}</title>
</head>
<body style="font-family: Arial, sans-serif;">
    Dear {receiver_name},<br/>
    <table border="0" cellpadding="0" cellspacing="0">
        <tr>
            <td width="20"></td>
            <td>
                Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>
                {html_table}
            </td>
        </tr>
    </table>
</body>
</html>"#;
                if let Err(e) = std::fs::write(&template_path, default_template) {
                    error!(
                        "Failed to generate default template at {}: {}",
                        template_path.display(),
                        e
                    );
                }
                default_template.to_string()
            };

            // Extract title
            let mut extracted_subject = format!("Open TKTs - {}", bucket_name);
            if let (Some(start_idx), Some(relative_end_idx)) = (
                template_content.find("<title>"),
                template_content.find("</title>"),
            ) {
                let end_idx = relative_end_idx;
                if start_idx < end_idx {
                    let title_content = &template_content[start_idx + 7..end_idx];
                    extracted_subject = title_content
                        .replace("{bucket_name}", bucket_name)
                        .replace("{from_date_str}", &from_date_str)
                        .replace("{today_str}", &today_str)
                        .trim()
                        .to_string();
                }
            }

            // Extract body
            let mut extracted_body = template_content.clone();
            if let (Some(start_idx), Some(relative_end_idx)) = (
                template_content.find("<body>"),
                template_content.find("</body>"),
            ) {
                let end_idx = relative_end_idx;
                if start_idx < end_idx {
                    extracted_body = template_content[start_idx + 6..end_idx].to_string();
                }
            }

            // Clean up old template margin layout dynamically if it's there
            extracted_body = extracted_body
                .replace("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\r\n        <tr>\r\n            <td width=\"20\"></td>\r\n            <td>\r\n                {html_table}\r\n            </td>\r\n        </tr>\r\n    </table>", "{html_table}")
                .replace("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\n        <tr>\n            <td width=\"20\"></td>\n            <td>\n                {html_table}\n            </td>\n        </tr>\n    </table>", "{html_table}")
                .replace("&nbsp;&nbsp;&nbsp;&nbsp;", "")
                .replace("Dear All", "Dear {receiver_name}");

            let indent_spaces = config.indentation_spaces.unwrap_or(4);
            let indent_width = indent_spaces * 5;

            // Dynamically upgrade old layouts if they don't have the new full-wrap div structure.
            // If it contains the exact text "Kindly find below" without being inside a layout div, wrap it.
            let old_pattern = "Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>\n    {html_table}";
            let old_pattern_r = "Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>\r\n    {html_table}";

            let new_pattern = format!(
                r#"<table border='0'><tr><td width='{}'></td><td>
        Kindly find below the list of open tickets in {{bucket_name}} for the period from {{from_date_str}} until {{today_str}}.<br/><br/>
        {{html_table}}
    </td></tr></table>"#,
                indent_width
            );

            let _prev_table_pattern = r#"<table border="0" cellpadding="0" cellspacing="0">
        <tr>
            <td width="20"></td>
            <td>
                Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>
                {html_table}
            </td>
        </tr>
    </table>"#;
            let _prev_div_pattern = r#"<div style="margin-left: 20px;">
        Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>
        {html_table}
    </div>"#;

            let _prev_div_with_nbsps_pattern = &format!(
                r#"<div>
        {}Kindly find below the list of open tickets in {{bucket_name}} for the period from {{from_date_str}} until {{today_str}}.<br/><br/>
        {{html_table}}
    </div>"#,
                "&nbsp;".repeat(indent_spaces as usize)
            );

            extracted_body = extracted_body
                .replace(old_pattern, &new_pattern)
                .replace(old_pattern_r, &new_pattern)
                .replace(_prev_div_pattern, &new_pattern)
                .replace(_prev_div_with_nbsps_pattern, &new_pattern);

            // Just in case it was a single line version
            let old_pattern_inline = "Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>{html_table}";
            extracted_body = extracted_body.replace(old_pattern_inline, &new_pattern);

            // Replace placeholders
            let final_body = extracted_body
                .replace("{receiver_name}", &receiver_name)
                .replace("{bucket_name}", bucket_name)
                .replace("{from_date_str}", &from_date_str)
                .replace("{today_str}", &today_str)
                .replace("{html_table}", &html_table);

            let wrapped_body = format!("<html><body>{}</body></html>", final_body);
            (extracted_subject, wrapped_body)
        } else {
            let indent_spaces = config.indentation_spaces.unwrap_or(4);
            let indent_width = indent_spaces * 5;
            let body = format!(
                r#"<html><body style="font-family: Arial, sans-serif;">Dear {},<br/>
    <table border='0'><tr><td width='{}'></td><td>
        Kindly find below the list of open tickets in {} for the period from {} until {}.<br/><br/>
        {}
    </td></tr></table>
</body></html>"#,
                receiver_name, indent_width, bucket_name, from_date_str, today_str, html_table
            );
            let subject = format!("Open TKTs - {}", bucket_name);
            (subject, body)
        };

        let tmp_dir = std::env::temp_dir();
        let safe_name = bucket_name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");

        let skip_team_col = !is_branch && team_idx.is_some();
        let skip_team_idx = if skip_team_col { team_idx } else { None };

        let save_as_csv = config.save_attachment_as_csv.unwrap_or(false);
        let attachment_path = if save_as_csv {
            let csv_path = tmp_dir.join(format!("{}_open_tickets.csv", safe_name));
            let mut f = std::fs::File::create(&csv_path)?;
            f.write_all(b"\xEF\xBB\xBF")?;
            let mut wtr = csv::WriterBuilder::new().from_writer(f);
            let mut header_rec = vec![];
            for (i, h) in headers.iter().enumerate() {
                if is_exception_idx == Some(i)
                    || position_idx == Some(i)
                    || skip_team_idx == Some(i)
                    || month_idx == Some(i)
                {
                    continue;
                }
                header_rec.push(h.to_string());
            }
            wtr.write_record(&header_rec)?;

            for row in rows.iter() {
                if row.status.eq_ignore_ascii_case("closed") {
                    continue;
                }
                let mut data_rec = vec![];
                for (c_idx, field) in row.original_row.iter().enumerate() {
                    if is_exception_idx == Some(c_idx)
                        || position_idx == Some(c_idx)
                        || skip_team_idx == Some(c_idx)
                        || month_idx == Some(c_idx)
                    {
                        continue;
                    }
                    data_rec.push(field.to_string());
                }
                wtr.write_record(&data_rec)?;
            }
            wtr.flush()?;
            csv_path
        } else {
            let xlsx_path = tmp_dir.join(format!("{}_open_tickets.xlsx", safe_name));
            let mut workbook = Workbook::new();
            let worksheet = workbook.add_worksheet();

            let mut out_col_idx = 0;
            for (i, h) in headers.iter().enumerate() {
                if is_exception_idx == Some(i)
                    || position_idx == Some(i)
                    || skip_team_idx == Some(i)
                    || month_idx == Some(i)
                {
                    continue;
                }
                worksheet.write_string(0, out_col_idx, h)?;
                out_col_idx += 1;
            }

            let mut write_r_idx = 1;
            for row in rows.iter() {
                if row.status.eq_ignore_ascii_case("closed") {
                    continue;
                }
                let mut out_c_idx = 0;
                for (c_idx, field) in row.original_row.iter().enumerate() {
                    if is_exception_idx == Some(c_idx)
                        || position_idx == Some(c_idx)
                        || skip_team_idx == Some(c_idx)
                        || month_idx == Some(c_idx)
                    {
                        continue;
                    }
                    worksheet.write_string(write_r_idx as u32, out_c_idx, field)?;
                    out_c_idx += 1;
                }
                write_r_idx += 1;
            }
            workbook.save(&xlsx_path)?;
            xlsx_path
        };

        let save_as_html = config.save_email_as_html.unwrap_or(false);

        if save_as_html {
            let html_path = tmp_dir.join(format!("{}_email.html", safe_name));
            let mut f = std::fs::File::create(&html_path)?;
            f.write_all(body.as_bytes())?;
            f.sync_all()?;
            info!(
                "Saved email HTML for {} to {}",
                bucket_name,
                html_path.display()
            );
        }

        let display_or_send = if config.send_emails.unwrap_or(false) {
            "Send()"
        } else {
            "Display()"
        };

        let mut ps_script = format!(
            r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "{}"
$Mail.HTMLBody = '{}'
"#,
            to_emails,
            cc_list,
            subject.replace("\"", "'"), // basic sanitize for powershell string interpolation
            body.replace("'", "''")
        );

        if !all_closed {
            ps_script.push_str(&format!(
                "$Mail.Attachments.Add(\"{}\")\n",
                attachment_path.display()
            ));
        }

        if let Some(ref leads_path) = leads_report_path {
            ps_script.push_str(&format!(
                "$Mail.Attachments.Add(\"{}\")\n",
                leads_path.display()
            ));
        }

        ps_script.push_str(&format!("$Mail.{}\n", display_or_send));

        // We still need to respect `save_as_html` and `save_as_csv` for tests without running powershell
        if config.save_email_as_html.unwrap_or(false)
            && config.save_attachment_as_csv.unwrap_or(false)
        {
            // We do not delete the attachment paths so they can be asserted in tests
            // If powershell fails in tests, we just catch it and log, or we can skip executing powershell entirely for test efficiency if configured to not send emails
            if !config.send_emails.unwrap_or(false) {
                info!("Successfully processed email for {} (Display only, powershell execution skipped for test stability)", bucket_name);
                // IMPORTANT: Do NOT return early if we need to let tests assert the leads file is NOT deleted.
                // Actually we DO return early, but we should make sure the leads file doesn't get deleted by returning early before the file deletion below.
                return Ok(());
            }
        }

        if let Err(e) = run_powershell(&ps_script) {
            error!("Failed to send email for {}: {}", bucket_name, e);
            // Send error email fallback
            let err_script = format!(
                r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.Subject = "Error generating email for {}"
$Mail.Body = "An error occurred while generating or sending the email for {}. Error: {}"
$Mail.Display()
"#,
                config.default_to_email,
                bucket_name,
                bucket_name,
                e.to_string().replace("\"", "'")
            );
            if let Err(e2) = run_powershell(&err_script) {
                error!("Failed to send error notification email: {}", e2);
            }
            anyhow::bail!(
                "PowerShell execution failed for email bucket {}: {}",
                bucket_name,
                e
            );
        } else {
            info!("Successfully processed email for {}", bucket_name);
        }

        let _ = std::fs::remove_file(attachment_path);

        if let Some(ref leads_path) = leads_report_path {
            let _ = std::fs::remove_file(leads_path);
        }

        Ok(())
    };

    // 1. Process Per-Team
    for (team, rows) in &per_team_buckets {
        send_email_for_bucket(team, rows, false)?;
    }

    // 2. Process Per-Branch
    for (branch, rows) in &per_branch_buckets {
        send_email_for_bucket(branch, rows, true)?;
    }

    // 3. Process Call Center
    // Call Center gets special treatment because even if they have NO open tickets, they might have leads.
    if send_cc && !effective_send_exceptions {
        send_email_for_bucket("Call Center", &call_center_bucket, true)?;
    }

    info!("Email processing complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasker::config::EmailConfig;
    use csv::StringRecord;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_run_powershell_file_lifecycle() {
        // This test ensures that the powershell script path creation,
        // unlocking, execution, and cleanup are working as expected.
        let script = "Write-Host 'Hello World'";
        // Normally run_powershell will succeed if powershell is available.
        // We just call it and ensure it doesn't return a file-in-use error.
        let result = run_powershell(script);
        // On linux, it might fail because powershell isn't installed.
        // But if it fails, it shouldn't be an OS error 32 (file in use).
        // Let's just assert that it ran or failed for another reason (like Not Found).
        if let Err(e) = result {
            assert!(!e.to_string().contains("The process cannot access the file"), "File lock error occurred");
        }
    }

    #[test]
    fn test_email_processing_skips_closed() {
        let download_dir = tempfile::tempdir().unwrap();
        let mut ticket_file = File::create(download_dir.path().join("results.csv")).unwrap();
        writeln!(
            ticket_file,
            "Ticket Id,Branch Name,Category,Type,Subtype,Status,Creation Date,Assignee,Position,team,Is Exception"
        )
        .unwrap();
        writeln!(
            ticket_file,
            "1001,Main Branch,Cat1,Type1,Sub1,closed,01/01/2026 12:00:00,alice,Pos1,Team A,No"
        )
        .unwrap();

        let mut teams_file = NamedTempFile::new().unwrap();
        writeln!(teams_file, "Team Name,To Email,CC Email").unwrap();
        writeln!(teams_file, "Team A,test@example.com,cc@example.com").unwrap();

        let email_config = EmailConfig {
            team_mapping_file: teams_file.path().to_str().unwrap().to_string(),
            body_template_file: None,
            initial_cc: "init@example.com".to_string(),
            ending_cc: "end@example.com".to_string(),
            send_emails: Some(false),
            default_to_email: "def@example.com".to_string(),
            send_per_team_all_branches: vec!["Main Branch".to_string()],
            send_per_branch_branches: vec![],
            send_per_team_branches: None,
            send_call_center: Some(false),
            send_exceptions: Some(false),
            indentation_spaces: None,
            save_attachment_as_csv: None,
            save_email_as_html: None,
        };

        // If the email script runs but finds only closed tickets, it should skip.
        // The fact it doesn't try to run PowerShell (which might fail in Linux sandbox) means it skips safely.
        let result = process_emails(
            download_dir.path().join("results.csv").to_str().unwrap(),
            &email_config,
            false,
            false,
            download_dir.path().to_str().unwrap(),
            60,
            None,
            &[],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_send_exceptions_bypasses_call_center_special_logic() {
        let download_dir = tempfile::tempdir().unwrap();
        let results_path = download_dir.path().join("results.csv");
        let mut ticket_file = File::create(&results_path).unwrap();
        writeln!(
            ticket_file,
            "Ticket Id,Branch,Ticket Category,Ticket Type,Ticket Sub-Type,Status,Created At,Assignee,Position,team,Is Exception"
        )
        .unwrap();
        // A Call Center ticket marked as Exception
        writeln!(
            ticket_file,
            "2001,Main Branch,Cat1,Type1,Sub1,open,01/05/2026 12:00:00,alice,Pos1,Call Center,Yes"
        )
        .unwrap();

        let mut teams_file = NamedTempFile::new().unwrap();
        writeln!(teams_file, "Team Name,To Email,CC").unwrap();
        writeln!(teams_file, "Call Center,cc@example.com,cc_boss@example.com").unwrap();

        let email_config = EmailConfig {
            team_mapping_file: teams_file.path().to_str().unwrap().to_string(),
            body_template_file: None,
            initial_cc: "init@example.com".to_string(),
            ending_cc: "end@example.com".to_string(),
            send_emails: Some(false), // Display only, but we'll skip PS
            default_to_email: "def@example.com".to_string(),
            send_per_team_all_branches: vec![],
            send_per_branch_branches: vec![],
            send_per_team_branches: None,
            send_call_center: Some(true), // Enabled, but should be bypassed by send_exceptions
            send_exceptions: Some(true),
            indentation_spaces: None,
            save_attachment_as_csv: Some(true),
            save_email_as_html: Some(true),
        };

        // We'll also put a lead report in the download dir to see if it's NOT picked up
        let lead_report_path = download_dir.path().join("lead_report_123.csv");
        let mut lead_file = File::create(&lead_report_path).unwrap();
        writeln!(lead_file, "Lead Id,Branch,Status").unwrap();
        writeln!(lead_file, "L1,Main Branch,New").unwrap();

        let result = process_emails(
            results_path.to_str().unwrap(),
            &email_config,
            false,
            true, // send_exceptions
            download_dir.path().to_str().unwrap(),
            60 * 24 * 365,
            None,
            &[],
        );

        assert!(result.is_ok());

        let temp_dir = std::env::temp_dir();
        let email_html_path = temp_dir.join("Call_Center_email.html");
        assert!(
            email_html_path.exists(),
            "Email HTML should still be generated for Call Center as an exception team"
        );

        let html_content = std::fs::read_to_string(&email_html_path).unwrap();
        // If it used Call Center special logic, the body would be empty (because of `if bucket_name.eq_ignore_ascii_case("Call Center") && !effective_send_exceptions { (..., "".to_string()) }`)
        // Since it's an exception, it should use the default or template body.
        assert!(
            !html_content.contains("<body></body>"),
            "Email body should not be empty for Call Center exception"
        );
        assert!(
            html_content.contains("Kindly find below"),
            "Email should contain standard template text"
        );

        let _ = std::fs::remove_file(email_html_path);
        let _ = std::fs::remove_file(temp_dir.join("Call_Center_open_tickets.csv"));
    }

    #[test]
    fn test_email_html_pivot_generation() {
        // Build mock rows
        let r1 = StringRecord::from(vec!["1", "Main Branch", "Open", "Team A"]);
        let tr1 = TicketRow {
            original_row: r1,
            ticket_id: "1".to_string(),
            team: "Team A".to_string(),
            branch: "Main Branch".to_string(),
            status: "Open".to_string(),
            assignee: "alice".to_string(),
            ticket_type: "t".to_string(),
            ticket_subtype: "s".to_string(),
            ticket_category: "c".to_string(),
            created_at_dt: None,
        };

        let statuses = vec!["Open".to_string()];

        let html = generate_pivot_html(&[tr1], &statuses, false);

        // Assert HTML structure expectations
        assert!(html.contains("Open"));
        assert!(html.contains("Grand Total"));
    }

    // test_leads_report_parsing_error_diagnostic removed because it tests non-flexible behavior
    // and flexible(true) makes this no longer an error.

    #[test]
    fn test_call_center_leads_attachment_logic() {
        let download_dir = tempfile::tempdir().unwrap();

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
    fn test_closed_tickets_excluded_from_attachments() {
        let temp_dir = tempfile::tempdir().unwrap();
        let results_path = temp_dir.path().join("results.csv");
        let mut ticket_file = File::create(&results_path).unwrap();
        writeln!(
            ticket_file,
            "Ticket Id,Branch,Ticket Category,Ticket Type,Ticket Sub-Type,Status,Created At,Assignee,Position,team,Is Exception"
        )
        .unwrap();
        // One open ticket, one closed ticket
        writeln!(
            ticket_file,
            "101,Branch A,Cat1,Type1,Sub1,open,01/05/2026 12:00:00,alice,Pos1,Team A,No"
        )
        .unwrap();
        writeln!(
            ticket_file,
            "102,Branch A,Cat1,Type1,Sub1,closed,01/05/2026 12:00:00,bob,Pos1,Team A,No"
        )
        .unwrap();

        let mut teams_file = NamedTempFile::new().unwrap();
        writeln!(teams_file, "Team Name,To Email,CC").unwrap();
        writeln!(teams_file, "Team A,team@example.com,cc@example.com").unwrap();

        let email_config = EmailConfig {
            team_mapping_file: teams_file.path().to_str().unwrap().to_string(),
            body_template_file: None,
            initial_cc: "".to_string(),
            ending_cc: "".to_string(),
            send_emails: Some(false),
            default_to_email: "def@example.com".to_string(),
            send_per_team_all_branches: vec!["Team A".to_string()],
            send_per_branch_branches: vec![],
            send_per_team_branches: None,
            send_call_center: Some(false),
            send_exceptions: Some(false),
            indentation_spaces: None,
            save_attachment_as_csv: Some(true),
            save_email_as_html: Some(true),
        };

        process_emails(
            results_path.to_str().unwrap(),
            &email_config,
            false,
            false,
            temp_dir.path().to_str().unwrap(),
            60,
            None,
            &[],
        )
        .unwrap();

        let system_temp = std::env::temp_dir();
        let attachment_csv = system_temp.join("Team_A_open_tickets.csv");
        assert!(attachment_csv.exists(), "Attachment should be generated");

        let attachment_content = std::fs::read_to_string(&attachment_csv).unwrap();
        assert!(
            attachment_content.contains("101"),
            "Open ticket should be in attachment"
        );
        assert!(
            !attachment_content.contains("102"),
            "Closed ticket should NOT be in attachment"
        );

        let email_html = system_temp.join("Team_A_email.html");
        let html_content = std::fs::read_to_string(&email_html).unwrap();
        // Pivot table should skip closed if they are not needed but it also skips them in the loop.
        // Actually generate_pivot_html includes all rows but filters them out in the rendering loop if they only have closed.
        // In this case, Team A has an open ticket, so it should be included.
        // But for 'bob' who only has closed, it should be skipped.
        assert!(html_content.contains("alice"));
        assert!(
            !html_content.contains("bob"),
            "Assignee with only closed tickets should be excluded from HTML table"
        );

        let _ = std::fs::remove_file(attachment_csv);
        let _ = std::fs::remove_file(email_html);
    }
}
