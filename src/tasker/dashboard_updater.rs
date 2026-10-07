use crate::tasker::config::DashboardUpdaterConfig;
use anyhow::Result;
use tracing::{error, info};

fn run_persistent_powershell_with_args(
    logical_name: &str,
    script_template: &str,
    args: &[(&str, &str)],
) -> Result<()> {
    let script_manager = crate::tasker::script_manager::ScriptManager::new();
    let script_path =
        script_manager.get_or_create_script("Dashboard Updater", logical_name, script_template)?;
    script_manager.execute_script_with_args(&script_path, args)
}

#[allow(dead_code)]
fn run_persistent_powershell(logical_name: &str, script: &str) -> Result<()> {
    run_persistent_powershell_with_args(logical_name, script, &[])
}

pub fn run(config: &DashboardUpdaterConfig) -> Result<()> {
    info!("Starting DashboardUpdater task. Config: {:?}", config);

    let abs_dashboard_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_file);
    let dashboard_path_str = abs_dashboard_path.to_string_lossy().to_string();

    info!("Updating dashboard '{}'", dashboard_path_str);

    let ps_script = r#"
param(
    [Parameter(Mandatory = $true)]
    [string]$DashboardPath
)

$ErrorActionPreference = "Stop"

$resolvedDashboardPath = [System.IO.Path]::GetFullPath($DashboardPath)

if (-not (Test-Path -LiteralPath $resolvedDashboardPath -PathType Leaf)) {
    throw "Dashboard file not found: $resolvedDashboardPath"
}

$excel = $null
$workbooks = $null
$workbook = $null
$worksheets = $null
$worksheet = $null
$pivotTables = $null
$pivotTable = $null
$pivotCache = $null
$originalCalculation = $null

try {
    Write-Host "PS: Starting Excel..."

    $excel = New-Object -ComObject Excel.Application

    $excel.Visible = $false
    $excel.DisplayAlerts = $false
    $excel.ScreenUpdating = $false
    $excel.EnableEvents = $false
    $excel.AskToUpdateLinks = $false

    Write-Host "PS: Opening dashboard: $resolvedDashboardPath"

    $workbooks = $excel.Workbooks

    $workbook = $workbooks.Open(
        $resolvedDashboardPath,
        0,
        $false
    )

    Write-Host "PS: Workbook opened successfully."

    # Changing Excel calculation mode can fail with HRESULT 0x800A03EC.
    # Therefore, calculation mode is treated as an optional optimization.
    try {
        $originalCalculation = $excel.Calculation

        # xlCalculationManual = -4135
        $excel.Calculation = -4135

        Write-Host "PS: Calculation mode changed to manual."
    }
    catch {
        $originalCalculation = $null

        Write-Host (
            "PS: Warning: Unable to change calculation mode. " +
            "Continuing with the current Excel calculation mode. " +
            "Details: $($_.Exception.Message)"
        )
    }

    $pivotFound = $false
    $worksheets = $workbook.Worksheets
    $worksheetCount = $worksheets.Count

    Write-Host "PS: Searching $worksheetCount worksheet(s) for PivotTable2..."

    for ($index = 1; $index -le $worksheetCount; $index++) {
        try {
            $worksheet = $worksheets.Item($index)

            Write-Host "PS: Checking worksheet '$($worksheet.Name)'..."

            try {
                $pivotTables = $worksheet.PivotTables()

                if ($pivotTables.Count -gt 0) {
                    try {
                        $pivotTable = $pivotTables.Item("PivotTable2")
                    }
                    catch {
                        $pivotTable = $null
                    }
                }
            }
            catch {
                $pivotTable = $null
            }

            if ($null -ne $pivotTable) {
                Write-Host (
                    "PS: PivotTable2 found on worksheet " +
                    "'$($worksheet.Name)'."
                )

                $pivotCache = $pivotTable.PivotCache()

                try {
                    $pivotCache.EnableRefresh = $true
                }
                catch {
                    Write-Host "PS: PivotCache EnableRefresh isn't supported."
                }

                try {
                    $pivotCache.BackgroundQuery = $false
                }
                catch {
                    Write-Host "PS: PivotCache BackgroundQuery isn't supported."
                }

                Write-Host "PS: Refreshing PivotCache..."

                try {
                    $pivotCache.Refresh()
                }
                catch {
                    Write-Host (
                        "PS: PivotCache refresh warning: " +
                        "$($_.Exception.Message)"
                    )
                }

                Write-Host "PS: Refreshing PivotTable2..."

                $refreshResult = $pivotTable.RefreshTable()

                Write-Host "PS: PivotTable refresh result: $refreshResult"

                $pivotFound = $true
                break
            }
        }
        finally {
            if ($null -ne $pivotCache) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $pivotCache
                    )
                }
                catch {}

                $pivotCache = $null
            }

            if ($null -ne $pivotTable) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $pivotTable
                    )
                }
                catch {}

                $pivotTable = $null
            }

            if ($null -ne $pivotTables) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $pivotTables
                    )
                }
                catch {}

                $pivotTables = $null
            }

            if ($null -ne $worksheet) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $worksheet
                    )
                }
                catch {}

                $worksheet = $null
            }
        }
    }

    if (-not $pivotFound) {
        Write-Host (
            "PS: PivotTable2 wasn't found. " +
            "Refreshing all workbook connections..."
        )

        $workbook.RefreshAll()

        try {
            $excel.CalculateUntilAsyncQueriesDone()

            Write-Host "PS: Async queries completed."
        }
        catch {
            Write-Host (
                "PS: CalculateUntilAsyncQueriesDone warning: " +
                "$($_.Exception.Message)"
            )
        }
    }

    Write-Host "PS: Starting workbook calculation..."

    # Don't force Excel.Calculation because that property caused 0x800A03EC.
    try {
        $excel.CalculateFull()

        Write-Host "PS: Full calculation completed."
    }
    catch {
        Write-Host (
            "PS: CalculateFull warning: $($_.Exception.Message)"
        )

        try {
            $workbook.Application.Calculate()

            Write-Host "PS: Standard calculation completed."
        }
        catch {
            Write-Host (
                "PS: Standard calculation warning: " +
                "$($_.Exception.Message)"
            )
        }
    }

    Write-Host "PS: Saving dashboard..."

    $workbook.Save()

    Write-Host "PS: Dashboard updated successfully."
}
catch {
    Write-Host "PS ERROR: $($_.Exception.Message)"

    if ($_.ScriptStackTrace) {
        Write-Host "PS ERROR STACK: $($_.ScriptStackTrace)"
    }

    throw
}
finally {
    Write-Host "PS: Starting cleanup..."

    if ($null -ne $originalCalculation -and $null -ne $excel) {
        try {
            $excel.Calculation = $originalCalculation

            Write-Host "PS: Original calculation mode restored."
        }
        catch {
            Write-Host (
                "PS: Calculation mode restoration warning: " +
                "$($_.Exception.Message)"
            )
        }
    }

    if ($null -ne $pivotCache) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $pivotCache
            )
        }
        catch {}

        $pivotCache = $null
    }

    if ($null -ne $pivotTable) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $pivotTable
            )
        }
        catch {}

        $pivotTable = $null
    }

    if ($null -ne $pivotTables) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $pivotTables
            )
        }
        catch {}

        $pivotTables = $null
    }

    if ($null -ne $worksheet) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $worksheet
            )
        }
        catch {}

        $worksheet = $null
    }

    if ($null -ne $worksheets) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $worksheets
            )
        }
        catch {}

        $worksheets = $null
    }

    if ($null -ne $workbook) {
        try {
            $workbook.Close($false)
        }
        catch {
            Write-Host (
                "PS: Workbook close warning: " +
                "$($_.Exception.Message)"
            )
        }

        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $workbook
            )
        }
        catch {}

        $workbook = $null
    }

    if ($null -ne $workbooks) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $workbooks
            )
        }
        catch {}

        $workbooks = $null
    }

    if ($null -ne $excel) {
        try {
            $excel.Quit()
        }
        catch {
            Write-Host (
                "PS: Excel quit warning: $($_.Exception.Message)"
            )
        }

        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $excel
            )
        }
        catch {}

        $excel = $null
    }

