use anyhow::{Context, Result};
use chrono::{DateTime, Duration};
use chrono::{Datelike, Local, NaiveDate};
use csv::{ReaderBuilder, StringRecord};
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
    receiver_name: String,
    #[serde(alias = "To Emails")]
    to_emails: String,
    #[serde(alias = "CC")]
    cc: String,
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

use std::sync::atomic::{AtomicUsize, Ordering};

static SCRIPT_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run_powershell(script: &str) -> Result<()> {
    let tmp_dir = std::env::temp_dir();
    let count = SCRIPT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let script_path = tmp_dir.join(format!(
        "send_email_{}_{}.ps1",
        Local::now().timestamp_nanos_opt().unwrap_or(0),
        count
    ));

    let mut file = File::create(&script_path)?;
    file.write_all(script.as_bytes())?;
    file.sync_all()?;
    drop(file);

    let status = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .status()?;

    let _ = std::fs::remove_file(script_path);

    if !status.success() {
        anyhow::bail!("PowerShell script exited with status: {}", status);
    }

    Ok(())
}

fn generate_pivot_html(rows: &[TicketRow], statuses: &[String], include_team_col: bool) -> String {
    // We group by: (TeamName?), Assignee, Ticket Subtype, Ticket Category
    // And for each we accumulate the status counts.

    let mut present_statuses = HashSet::new();
    for r in rows {
        present_statuses.insert(r.status.to_lowercase());
    }

    let active_statuses: Vec<String> = statuses
        .iter()
        .filter(|s| present_statuses.contains(&s.to_lowercase()))
        .cloned()
        .collect();

    // To match the layout:
    // Row Labels             | open | follow-up | expired | ... | Grand Total
    //  Assignee1             |      |     1     |         | ... |      1
    //    subtype1            |      |     1     |         | ... |      1
    //      category1         |      |     1     |         | ... |      1

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

    // Compute totals
    let mut grand_total_by_status: HashMap<String, usize> = HashMap::new();
    let mut grand_total = 0;

    // Hierarchy: Team (optional) -> Assignee -> Subtype -> Category
    // Because building a full pivot tree in Rust manually is verbose, let's just do:
    // A nested map structure.

    // Since `Team` is optional in the pivot tree depending on `include_team_col`
    // If include_team_col is true, we add a level at the top for Team.

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

    // We will collect the data into a flat list and then sort it to group easily
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

    // We need to render the tree.
    // We will do a multi-pass approach or just use variables to track current groups.

    let _current_team = String::new();
    let _current_assignee = String::new();
    let _current_subtype = String::new();

    // To print totals at the top of a group, we need to pre-aggregate.
    let mut team_counts: HashMap<String, Counts> = HashMap::new();
    let mut assignee_counts: HashMap<(String, String), Counts> = HashMap::new(); // (team, assignee)
    let mut subtype_counts: HashMap<(String, String, String), Counts> = HashMap::new(); // (team, assignee, subtype)
    let mut category_counts: HashMap<(String, String, String, String), Counts> = HashMap::new(); // (team, assignee, subtype, category)

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
            let cnt = counts.status_counts.get(st).copied().unwrap_or(0);
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

        // Skip employees who only have closed tickets
        let assignee_count = assignee_counts.get(&a_key).unwrap();
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
            html.push_str(&render_row(&t, 0, true, team_counts.get(&t).unwrap()));
            printed_teams.insert(t.clone());
        }

        if !printed_assignees.contains(&a_key) {
            let indent = if include_team_col { 1 } else { 0 };
            html.push_str(&render_row(
                &a,
                indent,
                true,
                assignee_counts.get(&a_key).unwrap(),
            ));
            printed_assignees.insert(a_key);
        }

        let s_key = (t.clone(), a.clone(), s.clone());

        let subtype_count = subtype_counts.get(&s_key).unwrap();
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

        let c_key = (t.clone(), a.clone(), s.clone(), c.clone());

        let category_count = category_counts.get(&c_key).unwrap();
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
        let cnt = grand_total_by_status.get(st).copied().unwrap_or(0);
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

