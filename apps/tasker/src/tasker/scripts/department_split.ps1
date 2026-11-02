
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

    $headerValues = $Sheet.Range($Sheet.Cells.Item(1, 1), $Sheet.Cells.Item(20, 50)).Value2
    for ($r = 1; $r -le 20 -and $deptCol -eq -1; $r++) {
        for ($c = 1; $c -le 50; $c++) {
            $cellVal = [string]$headerValues[$r, $c]
            if ($cellVal -and $cellVal.Trim().ToUpper() -eq "DEPT") {
                $headerRow = $r
                $deptCol = $c
                break
            }
        }
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
            $visibleRows = $null

            # Explicit try/catch specifically for the SpecialCells call to avoid terminating the script
            try {
                $visibleRows = $dataBodyRange.SpecialCells(12) # xlCellTypeVisible
            } catch {
                # This is normal if 0 rows matched the AutoFilter criteria
                $visibleRows = $null
            }

            if ($null -ne $visibleRows) {
                # Ensure we actually have rows before copying, sometimes COM returns a 1x1 empty range
                $count = 0
                try { $count = $visibleRows.Rows.Count } catch {}
                if ($count -gt 0) {
                    $null = $visibleRows.Copy($TargetSheet.Cells.Item($startRow, 1))
                } else {
                    Write-Log "  -> Warning: VisibleRows returned but Count is 0 for ${target}"
                }
            } else {
                Write-Log "  -> Warning: No visible rows found for ${target} (Empty Result)"
            }
        } catch {
            Write-Log "  -> Warning: Failed during row copy for ${target}: $_"
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
        if ($TargetSheet) { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($TargetSheet) | Out-Null }
        if ($TargetWB) { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($TargetWB) | Out-Null }
        if ($Sheet) { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Sheet) | Out-Null }
        if ($Workbook) { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Workbook) | Out-Null }

        $TargetSheet = $null
        $TargetWB = $null
        $Sheet = $null
        $Workbook = $null

        if ($Excel) {
            $Excel.ScreenUpdating = $true
            $Excel.EnableEvents = $true
            $Excel.DisplayAlerts = $true
            if ($originalCalculation) { try { $Excel.Calculation = $originalCalculation } catch {} }
            try { $Excel.Quit() } catch {}
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($Excel) | Out-Null
            $Excel = $null
        }
    } catch {
        Write-Log "Warning: Failed to cleanly quit and release Excel COM objects: $_"
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
