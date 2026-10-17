use crate::tasker::config::OpdAnalysisConfig;
use anyhow::Result;
use std::path::Path;
use tracing::info;

pub fn get_opd_script_template() -> &'static str {
    r#" 
param(
    [string]$CsvPath,
    [string]$EmailTo,
    [string]$EmailSubject,
    [string]$SpecialCol,
    [string]$DateCol,
    [string]$CheckCurrentYear
)

$ErrorActionPreference = "Stop"

$checkCurrentYear = if ($CheckCurrentYear -eq "true") { $true } else { $false }

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $true
$excel.WindowState = -4140 # xlMinimized
$excel.DisplayAlerts = $false
$excel.ScreenUpdating = $false
$excel.EnableEvents = $false

try {
    Write-Output "TRACE: Opening CSV workbook: $CsvPath"
    $workbook = $excel.Workbooks.Open($CsvPath)
    $ws = $workbook.Sheets.Item(1)

    if (-not $ws.AutoFilterMode) {
        $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item(1, $ws.UsedRange.Columns.Count)).AutoFilter() | Out-Null
    }

    $xlValues = -4123; $xlPart = 2; $xlByRows = 1; $xlByColumns = 2; $xlPrevious = 2

    $realLastRow = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByRows, $xlPrevious).Row
    $realLastCol = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByColumns, $xlPrevious).Column
    if (-not $realLastRow) { $realLastRow = 1 }
    if (-not $realLastCol) { $realLastCol = 1 }

    $exactRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($realLastRow, $realLastCol))
    $exactRange.Range($exactRange.Cells.Item(1, 1), $exactRange.Cells.Item($realLastRow, 1)).NumberFormat = "dddd, dd mmmm yyyy"

    $headers = $exactRange.Rows(1).Value2
    $specialColIdx = -1
    $dateColIdx = -1
    $dayColIdx = -1

    $exactRange.Rows.AutoFit() | Out-Null

    if ($headers -is [System.Array]) {
        for ($c = 1; $c -le $headers.GetLength(1); $c++) {
            $colName = [string]$headers.GetValue(1, $c)
            if (-not [string]::IsNullOrWhiteSpace($colName)) {
                $cleanColName = $colName.Trim().ToLower()
                if ($cleanColName -eq $SpecialCol.Trim().ToLower()) { $specialColIdx = $c }
                if ($cleanColName -eq $DateCol.Trim().ToLower()) { $dateColIdx = $c }
                if ($cleanColName -eq "d") { $dayColIdx = $c }
            }
        }
    }

    if ($specialColIdx -gt 0) {
        Write-Output "TRACE: Applying Special Column Filter"
     #   $exactRange.AutoFilter($specialColIdx, "<>0", 1, [Type]::Missing, $false) | Out-Null
    }

    if ($checkCurrentYear -and $dateColIdx -gt 0) {
        Write-Output "TRACE: Applying Date Filter"
        $currentYear = (Get-Date).Year
        $startDate = Get-Date -Year $currentYear -Month 1 -Day 1 
        $endDate = Get-Date -Year $currentYear -Month 12 -Day 31
        $exactRange.AutoFilter($dateColIdx, ">=$($startDate.ToOADate())", 1, "<=$($endDate.ToOADate())", $false) | Out-Null
    }
    
    if ($dayColIdx -gt 0) {
        $todayDat = Get-Date -Format "ddd"
        $exactRange.AutoFilter($dayColIdx, $todayDat, 1, [Type]::Missing, $false) | Out-Null
    }

    Write-Output "TRACE: Forcing filter icons to hide..."
    for ($col = 1; $col -le $exactRange.Columns.Count; $col++) {
        try {
            $filter = $ws.AutoFilter.Filters.Item($col)
            if ($filter -and $filter.On) {
                $c1 = [Type]::Missing; $op = 1; $c2 = [Type]::Missing
                try { $c1 = $filter.Criteria1 } catch {}
                try { $op = $filter.Operator } catch {}
                try { $c2 = $filter.Criteria2 } catch {}

                if ($null -ne $c1 -and $c1 -ne [Type]::Missing -and $null -ne $c2 -and $c2 -ne [Type]::Missing) {
                    $exactRange.AutoFilter($col, $c1, $op, $c2, $false) | Out-Null
                } elseif ($null -ne $c1 -and $c1 -ne [Type]::Missing) {
                    $exactRange.AutoFilter($col, $c1, $op, [Type]::Missing, $false) | Out-Null
                }
            } else {
                $exactRange.AutoFilter($col, [Type]::Missing, 1, [Type]::Missing, $false) | Out-Null
            }
        } catch { }
    }

    $exactRange.ColumnWidth = 15
    $exactRange.Columns.Item(1).ColumnWidth = 30

    $visibleRows = $exactRange.SpecialCells(12) 
    $lastRow = 1
    foreach ($area in $visibleRows.Areas) {
        $areaLastRow = $area.Row + $area.Rows.Count - 1
        if ($areaLastRow -gt $lastRow) { $lastRow = $areaLastRow }
    }

    Write-Output "TRACE: Hiding blank columns at last row"
    for ($c = 1; $c -le $realLastCol; $c++) {
        $cellVal = $ws.Cells.Item($lastRow, $c).Text
        if ([string]::IsNullOrWhiteSpace($cellVal) -or $cellVal.Trim() -eq "0") {
            $ws.Columns.Item($c).Hidden = $true
        }
    }

    if ($dayColIdx -gt 0) {
        Write-Output "TRACE: Ensuring Column 'D' is hidden"
        $ws.Columns.Item($dayColIdx).Hidden = $true
    }

    $copyRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($lastRow, $exactRange.Columns.Count))

    Write-Output "TRACE: Applying styles to the table"
    $copyRange.Interior.ColorIndex = 0
    $copyRange.Font.ColorIndex = 1
    $copyRange.Font.Bold = $false

    $xlContinuous = 1
    $xlThin = 2
    for ($b = 7; $b -le 12; $b++) {
        $border = $copyRange.Borders.Item($b)
        $border.LineStyle = $xlContinuous
        $border.Weight = $xlThin
        $border.ColorIndex = 1 
    }

    $copyRange.HorizontalAlignment = -4108 # xlCenter
    $copyRange.VerticalAlignment = -4108   # xlCenter

    $headerRow = $copyRange.Rows.Item(1)
    $headerRow.Interior.Color = 3506772 
    $headerRow.Font.Color = 16777215 
    $headerRow.Font.Bold = $true

    # Wake up Excel's renderer right before copying
    $excel.ScreenUpdating = $true

    # CRITICAL: Allow the Windows Desktop Window Manager time to paint the data cells
    Start-Sleep -Milliseconds 1000 

    Write-Output "TRACE: Copying table as picture..."
    $copyRange.CopyPicture(1, 2) | Out-Null

    # CRITICAL: Allow the clipboard time to receive the image memory
    Start-Sleep -Milliseconds 1000

    $chartObj = $ws.ChartObjects().Add(10, 10, $copyRange.Width, $copyRange.Height)
    $chart = $chartObj.Chart
    
    Start-Sleep -Milliseconds 500
    
    $chart.ChartArea.Select() | Out-Null
    
    # Native COM retry loop - bypassing the problematic .NET Forms clipboard check entirely
    $pasted = $false
    for ($i = 0; $i -lt 10; $i++) {
        try {
            $chart.Paste() | Out-Null
            $pasted = $true
            break
        } catch {
            Write-Output "TRACE: Chart Paste delayed. Retrying..."
            Start-Sleep -Milliseconds 500
        }
    }

    if (-not $pasted) {
        throw "Failed to extract image via clipboard. Native Chart Paste timed out."
    }
 
    Start-Sleep -Seconds 1
 
    $imagePath = Join-Path $env:TEMP "Query1.jpg"
    $chart.Export($imagePath, "JPG") | Out-Null
 
    $chartObj.Delete()
 
    if (-not (Test-Path $imagePath)) {
        throw "Image export failed."
    }

    Write-Output "TRACE: Initializing Outlook Mail Item..."
    $outlook = New-Object -ComObject Outlook.Application
    $mail = $outlook.CreateItem(0)
    $mail.To = $EmailTo
    $mail.Subject = $EmailSubject

    Write-Output "TRACE: Attaching image to email..."
    $mail.Attachments.Add($imagePath) | Out-Null

    $mail.Send()
    Write-Output "TRACE: Email sent successfully."

} finally {
    if ($workbook) { $workbook.Close($false) }
    if ($excel) { $excel.Quit() }
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
"#
}