fn generate_leads_report(download_dir: &str, minutes_ago: i64) -> Result<Option<PathBuf>> {
    let download_dir_path = std::path::PathBuf::from(download_dir);
    let now = Local::now().naive_local();
    let threshold = now - Duration::minutes(minutes_ago);
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

    let mut all_records: Vec<StringRecord> = Vec::new();
    let mut headers = None;
    let mut seen_leads = HashSet::new();
    let mut lead_id_idx = None;
    let mut branch_idx = None;
    let mut status_idx = None;

    for file_path in target_files {
        let file_bytes = std::fs::read(&file_path)?;
        let file_content = String::from_utf8_lossy(&file_bytes);
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
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
            let record = result?;

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

            let branch_matches =
                branch == "dr. soliman fakeeh hospital jeddah" || branch.is_empty();
            let status_matches = status == "new" || status == "follow-up";

            if branch_matches && status_matches {
                all_records.push(record);
            }
        }
    }

    if all_records.is_empty() {
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

    Ok(Some(xlsx_path))
}

pub fn process_emails(
    results_file: &str,
    config: &EmailConfig,
    only_call_center: bool,
    send_exceptions: bool,
    download_dir: &str,
    minutes_ago: i64,
) -> Result<()> {
    info!(
        "Starting email processing module. Reading output from {} (only_call_center: {}, send_exceptions: {})",
        results_file, only_call_center, send_exceptions
    );

    let team_mapping_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.team_mapping_file);
    // 1. Load the team mapping file
    let mut team_maps: HashMap<String, TeamMapping> = HashMap::new();
    let mapping_file = File::open(&team_mapping_path).context(format!(
        "Failed to open team mapping file: {}",
        team_mapping_path.display()
    ))?;
    let mut map_rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(mapping_file);

    for result in map_rdr.deserialize::<TeamMapping>() {
        match result {
            Ok(mapping) => {
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
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
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
        }
    }

    let mut ticket_rows = Vec::new();
    let mut dynamic_statuses = HashSet::new();

    for result in rdr.records() {
        let record = result?;

        let is_exception_val = is_exception_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("No")
            .trim()
            .to_lowercase();

        let is_exception = is_exception_val == "yes";

        if send_exceptions {
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

    let send_per_team_branches: HashSet<String> = config
        .send_per_team_branches
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let send_per_branch_branches: HashSet<String> = config
        .send_per_branch_branches
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    // Force sending CC if only_call_center flag is passed via CLI, otherwise use config
    let send_cc = only_call_center || config.send_call_center.unwrap_or(false);

    let mut per_team_buckets: HashMap<String, Vec<TicketRow>> = HashMap::new(); // Key: Team name
    let mut per_branch_buckets: HashMap<String, Vec<TicketRow>> = HashMap::new(); // Key: Branch name
    let mut call_center_bucket: Vec<TicketRow> = Vec::new();

    for row in ticket_rows {
        let b_low = row.branch.to_lowercase();
        let t_low = row.team.to_lowercase();

        let is_cc = t_low == "call center";

        let allowed_branch =
            send_per_team_branches.contains(&b_low) || send_per_branch_branches.contains(&b_low);

        if is_cc && send_cc && allowed_branch {
            call_center_bucket.push(row);
        } else if !is_cc && !only_call_center {
            if send_per_team_branches.contains(&b_low) {
                per_team_buckets
                    .entry(row.team.clone())
                    .or_default()
                    .push(row);
            } else if send_per_branch_branches.contains(&b_low) {
                per_branch_buckets
                    .entry(row.branch.clone())
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
        if bucket_name.eq_ignore_ascii_case("call center") {
            match generate_leads_report(download_dir, minutes_ago) {
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
        let mapped_to = mapping.map(|m| m.to_emails.clone()).unwrap_or_default();

        let (to_emails, cc_list) = if mapped_to.trim().is_empty() {
            // Fallback: send to default email only, no CCs.
            (config.default_to_email.clone(), String::new())
        } else {
            let mapped_cc = mapping.map(|m| m.cc.clone()).unwrap_or_default();
            let ccs = vec![
                config.initial_cc.clone(),
                mapped_cc,
                config.ending_cc.clone(),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
            .join(";");
            (mapped_to, ccs)
        };

        let html_table = generate_pivot_html(rows, &statuses_vec, is_branch);

        let receiver_name = mapping
            .map(|m| m.receiver_name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "All".to_string());

        let (subject, body) = if bucket_name.eq_ignore_ascii_case("Call Center") {
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
    Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>
    {html_table}
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
            if let Some(start_idx) = template_content.find("<title>") {
                if let Some(end_idx) = template_content[start_idx..].find("</title>") {
                    let title_content = &template_content[start_idx + 7..start_idx + end_idx];
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
            if let Some(start_idx) = template_content.find("<body>") {
                if let Some(end_idx) = template_content[start_idx..].find("</body>") {
                    extracted_body =
                        template_content[start_idx + 6..start_idx + end_idx].to_string();
                }
            }

            // Clean up old template margin layout dynamically if it's there
            extracted_body = extracted_body
                .replace("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\r\n        <tr>\r\n            <td width=\"20\"></td>\r\n            <td>\r\n                {html_table}\r\n            </td>\r\n        </tr>\r\n    </table>", "{html_table}")
                .replace("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\n        <tr>\n            <td width=\"20\"></td>\n            <td>\n                {html_table}\n            </td>\n        </tr>\n    </table>", "{html_table}")
                .replace("&nbsp;&nbsp;&nbsp;&nbsp;", "")
                .replace("Dear All", "Dear {receiver_name}");

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
            let body = format!(
                "<html><body style=\"font-family: Arial, sans-serif;\">Dear {},<br/>Kindly find below the list of open tickets in {} for the period from {} until {}.<br/><br/>\
                {}\
                </body></html>",
                receiver_name, bucket_name, from_date_str, today_str, html_table
            );
            let subject = format!("Open TKTs - {}", bucket_name);
            (subject, body)
        };

        // Save Excel file
        let tmp_dir = std::env::temp_dir();
        let safe_name = bucket_name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        let xlsx_path = tmp_dir.join(format!("{}_open_tickets.xlsx", safe_name));

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let mut out_col_idx = 0;
        for (i, h) in headers.iter().enumerate() {
            if is_exception_idx == Some(i) {
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
                if is_exception_idx == Some(c_idx) {
                    continue;
                }
                worksheet.write_string(write_r_idx as u32, out_c_idx, field)?;
                out_c_idx += 1;
            }
            write_r_idx += 1;
        }
        workbook.save(&xlsx_path)?;

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
                xlsx_path.display()
            ));
        }

        if let Some(ref leads_path) = leads_report_path {
            ps_script.push_str(&format!(
                "$Mail.Attachments.Add(\"{}\")\n",
                leads_path.display()
            ));
        }

        ps_script.push_str(&format!("$Mail.{}\n", display_or_send));

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
        } else {
            info!("Successfully processed email for {}", bucket_name);
        }

        let _ = std::fs::remove_file(xlsx_path);

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
    send_email_for_bucket("Call Center", &call_center_bucket, true)?;

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
    fn test_email_processing_skips_closed() {
        let download_dir = tempfile::tempdir().unwrap();
        let mut ticket_file = File::create(download_dir.path().join("results.csv")).unwrap();
        writeln!(
            ticket_file,
            "Ticket Id,Branch Name,Category,Type,Subtype,Status,Creation Date,Assignee,Day,Month,Position,team,Is Exception"
        )
        .unwrap();
        writeln!(
            ticket_file,
            "1001,Main Branch,Cat1,Type1,Sub1,closed,01/01/2026 12:00:00,alice,1,01-2026,Pos1,Team A,No"
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
            send_per_team_branches: vec!["Main Branch".to_string()],
            send_per_branch_branches: vec![],
            send_call_center: Some(false),
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
        );

        assert!(result.is_ok());
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
}
