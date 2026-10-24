# Tasker Application CLI and Configuration Analysis

## Agent Compliance Summary
- **AGENTS.md Policy Adherence:** Verified memory items related to Tasker. Identified rules like ignoring exceptions for the dashboard, excluding 'Position' from emails, strict Call Center lead status filters, avoiding `unwrap_or_default()` when unneeded, generating local temporary scripts without locking `std::fs` across threads unnecessarily, handling `csv` crate errors precisely, etc.
- **Goal Check:** Phase 1 execution purely focuses on Discovery and Analysis. No tests were created. No code was modified.
- **Reporting Scope:** Strict focus on `src/bin/tasker.rs` and the `src/tasker/*` modules (plus shared utilities where used).

---

## 1. Argument Summary

### Argument: `--config` (and hidden legacy positional config)

* **Short option:** N/A
* **Required/optional:** Optional
* **Default value:** `tasker_config.json` relative to the executable directory.
* **Accepted values:** Path to a valid JSON configuration file.
* **Validation rules:** Must point to a file that can be parsed as JSON matching the `TaskerConfig` struct schema.
* **File where defined:** `src/bin/tasker.rs` (in `TaskerCliOptions`)
* **Code path triggered:** `main` -> `TaskerCliOptions::parse()` -> `run_app` -> `fs::read_to_string` -> `serde_json::from_str`.
* **Workflow:**
    1. CLI receives `--config <path>` (or legacy positional `<path>`).
    2. Parser sets `config` option.
    3. Application attempts to resolve the path. If it does not exist, a default configuration is written to the path.
    4. The file is read and parsed as JSON.
    5. A merge function merges any missing default values into the parsed JSON.
    6. If changes were made during the merge, the updated JSON is written back to the file.
    7. The JSON is deserialized into a strongly-typed `TaskerConfig` object.
    8. Application loops over the `tasks` array. Depending on the `type` tag of the JSON block (`csv_analysis` or `dashboard_updater`), it triggers `csv_task::run(config_block)` or `dashboard_updater::run(config_block)` to drive the entire downstream file processing logic.
* **Dependencies:** `clap`, `serde_json`, `std::fs`.
* **Files Affected:** Reads/Writes to the specified configuration file.
* **Possible Failure Points:**
    - File path points to an invalid directory (cannot write default).
    - File permissions restrict reading/writing.
    - JSON is syntactically invalid or semantically conflicts with `TaskerConfig` schema.
* **Existing Tests:** Minimal parsing test in `src/bin/tasker.rs` (`test_tasker_args_parsing`). Config serialization tests exist in `src/tasker/config.rs`.
* **Missing Tests:**
    - Edge cases around invalid JSON structures.
    - Edge cases for lack of file permissions.
    - Legacy positional argument mapping checks.

---

### Argument: `--task`

* **Short option:** N/A
* **Long option:** `--task`
* **Required/optional:** Optional
* **Default value:** None
* **Accepted values:** `usize` (unsigned integer representing task index, 1-based).
* **Validation rules:** Must be a valid positive integer.
* **File where defined:** `src/bin/tasker.rs` (in `TaskerCliOptions`)
* **Code path triggered:** `run_app` -> iterator loop over `config.tasks`.
* **Workflow:**
    1. CLI receives `--task <N>`.
    2. Argument parsed into `task_filter`.
    3. During the `config.tasks` iteration loop in `run_app`, `task_idx` (1-based) is compared to `task_filter`.
    4. If it does not match, the task execution is skipped.
    5. If it does match, the specific JSON configuration block for that index (e.g., `CsvAnalysisConfig`) is extracted and passed directly into the execution function (like `csv_task::run(config)`). The integer task number itself is never passed into the underlying functions; it acts purely as an array filter in `main`.
* **Dependencies:** None.
* **Files Affected:** None directly (controls execution).
* **Possible Failure Points:**
    - Out of bounds index (e.g., passing `10` when only 1 task exists). Fails silently (no tasks run).
    - Invalid integer input (caught by `clap` parsing).
* **Existing Tests:** None explicitly testing filtering logic.
* **Missing Tests:**
    - Test filtering successfully skips tasks.
    - Test filtering out-of-bounds index logs/behaves correctly.

---

### Argument: `--only-call-center`

