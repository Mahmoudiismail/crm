# CRM Tool — Application Summary

## What It Does

This project ships five Rust executables:

- `runner`: tray scheduler + GUI + task engine
- `crm`: one-shot CRM fetch executable
- `yasweb`: headless browser automation tool for Yasweb login
- `wcxx`: fetching tool for Webex Contact Center data (calendars, agents, teams, queues, skills)
- `tasker`: background worker for CSV processing and emailing

Release builds are optimized for maximum runtime performance and minimal file size through the Cargo release profile (`opt-level=3`, `lto=fat`, `strip=symbols`, `panic=abort`). GitHub release publishing is split by executable so `runner_windows.zip`, `crm_windows.zip`, `yasweb_windows.zip`, and `wcxx_windows.zip` can be built and uploaded independently.

Together they:

- Run a `runner` layer for scheduling and task execution.
- Dynamically register and orchestrate apps using an `AppManifest` JSON schema.
- Authenticate to AWS Cognito using SRP.
- Fetch CRM report payloads (tickets, calls, leads).
- Extract signed download URLs from response JSON.
- Download CSV files to a local folder.
- Run automatically on task schedule and manually from tray + GUI.
- Fetch Webex CC metrics securely using an OAuth Access Token.

## Runtime Style

- `runner` is tray-oriented (`#![windows_subsystem = "windows"]` on Windows).
- Tasks can execute `ExternalApp` commands via `tokio::process`, mapping dynamically to apps registered in the GUI.
- Multiple reports can be run concurrently by setting up task chains using `shell_command` or `external_app`.
- `crm` is a console-style one-shot command.
- `yasweb` runs headless browser automation using `headless_chrome`.
- `wcxx` is a CLI tool that opens an HTML export dynamically in the browser.
- Single-instance lock via TCP bind on `127.0.0.1:14592`.
- Async orchestration with `tokio`.
- Non-blocking logs to file + stdout.
- Embedded runner GUI HTTP server bound from config (`gui_host`, `gui_port`) with Tailwind CSS loaded from cdnjs.

## Main Workflow (runner)

1. Load/create `runner_config.json` under executable directory.
2. Ensure CRM `config.json` exists under executable directory.
3. Start scheduler loop and runner GUI server.
4. Run tasks from runner config (`shell_command`, and `external_app`).
5. For external app tasks, execute the dynamically registered binary utilizing `--config` and any extra args.
6. Persist task run metadata (`next_run_at`, `last_status`, `last_run_at`).

## Main Workflow (crm)

1. Parse runtime CLI args (intercepts `--manifest` for app registration).
2. Resolve/create `config.json` under executable directory (or provided path).
3. Authenticate via Cognito SRP.
4. Fetch requested report set.
5. Download CSV artifacts when enabled.
6. Exit process.

Supported CRM args:

- `--manifest`
- `--report all|tickets|calls|leads|none`
- `--config <path>`

CRM always performs login.

## Scheduler + Manual Triggers

- Scheduler polls at `poll_interval_seconds` from runner config.
- Task execution supports legacy `repetition`/`frequency_seconds` fields and the newer multi-schedule `schedules` list.
- Schedule summaries, next-run times, and last-run times are rendered in human-readable local time in the GUI.
- Tray and GUI can trigger run-all, tickets-only CRM, or specific task by id.
- Atomic run guard prevents overlapping task execution.
- Shell tasks are controlled by runner safety policy (`allow_shell_tasks`, per-task timeouts or global fallback timeout, min interval) and can run commands sequentially or in parallel.

## Main Workflow (yasweb)

1. Parse CLI arguments (intercepts `--manifest` for app registration).
2. Load/create `yasweb_config.json` under executable directory.
3. Parse CLI inputs (`--name`, `--type`, `--filters`, `--monthly`, `--start-date`, `--end-date`).
4. If `--monthly` is set, chunk the date range into monthly segments.
5. Launch a single headless Chrome browser session.
6. For each date chunk (batched up to 6 concurrently), launch a new browser tab.
7. Set a unique temporary download directory per tab via CDP (`Page.setDownloadBehavior`) to isolate concurrent downloads.
8. Attach Chrome DevTools Protocol network listeners to log events.
9. Navigate to the configured Yasweb URL.
10. Identify and fill the username and password, submitting the login form.
11. Open menu via "#menuPinnedBtn", toggle it, and click the MIS Reports module.
12. Automatically populate filter dates dynamically mapped to `start_date_key` and `end_date_key` via `yasweb_config.json` during monthly execution.
13. Execute the report export via JS automation within the iframe.
14. Wait up to 3 minutes for `.xlsx` downloads in the tab's temporary download directory to complete, rename the file accurately, and move it to a central `downloads` output folder.
15. Logs are written to the `yasweblog` file. HTML content is extracted and logged heavily across all stages (successes and failures) for debugging purposes. Certificate errors are ignored during browser instantiation.

## Main Workflow (wcxx)

1. Parse CLI arguments (`--manifest` or `--config` to specify the config file path, defaulting to `wcxx_config.json`).
2. Read the `wcxx_config.json` configuration for the Webex Contact Center base URL, optional org ID, and Bearer token.
3. Automatically generate a template `wcxx_config.json` and exit if none exists.
4. Using the provided token, iterate over the organization endpoints (`/calendars`, `/agents`, `/teams`, `/queues`, `/skills`) and fetch the data asynchronously using `reqwest`.
5. Aggregate the responses into a single JSON map.
6. Generate a temporary HTML file embedding the JSON data payload formatted nicely.
7. Use the default system web browser to view the retrieved data via the HTML file.
8. Logs all operations strictly to a flat file `wcxx.log` via tracing.

## Primary Outputs

- `runner.log` for runner executable.\n- `logs/<task_name>/YYYYMMDD_HHMMSS_<task_name>_<task_id>.log` for detailed per-task execution logs.
- `crm.log` for crm executable.
- `yasweblog` for yasweb executable containing network request events.
- `wcxx.log` for wcxx execution logs.
- `Downloads/*.csv` in executable directory.
- `wcxx_output.html` (in OS temp dir) for browser presentation.

## Modules

- `src/lib.rs`
- `src/manifest.rs`
- `src/bin/runner.rs`
- `src/bin/crm.rs`
- `src/bin/yasweb.rs`
- `src/bin/wcxx.rs`
- `src/bin/tasker.rs`
- `src/runner/config.rs`
- `src/runner/engine.rs`
- `src/runner/gui.rs`
- `src/crm/mod.rs`
- `src/crm/auth.rs`
- `src/crm/config.rs`
- `src/crm/fetcher.rs`
- `src/crm/downloader.rs`
- `src/tasker/*`
