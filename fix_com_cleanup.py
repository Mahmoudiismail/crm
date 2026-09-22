import sys

# crm_open_sohail/powershell.rs
crm = "src/tasker/crm_open_sohail/powershell.rs"
content = open(crm, 'r').read()
old = """} finally {
    $Excel.Quit()
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($Excel) | Out-Null
    if ($processId) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
}"""
new = """} finally {
    if ($Excel) {
        try {
            $Excel.Quit()
            [System.Runtime.Interopservices.Marshal]::ReleaseComObject($Excel) | Out-Null
        } catch { }
    }
    if ($processId) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}"""
content = content.replace(old, new)
open(crm, 'w').write(content)

# opd_task/powershell_email.rs
opd = "src/tasker/opd_task/powershell_email.rs"
content = open(opd, 'r').read()
old = """} finally {
    if ($workbook) { $workbook.Close($false) }
    if ($excel) { $excel.Quit() }
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
}"""
new = """} finally {
    if ($workbook) { try { $workbook.Close($false) } catch {} }
    if ($excel) {
        try {
            $excel.Quit()
            [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
        } catch {}
    }
    if ($mail) { try { [System.Runtime.Interopservices.Marshal]::ReleaseComObject($mail) | Out-Null } catch {} }
    if ($outlook) { try { [System.Runtime.Interopservices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {} }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}"""
content = content.replace(old, new)
open(opd, 'w').write(content)
