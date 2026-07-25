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

When a task is executed on its schedule, the `runner/engine.rs` now executes it using a deterministic Pipeline Execution Engine. Tasks are represented as a sequence of `TaskStep`s. Each `TaskStep` contains one or more `ActionSpec`s (representing a `shell_command` or an `external_app`) and operates in either `Sequential` or `Parallel` execution mode.

- **Sequential TaskSteps** execute each action one-by-one. The step fails immediately if any action fails.
- **Parallel TaskSteps** execute all actions concurrently, waiting for all to complete before continuing. If any action fails, the step is considered failed.

If the primary pipeline completes successfully, any configured `post_run_steps` are subsequently executed using the same pipeline mechanics. The engine leverages `tokio::process` to spawn and track process states, piping logs, and capturing timeouts on a per-action basis.

## Data Persistence

*   **Config State:** Configuration files (`runner_config.json`, `config.json`, `yasweb_config.json`, `wcxx_config.json`, `tasker_config.json`) are automatically created in the exact same directory as their relative executables with sane defaults if missing.
*   **Logging State:** Application logs (`runner.log`, `crm.log`, `yasweblog`, `wcxx.log`, `task_csv_analysis.log`, etc.) are heavily emitted into the exact same local execution directory using non-blocking thread workers.

## Components Update

- **`tasker`:** Aggregates and emails reports based on configured bucket logic. Supports `--send-exceptions` to dynamically read teams mapped in `category_exceptions` and group exception tickets dynamically, using only mapped CC lists and ignoring standard global logic.
### Recent Fixes
