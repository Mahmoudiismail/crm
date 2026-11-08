
param(
    [string]$DashboardPath,
    [string]$OutputDir,
    [string]$MappingFile
)

$ErrorActionPreference = "Stop"

function ConvertTo-ExcelCompatiblePath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $Path }
    if ($Path.StartsWith('\\?\', [System.StringComparison]::OrdinalIgnoreCase)) { return $Path.Substring(4) }
    return $Path
}
$dashboardPath = ConvertTo-ExcelCompatiblePath $DashboardPath
$outDir = ConvertTo-ExcelCompatiblePath $OutputDir
$mappingFile = ConvertTo-ExcelCompatiblePath $MappingFile

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
$Workbook = $null
$TargetWB = $null
$TargetSheet = $null
$Sheet = $null
$templatePath = $null
$target = $null

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
    $usedColumnCount = $Sheet.UsedRange.Columns.Count

    Write-Log "Reading OPD Report data once for all output groups..."
    $readTimer = [System.Diagnostics.Stopwatch]::StartNew()
    $sourceRange = $Sheet.Range(
        $Sheet.Cells.Item($startRow, 1),
        $Sheet.Cells.Item($lastRow, $usedColumnCount)
    )
    $sourceValues = $sourceRange.Value2
    $readTimer.Stop()
    Write-Log "  -> Read source data in $($readTimer.ElapsedMilliseconds) ms"

    # Store source row indexes per output group. This preserves source order and
    # avoids repeating AutoFilter/SpecialCells COM calls for every department.
    $targetRowsMap = @{}
    $rowCount = $lastRow - $startRow + 1
    for ($r = 1; $r -le $rowCount; $r++) {
        if ($sourceValues -is [System.Array]) {
            $val = [string]$sourceValues.GetValue($r, $deptCol)
        } else {
            $val = [string]$sourceValues
        }
        if ([string]::IsNullOrWhiteSpace($val)) { continue }

        $deptVal = $val.Trim().ToUpper()
        # Match the previous AutoFilter behavior: do not include source cells
        # whose surrounding whitespace would prevent an exact filter match.
        if ($val -ne $val.Trim()) { continue }
        $targetChair = "OTHERS"
        if ($mappingHash.ContainsKey($deptVal)) { $targetChair = $mappingHash[$deptVal] }

        if (-not $targetRowsMap.ContainsKey($targetChair)) {
            $targetRowsMap[$targetChair] = [System.Collections.Generic.List[int]]::new()
        }
        $targetRowsMap[$targetChair].Add($r)
    }
    if ($targetRowsMap.Count -eq 0) { throw "No department values found; no output files generated" }
    Write-Log "Grouped $rowCount source rows into $($targetRowsMap.Count) output groups"

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

        $sourceRowIndexes = $targetRowsMap[$target]
        $copiedRows = $sourceRowIndexes.Count
        if ($copiedRows -le 0) { throw "No source rows found for department group '${target}'" }

        Write-Log "  -> Building one $copiedRows x $usedColumnCount output matrix..."
        $transferTimer = [System.Diagnostics.Stopwatch]::StartNew()
        $outputValues = [System.Array]::CreateInstance([object], @($copiedRows, $usedColumnCount), @(1, 1))
        for ($destinationIndex = 0; $destinationIndex -lt $copiedRows; $destinationIndex++) {
            $sourceIndex = $sourceRowIndexes[$destinationIndex]
            for ($column = 1; $column -le $usedColumnCount; $column++) {
                if ($sourceValues -is [System.Array]) {
                    $outputValues.SetValue($sourceValues.GetValue($sourceIndex, $column), ($destinationIndex + 1), $column)
                } else {
                    $outputValues.SetValue($sourceValues, ($destinationIndex + 1), $column)
                }
            }
        }

        # One COM write per output workbook instead of one write per visible area.
        $destinationRange = $TargetSheet.Range(
            $TargetSheet.Cells.Item($startRow, 1),
            $TargetSheet.Cells.Item(($startRow + $copiedRows - 1), $usedColumnCount)
        )
        $destinationRange.Value2 = $outputValues
        $transferTimer.Stop()
        Write-Log "  -> Bulk data transfer completed in $($transferTimer.ElapsedMilliseconds) ms"

        $destinationDeptValues = $TargetSheet.Range(
            $TargetSheet.Cells.Item($startRow, $deptCol),
            $TargetSheet.Cells.Item(($startRow + $copiedRows - 1), $deptCol)
        ).Value2
        $verifiedRows = 0
        if ($destinationDeptValues -is [System.Array]) {
            for ($i = 1; $i -le $destinationDeptValues.GetLength(0); $i++) {
                if (-not [string]::IsNullOrWhiteSpace([string]$destinationDeptValues.GetValue($i, 1))) { $verifiedRows++ }
            }
        } elseif (-not [string]::IsNullOrWhiteSpace([string]$destinationDeptValues)) {
            $verifiedRows = 1
        }
        if ($verifiedRows -ne $copiedRows) {
            throw "Copy verification failed for '${target}': expected $copiedRows rows, found $verifiedRows non-empty DEPT values"
        }
        Write-Log "  -> Verified $verifiedRows copied rows for ${target}"

        $refreshTimer = [System.Diagnostics.Stopwatch]::StartNew()
        foreach ($pc in $TargetWB.PivotCaches()) {
            # xlMissingItemsNone = 0: do not retain deleted source items.
            $pc.MissingItemsLimit = 0
            Write-Log "  -> PivotCache MissingItemsLimit set to None"
            try {
                $null = $pc.Refresh()
            } catch {
                Write-Log "  -> Warning: PivotCache refresh failed: $_"
            }
        }
        $refreshTimer.Stop()
        Write-Log "  -> PivotCaches refreshed in $($refreshTimer.ElapsedMilliseconds) ms"

        $CoverSheet = $TargetWB.Worksheets.Item("Cover")
        $CoverSheet.Activate()
        $TargetWB.Windows.Item(1).DisplayWorkbookTabs = $false
        Write-Log "  -> Worksheet tabs hidden for ${target}"

        $saveTimer = [System.Diagnostics.Stopwatch]::StartNew()
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($CoverSheet)
        $CoverSheet = $null
        $TargetWB.Save()
        $TargetWB.Close($true)
        $saveTimer.Stop()
        Write-Log "  -> Saved and closed in $($saveTimer.ElapsedMilliseconds) ms"

        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($TargetSheet)
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($TargetWB)
        $TargetSheet = $null
        $TargetWB = $null
        $deptTimer.Stop()
        Write-Log "  => Target ${target} processed and saved ($copiedRows rows) in $($deptTimer.ElapsedMilliseconds) ms"
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
