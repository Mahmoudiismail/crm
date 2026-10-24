
param(
    [string]$CsvPath,
    [string]$OutlookHelperPath,
    [string]$EmailTo,
    [string]$EmailSubject,
    [string]$SpecialCol,
    [string]$DateCol,
    [string]$CheckCurrentYear
)

$ErrorActionPreference = "Stop"

$checkCurrentYear = if ($CheckCurrentYear -eq "true") { $true } else { $false }

$EmailTo = if ($null -eq $EmailTo) { "" } else { $EmailTo.Trim() }
Write-Output ("TRACE: Email recipient configured=" + (-not [string]::IsNullOrWhiteSpace($EmailTo)) + "; length=" + $EmailTo.Length)
if ([string]::IsNullOrWhiteSpace($EmailTo)) {
    throw "EmailTo is empty. Configure a recipient for the OPD Analysis task before sending."
}

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
 
    $imagePath = Join-Path $env:TEMP ("Query1_" + [guid]::NewGuid().ToString("N") + ".jpg")
    $chart.Export($imagePath, "JPG") | Out-Null
 
    $chartObj.Delete()
 
    if (-not (Test-Path $imagePath)) {
        throw "Image export failed."
    }

    Write-Output "TRACE: Sending exported image through shared Outlook function..."
    . $OutlookHelperPath
    Send-OutlookMail -Mode "New" -To $EmailTo -Subject $EmailSubject -AttachmentPaths @($imagePath) -DisplayOrSend "Send()"
    Write-Output "TRACE: Email sent successfully."

} catch {
    Write-Error "OPD processing or email sending failed: $_"
    exit 1
} finally {
    if ($imagePath -and (Test-Path -LiteralPath $imagePath)) {
        Remove-Item -LiteralPath $imagePath -Force -ErrorAction SilentlyContinue
    }
    if ($workbook) { $workbook.Close($false) }
    if ($excel) { $excel.Quit() }
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
