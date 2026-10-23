use crate::tasker::config::DashboardUpdaterConfig;
use anyhow::Result;
use tracing::{error, info};

pub fn run(config: &DashboardUpdaterConfig) -> Result<()> {
    info!("Starting DashboardUpdater task. Config: {:?}", config);

    let abs_dashboard_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_file);
    let dashboard_path_str = abs_dashboard_path.to_string_lossy().to_string();

    info!("Updating dashboard '{}'", dashboard_path_str);

    let ps_script = include_str!("scripts/dashboard_updater.ps1");

    // Only run PowerShell if not a dry run test
    if !config.save_email_as_html.unwrap_or(false) {
        let ps_result = crate::tasker::script_manager::ScriptManager::new().run_script_with_args(
            "Dashboard Updater",
            "dashboard_updater.ps1",
            ps_script,
            &[("-DashboardPath", &dashboard_path_str)],
        );

        if let Err(e) = ps_result {
            error!("Error executing dashboard update PowerShell script: {}", e);
            anyhow::bail!(e);
        }
    }

    info!("Successfully updated dashboard '{}'", dashboard_path_str);
    info!("Dashboard update completed");

    if let Some(email_to) = &config.email_to {
        let email_cc = config.email_cc.as_deref().unwrap_or("");
        info!("Sending dashboard via email to: {}", email_to);

        info!("Email generation started");
        let indentation_width =
            crate::tasker::email::html::indentation_width(config.indentation_spaces);
        let html_body = format!(
            r#"<html><body style="font-family: Arial, sans-serif;">Dear Aya,<br/><div style="margin-left:{indentation_width}px;">Please find the CRM Ticket dashboard attached.</div></body></html>"#
        );
        info!("Email generation completed");

        if config.save_email_as_html.unwrap_or(false) {
            let payloads_dir = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("logs")
                .join("email_payloads");
            std::fs::create_dir_all(&payloads_dir)?;

            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%f");
            let html_path =
                payloads_dir.join(format!("email_body_dashboard_updater_{}.html", timestamp));
            std::fs::write(&html_path, &html_body)?;
            info!("save_email_as_html is true. Saved email body to {}. Skipping PowerShell send for testing.", html_path.display());
            return Ok(());
        }

        if let Err(e) = crate::tasker::email::outlook::send_email(
            email_to,
            email_cc,
            "CRM Tickets Dashboard",
            &html_body,
            Some(&dashboard_path_str),
            None,
            "Send()",
        ) {
            error!("Failed to send dashboard email: {}", e);
            return Err(e);
        } else {
            info!("Successfully sent dashboard email.");
            info!("Email sent");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasker::config::DashboardUpdaterConfig;

    use tempfile::NamedTempFile;

    #[test]
    fn test_task2_dashboard_updater_calculation_mode() {
        let src = include_str!("scripts/dashboard_updater.ps1");
        let open_idx = src
            .find("$workbook = $workbooks.Open(")
            .expect("Should find open workbook");
        let calc_idx = src
            .find("$excel.Calculation = -4135")
            .expect("Should find calculation");
        assert!(
            open_idx < calc_idx,
            "Calculation mode must be set after opening the workbook to avoid COM exceptions"
        );
    }

    pub(crate) struct TestDataset {
        pub users_file: tempfile::NamedTempFile,
        pub assignments_file: tempfile::NamedTempFile,
        pub download_dir: tempfile::TempDir,
        pub output_file: tempfile::NamedTempFile,
        #[allow(dead_code)]
        pub leads_file: tempfile::NamedTempFile,
        #[allow(dead_code)]
        pub teams_file: tempfile::NamedTempFile,
        pub config_json: String,
    }

    pub(crate) fn setup_test_dataset() -> TestDataset {
        // Synthetic data keeps automated tests independent of local operational exports.
        let users_file = tempfile::NamedTempFile::new().unwrap();
        let assignments_file = tempfile::NamedTempFile::new().unwrap();
        let download_dir = tempfile::tempdir().unwrap();
        let output_file = tempfile::NamedTempFile::new().unwrap();
        let leads_file = tempfile::NamedTempFile::new().unwrap();
        let teams_file = tempfile::NamedTempFile::new().unwrap();

        std::fs::write(
            users_file.path(),
            "UserDepartmentName / Team Name,cognito_username\nSynthetic Team,synthetic.agent@example.test\n",
        ).unwrap();
        std::fs::write(
            assignments_file.path(),
            "type,subtype,Category,Auto agent/team assignment\nSynthetic Type,Synthetic Subtype,Synthetic Category,Synthetic Team\n",
        ).unwrap();

        let ticket_csv = concat!(
            "Ticket Id,Assignee,Ticket Type,Ticket Sub-Type,Ticket Category,Branch,Created At\n",
            "SYNTHETIC-001,synthetic.agent@example.test,Synthetic Type,Synthetic Subtype,Synthetic Category,Synthetic Branch,16-Apr-2026 10:00 AM\n"
        );
        for filename in [
            "ticket_report_synthetic_1.csv",
            "ticket_report_synthetic_2.csv",
            "ticket_report_synthetic_3.csv",
        ] {
            std::fs::write(download_dir.path().join(filename), ticket_csv).unwrap();
        }
        std::fs::write(
            leads_file.path(),
            "Lead Id,Contact Name,Branch,Status,Created At\nSYNTHETIC-LEAD-001,Example Contact,Synthetic Branch,new,16-Apr-2026\n",
        ).unwrap();
        std::fs::copy(
            leads_file.path(),
            download_dir.path().join("lead_report_synthetic.csv"),
        )
        .unwrap();

        let config_json = r#"{
            "tasks": [{
                "type": "csv_analysis",
                "download_path": "unused",
                "users_file": "unused",
                "assignment_settings_file": "unused",
                "minutes_ago": 60,
                "start_date": "15-Apr-2026",
                "exclude_branches": [],
                "exclude_categories": [],
                "output_file": "unused"
            }]
        }"#
        .to_string();
        {
            let mut teams_wtr = csv::Writer::from_writer(teams_file.as_file());
            teams_wtr
                .write_record(["Team Name", "Receiver Name", "To Emails", "CC"])
                .unwrap();
            teams_wtr
                .write_record([
                    "Synthetic Team",
                    "Synthetic Team",
                    "synthetic-team@example.test",
                    "",
                ])
                .unwrap();
            teams_wtr.flush().unwrap();
        }

        TestDataset {
            users_file,
            assignments_file,
            download_dir,
            output_file,
            leads_file,
            teams_file,
            config_json,
        }
    }

    #[test]
    fn test_task2_dashboard_updater() {
        let dataset = setup_test_dataset();
        let config: crate::tasker::config::TaskerConfig =
            serde_json::from_str(&dataset.config_json).unwrap();

        // Use the existing csv analysis config to build a dashboard updater config
        let _csv_config = match config.tasks.first().unwrap() {
            crate::tasker::config::TaskConfig::CsvAnalysis(c) => c.clone(),
            _ => panic!("Expected CsvAnalysis task"),
        };

        // Ensure start date doesn't filter out everything (tickets are in April 2026)
        // But let's set a start date to test filtering
        let start_date = "15-Apr-2026".to_string();

        let dummy_dashboard = NamedTempFile::new().unwrap();

        let dash_config = DashboardUpdaterConfig {
            download_path: dataset.download_dir.path().to_str().unwrap().to_string(),
            users_file: dataset.users_file.path().to_str().unwrap().to_string(),
            assignment_settings_file: dataset
                .assignments_file
                .path()
                .to_str()
                .unwrap()
                .to_string(),
            minutes_ago: 10000000,
            start_date: Some(start_date),
            exclude_branches: vec!["Jeddah".to_string(), "Invalid Branch".to_string()],
            exclude_categories: vec!["Internal".to_string(), "IT".to_string()],
            category_exceptions: None, /* Some(vec![CategoryException {
                                           category: "Internal".to_string(),
                                           allowed_call_type: "Patient Complain".to_string(),
                                       }]) */
            output_file: dataset.output_file.path().to_str().unwrap().to_string(),
            dashboard_file: dummy_dashboard.path().to_str().unwrap().to_string(),
            email_to: Some("test@example.com".to_string()),
            email_cc: Some("".to_string()),
            save_email_as_html: Some(true),
            indentation_spaces: Some(4),
        };

        // Get the latest file in the logs/email_payloads dir before running
        let payloads_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("logs")
            .join("email_payloads");
        std::fs::create_dir_all(&payloads_dir).unwrap();

        // Clean out directory to avoid getting old files
        let _ = std::fs::remove_dir_all(&payloads_dir);
        std::fs::create_dir_all(&payloads_dir).unwrap();
        let res = run(&dash_config);

        assert!(res.is_ok());

        // Verify HTML email was generated
        let mut entries: Vec<_> = std::fs::read_dir(&payloads_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.metadata().unwrap().modified().unwrap());
        let latest = entries
            .last()
            .expect("Should have generated an HTML file in email_payloads");
        let html_content = std::fs::read_to_string(latest.path()).unwrap();

        assert!(
            html_content.contains("margin-left:20px;"),
            "HTML email should contain the configured indentation. Found: {}",
            html_content
        );
    }
}