* **Short option:** N/A
* **Long option:** `--only-call-center`
* **Required/optional:** Optional
* **Default value:** `false`
* **Accepted values:** Boolean flag (no value required).
* **Validation rules:** Handled as a pure flag by `clap`.
* **File where defined:** `src/bin/tasker.rs` (in `TaskerCliOptions`), mapped in manifest.
* **Code path triggered:** `run_app` -> `csv_task::run` -> `email::process_emails`.
* **Workflow:**
    1. CLI flag passed.
    2. Stored as `only_call_center` boolean.
    3. Passed into `csv_task::run(config, only_call_center, send_exceptions)` alongside the hydrated configuration object.
    4. `csv_task::run` passes this flag directly down to `email::process_emails`.
    5. Inside `email::process_emails`, if this flag is true, all standard team-based grouping logic (`send_per_team_all_branches`, `send_per_branch_branches`) is suppressed.
    6. Instead, it exclusively funnels tickets belonging to the "Call Center" into a single bucket, formats their HTML table specifically without standard email headers, and triggers `generate_leads_report` to attach a contextual leads `.xlsx` file before dispatching via PowerShell.
    5. Inside `email::process_emails`, if this flag is true, all standard team-based grouping logic (`send_per_team_all_branches`, `send_per_branch_branches`) is suppressed.
    6. Instead, it exclusively funnels tickets belonging to the "Call Center" into a single bucket, formats their HTML table specifically without standard email headers, and triggers `generate_leads_report` to attach a contextual leads `.xlsx` file before dispatching via PowerShell.
* **Files Affected:** Alters the output structure of generated emails and CSV reports if applicable to Call Center logic.
* **Possible Failure Points:**
    - Conflict with `--send-exceptions` (though current logic processes both conditionally).
* **Existing Tests:** None explicitly.
* **Missing Tests:**
    - Test combination with/without config flags.
    - Validate generated emails contain proper Call Center formats when flag is active.

---

### Argument: `--send-exceptions`

* **Short option:** N/A
* **Long option:** `--send-exceptions`
* **Required/optional:** Optional
* **Default value:** `false`
* **Accepted values:** Boolean flag.
* **Validation rules:** Handled as a pure flag by `clap`.
* **File where defined:** `src/bin/tasker.rs` (in `TaskerCliOptions`).
* **Code path triggered:** `run_app` -> `csv_task::run` -> `email::process_emails`.
* **Workflow:**
    1. CLI flag passed.
    2. Stored as `send_exceptions` boolean.
    3. Passed into `csv_task::run(config, only_call_center, send_exceptions)` alongside the hydrated configuration object.
    4. `csv_task::run` passes this flag directly down to `email::process_emails`.
    5. In `email.rs`, the CLI flag is merged with the JSON config field (`send_exceptions || config.send_exceptions`).
    6. When `effective_send_exceptions` evaluates to true, the standard ticket bucketing loop skips all "normal" tickets and filters exclusively for rows where `Is Exception == Yes`.
    7. These exception tickets are then dynamically grouped into buckets based solely on the teams defined in the `category_exceptions` JSON array, completely overriding standard routing and completely suppressing Call Center logic.
* **Dependencies:** Configuration must have valid `category_exceptions` mapped.
* **Files Affected:** Alters data filtering and assignment rules during email dispatch.
* **Possible Failure Points:**
    - Missing `category_exceptions` config leads to empty/skipped email sends despite the flag being active.
* **Existing Tests:** None explicitly.
* **Missing Tests:**
    - Verify flag overrides logic properly when combined with other email properties.
    - Verify empty `category_exceptions` handles gracefully.

---

### Argument: `--manifest`

* **Short option:** N/A
* **Long option:** `--manifest` (Hidden in Clap Help)
* **Required/optional:** Optional
* **Default value:** `false`
* **Accepted values:** Boolean flag.
* **Validation rules:** Intercepted natively before standard parsing.
* **File where defined:** Handled via `crm_tool::utils::intercept_manifest` before `clap` parsing in `src/bin/tasker.rs`.
* **Code path triggered:** `main` -> `intercept_manifest` (in `src/utils.rs`).
* **Workflow:**
    1. OS passes arguments to binary.
    2. `main` constructs an `AppManifest` object defining the tasker schema.
    3. Calls `intercept_manifest(manifest)`.
    4. If `--manifest` is in the raw OS arguments (`std::env::args()`), it serializes the manifest to JSON, prints it to stdout, and strictly calls `std::process::exit(0)`.
* **Dependencies:** `serde_json`, `std::env::args`.
* **Files Affected:** None.
* **Possible Failure Points:** None structurally, unless `serde_json::to_string` fails.
* **Existing Tests:** None for the CLI interception itself.
* **Missing Tests:**
    - E2E test verifying calling binary with `--manifest` returns valid JSON matching expected schema.

---

## 2. Configuration Switches (JSON File)

The JSON file (`tasker_config.json`) acts as an environment/execution driver for the tasks.

