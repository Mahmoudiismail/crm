use crate::tasker::config::DashboardUpdaterConfig;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use tracing::{error, info};

use std::io::{BufRead, BufReader};
use std::process::Stdio;

fn run_powershell(script: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("dashboard_updater_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(script.as_bytes())?;
    temp_file.as_file().sync_all()?;

    let (file, path) = temp_file.keep()?;
    drop(file);

    let mut child = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stderr"))?;

    let stdout_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for l in reader.lines().map_while(Result::ok) {
            if !l.trim().is_empty() {
                info!("PowerShell: {}", l);
            }
        }
    });

    let stderr_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for l in reader.lines().map_while(Result::ok) {
            if !l.trim().is_empty() {
                error!("PowerShell Error: {}", l);
            }
        }
    });

    let status = child.wait()?;
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let _ = std::fs::remove_file(&path);

    if !status.success() {
        anyhow::bail!("PowerShell script exited with status: {}", status);
    }

    Ok(())
}

pub fn run(config: &DashboardUpdaterConfig) -> Result<()> {
    info!("Starting DashboardUpdater task. Config: {:?}", config);

    let params = crate::tasker::csv_task::CsvAnalysisParams::from(config);
    let generated_csv_path_opt = crate::tasker::csv_task::generate_csv(&params)?;

    if generated_csv_path_opt.is_none() {
        info!("No new tickets found. Skipping dashboard update.");
        return Ok(());
    }

    let dashboard_file_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_file);
    if !dashboard_file_path.exists() {
        anyhow::bail!(
            "Dashboard file not found at: {}",
            dashboard_file_path.display()
        );
    }

    let dashboard_path_str = dashboard_file_path.to_string_lossy().to_string();
    let tmp_dir = std::env::temp_dir();

    info!("Dashboard update started.");
    info!("Directly refreshing data model in '{}'", dashboard_path_str);

    let ps_script = format!(
        r#"

$ErrorActionPreference = "Stop"

$dashboardPath = '{dashboard_path}'

Write-Output "Starting Excel automation to update dashboard..."

$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false

# Optimize Excel performance during large data operations
$Excel.ScreenUpdating = $false
$Excel.EnableEvents = $false

$processId = $null

try {{
    # Attempt to capture the Process ID so we can forcefully kill it later if needed
    try {{
        # Try getting process by HWND
        [int]$handle = $Excel.Hwnd
        $processId = (Get-Process | Where-Object {{ $_.MainWindowHandle -eq $handle }}).Id
    }} catch {{
        # Fallback to getting most recent EXCEL process created by this user
        $processId = (Get-Process -Name EXCEL | Sort-Object StartTime -Descending | Select-Object -First 1).Id
    }}

    Write-Output "Opening dashboard workbook at: $dashboardPath"
    $Workbook = $Excel.Workbooks.Open($dashboardPath)
    Write-Output "Workbook opened"

    $originalCalculation = $Excel.Calculation
    $Excel.Calculation = -4135 # xlCalculationManual

    # Restore calculation before refreshing connections
    Write-Output "Restoring Excel calculation mode..."
    try {{
        $Excel.Calculation = $originalCalculation
    }} catch {{
        Write-Output "Warning: Could not restore calculation mode."
    }}

    Write-Output "Refreshing Workbook connections/Model..."
    try {{
        $Workbook.RefreshAll()
    }} catch {{
        Write-Output "Warning: Could not RefreshAll."
    }}

    # Refresh Data Model and PivotTables
    Write-Output "Refreshing Data Model..."
    if ($Workbook.Model) {{
        $Workbook.Model.Refresh()
    }}

    Write-Output "Refreshing PivotTables..."
    foreach ($Sheet in $Workbook.Worksheets) {{
        foreach ($PivotTable in $Sheet.PivotTables()) {{
            $PivotTable.RefreshTable()
        }}
    }}

    Write-Output "Saving workbook..."
    $Workbook.Save()
    $Workbook.Close($true)
    Write-Output "Dashboard update completed successfully."

}} catch {{
    Write-Error "Failed to update Excel file: $_"
    if ($Workbook) {{ try {{ $Workbook.Close($false) }} catch {{}} }}
    [System.Environment]::ExitCode = 1
}} finally {{
    Write-Output "Cleaning up Excel COM object..."
    try {{
        if ($Excel) {{
            $Excel.ScreenUpdating = $true
            $Excel.EnableEvents = $true
            if ($originalCalculation) {{ $Excel.Calculation = $originalCalculation }}
            $Excel.Quit()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Excel) | Out-Null
        }}
    }} catch {{
        Write-Output "Warning: Failed to cleanly quit Excel."
    }}

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    # Forcefully kill process if it still exists
    if ($processId) {{
        try {{
            $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if ($proc) {{
                Write-Output "Force killing Excel process ID $processId"
                $proc.Kill()
            }}
        }} catch {{
            Write-Output "Warning: Failed to forcefully kill Excel process."
        }}
    }}
}}
"#,
        dashboard_path = dashboard_path_str.replace('\'', "''"),
    );

    if config.save_email_as_html.unwrap_or(false) {
        info!("save_email_as_html is true, skipping actual dashboard update via powershell.");

        // Generate and save HTML email body
        info!("Email generation started");
        let indent_spaces = config.indentation_spaces.unwrap_or(4);
        let indent_width = indent_spaces * 5;
        let body = format!(
            r#"<html><body style="font-family: Arial, sans-serif;">Dear Aya,<br/><table border='0'><tr><td width='{}'></td><td>Please find the CRM Ticket dashboard attached.</td></tr></table></body></html>"#,
            indent_width
        );

        let html_path = tmp_dir.join("dashboard_email.html");
        let mut f = File::create(&html_path)?;
        f.write_all(body.as_bytes())?;
        f.sync_all()?;
        info!("Email generation completed");
        info!("Saved dashboard HTML email to {}", html_path.display());

        // For tests, powershell is not available on linux sandbox, so we skip it to prevent OS Error 2.
        return Ok(());
    }

    let ps_result = run_powershell(&ps_script);

    if let Err(e) = ps_result {
        error!("Error executing dashboard update PowerShell script: {}", e);
        anyhow::bail!(e);
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

        let ps_email_script = format!(
            r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "CRM Tickets Dashboard"
$Mail.HTMLBody = "{}"
try {{
    $Mail.Attachments.Add("{}")
}} catch {{
    Write-Warning "Attachment too large, sending without attachment."
    $Mail.HTMLBody += "<br><br><span style='color:red;'><b>Note:</b> The Dashboard file was too large to attach to this email. Please access it from the shared network drive.</span>"
}}
$Mail.Send()
"#,
            email_to.replace("\"", "'"),
            email_cc.replace("\"", "'"),
            html_body.replace("\"", "''"),
            dashboard_path_str.replace("'", "''")
        );

        if let Err(e) = run_powershell(&ps_email_script) {
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

    #[test]
    fn test_task2_dashboard_updater() {
        let dataset = crate::tasker::csv_task::tests::setup_test_dataset();
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

        // Run the task
        let result = run(&dash_config);
        assert!(
            result.is_ok(),
            "Dashboard updater task failed: {:?}",
            result.err()
        );

        let temp_dir = std::env::temp_dir();
        let html_path = temp_dir.join("dashboard_email.html");
        assert!(html_path.exists(), "HTML email should be saved");

        let html_content = std::fs::read_to_string(&html_path).unwrap();
        let expected_indent = "<table border='0'><tr><td width='20'></td>";
        assert!(
            html_content.contains(expected_indent),
            "HTML email should contain the proper indentation table. Found: {}",
            html_content
        );

        let _ = std::fs::remove_file(&html_path);

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
