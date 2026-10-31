# Architecture

## Repository and package structure

The repository builds a single Rust package, crm_tool, from one Cargo.toml. It declares five explicit binaries rather than separate Cargo packages:

| Binary | Main source | Responsibility |
|---|---|---|
| runner | src/bin/runner.rs, src/runner/** | Scheduling, local GUI, task admission, pipelines and child-process lifecycle |
| crm | src/bin/crm.rs, src/crm/** | CRM authentication, API requests and report downloads |
| yasweb | src/bin/yasweb.rs, src/yasweb/** | Headless browser report automation |
| tasker | src/bin/tasker.rs, src/tasker/** | CSV/report processing, Excel automation and Outlook workflows |
| crm_updater | src/bin/crm_updater.rs, src/crm_updater/** | Local diagnostics collection and application update orchestration |

Shared utilities, manifest types and common behavior are exported through src/lib.rs.

## Runtime responsibilities

### Runner

Runner loads runner_config.json, starts the scheduler and local HTTP GUI, and on Windows provides a system-tray interface. It dispatches configured shell commands or registered external applications. An external app can expose an AppManifest; Runner uses that schema to build dynamic GUI inputs and determine which CLI arguments the app supports.

Runner tasks can contain multiple steps and actions. Sequential steps execute actions in order; parallel steps execute actions concurrently. Post-run steps are a separate pipeline executed after the main pipeline succeeds. Application-level locks and the execution manager coordinate admission and prevent duplicate task execution while the same task is queued or running.

### CRM

CRM is a one-shot CLI worker. It authenticates against the configured CRM service, requests selected report types, downloads report payloads, and writes exports into the configured download directory. Ticket exports can be split into bounded date ranges and fetched concurrently; see [Fetcher](./FETCHER.md) and [Downloader](./DOWNLOADER.md).

### Yasweb

Yasweb automates configured reports through headless Chrome. Report definitions, selectors, filters, date-key mappings, and browser settings are stored in yasweb_config.json. A shared concurrency limit controls active browser operations in one Yasweb process. Runner owns recurring scheduling and date-period expansion; Yasweb receives dates for each invocation and does not independently split a Runner period into monthly batches.

### Tasker

Tasker is a configured worker that processes tasks from tasker_config.json. Its task implementations include CSV analysis, dashboard refresh, CRM Open Sohail, department splitting, OPD analysis, SAT figures and users export. It uses shared CSV utilities and, where configured, Windows Excel/Outlook COM automation through managed PowerShell scripts.

For csv_analysis, ticket output and lead output are distinct artifacts. The ticket result remains at output_file; when recent lead exports exist, lead_result.csv is written beside it. Lead team is derived from Assignee via the users mapping; only eligible Call Center leads are included in the Call Center leads attachment. Other teams remain in the durable lead result for review. See [Tasker](./TASKER.md).

### CRM Updater

CRM Updater supports two operational paths: collecting local logs/reports into timestamped folders and applying staged executable replacements. The replacement script runs detached so the updater process can exit before its own executable is replaced. The updater's package trust limitation is documented in [CRM Updater](./CRM_UPDATER.md) and [Production Readiness](./PRODUCTION_READINESS.md).

## Configuration and persistence

Each executable has a dedicated configuration file. Defaults are generally located beside the executable; CLI overrides and legacy path behavior are documented in [Configuration](./CONFIG.md) and [CLI](./CLI.md). Configuration writes use shared atomic-write/merge helpers where integrated. Runner mutations are serialized with in-process config access and read back after persistence to detect failed saves.

## Manifest contract

The AppManifest describes an application's name, description and accepted CLI arguments. Argument metadata supports data types, required flags, defaults, options, dependencies and autofill. Runner uses the manifest both to construct app forms and to avoid passing unsupported date arguments. See [Manifest Schema](./MANIFEST_SCHEMA.md).

## Logging and process management

Applications use the shared tracing setup with configured stdout/file levels. Runner maintains a top-level scheduler log and per-task execution logs. Its task logger can recover after its log file is moved or deleted and writes a redacted configuration snapshot when the log is reopened. Task-specific logs contain detailed child-process output.

Managed PowerShell templates are maintained as source files and embedded at build time where wired through include_str!. ScriptManager persists managed scripts under the executable directory and passes runtime values as separate arguments/data files rather than interpolating them into script source.

## Security boundaries

- Runner's control GUI is intended for loopback access; non-loopback binding is rejected by startup validation.
- State-changing GUI operations require POST rather than GET.
- CSV parsing is strict by default; code paths requiring different parsing behavior must opt in explicitly and document the reason.
- Runtime secrets and configuration values must not be written to logs without redaction.
- CRM Updater's legacy ZIP password and archive hash checks do not authenticate the publisher; see its production gate.
