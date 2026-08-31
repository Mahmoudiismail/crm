use crate::tasker::config::OpdAnalysisConfig;
use anyhow::Result;
use std::path::Path;
use tracing::info;

pub fn generate_powershell_script(
    cus_file_path: &Path,
    config: &OpdAnalysisConfig,
    email_to: &str,
    email_subject: &str,
) -> Result<String> {
    let script = format!(
        r#"
$ErrorActionPreference = "Stop"

$csvPath = "{0}"
$emailTo = "{1}"
$emailSubject = "{2}"
$specialCol = "{3}"
$dateCol = "{4}"
$checkCurrentYear = {5}

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
$excel.ScreenUpdating = $false
$excel.EnableEvents = $false

try {{
    $workbook = $excel.Workbooks.Open($csvPath)
    $ws = $workbook.Sheets.Item(1)

    # Apply AutoFilter
    if ($ws.AutoFilterMode) {{
        $ws.AutoFilterMode = $false
    }}

    $xlValues = -4123
    $xlPart = 2
    $xlByRows = 1
    $xlByColumns = 2
    $xlPrevious = 2

    $realLastRow = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByRows, $xlPrevious).Row
    $realLastCol = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByColumns, $xlPrevious).Column
    if (-not $realLastRow) {{ $realLastRow = 1 }}
    if (-not $realLastCol) {{ $realLastCol = 1 }}

    $exactRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($realLastRow, $realLastCol))
    $headers = $exactRange.Rows(1).Value2

    $realLastRow = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByRows, $xlPrevious).Row
    $realLastCol = $ws.Cells.Find("*", $ws.Cells.Item(1, 1), $xlValues, $xlPart, $xlByColumns, $xlPrevious).Column
    if (-not $realLastRow) {{ $realLastRow = 1 }}
    if (-not $realLastCol) {{ $realLastCol = 1 }}

    $exactRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($realLastRow, $realLastCol))

    $headers = $exactRange.Rows(1).Value2
    if (-not $headers) {{ throw "No headers found in CSV." }}

    # Convert 2D array to 1D
    $headerArray = @()
    for ($i = 1; $i -le $headers.GetLength(1); $i++) {{
        $headerArray += $headers[1, $i]
    }}

    $dColIdx = [array]::IndexOf($headerArray, "D") + 1
    $specialColIdx = [array]::IndexOf($headerArray, $specialCol) + 1
    $dateColIdx = [array]::IndexOf($headerArray, $dateCol) + 1

    # Apply Date format
    if ($dateColIdx -gt 0) {{
        $exactRange.Columns.Item($dateColIdx).NumberFormat = "dddd, dd mmmm yyyy"
    }}

    # Apply Visual Formatting
    # 1. Autofit all columns first so Date fits
    $exactRange.Columns.AutoFit() | Out-Null

    # 2. Force timing columns (everything after column A) to width 10
    for ($c = 2; $c -le $exactRange.Columns.Count; $c++) {{
        $exactRange.Columns.Item($c).ColumnWidth = 10
    }}

    # 3. Center alignment and borders for the entire range
    $exactRange.HorizontalAlignment = -4108 # xlCenter
    $exactRange.VerticalAlignment = -4108   # xlCenter
    $exactRange.Borders.LineStyle = 1       # xlContinuous
    $exactRange.Borders.Weight = 2          # xlThin

    # 4. Header styling (Green background, white bold text)
    $headerRange = $exactRange.Rows(1)
    $headerRange.Interior.Color = 3439443 # #548235 in BGR decimal (0x347c53) -> roughly Excel Green
    $headerRange.Font.Color = 16777215    # White
    $headerRange.Font.Bold = $true

    $dayName = (Get-Date).ToString("ddd") # "Mon", "Tue"

    # Turn on Autofilter first to establish the dropdowns
    $exactRange.AutoFilter() | Out-Null

    # Iterate through all columns in the range and hide AutoFilter dropdowns
    for ($i = 1; $i -le $exactRange.Columns.Count; $i++) {{
        $exactRange.AutoFilter($i, [Type]::Missing, [Type]::Missing, [Type]::Missing, $false) | Out-Null
    }}

    # Apply Filters and hide their specific dropdowns
    if ($dColIdx -gt 0) {{
        $exactRange.AutoFilter($dColIdx, $dayName, 1, [Type]::Missing, $false) | Out-Null
    }}

    if ($specialColIdx -gt 0) {{
        $exactRange.AutoFilter($specialColIdx, "=", 1, [Type]::Missing, $false) | Out-Null
    }}

    if ($checkCurrentYear -and $dateColIdx -gt 0) {{
        $yearStart = Get-Date -Year (Get-Date).Year -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0
        $yearEnd = $yearStart.AddYears(1)
        # Dates in Excel are often represented as numbers or strings depending on CSV load
        # For CSV loaded into Excel, standard > < text filter works if dates are formatted yyyy-mm-dd
        $exactRange.AutoFilter($dateColIdx, ">=$($yearStart.ToString('yyyy-MM-dd'))", 1, "<$($yearEnd.ToString('yyyy-MM-dd'))", $false) | Out-Null
    }}

    # Iterate through all columns in the range and hide AutoFilter dropdowns
    for ($i = 1; $i -le $exactRange.Columns.Count; $i++) {{
        $exactRange.AutoFilter($i, [Type]::Missing, [Type]::Missing, [Type]::Missing, $false) | Out-Null
    }}

    $visibleRows = $exactRange.SpecialCells(12) # xlCellTypeVisible

    # Find last visible row
    $lastRow = 1
    foreach ($area in $visibleRows.Areas) {{
        $areaLastRow = $area.Row + $area.Rows.Count - 1
        if ($areaLastRow -gt $lastRow) {{
            $lastRow = $areaLastRow
        }}
    }}

    # Hide blank columns at last row
    for ($c = 1; $c -le $exactRange.Columns.Count; $c++) {{
        $val = $ws.Cells.Item($lastRow, $c).Text
        if ([string]::IsNullOrWhiteSpace($val)) {{
            $ws.Columns.Item($c).Hidden = $true
        }}
    }}

    # Hide D column
    if ($dColIdx -gt 0) {{
        $ws.Columns.Item($dColIdx).Hidden = $true
    }}

    $imagePath = Join-Path $env:TEMP "Query1.jpg"
    if (Test-Path $imagePath) {{ Remove-Item $imagePath -Force }}

    $copyRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($lastRow, $exactRange.Columns.Count))
    $copyRange.CopyPicture(1, 2) | Out-Null # xlScreen=1, xlPicture=2

    Start-Sleep -Seconds 1

    # Find exact width and height of visible range to size the chart appropriately
    $totalWidth = 0
    $totalHeight = 0
    for ($c = 1; $c -le $exactRange.Columns.Count; $c++) {{
        if (-not $ws.Columns.Item($c).Hidden) {{
            $totalWidth += $ws.Columns.Item($c).Width
        }}
    }}
    for ($r = 1; $r -le $lastRow; $r++) {{
        if (-not $ws.Rows.Item($r).Hidden) {{
            $totalHeight += $ws.Rows.Item($r).Height
        }}
    }}

    if ($totalWidth -lt 50) {{ $totalWidth = 50 }}
    if ($totalHeight -lt 50) {{ $totalHeight = 50 }}

    $copyRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($lastRow, $exactRange.Columns.Count))

    # Select the range explicitly to ensure clipboard receives data
    $ws.Activate()
    $copyRange.Select() | Out-Null

    # Temporarily enable ScreenUpdating so Excel can render the xlScreen copy
    $excel.ScreenUpdating = $true

    # 1 = xlScreen, 2 = xlBitmap
    $copyRange.CopyPicture(1, 2) | Out-Null
    Start-Sleep -Seconds 1

    $chartObj = $ws.ChartObjects().Add(10, 10, $totalWidth, $totalHeight)
    $chartObj.Activate()

    # Select ChartArea to ensure Paste targets the chart
    $chartObj.Chart.ChartArea.Select() | Out-Null
    $chartObj.Chart.Paste() | Out-Null
    Start-Sleep -Seconds 1

    $chartObj.Chart.Export($imagePath, "JPG") | Out-Null
    $chartObj.Delete()

    if (Test-Path $imagePath) {{
        $outlook = New-Object -ComObject Outlook.Application
        $mail = $outlook.CreateItem(0)
        $mail.To = $emailTo
        $mail.Subject = $emailSubject
        $mail.Attachments.Add($imagePath) | Out-Null
        $mail.Send()
        Write-Output "Email sent successfully."
    }} else {{
        Write-Error "Failed to generate image from filtered table."
    }}

}} finally {{
    if ($workbook) {{ $workbook.Close($false) }}
    $excel.Quit()
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
}}
"#,
        cus_file_path
            .canonicalize()
            .unwrap_or_else(|_| cus_file_path.to_path_buf())
            .to_string_lossy()
            .replace(r"\\?\", ""),
        email_to,
        email_subject,
        config.special_column_name,
        config.date_column_name,
        if config.check_current_year {
            "$true"
        } else {
            "$false"
        }
    );
    Ok(script)
}

