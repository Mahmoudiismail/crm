
param(
    [string]$DashboardPath,
    [string]$JsonOutputPath,
    [string]$TargetSheetName,
    [string]$TargetPivotName,
    [string]$BranchFilterCsv,
    [string]$MonthFilterCsv,
    [string]$CurrentMonth
)

$ErrorActionPreference = "Stop"

$dashboardPath = $DashboardPath
$jsonOutputPath = $JsonOutputPath
$targetSheetName = $TargetSheetName
$targetPivotName = $TargetPivotName
$branchFilter = if ($BranchFilterCsv) { $BranchFilterCsv -split "," } else { $null }
$monthFilter = if ($MonthFilterCsv) { $MonthFilterCsv -split "," } else { $null }
$currentMonth = $CurrentMonth

$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false

$processId = $null

try {
    try {
        [int]$handle = $Excel.Hwnd
        $processId = (Get-Process | Where-Object { $_.MainWindowHandle -eq $handle }).Id
    } catch {
        $processId = (Get-Process -Name EXCEL | Sort-Object StartTime -Descending | Select-Object -First 1).Id
    }

    Write-Output "Opening workbook..."
    $Workbook = $Excel.Workbooks.Open($dashboardPath, $null, $true) # open read-only
    Write-Output "Workbook opened"

    $Sheet = $null
    foreach ($ws in $Workbook.Worksheets) {
        if ($ws.Name -eq $targetSheetName) {
            $Sheet = $ws
            break
        }
    }
    if (-not $Sheet) { throw "Sheet '$targetSheetName' not found" }

    $Pivot = $null
    foreach ($pt in $Sheet.PivotTables()) {
        if ($pt.Name -eq $targetPivotName) {
            $Pivot = $pt
            break
        }
    }
    if (-not $Pivot) { throw "PivotTable '$targetPivotName' not found in '$targetSheetName'" }

    # Identify slicer caches
    $branchSlicerCache = $null
    $monthSlicerCache = $null

    foreach ($cache in $Workbook.SlicerCaches) {
        if ($cache.Name -match "Branch" -or $cache.SourceName -match "Branch") {
            $branchSlicerCache = $cache
        }
        if ($cache.Name -match "Month" -or $cache.SourceName -match "Month") {
            $monthSlicerCache = $cache
        }
    }

    if (-not $branchSlicerCache) { throw "Branch slicer cache not found" }
    if (-not $monthSlicerCache) { throw "Month slicer cache not found" }

    Write-Output "Clearing saved filters..."
    $Pivot.ClearAllFilters()
    $branchSlicerCache.ClearAllFilters()
    $monthSlicerCache.ClearAllFilters()

    function Get-SlicerItems {
        param ($cache)
        $items = @()
        if ($cache.Olap) {
            $level = $cache.SlicerCacheLevels.Item(1)
            foreach ($item in $level.SlicerItems) {
                if ($item.HasData) {
                    $items += @{
                        Name = $item.Name
                        Caption = $item.Caption
                    }
                }
            }
        } else {
            foreach ($item in $cache.SlicerItems) {
                if ($item.HasData) {
                    $items += @{
                        Name = $item.Name
                        Caption = $item.Name
                    }
                }
            }
        }
        return $items
    }

    $branchItemsRaw = Get-SlicerItems -cache $branchSlicerCache
    $branchItems = @()
    foreach ($item in $branchItemsRaw) {
        if ($branchFilter) {
            $found = $false
            foreach ($f in $branchFilter) {
                if ($item.Caption.Trim() -match "^$([regex]::Escape($f.Trim()))$") {
                    $found = $true
                    break
                }
            }
            if (-not $found) { continue }
        }
        $branchItems += $item
    }

    $monthItemsRaw = Get-SlicerItems -cache $monthSlicerCache
    $monthItems = @()
    foreach ($item in $monthItemsRaw) {
        if ($monthFilter) {
            $found = $false
            foreach ($f in $monthFilter) {
                if ($item.Caption.Trim() -match "^$([regex]::Escape($f.Trim()))$") {
                    $found = $true
                    break
                }
            }
            if (-not $found) { continue }
        }
        $monthItems += $item
    }

    Write-Output "Discovered $($branchItems.Count) branches and $($monthItems.Count) months."

    function Add-PivotDataToCollection {
        param($PivotTable, [string]$BranchCaption, [string]$MonthTitle, [ref]$Collector)

        $PivotTable.RefreshTable()
        $dataBody = $PivotTable.DataBodyRange
        if ($null -eq $dataBody) { return }

        $rowRange = $PivotTable.RowRange
        $columnRange = $PivotTable.ColumnRange
        $values = $dataBody.Value2
        $columnCount = $dataBody.Columns.Count
        $headerRow = $columnRange.Rows.Count
        $headers = @{}
        for ($c = 1; $c -le $columnCount; $c++) {
            $headers[$c] = $columnRange.Cells.Item($headerRow, $c).Text
        }

        $rowCount = $dataBody.Rows.Count
        $dataset = [System.Collections.Generic.List[object]]::new()
        $rowOffset = $rowRange.Rows.Count - $rowCount
        for ($r = 1; $r -le $rowCount; $r++) {
            $teamName = $rowRange.Cells.Item($r + $rowOffset, 1).Text
            if ($teamName -eq "Grand Total") { continue }

            $row = [PSCustomObject]@{
                team = $teamName
                closed = 0
                open = 0
                "% of closed" = "0%"
                "% of open" = "0%"
                "Grand Total" = 0
            }
            for ($c = 1; $c -le $columnCount; $c++) {
                $header = $headers[$c]
                $value = if ($values -is [System.Array]) { $values[$r, $c] } else { $values }
                if ($header -eq "% of closed" -or $header -eq "% of open") {
                    $text = $dataBody.Cells.Item($r, $c).Text
                    if ($header -eq "% of closed") { $row."% of closed" = if ($text) { $text } else { "0%" } }
                    else { $row."% of open" = if ($text) { $text } else { "0%" } }
                } elseif ($header -eq "closed") {
                    $row.closed = if ($value -as [double]) { $value -as [double] } elseif ([double]::TryParse($value, [ref]$null)) { [double]$value } else { 0 }
                } elseif ($header -eq "open") {
                    $row.open = if ($value -as [double]) { $value -as [double] } elseif ([double]::TryParse($value, [ref]$null)) { [double]$value } else { 0 }
                } elseif ($header -match "Grand Total") {
                    $row."Grand Total" = if ($value -as [double]) { $value -as [double] } elseif ([double]::TryParse($value, [ref]$null)) { [double]$value } else { 0 }
                }
            }
            $dataset.Add($row)
        }
        if ($dataset.Count -gt 0) {
            $Collector.Value += [PSCustomObject]@{
                branch = $BranchCaption
                month = $MonthTitle
                data = $dataset.ToArray()
            }
        }
    }

    $AllData = @()

    foreach ($b in $branchItems) {
        $bName = $b.Name
        $bCaption = $b.Caption
        if ($branchSlicerCache.Olap) {
            try {
                $branchSlicerCache.VisibleSlicerItemsList = @($bName)
            } catch {
                Write-Output "Warning: Could not set branch OLAP slicer for $($bName) - $_. Skipping."
                continue
            }
        } else {
            $branchSlicerCache.SlicerItems($bName).Selected = $true
            foreach ($item in $branchSlicerCache.SlicerItems) {
                if ($item.Name -ne $bName) { $item.Selected = $false }
            }
        }

        $isExecutiveClinic = $bCaption.ToLower() -match "executive clinic"

        if ($isExecutiveClinic) {
            if ($monthSlicerCache.Olap) {
                $visibleList = @()
                foreach ($m in $monthItems) {
                    $visibleList += $m.Name
                }
                if ($visibleList.Count -gt 0) {
                    try {
                        $monthSlicerCache.VisibleSlicerItemsList = $visibleList
                    } catch {
                        Write-Output "Warning: Could not set month OLAP slicer list - $_. Skipping executive clinic extraction for branch $bCaption."
                        continue
                    }
                }
            } else {
                if ($monthItems.Count -gt 0) {
                    $firstM = $monthItems[0]
                    $monthSlicerCache.SlicerItems($firstM.Name).Selected = $true
                    foreach ($item in $monthSlicerCache.SlicerItems) {
                        $shouldSelect = $false
                        foreach ($m in $monthItems) {
                            if ($m.Name -eq $item.Name) {
                                $shouldSelect = $true
                                break
                            }
                        }
                        if ($item.Name -ne $firstM.Name) {
                            $item.Selected = $shouldSelect
                        }
                    }
                }
            }

            Write-Output "Extracting data for Branch: $bCaption (All Months Combined)"
            Add-PivotDataToCollection -PivotTable $Pivot -BranchCaption $bCaption -MonthTitle "All Months" -Collector ([ref]$AllData)
        } else {
            $otherMonths = @()
            $currentMonthItem = $null

            foreach ($m in $monthItems) {
                if ($m.Caption -eq $currentMonth) {
                    $currentMonthItem = $m
                } else {
                    $otherMonths += $m
                }
            }

            if ($otherMonths.Count -gt 0) {
                $otherMonthsTitle = "All Months (Except Current)"
                if ($otherMonths.Count -ge 2) {
                    $firstMName = $otherMonths[0].Caption
                    $lastMName = $otherMonths[$otherMonths.Count - 1].Caption

                    if ($firstMName -match "-") {
                        $firstPart = $firstMName.Split("-")[0]
                    } else {
                        $firstPart = $firstMName
                    }
                    $otherMonthsTitle = "from ($firstPart to $lastMName)"
                } elseif ($otherMonths.Count -eq 1) {
                    $otherMonthsTitle = $otherMonths[0].Caption
                }

                if ($monthSlicerCache.Olap) {
                    $visibleList = @()
                    foreach ($m in $otherMonths) {
                        $visibleList += $m.Name
                    }
                    $failedOlap = $false
                    if ($visibleList.Count -gt 0) {
                        try {
                            $monthSlicerCache.VisibleSlicerItemsList = $visibleList
                        } catch {
                            Write-Output "Warning: Could not set month OLAP slicer for 'other months' - $_. Skipping."
                            $failedOlap = $true
                        }
                    }
                    if ($failedOlap) { continue }
                } else {
                    $firstM = $otherMonths[0]
                    $monthSlicerCache.SlicerItems($firstM.Name).Selected = $true
                    foreach ($item in $monthSlicerCache.SlicerItems) {
                        $shouldSelect = $false
                        foreach ($m in $otherMonths) {
                            if ($m.Name -eq $item.Name) {
                                $shouldSelect = $true
                                break
                            }
                        }
                        if ($item.Name -ne $firstM.Name) {
                            $item.Selected = $shouldSelect
                        }
                    }
                }

                Write-Output "Extracting data for Branch: $bCaption (All Months Except Current)"
                Add-PivotDataToCollection -PivotTable $Pivot -BranchCaption $bCaption -MonthTitle $otherMonthsTitle -Collector ([ref]$AllData)
            }

            if ($null -ne $currentMonthItem) {
                $failedOlap = $false
                if ($monthSlicerCache.Olap) {
                    try {
                        $monthSlicerCache.VisibleSlicerItemsList = @($currentMonthItem.Name)
                    } catch {
                        Write-Output "Warning: Could not set month OLAP slicer for '$($currentMonthItem.Name)' - $_. Skipping current month extraction for branch $bCaption."
                        $failedOlap = $true
                    }
                } else {
                    $mName = $currentMonthItem.Name
                    $monthSlicerCache.SlicerItems($mName).Selected = $true
                    foreach ($item in $monthSlicerCache.SlicerItems) {
                        if ($item.Name -ne $mName) { $item.Selected = $false }
                    }
                }

                if ($failedOlap) { continue }

                Write-Output "Extracting data for Branch: $bCaption (Current Month)"
                Add-PivotDataToCollection -PivotTable $Pivot -BranchCaption $bCaption -MonthTitle $currentMonth -Collector ([ref]$AllData)
            }
        }
    }

    Write-Output "Table extraction completed. Total combinations extracted: $($AllData.Count)"
    $Workbook.Close($false)

    Write-Output "Converting AllData to JSON..."
    [System.IO.File]::WriteAllText($jsonOutputPath, (ConvertTo-Json -InputObject @($AllData) -Depth 100 -Compress), (New-Object System.Text.UTF8Encoding $False))
    Write-Output "JSON saved to $jsonOutputPath"
} catch {
    Write-Error $_.Exception.Message
    if ($Workbook) { $Workbook.Close($false) }
    throw $_
} finally {
    $Excel.Quit()
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($Excel) | Out-Null
    if ($processId) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
}
