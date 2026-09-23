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

    let params = crate::tasker::csv_task::CsvAnalysisParams::from(config);
    let generated_csv_path_opt = crate::tasker::csv_task::generate_csv(&params)?;

    if generated_csv_path_opt.is_none() {
        info!("No new tickets found. Skipping dashboard update.");
        return Ok(());
    }
    let generated_csv_path = generated_csv_path_opt.unwrap();
    let abs_csv_path = std::env::current_dir()?.join(&generated_csv_path);
    let csv_path_str = abs_csv_path.to_string_lossy().to_string();

    let abs_dashboard_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_file);
    let dashboard_path_str = abs_dashboard_path.to_string_lossy().to_string();

    info!(
        "Updating dashboard '{}' using CSV data '{}'",
        dashboard_path_str, csv_path_str
    );

    let ps_script = format!(
        r#"
$ErrorActionPreference = "Stop"
$csvPath = "{csv}"
$dashboardPath = "{dash}"

Write-Host "Opening Excel Application..."
$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false

Write-Host "Opening Dashboard workbook: $dashboardPath"
$Workbook = $Excel.Workbooks.Open($dashboardPath)

# Disable calculations to avoid hanging up the COM model
$Excel.Calculation = -4135 # xlCalculationManual
$Excel.ScreenUpdating = $false

try {{
    Write-Host "Activating 'Data' sheet..."
    $DataSheet = $Workbook.Worksheets.Item("Data")
    $DataSheet.Activate()

    Write-Host "Clearing old data from 'Data' sheet..."
    $lastRowData = $DataSheet.Cells.SpecialCells(11).Row # xlCellTypeLastCell = 11
    if ($lastRowData -gt 1) {{
        $DataSheet.Range("A2:Z$lastRowData").ClearContents()
    }}

    Write-Host "Reading new data from CSV: $csvPath"
    $CsvData = Import-Csv -Path $csvPath

    if ($CsvData.Count -gt 0) {{
        # Extract headers dynamically from the first row to ensure matching
        $Headers = $CsvData[0].psobject.properties.name

        # Create a 2D object array to hold the data for fast COM assignment
        $rowCount = $CsvData.Count
        $colCount = $Headers.Count
        $DataArray = New-Object 'object[,]' $rowCount, $colCount

        Write-Host "Converting CSV data to 2D array ($rowCount rows, $colCount columns)..."
        for ($i = 0; $i -lt $rowCount; $i++) {{
            for ($j = 0; $j -lt $colCount; $j++) {{
                $headerName = $Headers[$j]
                $DataArray[$i, $j] = $CsvData[$i].$headerName
            }}
        }}

        Write-Host "Writing data to 'Data' sheet in one operation..."
        # Calculate the target range (A2 to ColumnLetter + RowNumber)
        # Using a helper function to convert column index to letter (A, B, ..., Z, AA, etc.)
        function Get-ExcelColumnLetter ($ColumnNumber) {{
            $dividend = $ColumnNumber
            $columnName = ""
            while ($dividend -gt 0) {{
                $modulo = ($dividend - 1) % 26
                $columnName = [char](65 + $modulo) + $columnName
                $dividend = [int][math]::Floor(($dividend - $modulo) / 26)
            }}
            return $columnName
        }}

        $endColumnLetter = Get-ExcelColumnLetter $colCount
        $endRow = $rowCount + 1 # Start at row 2
        $targetRangeStr = "A2:$endColumnLetter$endRow"

        $TargetRange = $DataSheet.Range($targetRangeStr)
        $TargetRange.Value2 = $DataArray
        Write-Host "Data written successfully."

    }} else {{
        Write-Host "CSV file is empty or only contains headers."
    }}

    Write-Host "Activating 'Pivot' sheet..."
    $PivotSheet = $Workbook.Worksheets.Item("Pivot")
    $PivotSheet.Activate()

    Write-Host "Refreshing all Pivot Tables in the workbook..."
    foreach ($pc in $Workbook.PivotCaches()) {{
        $pc.Refresh()
    }}

    Write-Host "Saving Dashboard..."
    $Workbook.Save()
    Write-Host "Dashboard saved successfully."
}}
catch {{
    Write-Error "An error occurred during Excel COM automation: $_"
    throw
}}
finally {{
    Write-Host "Cleaning up Excel COM objects..."
    # Re-enable settings
    $Excel.Calculation = -4105 # xlCalculationAutomatic
    $Excel.ScreenUpdating = $true

    if ($Workbook) {{ $Workbook.Close($false) }}
    if ($Excel) {{
        $Excel.Quit()
        [System.Runtime.Interopservices.Marshal]::ReleaseComObject($Excel) | Out-Null
    }}
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}}
"#,
        csv = csv_path_str.replace('\'', "''"),
        dash = dashboard_path_str.replace('\'', "''")
    );

    // Only run PowerShell if not a dry run test
    if !config.save_email_as_html.unwrap_or(false) {
        let ps_result = run_persistent_powershell_with_args(
            "dashboard_updater.ps1",
            &ps_script,
            &[("-DashboardPath", &dashboard_path_str)],
        );

        if let Err(e) = ps_result {
            error!("Error executing dashboard update PowerShell script: {}", e);
            anyhow::bail!(e);
        }
    }

    info!("Successfully updated dashboard '{}'", dashboard_path_str);
    info!("Dashboard update completed");

    if let (Some(email_to), Some(email_cc)) = (&config.email_to, &config.email_cc) {
        info!("Sending dashboard via email to: {}", email_to);

        info!("Email generation started");
        let indent_spaces = config.indentation_spaces.unwrap_or(4);
        let indent_width = indent_spaces * 5;

        let html_body = format!(
            r#"<html><body style='font-family: Arial, sans-serif;'>Dear Aya,<br/><table border='0'><tr><td width='{}'></td><td>Please find the CRM Ticket dashboard attached.</td></tr></table></body></html>"#,
            indent_width
        );
        info!("Email generation completed");

        let payloads_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("logs")
            .join("email_payloads");
        std::fs::create_dir_all(&payloads_dir)?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%f");
        let html_path =
            payloads_dir.join(format!("email_body_dashboard_updater_{}.html", timestamp));
        std::fs::write(&html_path, &html_body)?;

        let ps_email_script = r#"
param(
    [string]$EmailTo,
    [string]$EmailCc,
    [string]$Subject,
    [string]$HtmlBodyPath,
    [string]$AttachmentPath
)

$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
if ($EmailTo) { $Mail.To = $EmailTo }
if ($EmailCc) { $Mail.CC = $EmailCc }
if ($Subject) { $Mail.Subject = $Subject }

if ($HtmlBodyPath -and (Test-Path $HtmlBodyPath)) {
    $Mail.HTMLBody = Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8
}

if ($AttachmentPath -and (Test-Path $AttachmentPath)) {
    try {
        $Mail.Attachments.Add($AttachmentPath)
    } catch {
        Write-Warning "Attachment too large, sending without attachment."
        $Mail.HTMLBody += "<br><br><span style='color:red;'><b>Note:</b> The Dashboard file was too large to attach to this email. Please access it from the shared network drive.</span>"
    }
}
$Mail.Send()
"#;

        if config.save_email_as_html.unwrap_or(false) {
            info!("save_email_as_html is true. Saved email body to {}. Skipping PowerShell send for testing.", html_path.display());
            return Ok(());
        }

        if let Err(e) = run_persistent_powershell_with_args(
            "dashboard_email.ps1",
            ps_email_script,
            &[
                ("-EmailTo", email_to.as_str()),
                ("-EmailCc", email_cc.as_str()),
                ("-Subject", "CRM Tickets Dashboard"),
                ("-HtmlBodyPath", html_path.to_string_lossy().as_ref()),
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
    use crate::tasker::config::{CategoryException, DashboardUpdaterConfig};

    use tempfile::NamedTempFile;

    #[test]
    fn test_task2_dashboard_updater_calculation_mode() {
        let src = include_str!("dashboard_updater.rs");
        let open_idx = src
            .find("$Workbook = $Excel.Workbooks.Open($dashboardPath)")
            .expect("Should find open workbook");
        let calc_idx = src
            .find("$Excel.Calculation = -4135 # xlCalculationManual")
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
            minutes_ago: 60 * 24 * 365 * 10,
            start_date: Some(start_date),
            exclude_branches: vec!["Branch To Exclude".to_string()],
            exclude_categories: vec!["ExcludedCategory".to_string()],
            category_exceptions: Some(vec![CategoryException {
                category: "Incomplete Reservation".to_string(),
                branch: None,
                team: None,
            }]),
            output_file: dataset.output_file.path().to_str().unwrap().to_string(),
            dashboard_file: dummy_dashboard.path().to_str().unwrap().to_string(),
            email_to: Some("test@example.com".to_string()),
            email_cc: None,
            save_email_as_html: Some(true),
            indentation_spaces: Some(4),
        };

        // Get the latest file in the logs/email_payloads dir before running
        let payloads_dir = std::env::current_dir()
            .unwrap()
            .join("logs")
            .join("email_payloads");

        let mut count_before = 0;
        if payloads_dir.exists() {
            count_before = std::fs::read_dir(&payloads_dir).unwrap().count();
        }

        // Run the task
        let result = run(&dash_config);
        assert!(
            result.is_ok(),
            "Dashboard updater task failed: {:?}",
            result.err()
        );

        if payloads_dir.exists() {
            let count_after = std::fs::read_dir(&payloads_dir).unwrap().count();
            assert!(
                count_after > count_before,
                "A new HTML payload file should have been created"
            );

            let mut entries: Vec<_> = std::fs::read_dir(&payloads_dir)
                .unwrap()
                .map(|r| r.unwrap())
                .collect();

            entries.sort_by_key(|dir| dir.metadata().unwrap().modified().unwrap());

            if let Some(latest) = entries.last() {
                let html_content = std::fs::read_to_string(latest.path()).unwrap();
                let expected_indent = "<table border='0'><tr><td width='20'></td>";
                assert!(
                    html_content.contains(expected_indent),
                    "HTML email should contain the proper indentation table. Found: {}",
                    html_content
                );
            }
        }

        let output_csv_path = std::path::PathBuf::from(&dash_config.output_file);
        assert!(
            output_csv_path.exists(),
            "results.csv should be saved for tests"
        );

        let output_csv_content = std::fs::read_to_string(&output_csv_path).unwrap();
        let mut rdr = crate::utils::build_csv_reader_from_reader(output_csv_content.as_bytes());

        let mut count = 0;
        for result in rdr.records() {
            let _record = result.unwrap();
            count += 1;
        }

        assert!(count > 0, "There should be some records in the results.csv");
    }
}