* **Task types:**
    - `csv_analysis`: Orchestrates `csv_task::run` (Data filtering, mapping, CSV output).
    - `dashboard_updater`: Orchestrates `dashboard_updater::run` (Reading CSV output, updating Excel dashboard files).
* **Key Environment variables / Switches:**
    * `download_path`: Path to input CRM CSV files.
    * `users_file` / `assignment_settings_file`: Mapping datasets.
    * `start_date`: Flexible date parsing (ISO, `today`, `yesterday`). Supports memory rule for normalizations.
    * `email_config`:
        * Sub-flags: `send_emails`, `send_call_center`, `send_exceptions` act identically to CLI but are baked into the task level.
        * `save_attachment_as_csv`, `save_email_as_html`: Output file triggers.

---

## 3. General Workflow & Dependency Map

**General Application Startup Workflow:**
1. OS launch -> `intercept_manifest()` check.
2. `setup_logging("task_csv_analysis")` called. TRACE level initialized.
3. `TaskerCliOptions::parse()` resolves CLI flags.
4. `run_app()` initialized. Configuration loaded/defaulted.
5. Task iterator runs.

**Task Dependencies:**
* **`csv_task` & `dashboard_updater` dependencies:**
    * Database interactions: None (purely file-system driven based on CSV/XLSX files).
    * External APIs: None directly.
    * Background processes:
        * Email sending logic in `email.rs` and `dashboard_updater.rs` invokes the host OS shell via `std::process::Command::new("powershell")`.
        * Temporary scripts (`.ps1`) are dropped into `std::env::temp_dir()`.
* **Services Affected:** Outlook COM Object (accessed via generated PowerShell scripts).

---

## 4. Potential Defects & Risk Analysis

1. **Race Conditions in PowerShell Script generation:**
    * **Defect:** `email.rs` and `dashboard_updater.rs` create temp scripts based on nanosecond timestamps. While an `AtomicUsize` counter mitigates overlaps, using a dedicated tempfile library (like `tempfile::NamedTempFile`) would be structurally safer and ensure automatic cleanup on panic.
2. **Missing Validation (Out of Bounds Filters):**
    * **Defect:** Providing a `--task` filter for a non-existent task silently skips execution without raising a warning or error.
3. **PowerShell Exit Errors not propagating strictly:**
    * **Defect:** In `email.rs:1241`, if the first email sending PowerShell script fails, a fallback PowerShell script runs to send an error email. If *that* fallback fails, it logs an error but still ultimately falls through to deleting temporary files and potentially returning `Ok(())` from the closure depending on parent scoping, masking critical failures.
4. **Platform Binding:**
    * **Defect/Debt:** The explicit binding to `powershell` with COM objects forces the binary to be Windows-only. It functions for this specific deployment scenario, but creates rigid debt.

---

## 5. Recommended Actions for Phase 2

**Fixes:**
1. Refactor PowerShell temp file generation to use `tempfile` to guarantee cleanup and eliminate name collision risks.
2. Enhance `run_app` task filtering to warn or error if `--task <N>` does not match any valid index.
3. Ensure error propagations from PowerShell explicitly fail the overarching task sequence if required.

**Recommended Tests (Missing Coverage):**
1. **CLI Tests:** Assert that passing `--only-call-center` properly initializes the boolean flag. Assert `--config` correctly overrides default paths.
2. **Task Filter Tests:** Provide mock configurations with multiple tasks, assert `--task 2` runs only the second config block.
3. **Config Merger Tests:** Deep unit tests for the recursive configuration JSON merger function located in `run_app` (currently untested logic inside `src/bin/tasker.rs`).
4. **Integration Tests for Task Execution:** Mock local CSV files matching `AGENTS.md` testing policies (`TestingDownloads`), feed them to `csv_analysis`, and verify output outputs strictly match expected configurations.

---
**END OF REPORT. WAITING FOR APPROVAL TO PROCEED TO PHASE 2.**
---

## 6. Core Function Deep Dives

