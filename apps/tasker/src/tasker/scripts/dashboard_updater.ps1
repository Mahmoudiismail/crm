
param(
    [Parameter(Mandatory = $true)]
    [string]$DashboardPath
)

$ErrorActionPreference = "Stop"

$resolvedDashboardPath = [System.IO.Path]::GetFullPath($DashboardPath)

if (-not (Test-Path -LiteralPath $resolvedDashboardPath -PathType Leaf)) {
    throw "Dashboard file not found: $resolvedDashboardPath"
}

$excel = $null
$workbooks = $null
$workbook = $null
$worksheets = $null
$worksheet = $null
$pivotTables = $null
$pivotTable = $null
$pivotCache = $null
$originalCalculation = $null

try {
    Write-Host "PS: Starting Excel..."

    $excel = New-Object -ComObject Excel.Application

    $excel.Visible = $false
    $excel.DisplayAlerts = $false
    $excel.ScreenUpdating = $false
    $excel.EnableEvents = $false
    $excel.AskToUpdateLinks = $false

    Write-Host "PS: Opening dashboard: $resolvedDashboardPath"

    $workbooks = $excel.Workbooks

    $workbook = $workbooks.Open(
        $resolvedDashboardPath,
        0,
        $false
    )

    Write-Host "PS: Workbook opened successfully."

    # Changing Excel calculation mode can fail with HRESULT 0x800A03EC.
    # Therefore, calculation mode is treated as an optional optimization.
    try {
        $originalCalculation = $excel.Calculation

        # xlCalculationManual = -4135
        $excel.Calculation = -4135

        Write-Host "PS: Calculation mode changed to manual."
    }
    catch {
        $originalCalculation = $null

        Write-Host (
            "PS: Warning: Unable to change calculation mode. " +
            "Continuing with the current Excel calculation mode. " +
            "Details: $($_.Exception.Message)"
        )
    }

    $pivotFound = $false
    $worksheets = $workbook.Worksheets
    $worksheetCount = $worksheets.Count

    Write-Host "PS: Searching $worksheetCount worksheet(s) for PivotTable2..."

    for ($index = 1; $index -le $worksheetCount; $index++) {
        try {
            $worksheet = $worksheets.Item($index)

            Write-Host "PS: Checking worksheet '$($worksheet.Name)'..."

            try {
                $pivotTables = $worksheet.PivotTables()

                if ($pivotTables.Count -gt 0) {
                    try {
                        $pivotTable = $pivotTables.Item("PivotTable2")
                    }
                    catch {
                        $pivotTable = $null
                    }
                }
            }
            catch {
                $pivotTable = $null
            }

            if ($null -ne $pivotTable) {
                Write-Host (
                    "PS: PivotTable2 found on worksheet " +
                    "'$($worksheet.Name)'."
                )

                $pivotCache = $pivotTable.PivotCache()

                try {
                    $pivotCache.EnableRefresh = $true
                }
                catch {
                    Write-Host "PS: PivotCache EnableRefresh isn't supported."
                }

                try {
                    $pivotCache.BackgroundQuery = $false
                }
                catch {
                    Write-Host "PS: PivotCache BackgroundQuery isn't supported."
                }

                Write-Host "PS: Refreshing PivotCache..."

                try {
                    $pivotCache.Refresh()
                }
                catch {
                    Write-Host (
                        "PS: PivotCache refresh warning: " +
                        "$($_.Exception.Message)"
                    )
                }

                Write-Host "PS: Refreshing PivotTable2..."

                $refreshResult = $pivotTable.RefreshTable()

                Write-Host "PS: PivotTable refresh result: $refreshResult"

                $pivotFound = $true
                break
            }
        }
        finally {
            if ($null -ne $pivotCache) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $pivotCache
                    )
                }
                catch {}

                $pivotCache = $null
            }

            if ($null -ne $pivotTable) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $pivotTable
                    )
                }
                catch {}

                $pivotTable = $null
            }

            if ($null -ne $pivotTables) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $pivotTables
                    )
                }
                catch {}

                $pivotTables = $null
            }

            if ($null -ne $worksheet) {
                try {
                    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                        $worksheet
                    )
                }
                catch {}

                $worksheet = $null
            }
        }
    }

    if (-not $pivotFound) {
        Write-Host (
            "PS: PivotTable2 wasn't found. " +
            "Refreshing all workbook connections..."
        )

        $workbook.RefreshAll()

        try {
            $excel.CalculateUntilAsyncQueriesDone()

            Write-Host "PS: Async queries completed."
        }
        catch {
            Write-Host (
                "PS: CalculateUntilAsyncQueriesDone warning: " +
                "$($_.Exception.Message)"
            )
        }
    }

    Write-Host "PS: Starting workbook calculation..."

    # Don't force Excel.Calculation because that property caused 0x800A03EC.
    try {
        $excel.CalculateFull()

        Write-Host "PS: Full calculation completed."
    }
    catch {
        Write-Host (
            "PS: CalculateFull warning: $($_.Exception.Message)"
        )

        try {
            $workbook.Application.Calculate()

            Write-Host "PS: Standard calculation completed."
        }
        catch {
            Write-Host (
                "PS: Standard calculation warning: " +
                "$($_.Exception.Message)"
            )
        }
    }

    Write-Host "PS: Saving dashboard..."

    $workbook.Save()

    Write-Host "PS: Dashboard updated successfully."
}
catch {
    Write-Host "PS ERROR: $($_.Exception.Message)"

    if ($_.ScriptStackTrace) {
        Write-Host "PS ERROR STACK: $($_.ScriptStackTrace)"
    }

    throw
}
finally {
    Write-Host "PS: Starting cleanup..."

    if ($null -ne $originalCalculation -and $null -ne $excel) {
        try {
            $excel.Calculation = $originalCalculation

            Write-Host "PS: Original calculation mode restored."
        }
        catch {
            Write-Host (
                "PS: Calculation mode restoration warning: " +
                "$($_.Exception.Message)"
            )
        }
    }

    if ($null -ne $pivotCache) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $pivotCache
            )
        }
        catch {}

        $pivotCache = $null
    }

    if ($null -ne $pivotTable) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $pivotTable
            )
        }
        catch {}

        $pivotTable = $null
    }

    if ($null -ne $pivotTables) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $pivotTables
            )
        }
        catch {}

        $pivotTables = $null
    }

    if ($null -ne $worksheet) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $worksheet
            )
        }
        catch {}

        $worksheet = $null
    }

    if ($null -ne $worksheets) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $worksheets
            )
        }
        catch {}

        $worksheets = $null
    }

    if ($null -ne $workbook) {
        try {
            $workbook.Close($false)
        }
        catch {
            Write-Host (
                "PS: Workbook close warning: " +
                "$($_.Exception.Message)"
            )
        }

        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $workbook
            )
        }
        catch {}

        $workbook = $null
    }

    if ($null -ne $workbooks) {
        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $workbooks
            )
        }
        catch {}

        $workbooks = $null
    }

    if ($null -ne $excel) {
        try {
            $excel.Quit()
        }
        catch {
            Write-Host (
                "PS: Excel quit warning: $($_.Exception.Message)"
            )
        }

        try {
            [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject(
                $excel
            )
        }
        catch {}

        $excel = $null
    }

[System.GC]::Collect()
[System.GC]::WaitForPendingFinalizers()
[System.GC]::Collect()
[System.GC]::WaitForPendingFinalizers()

    Write-Host "PS: Cleanup completed."
}
