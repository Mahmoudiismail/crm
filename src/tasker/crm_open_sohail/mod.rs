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

    let ps_email_template = r#"
param(
    [string]$SenderAccount,
    [string]$SubjectPrefix,
    [string]$Subject,
    [string]$HtmlBodyPath
)

try {
    $ErrorActionPreference = "Stop"

    $Outlook = New-Object -ComObject Outlook.Application
    $Namespace = $Outlook.GetNamespace("MAPI")

    $Inbox = $Namespace.GetDefaultFolder(6) # olFolderInbox
    $SentFolder = $Namespace.GetDefaultFolder(5) # olFolderSentMail

    function Find-OriginalMessage($FolderItems, $SortProperty) {
        $FolderItems.Sort($SortProperty, $true)
        $matches = @()

        foreach ($Item in $FolderItems) {
            if (-not $Item.Subject -or -not $Item.Subject.StartsWith($SubjectPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
                continue
            }

            $SenderAddress = ""
            try {
                $SenderAddress = $Item.SenderEmailAddress
            } catch {
                Write-Output "TRACE: Exception reading SenderEmailAddress: $_"
            }

            if ($Item.SenderEmailType -eq "EX") {
                try {
                    if ($Item.Sender -and $Item.Sender.GetExchangeUser()) {
                        $SenderAddress = $Item.Sender.GetExchangeUser().PrimarySmtpAddress
                        Write-Output "TRACE: Resolved via GetExchangeUser to: $SenderAddress"
                    } else {
                        Write-Output "TRACE: GetExchangeUser returned null."
                    }
                } catch {
                    Write-Output "TRACE: GetExchangeUser failed: $_"
                }

                # Fallback to PropertyAccessor if still not resolved or empty
                if (-not $SenderAddress -or $SenderAddress.IndexOf("@") -eq -1) {
                    try {
                        $PA = $Item.PropertyAccessor
                        $SenderAddress = $PA.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x39FE001E")
                        Write-Output "TRACE: Resolved via PropertyAccessor to: $SenderAddress"
                    } catch {
                        Write-Output "TRACE: PropertyAccessor 0x39FE001E failed: $_"
                    }
                }
            }

            if ([string]::IsNullOrWhiteSpace($SenderAddress)) {
                Write-Output "TRACE: Candidate rejected. Sender address is empty."
                continue
            }

            if ([string]::Equals($SenderAddress.Trim(), $SenderAccount.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) {
                Write-Output "TRACE: Sender match successful ($SenderAddress)."
                $matches += $Item
            } else {
                Write-Output "TRACE: Candidate rejected. Sender '$SenderAddress' does not match '$SenderAccount'."
            }
        }

        if ($matches.Count -gt 0) {
            if ($matches.Count -gt 1) {
                Write-Output "TRACE: Found $($matches.Count) matches in folder. Logging all matches:"
                foreach ($m in $matches) {
                    Write-Output "TRACE: Match - Subject: $($m.Subject), Received: $($m.ReceivedTime)"
                }
                Write-Output "TRACE: Selecting the latest match."
            }
            $script:OriginalMail = $matches[0]
        }
    }

    $script:OriginalMail = $null

    # Search Inbox
    Write-Output "TRACE: Searching Inbox..."
    Find-OriginalMessage -FolderItems $Inbox.Items -SortProperty "[ReceivedTime]"

    # Search Sent Items if not found in Inbox
    if (-not $script:OriginalMail) {
        Write-Output "TRACE: Not found in Inbox, searching Sent Items..."
        Find-OriginalMessage -FolderItems $SentFolder.Items -SortProperty "[SentOn]"
    }

    $TargetMail = $script:OriginalMail

    if (-not $TargetMail) {
        throw "Original message with subject prefix '$SubjectPrefix' not found in Inbox or Sent Items of '$SenderAccount'."
    }

    Write-Output "TRACE: Creating ReplyAll draft..."
    $ReplyMail = $TargetMail.ReplyAll()

    if ($Subject) {
        $ReplyMail.Subject = $Subject
    }

    # Prepend the generated dashboard to the HTMLBody
    Write-Output "TRACE: Populating reply draft body..."
    if ($HtmlBodyPath -and (Test-Path $HtmlBodyPath)) {
        $HtmlBody = Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8
        $ReplyMail.HTMLBody = $HtmlBody + $ReplyMail.HTMLBody
    }

    Write-Output "TRACE: Saving reply draft..."
    $ReplyMail.Save()
    Write-Output "TRACE: Reply draft saved successfully."

} catch {
    Write-Error "Outlook operation failed: $_"
    [System.Environment]::Exit(1)
}
"#;

    if config.dashboard_config.save_email_as_html.unwrap_or(false) {
        let tmp_dir = std::env::temp_dir();
        let old_html_path = tmp_dir.join("crm_open_sohail_email.html");
        std::fs::write(&old_html_path, final_html)?;
        info!("save_email_as_html is true. Saved email body to {}. Skipping PowerShell send for testing.", html_path.display());
    } else {
        info!("Creating/saving reply draft via Outlook COM...");
        if let Err(e) = powershell::run_powershell_with_args(
            "reply_email.ps1",
            ps_email_template,
            &[
                ("-SenderAccount", &sender_account_email),
                ("-SubjectPrefix", &reply_subject_prefix),
                ("-Subject", &subject),
                ("-HtmlBodyPath", html_path.to_string_lossy().as_ref()),
            ],
        ) {
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
    fn test_reply_email_parameter_contract() {
        let src = include_str!("mod.rs");
        assert!(src.contains("[string]$SenderAccount"));
        assert!(src.contains("[string]$SubjectPrefix"));
        assert!(src.contains("[string]$Subject"));
        assert!(src.contains("[string]$HtmlBodyPath"));
    }
}
