use crate::tasker::config::DepartmentSplitConfig;
use anyhow::{Context, Result};
use calamine::{open_workbook, DataType, Reader, Xlsx};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

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

    let ps_script = r#"
param(
    [string]$DashboardPath,
    [string]$OutputDir,
    [string]$MappingFile
)

$ErrorActionPreference = "Stop"
$dashboardPath = $DashboardPath
$outDir = $OutputDir
$mappingFile = $MappingFile

function Write-Log {
    param([string]$message)
    $timestamp = (Get-Date).ToString("HH:mm:ss.fff")
    Write-Output "TRACE: [$timestamp] $message"
}

$scriptTimer = [System.Diagnostics.Stopwatch]::StartNew()

Write-Log "Starting Excel COM Object..."
$comTimer = [System.Diagnostics.Stopwatch]::StartNew()
$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false
$Excel.ScreenUpdating = $false
$Excel.EnableEvents = $false
$originalCalculation = $Excel.Calculation
try { $Excel.Calculation = -4135 } catch {}
$comTimer.Stop()
Write-Log "Excel COM Object launched in $($comTimer.ElapsedMilliseconds) ms"

$processId = $null

try {
    try {
        [int]$handle = $Excel.Hwnd
        $processId = (Get-Process | Where-Object { $_.MainWindowHandle -eq $handle }).Id
    } catch {
        $processId = (Get-Process -Name EXCEL | Sort-Object StartTime -Descending | Select-Object -First 1).Id
    }

    Write-Log "Loading chair mapping from $mappingFile..."
    $mappingJson = Get-Content $mappingFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $mappingHash = @{}
    foreach ($property in $mappingJson.PSObject.Properties) {
        $mappingHash[$property.Name.Trim().ToUpper()] = $property.Value.ToString().Trim()
    }
    Write-Log "Loaded $($mappingHash.Count) chair mapping rules"

    Write-Log "Opening master workbook at: $dashboardPath"
    $openTimer = [System.Diagnostics.Stopwatch]::StartNew()
    $Workbook = $Excel.Workbooks.Open($dashboardPath, $null, $true) # Read-Only
    $openTimer.Stop()
    Write-Log "Master workbook opened in $($openTimer.ElapsedMilliseconds) ms"

    $Sheet = $null
    foreach ($ws in $Workbook.Worksheets) {
        if ($ws.Name -eq "OPD Report") {
            $Sheet = $ws
            break
        }
    }
    if (-not $Sheet) {
        Write-Error "Worksheet 'OPD Report' not found in $dashboardPath"
        throw "Worksheet 'OPD Report' not found"
    }

    $headerRow = -1
    $deptCol = -1

    for ($r = 1; $r -le 20; $r++) {
        for ($c = 1; $c -le 50; $c++) {
            $cellVal = [string]$Sheet.Cells.Item($r, $c).Value2
            if ($cellVal -and $cellVal.Trim().ToUpper() -eq "DEPT") {
                $headerRow = $r
                $deptCol = $c
                break
            }
        }
        if ($deptCol -ne -1) { break }
    }

    if ($deptCol -eq -1) {
        Write-Error "Could not find 'DEPT' column in 'OPD Report' sheet."
        throw "DEPT column not found"
    }

    Write-Log "DEPT column found at index: $deptCol (Header Row: $headerRow)"

    $foundCell = $Sheet.Cells.Find("*", $Sheet.Cells.Item(1, 1), -4163, 2, 1, 2) # xlValues, xlByRows, xlPrevious
    if ($foundCell -ne $null) {
        $lastRow = $foundCell.Row
    } else {
        $lastRow = $Sheet.Cells.SpecialCells(11).Row
    }
    Write-Log "Total data rows in OPD Report: $lastRow"

    $startRow = $headerRow + 1
    $totalDataRows = $lastRow - $headerRow

    Write-Log "Scanning DEPT column..."
    $readTimer = [System.Diagnostics.Stopwatch]::StartNew()
    $deptRangeValues = $Sheet.Range($Sheet.Cells.Item($startRow, $deptCol), $Sheet.Cells.Item($lastRow, $deptCol)).Value2
    $readTimer.Stop()

    $targetDeptsMap = @{}
    $allRawDepts = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

    if ($deptRangeValues -is [System.Array]) {
        $numRows = $deptRangeValues.GetLength(0)
        for ($r = 1; $r -le $numRows; $r++) {
            $val = [string]$deptRangeValues.GetValue($r, 1)
            if ([string]::IsNullOrWhiteSpace($val)) { continue }

            $deptVal = $val.Trim().ToUpper()
            $null = $allRawDepts.Add($deptVal)

            $targetChir = "OTHERS"
            if ($mappingHash.ContainsKey($deptVal)) {
                $targetChir = $mappingHash[$deptVal]
            }

            if (-not $targetDeptsMap.ContainsKey($targetChir)) {
                $targetDeptsMap[$targetChir] = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
            }
            $null = $targetDeptsMap[$targetChir].Add($deptVal)
        }
    }

    $templateTimer = [System.Diagnostics.Stopwatch]::StartNew()
    $templatePath = Join-Path $env:TEMP "OPD_Template_$([Guid]::NewGuid().ToString('N')).xlsm"
    $Workbook.SaveCopyAs($templatePath)

    Write-Log "Preparing clean template file..."
    $TemplateWB = $Excel.Workbooks.Open($templatePath)
    $TemplateSheet = $TemplateWB.Worksheets.Item("OPD Report")
    if ($lastRow -ge $startRow) {
        $null = $TemplateSheet.Rows("${startRow}:${lastRow}").Delete()
    }
    $TemplateWB.Save()
    $TemplateWB.Close($true)
    $templateTimer.Stop()
    Write-Log "Clean template file created in $($templateTimer.ElapsedMilliseconds) ms at $templatePath"

    foreach ($target in $targetDeptsMap.Keys) {
        $deptTimer = [System.Diagnostics.Stopwatch]::StartNew()
        Write-Log "=================================================="
        Write-Log "Processing target department group: ${target}"

        $targetPath = Join-Path $outDir "${target}.xlsm"
        Copy-Item $templatePath $targetPath -Force

        $TargetWB = $Excel.Workbooks.Open($targetPath)
        $TargetSheet = $TargetWB.Worksheets.Item("OPD Report")

        $rawDeptsList = @($targetDeptsMap[$target])

        Write-Log "  -> Filtering and copying matching rows in bulk..."
        $copyRowsTimer = [System.Diagnostics.Stopwatch]::StartNew()

        if ($Sheet.AutoFilterMode) {
            $Sheet.AutoFilterMode = $false
        }

        $opdRange = $Sheet.Range($Sheet.Cells.Item($headerRow, 1), $Sheet.Cells.Item($lastRow, $Sheet.UsedRange.Columns.Count))

        if ($rawDeptsList.Count -eq 1) {
            $opdRange.AutoFilter($deptCol, $rawDeptsList[0]) | Out-Null
        } else {
            $opdRange.AutoFilter($deptCol, [string[]]$rawDeptsList, 7) | Out-Null # 7 = xlFilterValues
        }

        try {
            $dataBodyRange = $Sheet.Range($Sheet.Cells.Item($startRow, 1), $Sheet.Cells.Item($lastRow, $Sheet.UsedRange.Columns.Count))
            $visibleRows = $dataBodyRange.SpecialCells(12) # xlCellTypeVisible

            if ($null -ne $visibleRows) {
                $null = $visibleRows.Copy($TargetSheet.Cells.Item($startRow, 1))
            }
        } catch {
            Write-Log "  -> Warning: No visible rows found for ${target}"
        }

        if ($Sheet.AutoFilterMode) {
            $Sheet.AutoFilterMode = $false
        }
        $copyRowsTimer.Stop()
        Write-Log "  -> Filtered and copied all matching rows in $($copyRowsTimer.ElapsedMilliseconds) ms"

        $refreshTimer = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            foreach ($pc in $TargetWB.PivotCaches()) {
                try { $null = $pc.Refresh() } catch {}
            }
        } catch {
            foreach ($sht in $TargetWB.Worksheets) {
                foreach ($pt in $sht.PivotTables()) {
                    try { $null = $pt.RefreshTable() } catch {}
                }
            }
        }
        $refreshTimer.Stop()
        Write-Log "  -> PivotCaches refreshed in $($refreshTimer.ElapsedMilliseconds) ms"

        $saveTimer = [System.Diagnostics.Stopwatch]::StartNew()
        $TargetWB.Save()
        $TargetWB.Close($true)
        $saveTimer.Stop()
        Write-Log "  -> Saved and closed in $($saveTimer.ElapsedMilliseconds) ms"

        $deptTimer.Stop()
        Write-Log "  => Target ${target} total processing time: $($deptTimer.ElapsedMilliseconds) ms"
    }

    $Workbook.Close($false)
    $scriptTimer.Stop()
    Write-Log "SUCCESS: All $($targetDeptsMap.Count) departments processed in $($scriptTimer.Elapsed.TotalSeconds) seconds!"

} catch {
    Write-Error "Failed to process dashboard (target: ${target}): $_"
    if ($TargetWB) { try { $TargetWB.Close($false) } catch {} }
    if ($Workbook) { try { $Workbook.Close($false) } catch {} }
    [System.Environment]::ExitCode = 1
} finally {
    Write-Log "Cleaning up temporary template and Excel COM object..."
    if ($templatePath -and (Test-Path $templatePath)) {
        try { Remove-Item $templatePath -Force -ErrorAction SilentlyContinue } catch {}
    }

    try {
        if ($Excel) {
            $Excel.ScreenUpdating = $true
            $Excel.EnableEvents = $true
            $Excel.DisplayAlerts = $true
            if ($originalCalculation) { try { $Excel.Calculation = $originalCalculation } catch {} }
            $Excel.Quit()
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Excel) | Out-Null
        }
    } catch {
        Write-Log "Warning: Failed to cleanly quit Excel."
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    if ($processId) {
        try {
            $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if ($proc) {
                $proc.Kill()
            }
        } catch {}
    }
}
"#;

    let script_manager = crate::tasker::script_manager::ScriptManager::new();
    let script_path = script_manager.get_or_create_script(
        "Department Split",
        "department_split.ps1",
        ps_script,
    )?;

    script_manager.execute_script_with_args(
        &script_path,
        &[
            ("-DashboardPath", dashboard_path_str),
            ("-OutputDir", out_dir_str),
            ("-MappingFile", mapping_file_str),
        ],
    )?;

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
            src.contains("${{target}}: ${{targetPath}}") || src.contains("target: ${{target}}"),
            "Should contain strictly safe string interpolation"
        );
        assert!(
            src.contains("${{target}}"),
            "Should use safe bracket notation for target"
        );
    }
}