### `csv_task::run` & `csv_task::generate_csv`
* **Purpose:** Acts as the primary data extraction and transformation engine. It aggregates raw CRM CSV reports, filters them based on configuration rules (branches, dates, exception categories), maps agents to teams using lookup files, and generates a unified `results.csv`.
* **Workflow:**
    1. **Initialization:** The function does not receive a task number. Instead, it receives a fully populated `CsvAnalysisConfig` object (extracted from `tasker_config.json` based on the `--task` filter in `main`), along with the `--only-call-center` and `--send-exceptions` boolean flags.
    2. **Data Loading (Lookups):** Reads the `users.csv` file to map `cognito_username` to `UserDepartmentName / Team Name`. It parses these fields into an `assignee_map`.
    3. **Assignment Rules:** Reads `assignments.csv` to map `(Category, Type, Subtype)` combinations to a specific default team, stored in `assignment_map`.
    4. **File Discovery:** Scans the target `download_path` for files matching `ticket_report*.csv`. It strictly filters these files based on their OS modification time, comparing them against the `minutes_ago` threshold.
    5. **Parsing & Filtering:** Uses a custom `build_csv_reader_from_reader` (which supports dynamic delimiter detection).
    6. **Row Iteration:** For every row:
        * Removes underscores from Type, Subtype, and Category fields.
        * Derives the final team assignment by consulting the `assignee_map` and falling back to `assignment_map` logic if the agent has no explicit team.
        * Deduplicates tickets using a `seen_tickets` HashSet based on `Ticket Id`.
        * Applies branch exclusions.
        * Applies category exclusions *unless* the category and branch explicitly match a rule in `category_exceptions`. If an exception rule matches, the row is flagged as `Is Exception: Yes` and the team is forcefully overridden.
        * Filters out rows created before the `start_date` parameter (parsing Excel float dates and ISO dates dynamically).
        7. **Output:** Writes the enriched rows to `results.csv`, appending four new columns: `Position`, `team`, `Is Exception`, and `Month`.
        8. **Post-Processing:** If an `email_config` is defined, it immediately invokes `email::process_emails` passing the path to the newly generated `results.csv`.

### `dashboard_updater::run`
* **Purpose:** Filters the generated `results.csv` to exclude exception tickets and seamlessly injects the clean data into an existing Excel (`.xlsx`) dashboard template using a dynamically generated PowerShell script.
* **Workflow:**
    1. **Initialization:** Like `csv_task::run`, this function does not receive a task index. It strictly accepts a hydrated `DashboardUpdaterConfig` object extracted directly from `tasker_config.json` based on the active CLI filter.
    2. **Data Generation:** Invokes `csv_task::generate_csv` to retrieve or generate the base dataset.
    3. **Exception Filtering:** Reads the generated `results.csv`. It streams the rows into a new temporary file (`dashboard_filtered_{timestamp}.csv`), explicitly dropping any row where `Is Exception` equals `Yes`, as well as entirely removing the `Position` column.
    4. **PowerShell Generation:** Crafts a rigid PowerShell script block.
    6. **Data Injection:** The script clears the old data body range of the Excel table, opens the temporary filtered CSV via Excel COM, copies its UsedRange, and pastes it into the destination table.
    7. **Execution & Cleanup:** The script is executed via `std::process::Command`. If successful, the temporary filtered CSV and PowerShell script are deleted.

### `email::process_emails`
* **Purpose:** Buckets the generated `results.csv` data by Team, Branch, or Call Center logic, generates HTML pivot tables, optionally generates contextual lead reports, and dispatches them via Outlook COM objects using PowerShell.
* **Workflow:**
    1. **Mapping Loading:** Parses `teams.csv` to resolve human-readable `Team Name` to `To Email`, `CC`, and `Receiver Name`.
    2. **Data Bucketing:** Reads `results.csv` into memory as a list of `TicketRow` structs.
        * If `effective_send_exceptions` is true: Filters purely for `Is Exception == Yes` and buckets by team name.
        * If `only_call_center` is true: Buckets all Call Center tickets into one list.
        * Otherwise: Evaluates `send_per_team_all_branches`, `send_per_branch_branches`, and `send_per_team_branches` to bucket rows accordingly.
    3. **Leads Generation (Call Center only):** If processing a Call Center bucket, it searches the download directory for `lead_report*.csv` files, parses them dynamically (supporting tabs or commas), applies branch exclusions, and writes them to a temporary `Call_Center_Leads.xlsx` using the `rust_xlsxwriter` crate.
    4. **HTML Generation:** For each bucket, calls `generate_pivot_html` to build an inline HTML table summarizing Open vs Closed ticket statuses across Assignees, Subtypes, and Categories. Assignees whose tickets are *all* closed are omitted from the HTML view.
    5. **Attachment Generation:** Generates a targeted CSV attachment for the specific bucket, filtering out "Closed" or "Resolved" tickets so the attachment only contains actionable items.
    6. **Email Dispatch:** Crafts a PowerShell script utilizing the `Outlook.Application` COM object. It merges the HTML body, adds the CCs and To addresses (respecting exception overrides vs global configs), attaches the targeted CSV and Leads XLSX (if applicable), and triggers `.Send()` or `.Display()`. If the Outlook COM operation fails, a fallback error notification script is generated and executed.
