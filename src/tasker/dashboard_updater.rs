use crate::tasker::config::DashboardUpdaterConfig;
use anyhow::Result;
use chrono::Local;
use std::fs::File;
use std::io::Write;
use tracing::{error, info};

use std::sync::atomic::{AtomicUsize, Ordering};

static SCRIPT_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run_powershell(script: &str) -> Result<()> {
    let tmp_dir = std::env::temp_dir();
    let count = SCRIPT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let script_path = tmp_dir.join(format!(
        "dashboard_updater_{}_{}.ps1",
        Local::now().timestamp_nanos_opt().unwrap_or(0),
        count
    ));

    let mut file = File::create(&script_path)?;
    file.write_all(script.as_bytes())?;
    file.sync_all()?;
    drop(file);

    let status = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .status()?;

    let _ = std::fs::remove_file(script_path);

    if !status.success() {
        anyhow::bail!("PowerShell script exited with status: {}", status);
    }

    Ok(())
}

pub fn run(config: &DashboardUpdaterConfig) -> Result<()> {
    info!("Starting DashboardUpdater task. Config: {:?}", config);

    let params = crate::tasker::csv_task::CsvAnalysisParams::from(config);
    let generated_csv_path_opt = crate::tasker::csv_task::generate_csv(&params)?;

    let generated_csv_path = match generated_csv_path_opt {
        Some(path) => path,
        None => {
            info!("No new tickets found. Skipping dashboard update.");
            return Ok(());
        }
    };

    let dashboard_file_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_file);
    if !dashboard_file_path.exists() {
        anyhow::bail!(
            "Dashboard file not found at: {}",
            dashboard_file_path.display()
        );
    }

    let csv_path_str = generated_csv_path.to_string_lossy().to_string();
    let dashboard_path_str = dashboard_file_path.to_string_lossy().to_string();

    info!(
        "Updating dashboard table '{}' in file '{}' with CSV data from '{}'",
        config.dashboard_table_name, dashboard_path_str, csv_path_str
    );

    let ps_script = format!(
        r#"
$ErrorActionPreference = "Stop"

$csvPath = '{csv_path}'
$dashboardPath = '{dashboard_path}'
$tableName = '{table_name}'

$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false

try {{
    $Workbook = $Excel.Workbooks.Open($dashboardPath)

    # Find the table by name across all sheets
    $foundTable = $null
    foreach ($Sheet in $Workbook.Worksheets) {{
        foreach ($ListObject in $Sheet.ListObjects) {{
            if ($ListObject.Name -eq $tableName) {{
                $foundTable = $ListObject
                break
            }}
        }}
        if ($foundTable) {{ break }}
    }}

    if (-not $foundTable) {{
        throw "Table '$tableName' not found in any worksheet."
    }}

    # Clear old data if any
    if ($foundTable.DataBodyRange) {{
        $foundTable.DataBodyRange.Delete()
    }}

    # We use QueryTables to cleanly dump the CSV, or since COM is slow but CSV is simple,
    # we can open the CSV in Excel, copy the range, and paste it. That is robust for formatting.

    $CsvWorkbook = $Excel.Workbooks.Open($csvPath)
    $CsvSheet = $CsvWorkbook.Worksheets.Item(1)

    # We assume CSV has headers, we just want to paste data?
    # Or replace the whole table.
    # The requirement: "Replace table content named 'table2'... witou any other changes"
    # The safest way is to clear data body range, copy data body from CSV, and paste.

    $UsedRange = $CsvSheet.UsedRange

    # If the CSV has data (more than just headers)
    if ($UsedRange.Rows.Count -gt 1) {{
        $SourceRange = $CsvSheet.Range($CsvSheet.Cells.Item(2, 1), $CsvSheet.Cells.Item($UsedRange.Rows.Count, $UsedRange.Columns.Count))

        # Determine top left cell of destination table's data body range
        # If DataBodyRange is null (table is empty), we insert at the cell just below header
        if ($foundTable.DataBodyRange) {{
            $DestCell = $foundTable.DataBodyRange.Cells.Item(1, 1)
        }} else {{
            $DestCell = $foundTable.HeaderRowRange.Offset(1, 0).Cells.Item(1, 1)
        }}

        $SourceRange.Copy() | Out-Null
        $DestCell.PasteSpecial(-4104) | Out-Null # xlPasteAll

        # Update Table Range to encompass the new data
        # Excel often auto-expands the table when pasting immediately below the header.
    }}

    $CsvWorkbook.Close($false)

    # Refresh PivotTables
    foreach ($Sheet in $Workbook.Worksheets) {{
        foreach ($PivotTable in $Sheet.PivotTables()) {{
            $PivotTable.RefreshTable()
        }}
    }}

    $Workbook.Save()
    $Workbook.Close($true)

}} catch {{
    Write-Error "Failed to update Excel file: $_"
    if ($Workbook) {{ $Workbook.Close($false) }}
    if ($CsvWorkbook) {{ $CsvWorkbook.Close($false) }}
    $Excel.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Excel) | Out-Null
    exit 1
}}

$Excel.Quit()
[System.Runtime.InteropServices.Marshal]::ReleaseComObject($Excel) | Out-Null
"#,
        csv_path = csv_path_str.replace('\'', "''"),
        dashboard_path = dashboard_path_str.replace('\'', "''"),
        table_name = config.dashboard_table_name.replace('\'', "''")
    );

    if let Err(e) = run_powershell(&ps_script) {
        error!("Error executing dashboard update PowerShell script: {}", e);
        anyhow::bail!(e);
    }

    info!("Successfully updated dashboard '{}'", dashboard_path_str);

    if let (Some(email_to), Some(email_cc)) = (&config.email_to, &config.email_cc) {
        info!("Sending dashboard via email to: {}", email_to);

        let ps_email_script = format!(
            r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "CRM Tickets Dashboard"
$Mail.Body = "Dear Aya,`r`nPlease find the CRM Ticket dashboard attached."
$Mail.Attachments.Add("{}")
$Mail.Send()
"#,
            email_to.replace("\"", "'"),
            email_cc.replace("\"", "'"),
            dashboard_path_str.replace("'", "''")
        );

        if let Err(e) = run_powershell(&ps_email_script) {
            error!("Failed to send dashboard email: {}", e);
            // Optionally, try a fallback email or bubble up
        } else {
            info!("Successfully sent dashboard email.");
        }
    }

    Ok(())
}
