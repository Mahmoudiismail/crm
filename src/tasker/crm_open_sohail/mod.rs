use crate::tasker::config::CrmOpenSohailConfig;
use anyhow::Result;
use tracing::{error, info, warn};

pub mod models;
pub mod powershell;
pub mod processing;
pub mod reports;

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
    let extracted_data = powershell::extract_data(config)?;

    // Step 5: Process Data & Enrich OUL Column
    let final_datasets = processing::process_extracted_data(config, extracted_data)?;

    // Step 6: Generate HTML Email
    info!("Email generation started");
    info!(
        "Generating HTML email layout from {} datasets",
        final_datasets.len()
    );

    let final_html = reports::generate_html_report(config, &final_datasets);

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
    use super::*;
    use crate::tasker::config::DashboardUpdaterConfig;

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
