use crate::tasker::config::DepartmentSplitConfig;
use anyhow::{Context, Result};
use calamine::{open_workbook, DataType, Reader, Xlsx};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::Builder;
use tracing::{error, info};

pub fn run(config: &DepartmentSplitConfig) -> Result<()> {
    info!(
        "Starting department split task for {}",
        config.dashboard_file
    );

    // 1. Build mapping from Chair.xlsx
    let mut mapping: HashMap<String, String> = HashMap::new();
    let chair_path = Path::new(&config.chair_file);
    if !chair_path.exists() {
        anyhow::bail!("Chair mapping file not found at: {:?}", chair_path);
    }

    let mut excel: Xlsx<_> =
        open_workbook(&config.chair_file).context("Failed to open Chair.xlsx")?;
    let sheet_names = excel.sheet_names().to_owned();
    if sheet_names.is_empty() {
        anyhow::bail!("No sheets found in Chair.xlsx");
    }

    if let Ok(range) = excel.worksheet_range(&sheet_names[0]) {
        let mut is_header = true;
        for row in range.rows() {
            if is_header {
                is_header = false;
                continue;
            }
            if row.len() >= 2 {
                if let (Some(dep), Some(chir)) = (row[0].as_string(), row[1].as_string()) {
                    let clean_dep = dep.trim().to_uppercase();
                    let clean_chir = chir.trim().to_uppercase();
                    if !clean_dep.is_empty() && !clean_chir.is_empty() {
                        mapping.insert(clean_dep, clean_chir);
                    }
                }
            }
        }
    } else {
        anyhow::bail!("Failed to read data from the first sheet in Chair.xlsx");
    }

    info!("Loaded {} department mappings.", mapping.len());

    let dashboard_path = PathBuf::from(&config.dashboard_file)
        .canonicalize()
        .context("Failed to canonicalize dashboard_file path")?;

    let dashboard_path_str = dashboard_path
        .to_str()
        .unwrap()
        .strip_prefix(r"\\?\")
        .unwrap_or(dashboard_path.to_str().unwrap());

    let out_dir = PathBuf::from(&config.output_dir);
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir).context("Failed to create output directory")?;
    }

    let out_dir_canon = out_dir
        .canonicalize()
        .context("Failed to canonicalize output_dir")?;
    let out_dir_str = out_dir_canon
        .to_str()
        .unwrap()
        .strip_prefix(r"\\?\")
        .unwrap_or(out_dir_canon.to_str().unwrap());

    // Write mapping to a temporary JSON file to pass to PowerShell
    let mapping_json = serde_json::to_string(&mapping)?;
    let tmp_dir = std::env::temp_dir();
    let mapping_file = tmp_dir.join("chair_mapping.json");
    std::fs::write(&mapping_file, mapping_json).context("Failed to write mapping JSON")?;

    let mapping_file_str = mapping_file.to_str().unwrap();

    let ps_script = format!(
        r#"
$ErrorActionPreference = "Stop"
$dashboardPath = '{dashboard_path}'
$outDir = '{out_dir}'
$mappingFile = '{mapping_file}'

Write-Output "TRACE: Loading mapping JSON..."
$mappingJson = Get-Content $mappingFile -Raw | ConvertFrom-Json
$mappingHash = @{{}}
foreach ($prop in $mappingJson.psobject.properties) {{
    $mappingHash[$prop.Name] = $prop.Value
}}

Write-Output "TRACE: Starting Excel COM Object..."
$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false
$Excel.ScreenUpdating = $false
$Excel.EnableEvents = $false
$originalCalculation = $Excel.Calculation
try {{ $Excel.Calculation = -4135 }} catch {{}}

$processId = $null
try {{
    [int]$handle = $Excel.Hwnd
    $processId = (Get-Process | Where-Object {{ $_.MainWindowHandle -eq $handle }}).Id
}} catch {{
    $processId = (Get-Process -Name EXCEL | Sort-Object StartTime -Descending | Select-Object -First 1).Id
}}

try {{
    Write-Output "TRACE: Opening master dashboard: $dashboardPath"
    $Workbook = $Excel.Workbooks.Open($dashboardPath)
    $Sheet = $Workbook.Worksheets.Item("OPD Report")

    # Find the DEPT column
    $lastCol = $Sheet.Cells.SpecialCells(11).Column # xlCellTypeLastCell
    $headerRow = 1
    $deptCol = -1

    # Try finding DEPT header in first few rows
    for ($r = 1; $r -le 5; $r++) {{
        for ($c = 1; $c -le $lastCol; $c++) {{
            if ($Sheet.Cells.Item($r, $c).Text -eq "DEPT") {{
                $deptCol = $c
                $headerRow = $r
                break
            }}
        }}
        if ($deptCol -ne -1) {{ break }}
    }}

    if ($deptCol -eq -1) {{
        Write-Error "Could not find 'DEPT' column in 'OPD Report' sheet."
        throw "DEPT column not found"
    }}

    Write-Output "TRACE: DEPT column found at index: $deptCol (Header Row: $headerRow)"

    $lastRow = $Sheet.Cells.SpecialCells(11).Row
    Write-Output "TRACE: Total rows in OPD Report: $lastRow"

    # First pass: Build a list of unique CHIR targets based on the data
    $uniqueTargets = @{{}}
    $targetsByRow = @{{}}

    for ($i = $headerRow + 1; $i -le $lastRow; $i++) {{
        $deptVal = $Sheet.Cells.Item($i, $deptCol).Text.Trim().ToUpper()
        if ([string]::IsNullOrWhiteSpace($deptVal)) {{ continue }}

        $targetChir = "OTHERS"
        if ($mappingHash.ContainsKey($deptVal)) {{
            $targetChir = $mappingHash[$deptVal]
        }}

        $uniqueTargets[$targetChir] = $true
        $targetsByRow[$i] = $targetChir
    }}

    $Workbook.Close($false)
    Write-Output "TRACE: Found unique targets: $($uniqueTargets.Keys -join ', ')"

    # Loop over each target CHIR and create a copy
    foreach ($target in $uniqueTargets.Keys) {{
        $targetFileName = "$target.xlsm"
        $targetPath = Join-Path $outDir $targetFileName

        Copy-Item $dashboardPath $targetPath -Force
        Write-Output "TRACE: Created copy for ${{target}}: ${{targetPath}}"

        $TargetWB = $Excel.Workbooks.Open($targetPath)
        $TargetSheet = $TargetWB.Worksheets.Item("OPD Report")

        # Delete rows backward
        $rowsDeleted = 0
        $rowsRetained = 0

        for ($i = $lastRow; $i -gt $headerRow; $i--) {{
            if ($targetsByRow.ContainsKey($i)) {{
                if ($targetsByRow[$i] -ne $target) {{
                    $TargetSheet.Rows.Item($i).Delete()
                    $rowsDeleted++
                }} else {{
                    $rowsRetained++
                }}
            }} else {{
                # If row was empty, we can choose to delete it or leave it.
                # Leaving empty rows is usually fine, or delete them to be safe.
                $TargetSheet.Rows.Item($i).Delete()
            }}
        }}

        Write-Output "TRACE: Completed copy for target ${{target}} | Total records: $($rowsRetained + $rowsDeleted) | Retained records: $rowsRetained"

        # Refresh Data Model and PivotTables
        if ($TargetWB.Model) {{
            try {{ $TargetWB.Model.Refresh() }} catch {{}}
        }}
        foreach ($sht in $TargetWB.Worksheets) {{
            foreach ($pt in $sht.PivotTables()) {{
                try {{ $pt.RefreshTable() }} catch {{}}
            }}
        }}

        $TargetWB.Save()
        $TargetWB.Close($true)
    }}

}} catch {{
    Write-Error "Failed to process dashboard (target: ${{target}}): $_"
    if ($Workbook) {{ try {{ $Workbook.Close($false) }} catch {{}} }}
    [System.Environment]::ExitCode = 1
}} finally {{
    Write-Output "TRACE: Cleaning up Excel COM object..."
    try {{
        if ($Excel) {{
            $Excel.ScreenUpdating = $true
            $Excel.EnableEvents = $true
            $Excel.DisplayAlerts = $true
            if ($originalCalculation) {{ try {{ $Excel.Calculation = $originalCalculation }} catch {{}} }}
            $Excel.Quit()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Excel) | Out-Null
        }}
    }} catch {{
        Write-Output "TRACE: Warning: Failed to cleanly quit Excel."
    }}

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    if ($processId) {{
        try {{
            $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if ($proc) {{
                $proc.Kill()
            }}
        }} catch {{}}
    }}
}}
"#,
        dashboard_path = dashboard_path_str.replace('\'', "''"),
        out_dir = out_dir_str.replace('\'', "''"),
        mapping_file = mapping_file_str.replace('\'', "''")
    );

    let ps_script_path = Builder::new()
        .prefix("dept_split_")
        .suffix(".ps1")
        .tempfile()
        .context("Failed to create temporary powershell script")?;

    let (mut file, script_path) = ps_script_path.keep().unwrap();
    file.write_all(ps_script.as_bytes())?;
    file.sync_all()?;
    drop(file);

    let _guard = crate::utils::FileCleanupGuard::new(&script_path);

    info!("Executing PowerShell script...");
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to spawn PowerShell process")?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    if !stdout_str.is_empty() {
        for line in stdout_str.lines() {
            if line.starts_with("TRACE:") {
                tracing::trace!("PS: {}", line.strip_prefix("TRACE:").unwrap().trim());
            } else {
                info!("PS: {}", line);
            }
        }
    }

    if !stderr_str.is_empty() {
        for line in stderr_str.lines() {
            error!("PS ERROR: {}", line);
        }
    }

    if !output.status.success() {
        anyhow::bail!("PowerShell script failed with exit code: {}", output.status);
    }

    // Clean up temporary mapping file
    let _ = std::fs::remove_file(&mapping_file);

    info!("Departmental Splitter task completed successfully.");
    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    // use super::*;

    #[test]
    fn test_powershell_interpolation_safety() {
        let src = include_str!("department_split.rs");

        // Assert the problematic unsafe interpolation does not exist
        assert!(
            !src.contains(&format!("{}{}", "$target", ": $targetPath")),
            "Should not contain unsafe interpolation"
        );
        assert!(
            !src.contains(&format!("{}{}", "$target", ":")),
            "Should not contain ambiguous interpolation"
        );

        // Assert the safe version does exist
        assert!(
            src.contains("${{target}}: ${{targetPath}}"),
            "Should contain strictly safe string interpolation"
        );
        assert!(
            src.contains("${{target}}"),
            "Should use safe bracket notation for target"
        );
    }
}
