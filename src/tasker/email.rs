use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use csv::ReaderBuilder;
use rust_xlsxwriter::Workbook;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use tracing::{error, info};

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
    for s in statuses {
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

        for st in statuses {
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
    for st in statuses {
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

pub fn process_emails(
    results_file: &str,
    config: &EmailConfig,
    only_call_center: bool,
    send_exceptions: bool,
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
        if rows.is_empty() {
            return Ok(());
        }

        // Check if all tickets in the bucket are closed
        let all_closed = rows.iter().all(|r| r.status.eq_ignore_ascii_case("closed"));
        if all_closed {
            info!(
                "Skipping email for {} because all tickets are closed.",
                raw_bucket_name
            );
            return Ok(());
        }

        let bucket_name_cleaned = raw_bucket_name.replace('\u{FFFD}', "").replace("ï¿½", "");
        let bucket_name = bucket_name_cleaned.as_str();

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
        let to_emails = mapping
            .map(|m| m.to_emails.clone())
            .unwrap_or_else(|| config.default_to_email.clone());
        let mapped_cc = mapping.map(|m| m.cc.clone()).unwrap_or_default();
        let cc_list = vec![
            config.initial_cc.clone(),
            mapped_cc,
            config.ending_cc.clone(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(";");

        let html_table = generate_pivot_html(rows, &statuses_vec, is_branch);

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
    Dear All,<br/>
    &nbsp;&nbsp;&nbsp;&nbsp;Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>
    <table border="0" cellpadding="0" cellspacing="0">
        <tr>
            <td width="20"></td>
            <td>
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

            // Replace placeholders
            let final_body = extracted_body
                .replace("{bucket_name}", bucket_name)
                .replace("{from_date_str}", &from_date_str)
                .replace("{today_str}", &today_str)
                .replace("{html_table}", &html_table);

            let wrapped_body = format!("<html><body>{}</body></html>", final_body);
            (extracted_subject, wrapped_body)
        } else {
            let body = format!(
                "<html><body style=\"font-family: Arial, sans-serif;\">Dear All,<br/>&nbsp;&nbsp;&nbsp;&nbsp;Kindly find below the list of open tickets in {} for the period from {} until {}.<br/><br/>\
                <table border=\"0\" cellpadding=\"0\" cellspacing=\"0\"><tr><td width=\"20\"></td><td>\
                {}\
                </td></tr></table>\
                </body></html>",
                bucket_name, from_date_str, today_str, html_table
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

        let ps_script = format!(
            r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "{}"
$Mail.HTMLBody = '{}'
$Mail.Attachments.Add("{}")
$Mail.{}
"#,
            to_emails,
            cc_list,
            subject.replace("\"", "'"), // basic sanitize for powershell string interpolation
            body.replace("'", "''"),
            xlsx_path.display(),
            display_or_send
        );

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
    if !call_center_bucket.is_empty() {
        send_email_for_bucket("Call Center", &call_center_bucket, true)?;
    }

    info!("Email processing complete.");
    Ok(())
}
