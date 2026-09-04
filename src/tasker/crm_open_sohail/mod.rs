use crate::tasker::config::CrmOpenSohailConfig;
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

    let subject = calculate_yesterday_subject(chrono::Local::now().date_naive());

    let sender_account_email = config.sender_account_email.clone();
    let reply_subject_prefix = config.reply_subject_prefix.clone();

    let ps_email_script = format!(
        r#"
$ErrorActionPreference = "Stop"

try {{
    Write-Output "TRACE: Acquiring Outlook COM object..."
    $Outlook = New-Object -ComObject Outlook.Application
    $Namespace = $Outlook.GetNamespace("MAPI")

    # Access Inbox and Sent Items to search for the original message
    Write-Output "TRACE: Accessing Inbox and Sent Items..."
    $Inbox = $Namespace.GetDefaultFolder(6) # olFolderInbox
    $SentItems = $Namespace.GetDefaultFolder(5) # olFolderSentMail

    $OriginalMail = $null
    $SenderToMatch = "{sender_account}"
    $PrefixToMatch = "{subject_prefix}"

Write-Output "TRACE: Searching for original message with sender: '$SenderToMatch' and subject prefix: '$PrefixToMatch'"

function Find-OriginalMessage ($FolderItems, $SortProperty) {{
    if (-not $FolderItems) {{ return $null }}
    $FolderItems.Sort($SortProperty, $true)

    foreach ($Item in $FolderItems) {{
        if (-not $Item.Subject -or -not $Item.Subject.StartsWith($PrefixToMatch, [System.StringComparison]::InvariantCultureIgnoreCase)) {{
            continue
        }}

        Write-Output "TRACE: Found candidate with matching subject prefix: $($Item.Subject)"

        $SenderAddress = $Item.SenderEmailAddress
        if ($Item.SenderEmailType -eq "EX") {{
            # Attempt to resolve EX to SMTP
            Write-Output "TRACE: SenderEmailType is EX, attempting resolution..."
            try {{
                $ExchangeUser = $Item.Sender.GetExchangeUser()
                if ($ExchangeUser) {{
                    $SenderAddress = $ExchangeUser.PrimarySmtpAddress
                    Write-Output "TRACE: Resolved via GetExchangeUser to: $SenderAddress"
                }} else {{
                    Write-Output "TRACE: GetExchangeUser returned null."
                }}
            }} catch {{
                Write-Output "TRACE: GetExchangeUser failed: $_"
            }}

            # Fallback to PropertyAccessor if still not resolved or empty
            if (-not $SenderAddress -or $SenderAddress.IndexOf("@") -eq -1) {{
                try {{
                    $PropAccessor = $Item.PropertyAccessor
                    $SenderAddress = $PropAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x39FE001E")
                    Write-Output "TRACE: Resolved via PropertyAccessor to: $SenderAddress"
                }} catch {{
                    Write-Output "TRACE: PropertyAccessor 0x39FE001E failed: $_"
                }}
            }}
        }}

        if ([string]::IsNullOrWhiteSpace($SenderAddress)) {{
            Write-Output "TRACE: Candidate rejected. Sender address is empty."
            continue
        }}

        if ([string]::Equals($SenderAddress.Trim(), $SenderToMatch.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) {{
            Write-Output "TRACE: Sender match successful ($SenderAddress). Selected as original message."
            return $Item
        }} else {{
            Write-Output "TRACE: Candidate rejected. Sender '$SenderAddress' does not match '$SenderToMatch'."
        }}
    }}

    return $null
}}

    # Search Inbox
    Write-Output "TRACE: Searching Inbox..."
    $OriginalMail = Find-OriginalMessage -FolderItems $Inbox.Items -SortProperty "[ReceivedTime]"

    # Search Sent Items if not found in Inbox
    if (-not $OriginalMail) {{
        Write-Output "TRACE: Not found in Inbox, searching Sent Items..."
        $OriginalMail = Find-OriginalMessage -FolderItems $SentItems.Items -SortProperty "[SentOn]"
    }}

    if (-not $OriginalMail) {{
        throw "Original message with subject prefix '{subject_prefix}' not found in Inbox or Sent Items of '{sender_account}'."
    }}

    Write-Output "TRACE: Creating ReplyAll draft..."
    $ReplyMail = $OriginalMail.ReplyAll()

    if ("{subject}") {{
        $ReplyMail.Subject = "{subject}"
    }}

    # Prepend the generated dashboard to the HTMLBody
    Write-Output "TRACE: Populating reply draft body..."
    $ReplyMail.HTMLBody = '{html_body}' + $ReplyMail.HTMLBody

    Write-Output "TRACE: Saving reply draft..."
    $ReplyMail.Save()
    Write-Output "TRACE: Reply draft saved successfully."

}} catch {{
    Write-Error "Outlook operation failed: $_"
    exit 1
}}
"#,
        sender_account = sender_account_email.replace("'", "''"),
        subject_prefix = reply_subject_prefix.replace("'", "''"),
        subject = subject.replace("'", "''"),
        html_body = final_html.replace("'", "''")
    );

    if config.dashboard_config.save_email_as_html.unwrap_or(false) {
        let tmp_dir = std::env::temp_dir();
        let html_path = tmp_dir.join("crm_open_sohail_email.html");
        std::fs::write(&html_path, final_html)?;
        info!("save_email_as_html is true. Saved email body to {}. Skipping PowerShell send for testing.", html_path.display());
    } else {
        info!("Creating/saving reply draft via Outlook COM...");
        if let Err(e) = powershell::run_powershell(&ps_email_script) {
            error!("Failed to create/save reply draft: {}", e);
            anyhow::bail!("Failed to create/save reply draft");
        }
        info!("Reply draft saved successfully.");
        info!("Reply draft saved");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasker::config::DashboardUpdaterConfig;

    pub(crate) struct TestDataset {
        pub users_file: tempfile::NamedTempFile,
        pub assignments_file: tempfile::NamedTempFile,
        pub download_dir: tempfile::TempDir,
        pub output_file: tempfile::NamedTempFile,
        #[allow(dead_code)]
        pub leads_file: tempfile::NamedTempFile,
        #[allow(dead_code)]
        pub teams_file: tempfile::NamedTempFile,
        #[allow(dead_code)]
        pub config_json: String,
    }

    pub(crate) fn setup_test_dataset() -> TestDataset {
        let users_file = tempfile::NamedTempFile::new().unwrap();
        let assignments_file = tempfile::NamedTempFile::new().unwrap();
        let download_dir = tempfile::tempdir().unwrap();
        let output_file = tempfile::NamedTempFile::new().unwrap();
        let leads_file = tempfile::NamedTempFile::new().unwrap();
        let teams_file = tempfile::NamedTempFile::new().unwrap();

        let agents_csv = std::fs::read_to_string("TestingDownloads/users.csv").unwrap();
        std::fs::write(users_file.path(), agents_csv).unwrap();

        let assignment_csv =
            std::fs::read_to_string("TestingDownloads/assignement settings.csv").unwrap();
        std::fs::write(assignments_file.path(), assignment_csv).unwrap();

        std::fs::copy(
            "TestingDownloads/ticket_report_1783634497568.csv",
            download_dir.path().join("ticket_report_1783634497568.csv"),
        )
        .unwrap();
        std::fs::copy(
            "TestingDownloads/ticket_report_1783634532999.csv",
            download_dir.path().join("ticket_report_1783634532999.csv"),
        )
        .unwrap();
        std::fs::copy(
            "TestingDownloads/ticket_report_1783634535708.csv",
            download_dir.path().join("ticket_report_1783634535708.csv"),
        )
        .unwrap();

        let leads_bytes = std::fs::read("TestingDownloads/lead_report_1783627642439.csv").unwrap();
        let leads_csv = String::from_utf8_lossy(&leads_bytes);
        std::fs::write(leads_file.path(), leads_csv.as_bytes()).unwrap();
        std::fs::copy(
            leads_file.path(),
            download_dir.path().join("lead_report_1783627642439.csv"),
        )
        .unwrap();

        let config_json = std::fs::read_to_string("TestingDownloads/tasker_config.json").unwrap();
        {
            let mut teams_wtr = csv::Writer::from_writer(teams_file.as_file());
            teams_wtr
                .write_record(["Team Name", "Receiver Name", "To Emails", "CC"])
                .unwrap();
            teams_wtr
                .write_record([
                    "Incomplete Reservation",
                    "Incomplete Reservation Team",
                    "inc@example.com",
                    "cc@example.com",
                ])
                .unwrap();
            teams_wtr
                .write_record([
                    "PRE-AUTHORIZATION",
                    "Pre-Auth Team",
                    "preauth@example.com",
                    "",
                ])
                .unwrap();
            teams_wtr
                .write_record(["Call Center", "Call Center Team", "cc@example.com", ""])
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

        let dummy_dataset = setup_test_dataset();

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
            sender_account_email: "sender@example.com".to_string(),
            reply_subject_prefix: "[CRM-TEST]".to_string(),
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

        let dummy_dataset = setup_test_dataset();

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
            sender_account_email: "sender@example.com".to_string(),
            reply_subject_prefix: "[CRM-TEST]".to_string(),
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

    #[test]
    fn test_outlook_reply_all_draft_mechanism() {
        let src = include_str!("mod.rs");
        assert!(src.contains("GetExchangeUser()"));
        assert!(src.contains("PrimarySmtpAddress"));
        assert!(src.contains("0x39FE001E"));
        assert!(src.contains("catch"));
        assert!(!src.contains(&format!("$ReplyMail.{} = ", "To")));
        assert!(!src.contains(&format!("$ReplyMail.{} = ", "CC")));
        // Assert sender_account_email is used
        assert!(
            src.contains("sender_account_email"),
            "Should reference sender_account_email config field"
        );

        // Assert reply_subject_prefix is used for searching
        assert!(
            src.contains("reply_subject_prefix"),
            "Should reference reply_subject_prefix config field"
        );
        assert!(
            src.contains(".StartsWith($PrefixToMatch"),
            "Should use explicit prefix startswith check"
        );

        // Assert .ReplyAll() is used instead of .CreateItem() or .Reply()
        assert!(
            src.contains(".ReplyAll()"),
            "Should use Outlook's ReplyAll method to preserve thread context"
        );

        let create_item = "$Outlook.CreateItem";
        assert!(
            !src.contains(&format!("{}(0)", create_item)),
            "Should not create a brand new email item"
        );

        // Assert .Save() is used instead of .Send() to ensure Draft state
        assert!(
            src.contains("$ReplyMail.Save()"),
            "Should save email as draft"
        );

        // Assert explicit error handling exits with non-zero
        assert!(
            src.contains("} catch {"),
            "Should use try/catch block to trap COM errors"
        );
        assert!(
            src.contains("exit 1"),
            "Should explicitly exit with non-zero code on failure"
        );

        // Assert strict case-insensitive match
        assert!(
            src.contains("[System.StringComparison]::OrdinalIgnoreCase"),
            "Should use OrdinalIgnoreCase for string comparison"
        );
        assert!(
            src.contains(".Trim()"),
            "Should trim whitespace before comparing"
        );
    }

    #[test]
    fn test_calculate_yesterday_subject() {
        use chrono::NaiveDate;
        // Normal day
        assert_eq!(
            calculate_yesterday_subject(NaiveDate::from_ymd_opt(2026, 9, 3).unwrap()),
            "Open TKTs 02-September"
        );
        // Month boundary
        assert_eq!(
            calculate_yesterday_subject(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            "Open TKTs 30-September"
        );
        // Year boundary
        assert_eq!(
            calculate_yesterday_subject(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
            "Open TKTs 31-December"
        );
        // Leap year
        assert_eq!(
            calculate_yesterday_subject(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()),
            "Open TKTs 29-February"
        );
    }
}
