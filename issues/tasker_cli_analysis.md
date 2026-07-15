# Tasker Application CLI and Configuration Analysis

## Agent Compliance Summary
- **AGENTS.md Policy Adherence:** Verified memory items related to Tasker. Identified rules like ignoring exceptions for the dashboard, excluding 'Position' from emails, strict Call Center lead status filters, avoiding `unwrap_or_default()` when unneeded, generating local temporary scripts without locking `std::fs` across threads unnecessarily, handling `csv` crate errors precisely, etc.
- **Goal Check:** Phase 1 execution purely focuses on Discovery and Analysis. No tests were created. No code was modified.
- **Reporting Scope:** Strict focus on `src/bin/tasker.rs` and the `src/tasker/*` modules (plus shared utilities where used).

---

## 1. Argument Summary

### Argument: `--config` (and hidden legacy positional config)

* **Short option:** N/A
* **Long option:** `--config`
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
    8. Application proceeds to task execution iterating over `tasks` defined in the config.
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
    3. During task iteration in `run_app`, `task_idx` (1-based) is compared to `task_filter`.
    4. If it does not match, the task execution is skipped.
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
    3. Passed into `csv_task::run()`.
    4. Propagated to `email::process_emails()`.
    5. Modifies email dispatch logic: explicit trigger to process and generate "Call Center" specific lead reports and formats.
* **Dependencies:** `email.rs` processing logic.
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
    3. Passed into `csv_task::run()`.
    4. Propagated to `email::process_emails()`.
    5. In `email.rs`, it merges with `config.send_exceptions` (using `||` logic).
    6. When active, it suppresses Call Center logic and triggers grouping of tickets explicitly mapped in the `category_exceptions` configuration list.
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