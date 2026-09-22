import sys

def fix_file(path, search_for, replace_with):
    content = open(path, 'r').read()
    if search_for in content:
        content = content.replace(search_for, replace_with)
        open(path, 'w').write(content)
        print(f"Fixed {path}")
    else:
        print(f"Could not find exact text in {path}")

# department_split.rs
dep_split = "src/tasker/department_split.rs"
dep_search = """        try {
            $dataBodyRange = $Sheet.Range($Sheet.Cells.Item($startRow, 1), $Sheet.Cells.Item($lastRow, $Sheet.UsedRange.Columns.Count))
            $visibleRows = $dataBodyRange.SpecialCells(12) # xlCellTypeVisible

            if ($null -ne $visibleRows) {
                $null = $visibleRows.Copy($TargetSheet.Cells.Item($startRow, 1))
            }
        } catch {
            Write-Log "  -> Warning: No visible rows found for ${target}"
        }"""
dep_replace = """        try {
            if ($lastRow -ge $startRow) {
                $dataBodyRange = $Sheet.Range($Sheet.Cells.Item($startRow, 1), $Sheet.Cells.Item($lastRow, $Sheet.UsedRange.Columns.Count))
                $visibleRows = $dataBodyRange.SpecialCells(12) # xlCellTypeVisible

                if ($null -ne $visibleRows) {
                    $null = $visibleRows.Copy($TargetSheet.Cells.Item($startRow, 1))
                }
            } else {
                Write-Log "  -> No data rows exist to copy for ${target}."
            }
        } catch {
            Write-Log "  -> Warning: No visible rows found for ${target}"
        }"""
fix_file(dep_split, dep_search, dep_replace)

# opd_task/powershell_email.rs
opd = "src/tasker/opd_task/powershell_email.rs"
opd_search = """    $visibleRows = $exactRange.SpecialCells(12)

    Write-Output "TRACE: Finding last visible row after filters"
    $lastRow = 1
    foreach ($area in $visibleRows.Areas) {
        $areaLastRow = $area.Row + $area.Rows.Count - 1
        if ($areaLastRow -gt $lastRow) {
            $lastRow = $areaLastRow
        }
    }"""
opd_replace = """    try {
        $visibleRows = $exactRange.SpecialCells(12)
    } catch {
        Write-Output "TRACE: Warning: No visible rows found via SpecialCells(12)."
        $visibleRows = $null
    }

    Write-Output "TRACE: Finding last visible row after filters"
    $lastRow = 1
    if ($null -ne $visibleRows) {
        foreach ($area in $visibleRows.Areas) {
            $areaLastRow = $area.Row + $area.Rows.Count - 1
            if ($areaLastRow -gt $lastRow) {
                $lastRow = $areaLastRow
            }
        }
    }"""
fix_file(opd, opd_search, opd_replace)
