import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

# We need to insert `let current_month = chrono::Local::now().format("%b-%Y").to_string();`
# And add `{current_month}` in the format macro.
# We also need to rewrite the powershell `$AllData` accumulation logic.

new_ps_script = """
    let current_month = chrono::Local::now().format("%b-%Y").to_string();

    let ps_script = format!(
        r#"
$ErrorActionPreference = "Stop"

$dashboardPath = '{dashboard_path}'
$jsonOutputPath = '{json_path}'
$targetSheetName = '{target_sheet}'
$targetPivotName = '{target_pivot}'
$branchFilter = {branch_filter}
$monthFilter = {month_filter}
$currentMonth = '{current_month}'

$Excel = New-Object -ComObject Excel.Application
$Excel.Visible = $false
$Excel.DisplayAlerts = $false

$processId = $null

try {{
    try {{
        [int]$handle = $Excel.Hwnd
        $processId = (Get-Process | Where-Object {{ $_.MainWindowHandle -eq $handle }}).Id
    }} catch {{
        $processId = (Get-Process -Name EXCEL | Sort-Object StartTime -Descending | Select-Object -First 1).Id
    }}

    Write-Output "Opening workbook..."
    $Workbook = $Excel.Workbooks.Open($dashboardPath, $null, $true) # open read-only
    Write-Output "Workbook opened"

    $Sheet = $null
    foreach ($ws in $Workbook.Worksheets) {{
        if ($ws.Name -eq $targetSheetName) {{
            $Sheet = $ws
            break
        }}
    }}
    if (-not $Sheet) {{ throw "Sheet '$targetSheetName' not found" }}

    $Pivot = $null
    foreach ($pt in $Sheet.PivotTables()) {{
        if ($pt.Name -eq $targetPivotName) {{
            $Pivot = $pt
            break
        }}
    }}
    if (-not $Pivot) {{ throw "PivotTable '$targetPivotName' not found in '$targetSheetName'" }}

    # Identify slicer caches
    $branchSlicerCache = $null
    $monthSlicerCache = $null

    foreach ($cache in $Workbook.SlicerCaches) {{
        if ($cache.Name -match "Branch" -or $cache.SourceName -match "Branch") {{
            $branchSlicerCache = $cache
        }}
        if ($cache.Name -match "Month" -or $cache.SourceName -match "Month") {{
            $monthSlicerCache = $cache
        }}
    }}

    if (-not $branchSlicerCache) {{ throw "Branch slicer cache not found" }}
    if (-not $monthSlicerCache) {{ throw "Month slicer cache not found" }}

    function Get-SlicerItems {{
        param ($cache)
        $items = @()
        if ($cache.Olap) {{
            $level = $cache.SlicerCacheLevels.Item(1)
            foreach ($item in $level.SlicerItems) {{
                if ($item.HasData) {{
                    $items += @{{
                        Name = $item.Name
                        Caption = $item.Caption
                    }}
                }}
            }}
        }} else {{
            foreach ($item in $cache.SlicerItems) {{
                if ($item.HasData) {{
                    $items += @{{
                        Name = $item.Name
                        Caption = $item.Name
                    }}
                }}
            }}
        }}
        return $items
    }}

    $branchItemsRaw = Get-SlicerItems -cache $branchSlicerCache
    $branchItems = @()
    foreach ($item in $branchItemsRaw) {{
        if ($branchFilter -and $branchFilter -notcontains $item.Caption) {{ continue }}
        $branchItems += $item
    }}

    $monthItemsRaw = Get-SlicerItems -cache $monthSlicerCache
    $monthItems = @()
    foreach ($item in $monthItemsRaw) {{
        if ($monthFilter -and $monthFilter -notcontains $item.Caption) {{ continue }}
        $monthItems += $item
    }}

    Write-Output "Discovered $($branchItems.Count) branches and $($monthItems.Count) months."

    $AllData = @()

    foreach ($b in $branchItems) {{
        $bName = $b.Name
        $bCaption = $b.Caption
        if ($branchSlicerCache.Olap) {{
            $branchSlicerCache.VisibleSlicerItemsList = @($bName)
        }} else {{
            # Must select the target item first to prevent COM exception where all items are deselected
            $branchSlicerCache.SlicerItems($bName).Selected = $true
            foreach ($item in $branchSlicerCache.SlicerItems) {{
                if ($item.Name -ne $bName) {{ $item.Selected = $false }}
            }}
        }}

        $isExecutiveClinic = $bCaption.ToLower() -match "executive clinic"

        if ($isExecutiveClinic) {{
            # Select all months
            if ($monthSlicerCache.Olap) {{
                $visibleList = @()
                foreach ($m in $monthItems) {{
                    $visibleList += $m.Name
                }}
                if ($visibleList.Count -gt 0) {{
                    $monthSlicerCache.VisibleSlicerItemsList = $visibleList
                }}
            }} else {{
                # Select the first item to avoid deselecting all
                if ($monthItems.Count -gt 0) {{
                    $firstM = $monthItems[0]
                    $monthSlicerCache.SlicerItems($firstM.Name).Selected = $true
                    foreach ($item in $monthSlicerCache.SlicerItems) {{
                        $shouldSelect = $false
                        foreach ($m in $monthItems) {{
                            if ($m.Name -eq $item.Name) {{
                                $shouldSelect = $true
                                break
                            }}
                        }}
                        if ($item.Name -ne $firstM.Name) {{
                            $item.Selected = $shouldSelect
                        }}
                    }}
                }}
            }}

            Write-Output "Extracting data for Branch: $bCaption (All Months Combined)"
            $Pivot.RefreshTable()
            $DataBody = $Pivot.DataBodyRange
            $RowRange = $Pivot.RowRange
            $ColumnRange = $Pivot.ColumnRange

            if ($null -ne $DataBody) {{
                $colHeaders = @{{}}
                $colCount = $DataBody.Columns.Count
                $headerRow = $ColumnRange.Rows.Count
                for ($c = 1; $c -le $colCount; $c++) {{
                    $h = $ColumnRange.Cells.Item($headerRow, $c).Text
                    $colHeaders[$c] = $h
                }}
                $rowCount = $DataBody.Rows.Count
                $DatasetData = @()
                for ($r = 1; $r -le $rowCount; $r++) {{
                    $teamName = $RowRange.Cells.Item($r + ($RowRange.Rows.Count - $rowCount), 1).Text
                    if ($teamName -eq "Grand Total") {{ continue }}
                    $rowObj = [PSCustomObject]@{{
                        team = $teamName
                        closed = 0
                        open = 0
                        "% of closed" = "0%"
                        "% of open" = "0%"
                        "Grand Total" = 0
                    }}
                    for ($c = 1; $c -le $colCount; $c++) {{
                        $header = $colHeaders[$c]
                        $val = $DataBody.Cells.Item($r, $c).Value2
                        $text = $DataBody.Cells.Item($r, $c).Text
                        if ($header -eq "closed") {{ $rowObj.closed = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                        if ($header -eq "open") {{ $rowObj.open = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                        if ($header -eq "% of closed") {{ $rowObj."% of closed" = if ($text) {{ $text }} else {{ "0%" }} }}
                        if ($header -eq "% of open") {{ $rowObj."% of open" = if ($text) {{ $text }} else {{ "0%" }} }}
                        if ($header -match "Grand Total") {{ $rowObj."Grand Total" = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                    }}
                    $DatasetData += $rowObj
                }}
                $DatasetDataArray = @($DatasetData)
                if ($DatasetDataArray.Count -gt 0) {{
                    $AllData += [PSCustomObject]@{{
                        branch = $bCaption
                        month = "All Months"
                        data = $DatasetDataArray
                    }}
                }}
            }}
        }} else {{
            # Other branches: Combine all months EXCEPT current month
            # And also current month separate
            $otherMonths = @()
            $currentMonthItem = $null

            foreach ($m in $monthItems) {{
                if ($m.Caption -eq $currentMonth) {{
                    $currentMonthItem = $m
                }} else {{
                    $otherMonths += $m
                }}
            }}

            if ($otherMonths.Count -gt 0) {{
                if ($monthSlicerCache.Olap) {{
                    $visibleList = @()
                    foreach ($m in $otherMonths) {{
                        $visibleList += $m.Name
                    }}
                    if ($visibleList.Count -gt 0) {{
                        $monthSlicerCache.VisibleSlicerItemsList = $visibleList
                    }}
                }} else {{
                    $firstM = $otherMonths[0]
                    $monthSlicerCache.SlicerItems($firstM.Name).Selected = $true
                    foreach ($item in $monthSlicerCache.SlicerItems) {{
                        $shouldSelect = $false
                        foreach ($m in $otherMonths) {{
                            if ($m.Name -eq $item.Name) {{
                                $shouldSelect = $true
                                break
                            }}
                        }}
                        if ($item.Name -ne $firstM.Name) {{
                            $item.Selected = $shouldSelect
                        }}
                    }}
                }}

                Write-Output "Extracting data for Branch: $bCaption (All Months Except Current)"
                $Pivot.RefreshTable()
                $DataBody = $Pivot.DataBodyRange
                $RowRange = $Pivot.RowRange
                $ColumnRange = $Pivot.ColumnRange

                if ($null -ne $DataBody) {{
                    $colHeaders = @{{}}
                    $colCount = $DataBody.Columns.Count
                    $headerRow = $ColumnRange.Rows.Count
                    for ($c = 1; $c -le $colCount; $c++) {{
                        $h = $ColumnRange.Cells.Item($headerRow, $c).Text
                        $colHeaders[$c] = $h
                    }}
                    $rowCount = $DataBody.Rows.Count
                    $DatasetData = @()
                    for ($r = 1; $r -le $rowCount; $r++) {{
                        $teamName = $RowRange.Cells.Item($r + ($RowRange.Rows.Count - $rowCount), 1).Text
                        if ($teamName -eq "Grand Total") {{ continue }}
                        $rowObj = [PSCustomObject]@{{
                            team = $teamName
                            closed = 0
                            open = 0
                            "% of closed" = "0%"
                            "% of open" = "0%"
                            "Grand Total" = 0
                        }}
                        for ($c = 1; $c -le $colCount; $c++) {{
                            $header = $colHeaders[$c]
                            $val = $DataBody.Cells.Item($r, $c).Value2
                            $text = $DataBody.Cells.Item($r, $c).Text
                            if ($header -eq "closed") {{ $rowObj.closed = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                            if ($header -eq "open") {{ $rowObj.open = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                            if ($header -eq "% of closed") {{ $rowObj."% of closed" = if ($text) {{ $text }} else {{ "0%" }} }}
                            if ($header -eq "% of open") {{ $rowObj."% of open" = if ($text) {{ $text }} else {{ "0%" }} }}
                            if ($header -match "Grand Total") {{ $rowObj."Grand Total" = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                        }}
                        $DatasetData += $rowObj
                    }}
                    $DatasetDataArray = @($DatasetData)
                    if ($DatasetDataArray.Count -gt 0) {{
                        $AllData += [PSCustomObject]@{{
                            branch = $bCaption
                            month = "All Months (Except Current)"
                            data = $DatasetDataArray
                        }}
                    }}
                }}
            }}

            if ($null -ne $currentMonthItem) {{
                if ($monthSlicerCache.Olap) {{
                    $monthSlicerCache.VisibleSlicerItemsList = @($currentMonthItem.Name)
                }} else {{
                    $mName = $currentMonthItem.Name
                    $monthSlicerCache.SlicerItems($mName).Selected = $true
                    foreach ($item in $monthSlicerCache.SlicerItems) {{
                        if ($item.Name -ne $mName) {{ $item.Selected = $false }}
                    }}
                }}

                Write-Output "Extracting data for Branch: $bCaption (Current Month)"
                $Pivot.RefreshTable()
                $DataBody = $Pivot.DataBodyRange
                $RowRange = $Pivot.RowRange
                $ColumnRange = $Pivot.ColumnRange

                if ($null -ne $DataBody) {{
                    $colHeaders = @{{}}
                    $colCount = $DataBody.Columns.Count
                    $headerRow = $ColumnRange.Rows.Count
                    for ($c = 1; $c -le $colCount; $c++) {{
                        $h = $ColumnRange.Cells.Item($headerRow, $c).Text
                        $colHeaders[$c] = $h
                    }}
                    $rowCount = $DataBody.Rows.Count
                    $DatasetData = @()
                    for ($r = 1; $r -le $rowCount; $r++) {{
                        $teamName = $RowRange.Cells.Item($r + ($RowRange.Rows.Count - $rowCount), 1).Text
                        if ($teamName -eq "Grand Total") {{ continue }}
                        $rowObj = [PSCustomObject]@{{
                            team = $teamName
                            closed = 0
                            open = 0
                            "% of closed" = "0%"
                            "% of open" = "0%"
                            "Grand Total" = 0
                        }}
                        for ($c = 1; $c -le $colCount; $c++) {{
                            $header = $colHeaders[$c]
                            $val = $DataBody.Cells.Item($r, $c).Value2
                            $text = $DataBody.Cells.Item($r, $c).Text
                            if ($header -eq "closed") {{ $rowObj.closed = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                            if ($header -eq "open") {{ $rowObj.open = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                            if ($header -eq "% of closed") {{ $rowObj."% of closed" = if ($text) {{ $text }} else {{ "0%" }} }}
                            if ($header -eq "% of open") {{ $rowObj."% of open" = if ($text) {{ $text }} else {{ "0%" }} }}
                            if ($header -match "Grand Total") {{ $rowObj."Grand Total" = if ($val -as [double]) {{ $val -as [double] }} elseif ([double]::TryParse($val, [ref]$null)) {{ [double]$val }} else {{ 0 }} }}
                        }}
                        $DatasetData += $rowObj
                    }}
                    $DatasetDataArray = @($DatasetData)
                    if ($DatasetDataArray.Count -gt 0) {{
                        $AllData += [PSCustomObject]@{{
                            branch = $bCaption
                            month = $currentMonth
                            data = $DatasetDataArray
                        }}
                    }}
                }}
            }}
        }}
    }}

    Write-Output "Table extraction completed. Total combinations extracted: $($AllData.Count)"
    $Workbook.Close($false)

    Write-Output "Converting AllData to JSON..."
    # Wrap $AllData explicitly in an array to avoid formatting quirks on single-item outputs
    [System.IO.File]::WriteAllText($jsonOutputPath, (ConvertTo-Json -InputObject @($AllData) -Depth 100 -Compress), (New-Object System.Text.UTF8Encoding $False))
    Write-Output "JSON saved to $jsonOutputPath"
}} catch {{
    Write-Error $_.Exception.Message
    if ($Workbook) {{ $Workbook.Close($false) }}
    throw $_
}} finally {{
    $Excel.Quit()
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($Excel) | Out-Null
    if ($processId) {{
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }}
}}
"#,
        dashboard_path = dashboard_path_str,
        json_path = json_path_str,
        target_sheet = target_sheet_name.replace("'", "''"),
        target_pivot = target_pivot_name.replace("'", "''"),
        branch_filter = branch_filter_ps,
        month_filter = month_filter_ps,
        current_month = current_month
    );
"""

search_pattern = r"""    let ps_script = format!\(
        r#"
\$ErrorActionPreference = "Stop"

\$dashboardPath = '\{dashboard_path\}'
\$jsonOutputPath = '\{json_path\}'
\$targetSheetName = '\{target_sheet\}'
\$targetPivotName = '\{target_pivot\}'
\$branchFilter = \{branch_filter\}
\$monthFilter = \{month_filter\}.*?        month_filter = month_filter_ps
    \);"""

content = re.sub(search_pattern, new_ps_script.strip(), content, flags=re.DOTALL)

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
