use crate::tasker::config::CrmOpenSohailConfig;
use crate::tasker::utils::with_retry;
use anyhow::Result;
use tracing::{error, info};

/// Calculates the expected Subject format: "Open TKTs DD-MMMM" based on yesterday's local date.
pub fn calculate_yesterday_subject(today: chrono::NaiveDate) -> String {
    let yesterday = today - chrono::Duration::days(1);
    format!("Open TKTs {}", yesterday.format("%d-%B"))
}

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
    with_retry(|| crate::tasker::dashboard_updater::run(&dash_config))?;
    tracing::info!("DashboardUpdater logic completed successfully.");

    // Step 2-4: Extract Pivot Data via Slicers
    let extracted_data = with_retry(|| powershell::extract_data(config))?;

    // Step 5: Process and aggregate the extracted data
    let final_datasets = processing::process_extracted_data(config, extracted_data)?;

    // Step 6: Generate final HTML report structure
    let final_html = reports::generate_html_report(config, &final_datasets);

    info!("Email generation completed");

    let subject = calculate_yesterday_subject(chrono::Local::now().date_naive());

    let sender_account_email = config.sender_account_email.clone();
    let reply_subject_prefix = config.reply_subject_prefix.clone();

    let payloads_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("logs")
        .join("email_payloads");
    std::fs::create_dir_all(&payloads_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%f");
    let html_path = payloads_dir.join(format!("email_body_crm_open_sohail_{}.html", timestamp));
    std::fs::write(&html_path, &final_html)?;

    if config.dashboard_config.save_email_as_html.unwrap_or(false) {
        let tmp_dir = std::env::temp_dir();
        let old_html_path = tmp_dir.join("crm_open_sohail_email.html");
        std::fs::write(&old_html_path, final_html)?;
        info!("save_email_as_html is true. Saved email body to {}. Skipping PowerShell send for testing.", html_path.display());
    } else {
        info!("Sending scheduled ReplyAll through shared Outlook function...");
        if let Err(e) = crate::tasker::email::outlook::reply_all_email(
            &sender_account_email,
            &reply_subject_prefix,
            &subject,
            &final_html,
            "miismail@fakeeh.care",
            false,
        ) {
            error!("Failed to send scheduled ReplyAll: {}", e);
            anyhow::bail!("Failed to send scheduled ReplyAll: {e}");
        }
        info!("ReplyAll sent/scheduled successfully.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_calculate_yesterday_subject_single_digit_day() {
        let today = NaiveDate::from_ymd_opt(2023, 11, 2).unwrap();
        assert_eq!(calculate_yesterday_subject(today), "Open TKTs 01-November");
    }

    #[test]
    fn test_calculate_yesterday_subject_month_boundary() {
        let today = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();
        assert_eq!(calculate_yesterday_subject(today), "Open TKTs 31-October");
    }

    #[test]
    fn test_reply_all_uses_shared_outlook_function() {
        let src = include_str!("../email/outlook_mail.ps1");
        assert!(src.contains("function Send-OutlookMail"));
        assert!(src.contains("Find-OutlookOriginalMessage"));
        assert!(src.contains("ReplyAll"));
        assert!(src.contains("Get-OutlookHtmlFragment"));
        assert!(src.contains("SendAtNextSevenAm"));
        assert!(src.contains("AdditionalTo"));
        assert!(src.contains("RemoveCc"));
        assert!(include_str!("mod.rs").contains("reply_all_email("));
    }
}