pub fn generate_powershell_script(
    _cus_file_path: &Path,
    _config: &OpdAnalysisConfig,
    _email_to: &str,
    _email_subject: &str,
) -> Result<String> {
    Ok(get_opd_script_template().to_string())
}

pub fn generate_and_email_image(cus_file_path: &Path, config: &OpdAnalysisConfig) -> Result<()> {
    if let (Some(email_to), Some(email_subject)) = (&config.email_to, &config.email_subject) {
        let template = get_opd_script_template();

        let special_col = config.special_column_name.clone();
        let date_col = config.date_column_name.clone();
        let check_current_year_str = if config.check_current_year {
            "true"
        } else {
            "false"
        };
        let cus_path_str = cus_file_path.to_string_lossy().to_string();

        let script_manager = crate::tasker::script_manager::ScriptManager::new();
        let script_path = script_manager.get_or_create_script(
            "OPD Analysis",
            "opd_analysis_email.ps1",
            template,
        )?;

        info!("Running PowerShell for generating and emailing image...");
        script_manager.execute_script_with_args(
            &script_path,
            &[
                ("-CsvPath", &cus_path_str),
                ("-EmailTo", email_to),
                ("-EmailSubject", email_subject),
                ("-SpecialCol", &special_col),
                ("-DateCol", &date_col),
                ("-CheckCurrentYear", check_current_year_str),
            ],
        )?;
    }

    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_opd_template_does_not_repeat_last_cell_scan() {
        let script = get_opd_script_template();
        assert_eq!(script.matches("$ws.Cells.Find(\"*\",").count(), 2);
        assert_eq!(script.matches("$exactRange = $ws.Range(").count(), 1);
    }

    #[test]
    fn test_opd_template_applies_manual_formatting_and_attaches_exported_image() {
        let script = get_opd_script_template();
        assert!(script.contains("$excel.Visible = $true"));
        assert!(script.contains("$excel.WindowState = -4140"));
        assert!(script.contains("$dayColIdx = -1"));
        assert!(script
            .contains("$exactRange.AutoFilter($dayColIdx, $todayDat, 1, [Type]::Missing, $false)"));
        assert!(script.contains("$chart.Export($imagePath, \"JPG\")"));
        assert!(script.contains("$mail.Attachments.Add($imagePath)"));
        assert!(!script.contains("$inspector.WordEditor"));
    }

    #[test]
    fn test_regression_copy_picture_bounds() {
        let config = OpdAnalysisConfig {
            download_path: "".to_string(),
            cus_input: "".to_string(),
            cus_file: "".to_string(),
            exclude_specialities: vec![],
            exclude_emp_names: vec![],
            exclude_depts: vec![],
            exclude_speciality_prefixes: vec![],
            email_to: Some("test@example.com".to_string()),
            email_subject: Some("Test".to_string()),
            special_column_name: "Special".to_string(),
            date_column_name: "KSA Time".to_string(),
            check_current_year: false,
        };

        let script = generate_powershell_script(
            &PathBuf::from("dummy.csv"),
            &config,
            "test@example.com",
            "Test Subject",
        )
        .unwrap();

        // The old buggy code would just have $usedRange.CopyPicture(1, 2)
        // The fix should replace it with a bounded range explicitly using $ws.Range and $exactRange
        assert!(
            script.contains("$copyRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($lastRow, $exactRange.Columns.Count))"),
            "Script does not bound the copy range using $copyRange"
        );
        assert!(
            script.contains("$copyRange.CopyPicture(1, 2)"),
            "Script does not copy the bounded range"
        );

        // Ensure the bug (unbounded copy) is completely removed
        assert!(
            !script.contains("$usedRange.CopyPicture(1, 2)"),
            "Script still contains unbounded $usedRange.CopyPicture"
        );
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod opd_autofilter_tests {
    use super::*;

    #[test]
    fn test_regression_autofilter_visible_dropdown_hidden() {
        let src = include_str!("powershell_email.rs");

        // Verify that the code iterates through columns and preserves existing filters

        assert!(
            src.contains("Forcing filter icons to hide..."),
            "Should contain trace logging for hiding filter icons"
        );

        // Check that AutoFilterMode is not indiscriminately disabled
        let bad_mode_set = format!("{}{} = $false", "$ws.AutoFilter", "Mode");
        assert!(
            !src.contains(&bad_mode_set),
            "Should not blindly clear AutoFilterMode which destroys existing filters"
        );

        // Check for extraction and reapplication of existing criteria
        assert!(
            src.contains("$filter = $ws.AutoFilter.Filters.Item($col)"),
            "Should check existing filters"
        );
        assert!(
            src.contains("$exactRange.AutoFilter($col, $c1, $op, $c2, $false)"),
            "Should reapply exact criteria with VisibleDropDown = $false"
        );
    }
}
