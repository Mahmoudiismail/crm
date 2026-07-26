# Summary of Capabilities

The system is a collection of Rust binaries designed to:
- Act as a local GUI runner orchestrator.
- Automate HTTP fetch requests dynamically to bypass token challenges (CRM).
- Extract filtered CSV exports via Selenium-style Headless Chrome DOM manipulations (Yasweb).
- Validate, filter, merge, and pivot heavily-structured CSV data files.
- Interact with COM objects natively to transmit reports effectively via internal channels using Outlook, injecting directly manipulated HTML tables.
- Actively manipulate COM instances of Excel to refresh sophisticated internal OLAP/Power Query Data Models directly connected to extracted CSVs, and interact robustly with Native and OLAP Pivot Slicers dynamically prior to extracting tabular results back into the system via dynamically constructed PowerShell scripts (CRM Open Sohail).

# Core Runner Engine Details

- Run a `runner` layer for scheduling and task execution.
- Auto-generate GUI arguments by invoking worker binaries dynamically with `--manifest` to resolve `AppManifest` definitions natively.
- GUI forms automatically generate correct dynamic fields, variables, checkboxes, and multi-selection structures.
- Support deep persistence layer auto-healing via `merge_json` handling nested array atomicity explicitly.
- Centralize all configurations securely without hardcoded credentials.
- Task execution operates on a deterministic pipeline engine (Sequential/Parallel TaskSteps supporting multiple actions). Schedules support legacy `repetition`/`frequency_seconds` fields and the newer multi-schedule `schedules` list.
- Run concurrent local web servers supporting HTML templates, REST APIs, and embedded frontend functionality.
- Atomic run guard prevents overlapping task execution.

# Worker Details

1. Automatically parse authentication payloads, sign parameters cryptographically (SRP), bypass Cloudflare WAF, and download required payload links robustly.
2. Provide comprehensive automated CLI reporting tools exposing standard inputs (`--report tickets,calls`, `--start-date`).
3. Leverage multi-threaded concurrent async streams using `tokio::join!` strictly isolated boundaries for high-performance concurrent data downloads, API fetching, and headless browser instance scraping.
4. Auto-chunk date ranges recursively on file-size exceptions.
5. Inject and extract Javascript dynamically for bypassing UI restrictions (Yasweb).
6. Provide deep CSV integration testing structures, dynamically parsing context frames directly to stdouts on failures via centralized generic tools.
7. Support highly variable configurations allowing custom routing per-category/branch exceptions and robust string/path handling logic dynamically resolving relative dependencies to execution boundaries.
8. Inject data via COM strictly preventing Excel exceptions (`0x800A03EC`) via native property disablements natively embedded within scripts.
9. Support deep PowerShell integration via custom string parsing scripts bridging multi-type boundaries (double-parsing fallback arrays natively mapped via Rust structures).
10. Send rich HTML outputs safely bypassing maximum file size barriers.
11. Support dynamic variables universally mapped to Date inputs allowing recursive logic calculations cleanly across all binary ecosystems (`today`, `yesterday`, `tomorrow`, `eomonth`).
12. Automatically populate filter dates dynamically mapped to `start_date_key` and `end_date_key` via `yasweb_config.json` during monthly execution.

## Executables
- `crm`: Fetches logs and API payloads via CLI arguments.
- `runner`: Exposes the HTTP dashboard and pipeline engine.
- `tasker`: Parses CSV structures, manipulates DOM arrays, communicates to Excel COM systems, and issues Emails dynamically.
- `wcxx`: Handles standard API metric fetching asynchronously.
- `yasweb`: Performs automated Headless Chrome iterations utilizing CDP events, concurrent tabs within a shared session, custom download paths via CLI, and advanced wait conditions dynamically processing MIS filters.

## Common paths
- `tasker_config.json` for tasker rules logic.
- `yasweb_config.json` for DOM mapping.
- `runner_config.json` for pipeline persistence.
- `teams.csv`, `users.csv` for structural mapping tasks.
- `runner.log` for runner executable.
- `logs/<task_name>/YYYYMMDD_HHMMSS_<task_name>_<task_id>.log` for detailed per-task execution logs.
- `wcxx.log` for wcxx execution logs.