[System.GC]::Collect()
[System.GC]::WaitForPendingFinalizers()
[System.GC]::Collect()
[System.GC]::WaitForPendingFinalizers()

    Write-Host "PS: Cleanup completed."
}
"#;

    // Only run PowerShell if not a dry run test
    if !config.save_email_as_html.unwrap_or(false) {
        let ps_result = run_persistent_powershell_with_args(
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
        let indent_spaces = config.indentation_spaces.unwrap_or(4);
        let indent_width = indent_spaces * 5;

        let html_body = format!(
            r#"<html><body style='font-family: Arial, sans-serif;'>Dear Aya,<br/><table border='0'><tr><td width='{}'></td><td>Please find the CRM Ticket dashboard attached.</td></tr></table></body></html>"#,
            indent_width
        );
        info!("Email generation completed");

        let ps_email_script = r#"
param(
    [string]$EmailTo,
    [string]$EmailCc,
    [string]$Subject,
    [string]$HtmlBody,
    [string]$AttachmentPath
)

$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
if ($EmailTo) { $Mail.To = $EmailTo }
if ($EmailCc) { $Mail.CC = $EmailCc }
if ($Subject) { $Mail.Subject = $Subject }
if ($HtmlBody) { $Mail.HTMLBody = $HtmlBody }
if ($AttachmentPath -and (Test-Path $AttachmentPath)) {
	$Mail.Attachments.Add($AttachmentPath)
}
$Mail.Send()
"#;

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

        if let Err(e) = run_persistent_powershell_with_args(
            "dashboard_email.ps1",
            ps_email_script,
            &[
                ("-EmailTo", email_to.as_str()),
                ("-EmailCc", email_cc),
                ("-Subject", "CRM Tickets Dashboard"),
                ("-HtmlBody", html_body.as_str()),
                ("-AttachmentPath", dashboard_path_str.as_str()),
            ],
        ) {
            error!("Failed to send dashboard email: {}", e);
            // Optionally, try a fallback email or bubble up
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
        let src = include_str!("dashboard_updater.rs");
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
            html_content.contains("<td width='20'></td>"),
            "HTML email should contain the proper indentation table. Found: {}",
            html_content
        );
    }
}