pub fn generate_and_email_image(cus_file_path: &Path, config: &OpdAnalysisConfig) -> Result<()> {
    if let (Some(email_to), Some(email_subject)) = (&config.email_to, &config.email_subject) {
        let ps_script = generate_powershell_script(cus_file_path, config, email_to, email_subject)?;

        let mut temp_file = tempfile::Builder::new()
            .prefix("opd_analysis_email_")
            .suffix(".ps1")
            .tempfile()?;

        use std::io::Write;
        temp_file.write_all(ps_script.as_bytes())?;
        temp_file.as_file().sync_all()?;

        let (file, path) = temp_file.keep()?;
        drop(file);

        info!("Running PowerShell for generating and emailing image...");
        let output = std::process::Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("PowerShell script failed:\n{}", stderr);
            anyhow::bail!("Failed to generate/email image. PS Error: {}", stderr);
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("PowerShell email success: {}", stdout);
        }

        let _ = std::fs::remove_file(path);
    }

    Ok(())
}

#[cfg(test)]
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

    #[test]
    fn test_autofilter_dropdown_hidden() {
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

        // Ensure the code loops through the columns to hide AutoFilter dropdowns
        assert!(
            script.contains("for ($i = 1; $i -le $exactRange.Columns.Count; $i++) {\n        $exactRange.AutoFilter($i, [Type]::Missing, [Type]::Missing, [Type]::Missing, $false) | Out-Null\n    }"),
            "Script does not contain the loop to hide AutoFilter dropdowns"
        );
    }
}
