# Execution Plan: Fix Dashboard Updater & Excel Cleanup

## Objective
Remove the raw data copy-pasting feature from `dashboard_updater`, relying entirely on PowerQuery/Data connections to `results.csv`. Remove `dashboard_table_name` configuration, and ensure background Excel processes are forcefully cleaned up for Tasks 2 and 3.

## Steps
1. **Remove `dashboard_table_name` configuration**
   - Remove `dashboard_table_name` from `DashboardUpdaterConfig` and `CrmOpenSohailConfig` in `src/tasker/config.rs`.
   - Update `src/bin/tasker.rs`, `md/TASKER.md`, `TestingDownloads/tasker_config.json`, `tasker_config.json.example`, `modify_tasker_config.py`, and `issues/tasker_cli_analysis.md` to remove the field.
   - Update tests in `src/tasker/config.rs`, `src/tasker/dashboard_updater.rs`, and `src/tasker/crm_open_sohail.rs`.

2. **Simplify `dashboard_updater.rs` logic**
   - Remove the generation of `dashboard_filtered_{timestamp}.csv`.
   - Remove the `$tableName`, `$csvPath`, copy, and paste logic from the PowerShell script.
   - The script will simply open `$dashboardPath`, switch to manual calculation, call `$Workbook.RefreshAll()`, `$Workbook.Model.Refresh()`, refresh PivotTables, restore calculation mode, save, and exit.

3. **Improve Excel background process cleanup (Task 2 & 3)**
   - Update the PowerShell script in `dashboard_updater.rs` and `crm_open_sohail.rs`.
   - Use `ReleaseComObject($Excel)`, `[System.GC]::Collect()`, and finally, a process cleanup via `$Excel.ProcessID` or `Get-Process` to forcefully kill Excel if it lingers.

4. **Update tests to match new behavior**
   - Modify the `test_task2_dashboard_updater` test to no longer look for the `dashboard_filtered_*.csv` file.

5. **Review and update documentation**
   - Update `AGENTS.md` and `md/TASKER.md` as required by the AI doc policy.

6. **Pre-commit verification**
   - Run tests and pre-commit checks.

7. **Submit changes**
   - Commit and submit.
