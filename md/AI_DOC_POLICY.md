# AI Documentation Update Policy

This policy is mandatory for any AI assistant or automation modifying this repository.

## Rule 1: Docs Must Track Code

After **every code/config/build command that changes behavior**, update relevant docs in `md/` within the same change set.

## Rule 2: Required Mapping

When a file changes, update docs as follows:

- `src/bin/runner.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md`
- `src/bin/crm.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md`
- `src/bin/tasker.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `OPERATIONS.md`
- `src/lib.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`
- `src/runner/*` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `CONFIG.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`
- `src/crm/*` -> `ARCHITECTURE.md`, `CONFIG.md`, `AUTH_FLOW.md`, `FETCHER.md`, `DOWNLOADER.md`, `OPERATIONS.md`
- `src/tasker/*` -> `ARCHITECTURE.md`, `CONFIG.md`
- `Cargo.toml` / dependency changes -> `BUILD_AND_RUN.md`, `DOCKER.md`, `APPLICATION_SUMMARY.md`
- `.github/workflows/*` -> `BUILD_AND_RUN.md`, `OPERATIONS.md`
- `.devcontainer/*` -> `DOCKER.md`, `BUILD_AND_RUN.md`
- `Dockerfile*`, scripts -> `DOCKER.md`, `BUILD_AND_RUN.md`
- `AGENTS.md` -> `README.md`, `AI_DOC_POLICY.md`

## Rule 3: Command-Level Discipline

For each engineering command/session:

1. Identify impacted behavior.
2. Update corresponding `md/*.md` files.
3. Verify docs still reflect actual code paths.
4. Do not defer documentation updates.

## Rule 4: Pull Request Gate

A change is incomplete if behavior changed and no matching doc update exists.

## Rule 5: Accuracy Standard

- Prefer exact function/file names.
- Document defaults, edge cases, and failure modes.
- Keep examples runnable and aligned with current CLI.

## Enforcement Suggestion

Before commit, run a manual checklist:

- [ ] Code changed?
- [ ] Matching docs updated in `md/`?
- [ ] `AGENTS.md` read before making agent-authored changes?
- [ ] Examples still valid?
- [ ] New config fields documented?
- [ ] New CLI flags documented?

If any answer is `no`, update docs before finalizing.

## Fixed CSV Parsing Issues
- Replaced raw inline diagnostic logging loop with a shared function limiting output to ±20 lines around the error line to keep logs compact but useful.
- Removed `flexible(true)` centrally in `utils.rs` via `build_csv_reader` to strictly enforce column counts and throw validation errors when data is malformed.

## Recent Fixes
- **Concurrent API Fetching:** Modified `fetch_with_signed_url_split` in `crm/fetcher.rs` to concurrently fetch split date ranges using a recursive boxed future approach (`fetch_recursive` with `tokio::join!`). This fixes the issue where split fetches were being executed sequentially.
- **Dashboard Updater Calculation Issue:** Fixed an issue where Excel threw a `0x800A03EC` COM exception when modifying `$Excel.Calculation = -4135` by ensuring the workbook is opened *before* altering application calculation modes.
- **CrmOpenSohail Pivot Extraction:** Fixed strict casting errors in PowerShell pivot parsing by migrating from `[double]$val` to safe casting (`-as [double]`) with fallback TryParse logic to handle string anomalies gracefully. Added matching Rust tests to ensure the generated scripts uphold this.
- **OLAP Slicer Support:** Added support for Excel Data Model (OLAP) Slicers in `tasker/crm_open_sohail.rs` PowerShell automation scripts, utilizing `SlicerCacheLevels` and `VisibleSlicerItemsList`.
- Enhanced date variable processing across all binaries by introducing an integrated `DateVar` argument type in the manifest and updating the Runner GUI to allow easy toggling between variable and calendar inputs.
- Moved variable resolution into `parse_flexible_date` natively, ensuring `today`, `tomorrow`, `yesterday`, and context-aware `eomonth` work systematically anywhere date parsing is utilized, complete with stringent validation logic.
- Added `--custom-download-folder` to `crm.rs` to override config paths via CLI without persisting the change.
- Fixed `yasweb.rs` date formatting for `Report Manager` (dd-MMM-yyyy) and `Standard Report` (dd-mm-yyyy HH:MM). Date formats are now customizable through a `DateKeyConfig` struct in the `yasweb_config.json`.
- Added TRACE level diagnostic logging for entire page HTML content during specific MIS module navigation steps in `src/yasweb/browser.rs`.
- Corrected Windows file lock exceptions when running PowerShell in `tasker/email.rs` and `tasker/crm_open_sohail.rs`.
- Added missing 3rd default task (`crm_open_sohail`) to `tasker_config.json` default generation.

## Rule 6: Test-Driven Bug Fixes
- When a bug is fixed, a corresponding test case must be added to prevent future regressions. The fix and the test case should be documented together in the relevant `md/*.md` file.
- **Task 2 (Dashboard Updater Fixes):** Removed the raw CSV copy and paste automation logic inside `dashboard_updater.rs`. It now relies on Excel's internal PowerQuery Data Model connection to point at `results.csv`. Simplified the PowerShell script to only trigger `.RefreshAll()`, Data Model refreshes, and PivotTable refreshes before emailing the report. Also removed `dashboard_table_name` configuration options as they are no longer necessary. Additionally implemented robust Excel COM process cleanup logic to forcefully kill lingering Excel tasks using the process ID.
- **Task 3 (CrmOpenSohail Email Fixes):** Fixed Pivot Data extraction by explicitly calling `$Pivot.RefreshTable()` before fetching data to ensure elements match Slicer constraints. Wrapped resulting `$DatasetData` array as `@($DatasetData)` before `.Count` to safely populate `$AllData`. Enhanced the OUL assignment lookup in `crm_open_sohail.rs` to generate only one integrated email containing dynamically built HTML tables.
- **Task 2 and 3 Logging:** Added standard trace context spanning `Email generation started/completed`, `Dashboard update started/completed`, `Workbook opened` to `crm_open_sohail.rs` and `dashboard_updater.rs`.
- **Email Attachment Size Limits:** Fixed an unhandled `System.Runtime.InteropServices.COMException` caused by large file attachments exceeding the Outlook server limits in `src/tasker/dashboard_updater.rs` by implementing a `try/catch` fallback that dynamically alters the HTML body when adding an attachment fails.
- **PowerShell JSON Parsing Bug:** Fixed an issue where the `crm_open_sohail` script generated `0 combinations` from Excel PivotTables due to a JSON serialization quirk in `ConvertTo-Json`. The fix explicitly converts custom row hashtables to `[PSCustomObject]` arrays before serialization.
