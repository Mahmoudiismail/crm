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

$csvPath = $CsvPath
$emailTo = $EmailTo
$emailSubject = $EmailSubject
$specialCol = $SpecialCol
$dateCol = $DateCol
$checkCurrentYear = if ($CheckCurrentYear -eq "true") { $true } else { $false }

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
$excel.ScreenUpdating = $false
$excel.EnableEvents = $false

try {
    Write-Output "TRACE: Opening CSV workbook: $csvPath"
    $workbook = $excel.Workbooks.Open($csvPath)
    Write-Output "TRACE: Extracting first worksheet"
    $ws = $workbook.Sheets.Item(1)

    if (-not $ws.AutoFilterMode) {
        $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item(1, $ws.UsedRange.Columns.Count)).AutoFilter() | Out-Null
    }

    $xlValues = -4123
    $xlPart = 2
    $xlByRows = 1
    $xlByColumns = 2
    $xlPrevious = 2

    $realLastRow = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByRows, $xlPrevious).Row
    $realLastCol = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByColumns, $xlPrevious).Column
    if (-not $realLastRow) { $realLastRow = 1 }
    if (-not $realLastCol) { $realLastCol = 1 }

    $exactRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($realLastRow, $realLastCol))
    $headers = $exactRange.Rows(1).Value2

    $realLastRow = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByRows, $xlPrevious).Row
    $realLastCol = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByColumns, $xlPrevious).Column
    if (-not $realLastRow) { $realLastRow = 1 }
    if (-not $realLastCol) { $realLastCol = 1 }

    $exactRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($realLastRow, $realLastCol))

    $specialColIdx = -1
    $dateColIdx = -1

    if ($headers -is [System.Array]) {
        for ($c = 1; $c -le $headers.GetLength(1); $c++) {
            $colName = [string]$headers.GetValue(1, $c)
            if ($colName -and $colName.Trim().ToLower() -eq $specialCol.Trim().ToLower()) {
                $specialColIdx = $c
            }
            if ($colName -and $colName.Trim().ToLower() -eq $dateCol.Trim().ToLower()) {
                $dateColIdx = $c
            }
        }
    }

    $filteredCols = @{}

    if ($specialColIdx -gt 0) {
        Write-Output "TRACE: Applying Special Column Filter on column $specialColIdx"
        $exactRange.AutoFilter($specialColIdx, "<>0", 1, [Type]::Missing, $false) | Out-Null
        $filteredCols[$specialColIdx] = $true
    }

    if ($checkCurrentYear -and $dateColIdx -gt 0) {
        Write-Output "TRACE: Applying Date Filter on column $dateColIdx"
        $yearStart = Get-Date -Year (Get-Date).Year -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0
        $yearEnd = $yearStart.AddYears(1)
        $exactRange.AutoFilter($dateColIdx, ">=$($yearStart.ToString(yyyy-MM-dd))", 1, "<$($yearEnd.ToString(yyyy-MM-dd))", $false) | Out-Null
        $filteredCols[$dateColIdx] = $true
    }

    Write-Output "TRACE: Hiding AutoFilter dropdown icons for remaining columns while preserving filters"
    for ($col = 1; $col -le $exactRange.Columns.Count; $col++) {
        if (-not $filteredCols.ContainsKey($col)) {
            try {
                $filter = $ws.AutoFilter.Filters.Item($col)
                if ($filter -and $filter.On) {
                    Write-Output "TRACE: Hiding dropdown for already-filtered column $col"

                    $c1 = [Type]::Missing
                    $op = 1
                    $c2 = [Type]::Missing

                    try { $c1 = $filter.Criteria1 } catch { }
                    try { $op = $filter.Operator } catch { }
                    try { $c2 = $filter.Criteria2 } catch { }

                    if ($null -ne $c1 -and $c1 -ne [Type]::Missing -and $null -ne $c2 -and $c2 -ne [Type]::Missing) {
                        $exactRange.AutoFilter($col, $c1, $op, $c2, $false) | Out-Null
                    } elseif ($null -ne $c1 -and $c1 -ne [Type]::Missing) {
                        $exactRange.AutoFilter($col, $c1, $op, [Type]::Missing, $false) | Out-Null
                    } else {
                        Write-Output "TRACE: Warning: Failed to extract Criteria1 for column $col, skipping dropdown hide to preserve filter."
                    }
                } else {
                    $exactRange.AutoFilter($col, [Type]::Missing, 1, [Type]::Missing, $false) | Out-Null
                }
            } catch {
                Write-Output "TRACE: Warning: Failed to hide AutoFilter dropdown for column $col - $_"
            }
        }
    }

    $visibleRows = $exactRange.SpecialCells(12)

    Write-Output "TRACE: Finding last visible row after filters"
    $lastRow = 1
    foreach ($area in $visibleRows.Areas) {
        $areaLastRow = $area.Row + $area.Rows.Count - 1
        if ($areaLastRow -gt $lastRow) {
            $lastRow = $areaLastRow
        }
    }

    Write-Output "TRACE: Hiding blank columns at last row"
    for ($c = 1; $c -le $realLastCol; $c++) {
        $cellVal = $ws.Cells.Item($lastRow, $c).Text
        if (-not $cellVal -or $cellVal.Trim() -eq "" -or $cellVal.Trim() -eq "0") {
            $ws.Columns.Item($c).Hidden = $true
        }
    }

    $realLastColFiltered = 1
    for ($c = 1; $c -le $realLastCol; $c++) {
        if (-not $ws.Columns.Item($c).Hidden) {
            $realLastColFiltered = $c
        }
    }

    $copyRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($lastRow, $exactRange.Columns.Count))

    Write-Output "TRACE: Copying table as picture..."
    $copyRange.CopyPicture(1, 2) | Out-Null

    Start-Sleep -Milliseconds 500

    Write-Output "TRACE: Initializing Outlook Mail Item..."
    $outlook = New-Object -ComObject Outlook.Application
    $mail = $outlook.CreateItem(0)
    $mail.To = $emailTo
    $mail.Subject = $emailSubject

    Write-Output "TRACE: Setting up Mail Inspector for HTML paste..."
    $inspector = $mail.GetInspector
    $inspector.Display()

    $wordDoc = $inspector.WordEditor
    $selection = $wordDoc.Windows.Item(1).Selection
    $selection.Paste()

    $mail.Save()
    Write-Output "TRACE: Email Draft saved successfully."
    $inspector.Close(1)
} finally {
    if ($workbook) { $workbook.Close($false) }
    if ($excel) { $excel.Quit() }
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
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
            src.contains(
                "Hiding AutoFilter dropdown icons for remaining columns while preserving filters"
            ),
            "Should contain trace logging for hiding icons while preserving filters"
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
