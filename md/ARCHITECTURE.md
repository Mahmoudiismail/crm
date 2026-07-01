# Architecture

The system is split into multiple executables designed to run together, orchestrated by a central `runner`. Executable behavior logic is shared and defined via library modules.

For a detailed guide on how to fundamentally reconstruct these components and their data models from scratch using AI, see the [AI Rebuild Guide](./REBUILD_GUIDE.md).

## Component Overview

1.  **Runner (`src/bin/runner.rs`, `src/runner/*`)**: The background daemon. It runs as a system tray application on Windows, provides a local HTTP GUI, parses the overall `runner_config.json`, manages a chron-like schedule, and orchestrates task execution (launching `shell_command` or dynamically registered `external_app` processes).
2.  **AppManifest System (`src/manifest.rs`, `md/MANIFEST_SCHEMA.md`)**: A JSON standard dictating how `crm`, `yasweb`, `wcxx`, and `tasker` advertise their command-line arguments to the `runner` application, allowing the runner GUI to construct inputs dynamically.
3.  **CRM Fetcher (`src/bin/crm.rs`, `src/crm/*`)**: A one-shot CLI utility handling Cognito SRP authentication, report payload requests (Tickets, Calls, Leads), and CSV downloading.
4.  **Yasweb Automation (`src/bin/yasweb.rs`)**: A headless Chrome automation utility used to log into an external Yasweb Angular dashboard, discover and configure filters, and extract generated data via iframe injection. It supports concurrent execution of monthly sliced reports via tab isolation and CDP file download interception.
5.  **WCXX Fetcher (`src/bin/wcxx.rs`)**: A simple CLI utility fetching operational metrics securely from the Webex Contact Center API and outputting them to a local JSON/HTML file for inspection.
6.  **Tasker (`src/bin/tasker.rs`, `src/tasker/*`)**: A backend utility dedicated to processing generated CSV datasets and transmitting HTML/Excel summary reports via Outlook COM automation or other channels.

## Execution Flow (Runner -> Dynamic Task)

The runner GUI exposes a central execution dashboard. If a user sets up a new application via the **Apps** page, the `runner` fetches its manifest via the `--manifest` flag (e.g. `crm.exe --manifest`).

When a `TaskKind::ExternalApp` is executed on its schedule, the `runner/engine.rs` dynamically converts the configured `AppManifest` argument values into CLI arguments, spawning the registered process (e.g. `crm.exe --report tickets --config path/to/config.json`), piping its logs, and tracking its termination status.

## Data Persistence

*   **Config State:** Configuration files (`runner_config.json`, `config.json`, `yasweb_config.json`, `wcxx_config.json`, `tasker_config.json`) are automatically created in the exact same directory as their relative executables with sane defaults if missing.
*   **Logging State:** Application logs (`runner.log`, `crm.log`, `yasweblog`, `wcxx.log`, `task_csv_analysis.log`, etc.) are heavily emitted into the exact same local execution directory using non-blocking thread workers.
