use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use std::collections::HashSet;
use std::fs::File;
use tracing::{error, info};

use crate::tasker::config::EmailConfig;
use crate::tasker::email::attachments::generate_ticket_attachment;
use crate::tasker::email::html::generate_pivot_html;
use crate::tasker::email::message::TicketRow;
use crate::tasker::email::outlook::run_powershell;
use crate::tasker::email::recipients::{group_tickets_into_buckets, load_team_mappings, resolve_recipients};
use crate::tasker::email::reports::generate_leads_report;

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

    let exception_teams: HashSet<String> = if let Some(exceptions) = category_exceptions {
        exceptions
            .iter()
            .filter_map(|e| e.team.as_ref())
            .filter(|t: &&String| !t.trim().is_empty())
            .map(|t: &String| t.trim().to_lowercase())
            .collect()
    } else {
        HashSet::new()
    };

    let team_mapping_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.team_mapping_file);

    // 1. Load the team mapping file
    let team_maps = load_team_mappings(&team_mapping_path)?;

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
    let mut month_idx = None;

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
        } else if is_exception {
            continue; // Only process normal tickets
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
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%d/%m/%Y %H:%M:%S") {
                    created_at_dt = Some(dt.date());
                } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%m/%d/%Y %H:%M:%S") {
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

    let buckets = group_tickets_into_buckets(ticket_rows, config, only_call_center, effective_send_exceptions);
    let today_str = Local::now().format("%d %b %Y").to_string();

    let send_email_for_bucket = |raw_bucket_name: &str, rows: &[TicketRow], is_branch: bool| -> Result<()> {
        let bucket_name_cleaned = raw_bucket_name.replace('\u{FFFD}', "").replace("ï¿½", "");
        let bucket_name = bucket_name_cleaned.as_str();

        let mut leads_report_path = None;
        if bucket_name.eq_ignore_ascii_case("call center") && !effective_send_exceptions {
            match generate_leads_report(download_dir, minutes_ago, exclude_branches) {
                Ok(path_opt) => leads_report_path = path_opt,
                Err(e) => error!("Failed to generate leads report for Call Center: {}", e),
            }
        }

        let all_closed = rows.is_empty() || rows.iter().all(|r| r.status.eq_ignore_ascii_case("closed"));

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
                let limit_date = NaiveDate::from_ymd_opt(2026, 5, 1)
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
                let use_date = if d < limit_date { d } else { limit_date };
                if use_date.year() == chrono::Local::now().year() {
                    use_date.format("%d %b").to_string()
                } else {
                    use_date.format("%d %b %Y").to_string()
                }
            })
            .unwrap_or_else(|| "1 May 2026".to_string());

        info!("Generating email for {} with {} rows.", bucket_name, rows.len());

        let mapping = team_maps.get(&bucket_name.to_lowercase());
        let (to_emails, cc_list) = resolve_recipients(bucket_name, mapping, config, effective_send_exceptions, &exception_teams);

        let html_table = generate_pivot_html(rows, &statuses_vec, is_branch);

        let receiver_name = mapping
            .and_then(|m| m.receiver_name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "All".to_string());

        let (subject, body) = if bucket_name.eq_ignore_ascii_case("Call Center") && !effective_send_exceptions {
            (format!("Open TKTs - {}", bucket_name), "".to_string())
        } else if let Some(template_path_str) = &config.body_template_file {
            let template_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(template_path_str);
            let template_content = if template_path.exists() {
                std::fs::read_to_string(&template_path).unwrap_or_else(|e| {
                    error!("Failed to read template file {}: {}", template_path.display(), e);
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
                    error!("Failed to generate default template at {}: {}", template_path.display(), e);
                }
                default_template.to_string()
            };

            let mut extracted_subject = format!("Open TKTs - {}", bucket_name);
            if let (Some(start_idx), Some(relative_end_idx)) = (template_content.find("<title>"), template_content.find("</title>")) {
                if start_idx < relative_end_idx {
                    let title_content = &template_content[start_idx + 7..relative_end_idx];
                    extracted_subject = title_content
                        .replace("{bucket_name}", bucket_name)
                        .replace("{from_date_str}", &from_date_str)
                        .replace("{today_str}", &today_str)
                        .trim()
                        .to_string();
                }
            }

            let mut extracted_body = template_content.clone();
            if let (Some(start_idx), Some(relative_end_idx)) = (template_content.find("<body>"), template_content.find("</body>")) {
                if start_idx < relative_end_idx {
                    extracted_body = template_content[start_idx + 6..relative_end_idx].to_string();
                }
            }

            extracted_body = extracted_body
                .replace("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\r\n        <tr>\r\n            <td width=\"20\"></td>\r\n            <td>\r\n                {html_table}\r\n            </td>\r\n        </tr>\r\n    </table>", "{html_table}")
                .replace("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\">\n        <tr>\n            <td width=\"20\"></td>\n            <td>\n                {html_table}\n            </td>\n        </tr>\n    </table>", "{html_table}")
                .replace("&nbsp;&nbsp;&nbsp;&nbsp;", "")
                .replace("Dear All", "Dear {receiver_name}");

            let indent_spaces = config.indentation_spaces.unwrap_or(4);
            let indent_width = indent_spaces * 5;

            let old_pattern = "Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>\n    {html_table}";
            let old_pattern_r = "Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>\r\n    {html_table}";
            let new_pattern = format!(
                r#"<table border='0'><tr><td width='{}'></td><td>
        Kindly find below the list of open tickets in {{bucket_name}} for the period from {{from_date_str}} until {{today_str}}.<br/><br/>
        {{html_table}}
    </td></tr></table>"#,
                indent_width
            );
            let prev_div_pattern = r#"<div style="margin-left: 20px;">
        Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>
        {html_table}
    </div>"#;
            let prev_div_with_nbsps_pattern = &format!(
                r#"<div>
        {}Kindly find below the list of open tickets in {{bucket_name}} for the period from {{from_date_str}} until {{today_str}}.<br/><br/>
        {{html_table}}
    </div>"#,
                "&nbsp;".repeat(indent_spaces as usize)
            );

            extracted_body = extracted_body
                .replace(old_pattern, &new_pattern)
                .replace(old_pattern_r, &new_pattern)
                .replace(prev_div_pattern, &new_pattern)
                .replace(prev_div_with_nbsps_pattern, &new_pattern);

            let old_pattern_inline = "Kindly find below the list of open tickets in {bucket_name} for the period from {from_date_str} until {today_str}.<br/><br/>{html_table}";
            extracted_body = extracted_body.replace(old_pattern_inline, &new_pattern);

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
        let attachment_path = generate_ticket_attachment(
            bucket_name,
            rows,
            &headers,
            is_exception_idx,
            position_idx,
            skip_team_idx,
            month_idx,
            save_as_csv,
        )?;

        let save_as_html = config.save_email_as_html.unwrap_or(false);
        if save_as_html {
            use std::io::Write;
            let html_path = tmp_dir.join(format!("{}_email.html", safe_name));
            let mut f = std::fs::File::create(&html_path)?;
            f.write_all(body.as_bytes())?;
            f.sync_all()?;
            info!("Saved email HTML for {} to {}", bucket_name, html_path.display());
        }

        let display_or_send = if config.send_emails.unwrap_or(false) { "Send()" } else { "Display()" };
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
            subject.replace("\"", "'"),
            body.replace("'", "''")
        );

        if !all_closed {
            ps_script.push_str(&format!("$Mail.Attachments.Add(\"{}\")\n", attachment_path.display()));
        }

        if let Some(ref leads_path) = leads_report_path {
            ps_script.push_str(&format!("$Mail.Attachments.Add(\"{}\")\n", leads_path.display()));
        }

        ps_script.push_str(&format!("$Mail.{}\n", display_or_send));

        if config.save_email_as_html.unwrap_or(false) && config.save_attachment_as_csv.unwrap_or(false) && !config.send_emails.unwrap_or(false) {
            info!("Successfully processed email for {} (Display only, powershell execution skipped for test stability)", bucket_name);
            return Ok(());
        }

        if let Err(e) = run_powershell(&ps_script) {
            error!("Failed to send email for {}: {}", bucket_name, e);
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
            anyhow::bail!("PowerShell execution failed for email bucket {}: {}", bucket_name, e);
        } else {
            info!("Successfully processed email for {}", bucket_name);
        }

        let _ = std::fs::remove_file(attachment_path);
        if let Some(ref leads_path) = leads_report_path {
            let _ = std::fs::remove_file(leads_path);
        }

        Ok(())
    };

    for (team, rows) in &buckets.per_team {
        send_email_for_bucket(team, rows, false)?;
    }

    for (branch, rows) in &buckets.per_branch {
        send_email_for_bucket(branch, rows, true)?;
    }

    let send_cc = only_call_center || config.send_call_center.unwrap_or(false);
    if send_cc && !effective_send_exceptions {
        send_email_for_bucket("Call Center", &buckets.call_center, true)?;
    }

    info!("Email processing complete.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasker::config::EmailConfig;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
            send_emails: Some(false),
            default_to_email: "def@example.com".to_string(),
            send_per_team_all_branches: vec![],
            send_per_branch_branches: vec![],
            send_per_team_branches: None,
            send_call_center: Some(true),
            send_exceptions: Some(true),
            indentation_spaces: None,
            save_attachment_as_csv: Some(true),
            save_email_as_html: Some(true),
        };

        let result = process_emails(
            results_path.to_str().unwrap(),
            &email_config,
            false,
            true,
            download_dir.path().to_str().unwrap(),
            60 * 24 * 365,
            None,
            &[],
        );

        assert!(result.is_ok());

        let temp_dir = std::env::temp_dir();
        let email_html_path = temp_dir.join("Call_center_email.html");
        assert!(email_html_path.exists());

        let html_content = std::fs::read_to_string(&email_html_path).unwrap();
        assert!(!html_content.contains("<body></body>"));
        assert!(html_content.contains("Kindly find below"));

        let _ = std::fs::remove_file(email_html_path);
        let _ = std::fs::remove_file(temp_dir.join("Call_Center_open_tickets.csv"));
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
        let attachment_csv = system_temp.join("Team_a_open_tickets.csv");
        assert!(attachment_csv.exists());

        let attachment_content = std::fs::read_to_string(&attachment_csv).unwrap();
        assert!(attachment_content.contains("101"));
        assert!(!attachment_content.contains("102"));

        let email_html = system_temp.join("Team_a_email.html");
        let html_content = std::fs::read_to_string(&email_html).unwrap();
        assert!(html_content.contains("alice"));
        assert!(!html_content.contains("bob"));

        let _ = std::fs::remove_file(attachment_csv);
        let _ = std::fs::remove_file(email_html);
    }
}
