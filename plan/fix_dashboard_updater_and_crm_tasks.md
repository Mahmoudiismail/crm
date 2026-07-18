# Plan: Fix Dashboard Updater and CRM Open Sohail Tasks

1. **Update `CrmOpenSohailConfig` and `DashboardUpdaterConfig` structs:**
   - In `src/tasker/config.rs`:
     - Make `dashboard_table_name` an `Option<String>` in `DashboardUpdaterConfig` instead of a required `String`.
     - Add `dashboard_sheet_name: Option<String>` and `dashboard_pivot_name: Option<String>` to `CrmOpenSohailConfig`.
     - Update deserialization tests appropriately.

2. **Update `DashboardUpdater` script (Task 2):**
   - In `src/tasker/dashboard_updater.rs`:
     - Change the PowerShell logic so that it only attempts to find and paste into the table IF `dashboard_table_name` is provided and is not empty.
     - If it's `None` or empty, skip deleting the DataBodyRange and skip copying the CSV.
     - The script will natively continue to execute `$Workbook.Model.Refresh()` and `$PivotTable.RefreshTable()` which handles the Power Query refresh without pasting any data into a sheet.

3. **Update `crm_open_sohail` script (Task 3):**
   - In `src/tasker/crm_open_sohail.rs`:
     - Provide `dashboard_sheet_name` and `dashboard_pivot_name` defaults inside the Rust code (`Sheet1` and `PivotTable2` respectively, if they are `None`).
     - Pass these variables into the PowerShell format string so it no longer looks for the hardcoded `TKT - Dashboard` and `PivotTable1` sheet and pivot table names.

4. **Update AGENTS.md:**
   - Add a rule to the top of `AGENTS.md` explicitly instructing agents to create a detailed `plan.md` on each run, place them in the `plan/` directory, and assign a unique filename identifying the content.

5. **Update tests:**
   - Fix all compilation errors in the codebase tests resulting from changing `dashboard_table_name` to an `Option<String>`.

6. **Pre-commit checks**
   - Execute formatting, linting, and tests.
