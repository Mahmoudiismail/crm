use crate::tasker::config::CrmOpenSohailConfig;
use anyhow::Result;
use tracing::{error, info, warn};

pub mod models;
pub mod powershell;

#[derive(Debug, Clone)]
struct TeamMappingInfo {
    owner_name: String,
    owner_email: String,
    is_shared: bool,
}

pub fn run(config: &CrmOpenSohailConfig) -> Result<()> {
    tracing::info!("Starting CRM Open Sohail task");

    // Step 1: Run dashboard updater
    tracing::info!("Executing DashboardUpdater logic as part of CrmOpenSohail task.");
    let mut dash_config = config.dashboard_config.clone();
    dash_config.email_to = None;
    dash_config.email_cc = None;
    crate::tasker::dashboard_updater::run(&dash_config)?;
    tracing::info!("DashboardUpdater logic completed successfully.");

    // Step 2-4: Extract Pivot Data via Slicers
    let mut extracted_data = powershell::extract_data(config)?;

    let current_month_dt = chrono::Local::now().format("%b-%Y").to_string();

    // Calculate accurate date range per branch from CSV
    let mut branch_date_ranges: std::collections::HashMap<
        String,
        (Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>),
    > = std::collections::HashMap::new();

    let output_file_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_config.output_file);
    if let Ok(mut output_rdr) = csv::ReaderBuilder::new().from_path(&output_file_path) {
        if let Ok(headers) = output_rdr.headers() {
            let created_at_idx = headers
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case("Created At"));
            let branch_idx = headers
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case("Branch"));

            if let (Some(c_idx), Some(b_idx)) = (created_at_idx, branch_idx) {
                for record in output_rdr.records().flatten() {
                    if let (Some(created_val), Some(branch_val)) =
                        (record.get(c_idx), record.get(b_idx))
                    {
                        if let Some(dt) = crate::tasker::csv_task::parse_created_at(created_val) {
                            // Only include this date if it is not in the current month
                            if dt.format("%b-%Y").to_string() != current_month_dt {
                                let branch = branch_val.trim().to_string();
                                let entry =
                                    branch_date_ranges.entry(branch).or_insert((None, None));
                                if entry.0.is_none_or(|m| dt < m) {
                                    entry.0 = Some(dt);
                                }
                                if entry.1.is_none_or(|m| dt > m) {
                                    entry.1 = Some(dt);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Apply the accurate CSV date range per branch
    for dataset in &mut extracted_data {
        // Any branch with a combined date range (starting with "from (") should be overridden
        // except if it happens to be explicitly the current month dt.
        if dataset.month.contains("from (") && dataset.month != current_month_dt {
            let mut matching_branch_range = None;
            // The CSV might contain variations of the branch name. Let's do a case-insensitive check.
            for (csv_branch, (min_date, max_date)) in &branch_date_ranges {
                if csv_branch.eq_ignore_ascii_case(dataset.branch.trim()) {
                    matching_branch_range = Some((*min_date, *max_date));
                    break;
                }
            }

            if let Some((Some(min), Some(max))) = matching_branch_range {
                let min_month = min.format("%b").to_string();
                let max_month = max.format("%b-%Y").to_string();

                let computed_month_range =
                    if min.format("%b-%Y").to_string() == max.format("%b-%Y").to_string() {
                        min.format("%b-%Y").to_string()
                    } else {
                        format!("{} to {}", min_month, max_month)
                    };
                dataset.month = computed_month_range;
            }
        }
    }

    // Step 5: Process Data & Enrich OUL Column
    let team_mapping_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.team_mapping_file);
    if !team_mapping_path.exists() {
        error!(
            "Team mapping file not found at: {}",
            team_mapping_path.display()
        );
        anyhow::bail!("Team mapping file not found");
    }

    let mut team_to_info = std::collections::HashMap::new();

    let file = std::fs::File::open(&team_mapping_path)?;
    let mut rdr = crate::utils::build_csv_reader_from_reader(file);

    // We expect columns like "Team Name", "Owner" (or Receiver Name), "To Emails"
    // We strictly prefer "owner_name" and "owner_email" over legacy "receiver_name" and "to_emails".

    let headers = rdr.headers()?.clone();
    let mut team_idx = None;
    let mut owner_name_idx = None;
    let mut receiver_name_idx = None;
    let mut owner_email_idx = None;
    let mut to_emails_idx = None;
    let mut is_shared_idx = None;

    for (i, h) in headers.iter().enumerate() {
        let h_lower = h.trim().to_lowercase();
        if (h_lower == "team name" || h_lower == "team" || h_lower.contains("team"))
            && team_idx.is_none()
        {
            team_idx = Some(i);
        } else if h_lower == "owner_name" || h_lower == "owner" || h_lower == "oul" {
            owner_name_idx = Some(i);
        } else if h_lower == "receiver name"
            || h_lower == "receiver_name"
            || h_lower.contains("receiver")
        {
            receiver_name_idx = Some(i);
        } else if h_lower == "owner_email" || h_lower == "email_to" || h_lower == "owner email" {
            owner_email_idx = Some(i);
        } else if (h_lower == "to emails"
            || h_lower == "to_emails"
            || h_lower == "email"
            || h_lower.contains("email"))
            && owner_email_idx.is_none()
        {
            to_emails_idx = Some(i);
        } else if h_lower == "is_shared"
            || h_lower == "is_shared_across_branches"
            || h_lower == "shared"
            || h_lower == "is_shared_team"
        {
            is_shared_idx = Some(i);
        }
    }

    for record in rdr.records().filter_map(|r| r.ok()) {
        if let Some(t_idx) = team_idx {
            let team_name = record.get(t_idx).unwrap_or("").trim().to_lowercase();
            if team_name.is_empty() {
                continue;
            }

            let owner = owner_name_idx
                .and_then(|idx| record.get(idx))
                .filter(|s| !s.trim().is_empty())
                .or_else(|| receiver_name_idx.and_then(|idx| record.get(idx)))
                .unwrap_or("")
                .trim()
                .to_string();

            let email = owner_email_idx
                .and_then(|idx| record.get(idx))
                .filter(|s| !s.trim().is_empty())
                .or_else(|| to_emails_idx.and_then(|idx| record.get(idx)))
                .unwrap_or("")
                .trim()
                .to_string();

            let is_shared = is_shared_idx
                .and_then(|idx| record.get(idx))
                .map(|s| {
                    let s_low = s.trim().to_lowercase();
                    s_low == "true" || s_low == "1" || s_low == "yes" || s_low == "y"
                })
                .unwrap_or(false);

            team_to_info.insert(
                team_name,
                TeamMappingInfo {
                    owner_name: owner,
                    owner_email: email,
                    is_shared,
                },
            );
        }
    }

    info!(
        "Loaded {} team mappings for OUL enrichment.",
        team_to_info.len()
    );

    let fallback_oul_text = config.fallback_oul.clone().unwrap_or_default();

    #[derive(Debug)]
    struct EnrichedRow {
        team: String,
        closed: f64,
        open: f64,
        perc_closed: String,
        perc_open: String,
        grand_total: f64,
        oul: String,
    }

    #[derive(Debug)]
    struct EnrichedDataset {
        branch: String,
        month: String,
        data: Vec<EnrichedRow>,
    }

    let mut final_datasets: Vec<EnrichedDataset> = Vec::new();

    for mut dataset in extracted_data {
        let mut enriched_rows = Vec::new();

        let is_main_branch = dataset
            .branch
            .trim()
            .eq_ignore_ascii_case("Dr. Soliman Fakeeh Hospital Jeddah")
            || dataset.branch.trim().eq_ignore_ascii_case("DSFH")
            || dataset
                .branch
                .trim()
                .to_lowercase()
                .contains("soliman fakeeh hospital");

        for row in dataset.data.drain(..) {
            let team_lower = row.team.trim().to_lowercase();
            let team_info = team_to_info.get(&team_lower);

            let mut oul = match team_info {
                Some(info) => {
                    if info.is_shared || is_main_branch {
                        match (&info.owner_name, &info.owner_email) {
                            (o, e) if !o.is_empty() && !e.is_empty() => {
                                format!("<a href=\"mailto:{}\">{}</a>", e, o)
                            }
                            (o, _) if !o.is_empty() => o.to_string(),
                            _ => fallback_oul_text.clone(),
                        }
                    } else {
                        fallback_oul_text.clone()
                    }
                }
                None => {
                    warn!("Missing team mapping for: {}", row.team);
                    fallback_oul_text.clone()
                }
            };

            if row.open == 0.0 {
                oul = String::new();
            }

            enriched_rows.push(EnrichedRow {
                team: row.team,
                closed: row.closed,
                open: row.open,
                perc_closed: row.perc_closed,
                perc_open: row.perc_open,
                grand_total: row.grand_total,
                oul,
            });
        }

        if !enriched_rows.is_empty() {
            final_datasets.push(EnrichedDataset {
                branch: dataset.branch,
                month: dataset.month,
                data: enriched_rows,
            });
        }
    }

    // Step 6: Generate HTML Email
    info!("Email generation started");
    info!(
        "Generating HTML email layout from {} datasets",
        final_datasets.len()
    );

    let mut sections_html = String::new();

    for dataset in &final_datasets {
        // Table Title
        let is_executive = dataset
            .branch
            .trim()
            .eq_ignore_ascii_case("executive clinic");
        let title = if is_executive {
            "Executive clinic".to_string()
        } else {
            // dataset.month already is either "Jan-2026" or "Jan to Jul-2026", so we wrap it once
            format!(
                "{} ({})",
                dataset.branch,
                dataset.month.trim_matches(|c| c == '(' || c == ')')
            )
        };

        sections_html.push_str(&format!(
            "<div style=\"font-family: Calibri, sans-serif; font-size: 14px; font-weight: bold; color: #44546A;\">{}</div>",
            title
        ));

        // Start Table
        sections_html.push_str("<table style=\"table-layout: fixed; border-collapse: collapse; font-family: Calibri, sans-serif; font-size: 14px; border: 1px solid #8EA9DB;\">");

        // Header widths from config
        let widths = config.table_column_widths.clone().unwrap_or_else(|| {
            vec![
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
                "auto".to_string(),
            ]
        });

        let mut safe_widths = widths.clone();
        while safe_widths.len() < 7 {
            safe_widths.push("auto".to_string());
        }

        // Header Row (Blue)
        sections_html.push_str(&format!(
            "<tr style=\"background-color: #4472C4; color: white; font-weight: bold; text-align: center; vertical-align: middle;\">
                <th width=\"{w0}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">Team</th>
                <th width=\"{w1}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">closed</th>
                <th width=\"{w2}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">open</th>
                <th width=\"{w3}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">% of closed</th>
                <th width=\"{w4}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">% of open</th>
                <th width=\"{w5}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">Grand Total</th>
                <th width=\"{w6}\" style=\"border: 1px solid #8EA9DB; padding: 5px;\">OUL</th>
            </tr>",
            w0 = safe_widths[0],
            w1 = safe_widths[1],
            w2 = safe_widths[2],
            w3 = safe_widths[3],
            w4 = safe_widths[4],
            w5 = safe_widths[5],
            w6 = safe_widths[6],
        ));

        let mut ds_closed_total = 0.0;
        let mut ds_open_total = 0.0;
        let mut ds_grand_total = 0.0;

        for row in dataset.data.iter() {
            ds_closed_total += row.closed;
            ds_open_total += row.open;
            ds_grand_total += row.grand_total;

            let closed_str = if row.closed == 0.0 {
                String::new()
            } else {
                row.closed.to_string()
            };
            let open_str = if row.open == 0.0 {
                String::new()
            } else {
                row.open.to_string()
            };
            let perc_closed_str = if row.perc_closed == "0%" || row.perc_closed == "0.00%" {
                String::new()
            } else {
                row.perc_closed.clone()
            };
            let perc_open_str = if row.perc_open == "0%" || row.perc_open == "0.00%" {
                String::new()
            } else {
                row.perc_open.clone()
            };
            let grand_total_str = if row.grand_total == 0.0 {
                String::new()
            } else {
                row.grand_total.to_string()
            };

            sections_html.push_str(&format!(
                "<tr style=\"color: black;\">
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                    <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                </tr>",
                row.team, closed_str, open_str, perc_closed_str, perc_open_str, grand_total_str, row.oul
            ));
        }

        // Grand Total row (Red) for each table
        let perc_closed_total = if ds_grand_total > 0.0 {
            format!("{:.2}%", (ds_closed_total / ds_grand_total) * 100.0)
        } else {
            "0.00%".to_string()
        };
        let perc_open_total = if ds_grand_total > 0.0 {
            format!("{:.2}%", (ds_open_total / ds_grand_total) * 100.0)
        } else {
            "0.00%".to_string()
        };

        let total_closed_str = if ds_closed_total == 0.0 {
            String::new()
        } else {
            ds_closed_total.to_string()
        };
        let total_open_str = if ds_open_total == 0.0 {
            String::new()
        } else {
            ds_open_total.to_string()
        };
        let total_perc_closed_str = if perc_closed_total == "0%" || perc_closed_total == "0.00%" {
            String::new()
        } else {
            perc_closed_total
        };
        let total_perc_open_str = if perc_open_total == "0%" || perc_open_total == "0.00%" {
            String::new()
        } else {
            perc_open_total
        };
        let total_grand_str = if ds_grand_total == 0.0 {
            String::new()
        } else {
            ds_grand_total.to_string()
        };

        sections_html.push_str(&format!(
            "<tr style=\"background-color: #C00000; color: white; font-weight: bold;\">
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">Grand Total</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\">{}</td>
                <td style=\"border: 1px solid #8EA9DB; padding: 5px; text-align: center; vertical-align: middle;\"></td>
            </tr>",
            total_closed_str, total_open_str, total_perc_closed_str, total_perc_open_str, total_grand_str
        ));

        sections_html.push_str("</table><br/>");
    }

    let indent_spaces = config.dashboard_config.indentation_spaces.unwrap_or(4);
    let indent_width = indent_spaces * 5;

    let default_template = format!(
        r#"<html>
<body style="font-family: Calibri, Arial, sans-serif;">
    Dear All,<br/>
    <table border='0'><tr><td width='{indent}'></td><td>
    Hope everyone is doing well!<br/>
    Kindly check CRM Updated open TKTs.<br/><br/>
    {{sections}}
    </td></tr></table>
</body>
</html>"#,
        indent = indent_width
    );

    let body_template = if let Some(template_file) = &config.body_template_file {
        let tp = crate::tasker::csv_task::resolve_relative_to_exe_dir(template_file);
        if tp.exists() {
            std::fs::read_to_string(&tp).unwrap_or_else(|_| default_template.to_string())
        } else {
            default_template.to_string()
        }
    } else {
        default_template.to_string()
    };

    let final_html = body_template.replace("{sections}", &sections_html);

    info!("Email generation completed");

    let subject = config
        .subject_template
        .clone()
        .unwrap_or("CRM Updated open TKTs".to_string());

    let email_to = config.dashboard_config.email_to.clone().unwrap_or_default();
    let email_cc = config.dashboard_config.email_cc.clone().unwrap_or_default();

    if email_to.is_empty() {
        warn!("No email_to specified. Skipping email send.");
        return Ok(());
    }

    let ps_email_script = format!(
        r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "{}"
$Mail.HTMLBody = '{}'
$Mail.Send()
"#,
        email_to.replace("\"", "'"),
        email_cc.replace("\"", "'"),
        subject.replace("\"", "''"),
        final_html.replace("'", "''")
    );

    if config.dashboard_config.save_email_as_html.unwrap_or(false) {
        let tmp_dir = std::env::temp_dir();
        let html_path = tmp_dir.join("crm_open_sohail_email.html");
        std::fs::write(&html_path, final_html)?;
        info!("save_email_as_html is true. Saved email body to {}. Skipping PowerShell send for testing.", html_path.display());
    } else {
        info!("Sending email via Outlook COM...");
        if let Err(e) = powershell::run_powershell(&ps_email_script) {
            error!("Failed to send email: {}", e);
            anyhow::bail!("Failed to send email");
        }
        info!("Email sent successfully.");
        info!("Email sent");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_oul_enrichment_rules() {
        let mut temp_mapping = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(temp_mapping, "Team Name,Owner Name,Owner Email,is_shared").unwrap();
        writeln!(
            temp_mapping,
            "Shared Team,Shared Owner,shared@example.com,true"
        )
        .unwrap();
        writeln!(
            temp_mapping,
            "Local Team,Local Owner,local@example.com,false"
        )
        .unwrap();
        writeln!(temp_mapping, "No Email Team,No Email,,true").unwrap();

        let dummy_dataset = crate::tasker::csv_task::tests::setup_test_dataset();

        let config = CrmOpenSohailConfig {
            dashboard_config: DashboardUpdaterConfig {
                download_path: dummy_dataset
                    .download_dir
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                users_file: dummy_dataset
                    .users_file
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                assignment_settings_file: dummy_dataset
                    .assignments_file
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                minutes_ago: 60,
                start_date: None,
                exclude_branches: vec![],
                exclude_categories: vec![],
                category_exceptions: None,
                output_file: dummy_dataset
                    .output_file
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                dashboard_file: temp_mapping.path().to_str().unwrap().to_string(),
                email_to: Some("test@example.com".to_string()),
                email_cc: None,
                save_email_as_html: Some(true),
                indentation_spaces: Some(4),
            },
            team_mapping_file: temp_mapping.path().to_str().unwrap().to_string(),
            body_template_file: None,
            subject_template: Some("Test Subject".to_string()),
            branch_filter: None,
            month_filter: None,
            fallback_oul: Some("".to_string()),
            dashboard_sheet_name: None,
            dashboard_pivot_name: None,
            table_column_widths: None,
        };

        // Running it against dummy data will yield no actual HTML tables of datasets since the mock PowerShell script output is `[]`.
        // We will just execute it to ensure no crashes occur with the new parsing logic.
        let result = run(&config);
        assert!(result.is_ok(), "Task failed: {:?}", result.err());
    }

    use super::*;
    use crate::tasker::config::DashboardUpdaterConfig;

    #[test]
    fn test_email_html_generation_and_team_mapping() {
        // We will mock the extracted data and team mapping and test the end-to-end execution
        // using the test mode flags (save_email_as_html = true)

        let mut temp_mapping = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(temp_mapping, "Team Name,Receiver Name,To Emails,is_shared").unwrap();
        writeln!(temp_mapping, "Team Alpha,Alice,alice@example.com,true").unwrap();
        writeln!(temp_mapping, "Team Beta,Bob,,false").unwrap();

        let dummy_dataset = crate::tasker::csv_task::tests::setup_test_dataset();

        let config = CrmOpenSohailConfig {
            dashboard_config: DashboardUpdaterConfig {
                download_path: dummy_dataset
                    .download_dir
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                users_file: dummy_dataset
                    .users_file
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                assignment_settings_file: dummy_dataset
                    .assignments_file
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                minutes_ago: 60,
                start_date: None,
                exclude_branches: vec![],
                exclude_categories: vec![],
                category_exceptions: None,
                output_file: dummy_dataset
                    .output_file
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string(),
                dashboard_file: temp_mapping.path().to_str().unwrap().to_string(), // use mapping as dummy file so it exists
                email_to: Some("test@example.com".to_string()),
                email_cc: None,
                save_email_as_html: Some(true),
                indentation_spaces: Some(4),
            },
            team_mapping_file: temp_mapping.path().to_str().unwrap().to_string(),
            body_template_file: None,
            subject_template: Some("Test Subject".to_string()),
            branch_filter: None,
            month_filter: None,
            fallback_oul: Some("".to_string()),
            dashboard_sheet_name: None,
            dashboard_pivot_name: None,
            table_column_widths: None,
        };

        // We run the task. Since save_email_as_html is true, PowerShell COM is skipped,
        // and an empty JSON will be created in place of the slicer extraction.
        // It should complete successfully without OS errors.

        let result = run(&config);
        assert!(result.is_ok(), "Task failed: {:?}", result.err());

        // Verify email HTML was generated
        let tmp_dir = std::env::temp_dir();
        let html_path = tmp_dir.join("crm_open_sohail_email.html");
        assert!(html_path.exists());

        let content = std::fs::read_to_string(&html_path).unwrap();
        assert!(content.contains("Dear All,"));
    }
}
