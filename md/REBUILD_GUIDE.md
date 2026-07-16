# AI Rebuild Guide

This document provides the essential structural and architectural blueprint required for an AI to regenerate the entire CRM Tool application suite from scratch. It distills the core ideas, data models, and binary logic flows without needing the full original source code.

## 1. The Core Idea

The system is a decentralized, multi-binary orchestration suite primarily targeting Windows.

- **The `runner` daemon** is the central nervous system. It schedules tasks, exposes a local HTTP GUI, and orchestrates other executables via `tokio::process`.
- **Worker Apps (`crm`, `yasweb`, `wcxx`, `tasker`)** are independent CLI tools that perform specific domain tasks (fetching data, web scraping, processing CSVs).
- **The Orchestration Contract (`AppManifest`)**: Instead of hardcoding how to run every worker, the `runner` dynamically registers them. If executed with `--manifest`, any worker app must print a JSON schema (the `AppManifest`) describing its required/optional CLI arguments and immediately exit. The `runner` uses this schema to generate GUI forms and spawn the worker with the correct parameters.

## 2. Core Data Models

To recreate the application, you must replicate the state structures used to parse JSON configs. All configuration structs should use `serde` for serialization.

### The Manifest Schema (`src/manifest.rs`)
Used by all worker bins to self-report their capabilities to the runner.

```rust
pub struct AppManifest {
    pub name: String,
    pub description: String,
    pub arguments: Vec<AppArg>,
}

pub enum ArgType { String, Number, List, Boolean, DateVar }

pub struct AppArg {
    pub name: String,
    pub arg_type: ArgType,
    pub required: bool,
    pub default_value: Option<String>,
    pub options: Option<Vec<String>>,
}
```

### Runner Config (`src/runner/config.rs`)
The state file for the `runner` daemon (`runner_config.json`).

```rust
pub struct RunnerConfig {
    pub gui_host: String,
    pub gui_port: u16,
    pub poll_interval_seconds: u64,
    pub tasks: Vec<RunnerTask>,
    pub registered_apps: Vec<RegisteredApp>,
    // Timings and bounds...
}

pub struct RegisteredApp {
    pub id: String,
    pub name: String,
    pub executable_path: String,
    pub config_path: String,
}

pub struct RunnerTask {
    pub id: String,
    pub name: String,
    pub kind: TaskKind, // Enum: ExternalApp, ShellCommand, etc.
    pub app_id: Option<String>, // Links to RegisteredApp
    pub arguments: HashMap<String, String>, // Mapped from AppManifest
    pub schedules: Vec<TaskSchedule>,
    // State tracking: last_run_at, next_run_at, etc.
}
```

### CRM Config (`src/crm/config.rs`)
Used by the `crm` binary (`config.json`) to authenticate and fetch APIs.

```rust
pub struct AppConfig {
    pub region: String,
    pub user_pool_id: String,
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub base_url: String,
    pub download_csv: bool,
    // Plus runtime cached fields: access_token, expiry, etc.
}
```

### Yasweb Config (`src/bin/yasweb.rs`)
Used for browser automation state (`yasweb_config.json`).

```rust
pub struct YaswebConfig {
    pub url: String,
    pub username: String,
    pub password: Option<String>,
    pub headless: bool,
    pub reports: HashMap<String, ReportConfig>,
}

pub struct ReportConfig {
    pub report_type: String,
    pub filters: HashMap<String, String>,
    pub start_date_key: Option<String>,
    pub end_date_key: Option<String>,
}
```

### Tasker Config (`src/tasker/config.rs`)
Defines background CSV processing and Outlook COM email dispatches.

```rust
pub struct TaskerConfig {
    pub tasks: Vec<TaskConfig>,
}

pub enum TaskConfig {
    CsvAnalysis(CsvAnalysisConfig),
}

pub struct CsvAnalysisConfig {
    pub download_path: String,
    pub users_file: String,
    pub output_file: String,
    pub email_config: Option<EmailConfig>,
    // Filtering logic (exclude_branches, category_exceptions)
}
```

## 3. Binary Entry Points & Workflows

### `runner`
1. Initializes `TcpListener` on `127.0.0.1:14592` as a single-instance lock.
2. Loads `runner_config.json`.
3. Starts the HTTP GUI Server (`src/runner/gui.rs`).
4. Enters an async `loop` (`src/runner/engine.rs`). Checks `poll_interval_seconds`.
5. For tasks due to run, maps their configured `arguments` against the dynamically registered `AppManifest`, builds a `tokio::process::Command`, and runs it.

### `crm`
1. If args contain `--manifest`, prints the JSON schema and exits 0.
2. Loads `config.json`.
3. Uses AWS Cognito SRP (Secure Remote Password) protocol to get an Access Token.
4. Uses `reqwest` to fetch JSON payloads.
5. If `download_csv` is true, extracts signed URLs from the payload and downloads them to `./Downloads/`.

### `yasweb`
1. If args contain `--manifest`, prints the JSON schema and exits 0.
2. Loads `yasweb_config.json`.
3. Launches a headless Chrome browser using the `headless_chrome` crate.
4. Uses the Chrome DevTools Protocol (`Page.setDownloadBehavior`) to assign unique temp directories per concurrent download tab.
5. Injects JS into the DOM iframe to fill dates/filters and trigger `.xlsx` generation.
6. Moves completed downloads to a central output folder.

### `wcxx`
1. If args contain `--manifest`, prints the JSON schema and exits 0.
2. Loads `wcxx_config.json`.
3. Uses a static Bearer Token to hit Webex CC APIs (e.g. `/calendars`, `/agents`).
4. Generates an HTML report embedding the JSON data and automatically opens the user's default browser to view it.

### `tasker`
1. If args contain `--manifest`, prints the JSON schema and exits 0.
2. Loads `tasker_config.json`.
3. Reads raw CSV datasets (handling bad UTF-8 with `String::from_utf8_lossy()`).
4. Processes the data according to the filters (`exclude_branches`, `category_exceptions`).
5. (Windows only) Interfaces with the local Outlook COM object via PowerShell scripts to send HTML emails with attached `.xlsx` reports.

## 4. Key Rebuild Rules
- **Non-blocking logging:** Logs must be written locally next to the executable, and log flush guards (`WorkerGuard`) must be held in `main()` until graceful termination.
- **Error Handling:** Avoid `unwrap()`. Bubble errors up to `main()` using `anyhow::Result` to ensure logging has time to flush.
- **Port Conflicts:** During test suites or concurrent execution, bind to ephemeral port `0` to prevent collisions.
