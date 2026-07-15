# Agent Instructions for This Repository

All AI coding agents must read this file before starting work in this repository and follow these rules.

## Documentation Is Mandatory

After every code/config/dependency/script change, update relevant files in `md/` in the same change.

Reference policy: `md/AI_DOC_POLICY.md`

## Required Documentation Behavior

1. Determine impacted runtime behavior.
2. Update the corresponding markdown files under `md/`.
3. Keep examples and commands runnable.
4. Ensure docs mention new flags, config fields, and operational impacts.
5. Provide Webex Contact Center metrics logic inside `src/bin/wcxx.rs`.

## Completion Criteria

A task is not complete unless code and docs are both updated where behavior changed.

## `csv_analysis` Task Testing
If you modify `src/tasker/csv_task.rs` or any logic related to the `csv_analysis` task, you **MUST** run a test mapping user pastebin inputs to their expected output file before submitting the Pull Request. Validate that joins, columns, filters, and row-counts map effectively using the provided datasets.

## Repository Memories

- Concurrency in yasweb is controlled exclusively via the concurrency field in yasweb_config.json (default 6) and is used to chunk date ranges for parallel task execution; it is not exposed as a CLI argument.
- To ensure logging parity between application-specific logs (e.g., crm.log) and the runner's task logs, setup_logging in src/utils.rs configures the file layer with a TRACE level filter and the stdout layer with a DEBUG level filter.
- Commit codebase refactoring and remediation changes incrementally, making individual 'one by one' Git commits for each distinct fix.
- The crm tool's configuration logic normalizes from_date, to_date, and calls_from_date to ISO format upon loading and finalization (finalize_runtime_fields) to ensure consistent data processing regardless of the format used in config.json.
- The yasweb application maps global --start-date and --end-date arguments into a report's active filters using the start_date_key and end_date_key defined in yasweb_config.json. These keys are optional; if missing, date filtering is skipped and --monthly execution runs the report once instead of chunking by month.
- System-wide logging verbosity is set to TRACE across all binaries (crm, yasweb, tasker, wcxx, runner) for the file logs, while stdout is set to DEBUG. This includes detailed tracing of browser actions, network requests/responses (with headers), and data processing steps to ensure maximum visibility for debugging and script improvement.
- The yasweb binary can append a timestamp (_{HHMMSS}) to output filenames if the --add-time-to-file CLI flag is used.
- Avoid unnecessary intermediate Vec allocations, such as using .collect::<Vec<_>>().join("") when .collect::<String>() can be used directly.
- For complex HTML generation in the runner's GUI (src/runner/gui.rs), the format! macro should use named arguments to enhance readability and maintainability.
- The tasker application implements diagnostic logging for CSV parsing errors in lead and ticket reports. When a parsing error occurs (e.g., due to unquoted newlines), it logs a diagnostic context of ±20 lines surrounding the problematic line, using accurate positioning from the csv crate's error position.
- Centralized date parsing is provided by parse_flexible_date in src/utils.rs, which supports multiple formats (ISO, dashes/slashes, and abbreviated month names like 'May') to ensure robust date handling across crm, yasweb, and tasker binaries.
- When starting a new task, always initiate a deep planning mode: ask clarifying questions to eliminate assumptions, then create and seek approval for a plan. Once approved, execute the plan autonomously without asking for further confirmation.
- When configuring UI element matching (e.g., via XPath or injected JavaScript) in the yasweb application, ensure that the values defined in the AppManifest options (such as report types) exactly match the case of the text displayed on the web page to prevent element selection errors.
- The runner application tracks its global concurrency state using running_tasks_count and queued_tasks_count within the RunnerStatus struct to provide UI feedback on active and pending tasks.
- The runner application supports grid-aligned Interval scheduling using a start_time field (e.g., 08:00) to align subsequent runs to specific hourly slots (09:00, 10:00, etc.).
- The test.yml GitHub Actions workflow should only trigger on Pull Requests when there are changes to Rust files (**/*.rs), Cargo.toml, or Cargo.lock.
- In the tasker application, the send_exceptions mode (CLI --send-exceptions) suppresses all specialized Call Center logic, including lead report generation and the unique Call Center email body format. Any 'Call Center' tickets that are exceptions are instead processed and reported as standard team exceptions.
- In the tasker application's configuration, exclude_categories acts as a base filter to remove tickets. category_exceptions acts as an assignment override; if a ticket matches the exception's branch and category, its team is overridden to the exception's team and it is kept (bypassing the exclusion).
- The project uses the clap crate (specifically with the derive feature) for all CLI argument parsing across its binaries (crm, runner, tasker, wcxx, yasweb).
- Redundant wildcard patterns in struct destructuring (e.g., working_hours: _, ..) must be avoided as they trigger the clippy::unneeded_wildcard_pattern lint, which is treated as an error in the project's CI.
- In tasker email exports (HTML and Excel), the 'Position' column is entirely excluded. The 'Team' column is also hidden when sending emails specific to a single team.
- In tasker email configurations, teams listed in send_per_team_all_branches are processed regardless of whether their branch is present in send_per_branch_branches.
- When managing browser tabs with headless_chrome (e.g., in yasweb), always ensure that any MutexGuard obtained from browser.get_tabs().lock() is dropped before calling browser.new_tab() to prevent internal deadlocks.
- When send_exceptions is active (via CLI flag --send-exceptions or EmailConfig), the tasker application filters exclusively for tickets saved by the category_exceptions rule. It dynamically groups these exception tickets by the teams defined in category_exceptions, bypassing standard branch/team lists.
- The user permits adding new external crates (dependencies) to the project, provided they are proven safe and do not significantly increase the application's overall binary size.
- Lead report CSV files in the tasker application are parsed with dynamic delimiter detection (supporting both commas and tabs) to handle variations in CRM data exports.
- The GitHub Actions release workflow (release.yml) builds Windows binaries and packages them into a single zip file named crm_tool_v<version>_<datetime>_windows.zip, which is published directly as a GitHub Release (overwriting existing assets on the same tag). Google Drive uploads are not used.
- The repository manages all of its multiple Rust binaries (runner, crm, yasweb, wcxx, tasker) via a single Cargo.toml file (package crm_tool) with autobins = false and explicit [[bin]] entries, rather than utilizing a Cargo workspace.
- The tasker application's generate_leads_report function resolves its download directory path relative to the executable directory and uses SystemTime for file age comparison to ensure reliable discovery across different local timezones and working directories.
- The crm binary accepts --start-date and --end-date CLI arguments to override configuration values. It normalizes various input formats (including DD-MM-YYYY and abbreviated months) to ISO format (YYYY-MM-DD) utilizing the shared parse_flexible_date and to_iso_date utilities.
- Do not use lazy defaults like .unwrap_or_default() unless semantically correct for the business logic. Avoid .unwrap() and .expect() in domain logic outside of tests to prevent panics.
- For Call Center and dynamically defined exception teams (from category_exceptions) in the tasker application, global initial_cc and ending_cc configurations are strictly ignored; only the CC addresses explicitly mapped in teams.csv are used.
- In the tasker application, the dashboard_updater task uses the results.csv generated by csv_analysis. It explicitly filters out rows where Is Exception is Yes to prevent exception tickets from being incorrectly included in the dashboard Excel file.
- Ensure strict compiler lints are present at the crate root (e.g., src/lib.rs), specifically #![forbid(unsafe_code)].
- When identifying blank tabs in headless_chrome (e.g., within yasweb), check if the tab's URL contains 'about:blank' or is an empty string (""), as newly created or un-navigated tabs may lack a URL entirely.
- When matching branches or categories in tasker's exception logic, use case-insensitive partial matching (.to_lowercase() and .contains()) rather than strict equality to account for slight data variations.
- The runner application's GUI task form utilizes a two-column grid layout for 'Post Run Script' and 'Timeout' fields to optimize space.
- In yasweb browser automation, a retry loop (5 attempts with 500ms intervals) is used to find the initial blank tab during browser startup, and the previous hardcoded 10-second navigation sleep has been replaced with explicit element-wait logic.
- The yasweb binary supports simultaneous report execution by utilizing isolated Chrome user data directories (profiles) named after each report.
- To suppress Chrome crash recovery prompts and "Restore pages" bubbles, yasweb uses the flags --disable-session-crashed-bubble, --no-first-run, --disable-infobars, and --skip-reopen-last-pages.
- The runner application utilizes an asynchronous ExecutionManager queue (via tokio::sync::mpsc) to manage concurrent task execution. It enforces concurrency rules to prevent simultaneous execution of tasks with identical IDs or identical external app configurations (matching App ID and arguments), queuing them instead to run sequentially.
- In the tasker Call Center report, lead status filtering is strictly restricted to English terms: 'new', 'follow up', and 'follow-up'. Arabic terms were removed as they are not present in the operational dataset.
- In asynchronous contexts (like Tokio), strictly avoid blocking I/O (e.g., std::fs); use tokio::fs instead to prevent executor starvation.
- The AppManifest JSON schema supports a depends_on object within AppArg, allowing the runner's frontend (form_script.js) to dynamically show or hide arguments based on the current values of parent arguments.
- Shared utility functions, including executable_dir, setup_logging, intercept_manifest, dynamic date variable replacement, parse_flexible_date, and to_iso_date format normalization, are centralized in src/utils.rs to adhere to DRY principles.
- For testing tasker CSV analysis (e.g., csv_task.rs), use the local mock datasets in the TestingDownloads directory instead of downloading remote files to avoid network timeouts and rate limiting constraints.
- The runner application uses an AppManifest JSON schema to dynamically configure external tasks. Child executables intercept the --manifest CLI flag (via crm_tool::utils::intercept_manifest) to print their schema as JSON and exit with code 0 to allow the runner to automatically generate GUI inputs and map arguments.
- The yasweb configuration logic ensures that existing start_date_key and end_date_key settings for a report are preserved when its other properties (like report type or filters) are updated via CLI arguments.
- In the yasweb binary, boolean-style CLI arguments like --add-time-to-file and --monthly are defined as bool types in the clap parser to ensure they behave as simple flags (true if present, false if absent).
- The project supports dynamic date variables (today, yesterday, tomorrow, eomonth) in date-related CLI arguments across yasweb and crm binaries, resolved via src/utils.rs.
- The tasker CLI uses the --only-call-center flag to explicitly trigger Call Center emails. Exception team emails are triggered dynamically via the --send-exceptions CLI flag or the send_exceptions configuration option.

- When a bug is fixed, a corresponding test case must be added to prevent future regressions. The fix and the test case should be documented together.
