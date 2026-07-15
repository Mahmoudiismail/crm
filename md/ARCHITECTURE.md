# Architecture

The system is split into multiple executables designed to run together, orchestrated by a central `runner`. Executable behavior logic is shared and defined via library modules.

For a detailed guide on how to fundamentally reconstruct these components and their data models from scratch using AI, see the [AI Rebuild Guide](./REBUILD_GUIDE.md).

## Component Overview

1.  **Runner (`src/bin/runner.rs`, `src/runner/*`)**: The background daemon. It runs as a system tray application on Windows, provides a local HTTP GUI, parses the overall `runner_config.json`, manages a chron-like schedule, and orchestrates task execution (launching `shell_command` or dynamically registered `external_app` processes).
2.  **AppManifest System (`src/manifest.rs`, `md/MANIFEST_SCHEMA.md`)**: A JSON standard dictating how `crm`, `yasweb`, `wcxx`, and `tasker` advertise their command-line arguments to the `runner` application, allowing the runner GUI to construct inputs dynamically.
3.  **CRM Fetcher (`src/bin/crm.rs`, `src/crm/*`)**: A one-shot CLI utility handling Cognito SRP authentication, report payload requests (Tickets, Calls, Leads), and CSV downloading. Handles edge cases like ignoring empty configuration flags injected by the runner, and overriding configurations via CLI arguments such as `--custom-download-folder`.
4.  **Yasweb Automation (`src/bin/yasweb.rs`)**: A headless Chrome automation utility used to log into an external Yasweb Angular dashboard, discover and configure filters, and extract generated data via iframe injection. It supports concurrent execution of monthly sliced reports via tab isolation and CDP file download interception.
5.  **WCXX Fetcher (`src/bin/wcxx.rs`)**: A simple CLI utility fetching operational metrics securely from the Webex Contact Center API and outputting them to a local JSON/HTML file for inspection.
6.  **Tasker (`src/bin/tasker.rs`, `src/tasker/*`)**: A backend utility dedicated to processing generated CSV datasets and transmitting HTML/Excel summary reports via Outlook COM automation or other channels. It also parses `lead_report` files and attaches filtered reports for the Call Center. All CSV reading utilizes a shared `build_csv_reader` utility (in `src/utils.rs`) configured with `.flexible(true)` to gracefully handle files with variable column lengths.

## Execution Flow (Runner -> Dynamic Task)

The runner GUI exposes a central execution dashboard. If a user sets up a new application via the **Apps** page, the `runner` fetches its manifest via the `--manifest` flag (e.g. `crm.exe --manifest`).

When a `TaskKind::ExternalApp` is executed on its schedule, the `runner/engine.rs` dynamically converts the configured `AppManifest` argument values into CLI arguments, spawning the registered process (e.g. `crm.exe --report tickets --config path/to/config.json --start-date 2024-01-01`), piping its logs, and tracking its termination status.

## Data Persistence

*   **Config State:** Configuration files (`runner_config.json`, `config.json`, `yasweb_config.json`, `wcxx_config.json`, `tasker_config.json`) are automatically created in the exact same directory as their relative executables with sane defaults if missing.
*   **Logging State:** Application logs (`runner.log`, `crm.log`, `yasweblog`, `wcxx.log`, `task_csv_analysis.log`, etc.) are heavily emitted into the exact same local execution directory using non-blocking thread workers.

## Components Update

- **`tasker`:** Aggregates and emails reports based on configured bucket logic. Supports `--send-exceptions` to dynamically read teams mapped in `category_exceptions` and group exception tickets dynamically, using only mapped CC lists and ignoring standard global logic.
### Recent Fixes
- **Dashboard Updater & PowerShell Locking:** Fixed a bug on Windows where `tempfile::Builder` kept file handles open during PowerShell execution. The tempfile `.ps1` handle is now explicitly dropped via `.keep()` before calling PowerShell, and manually removed afterwards, resolving `The process cannot access the file because it is being used by another process` errors.
- **Dashboard CSV Filtering:** Refined filtering in `src/tasker/dashboard_updater.rs` to guarantee the `Position` and `Is Exception` columns are removed. Added logic to parse the `Created At` date and append it as explicitly formatted `Month` (e.g. `Jan`) and `Day` (e.g. `01`) columns for dashboard Excel injection.
- **Tasker Call Center Leads:** Corrected conditional logic in `src/tasker/email.rs` where `--only-call-center` did not trigger lead report generation. The `send_cc` flag now implicitly assumes true if `only_call_center` is explicitly specified via the CLI, ensuring leads files are generated and attached as intended.
- **PowerShell Execution Stability:** Fixed Windows file lock exceptions during PowerShell script execution in `src/tasker/email.rs` and `src/tasker/crm_open_sohail.rs` by correctly dropping the file handle before invoking `std::process::Command`, matching the pattern established in `src/tasker/dashboard_updater.rs`.
- **Automated Testing:** Per AGENTS.md policy, unit tests were created or adapted to cover PowerShell file unlocking and Dashboard parsing edge cases to prevent regressions.
