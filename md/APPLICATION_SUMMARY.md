# CRM Tool — Application Summary

## What It Does

This project ships three Rust executables:

- `runner`: tray scheduler + GUI + task engine
- `crm`: one-shot CRM fetch executable
- `yasweb`: headless browser automation tool for Yasweb login

Release builds are optimized for maximum runtime performance and minimal file size through the Cargo release profile (`opt-level=3`, `lto=fat`, `strip=symbols`, `panic=abort`). GitHub release publishing is split by executable so `runner_windows.zip`, `crm_windows.zip`, and `yasweb_windows.zip` can be built and uploaded independently.

Together they:

- Run a `runner` layer for scheduling and task execution.
- Authenticate to AWS Cognito using SRP.
- Fetch CRM report payloads (tickets, calls, leads).
- Extract signed download URLs from response JSON.
- Download CSV files to a local folder.
- Run automatically on task schedule and manually from tray + GUI.

## Runtime Style

- `runner` is tray-oriented (`#![windows_subsystem = "windows"]` on Windows).
- `crm` is a console-style one-shot command.
- `yasweb` runs headless browser automation using `headless_chrome`.
- Single-instance lock via TCP bind on `127.0.0.1:14592`.
- Async orchestration with `tokio`.
- Non-blocking logs to file + stdout.
- Embedded runner GUI HTTP server bound from config (`gui_host`, `gui_port`) with Tailwind CSS loaded from cdnjs.

## Main Workflow (runner)

1. Load/create `runner_config.json` under executable directory.
2. Ensure CRM `config.json` exists under executable directory.
3. Start scheduler loop and runner GUI server.
4. Run tasks from runner config (`crm_fetch` and optional shell commands).
5. For CRM tasks, invoke external `crm` executable with CLI args.
6. Persist task run metadata (`next_run_at`, `last_status`, `last_run_at`).

## Main Workflow (crm)

1. Parse runtime CLI args.
2. Resolve/create `config.json` under executable directory (or provided path).
3. Authenticate via Cognito SRP.
4. Fetch requested report set.
5. Download CSV artifacts when enabled.
6. Exit process.

Supported CRM args:

- `--report all|tickets|calls|leads|none`
- `--config <path>`

CRM always performs login.

## Scheduler + Manual Triggers

- Scheduler polls at `poll_interval_seconds` from runner config.
- Task execution supports legacy `repetition`/`frequency_seconds` fields and the newer multi-schedule `schedules` list.
- Schedule summaries, next-run times, and last-run times are rendered in human-readable local time in the GUI.
- Tray and GUI can trigger run-all, tickets-only CRM, or specific task by id.
- Atomic run guard prevents overlapping task execution.
- Shell tasks are controlled by runner safety policy (`allow_shell_tasks`, timeout, min interval) and can run commands sequentially or in parallel.

## Main Workflow (yasweb)

1. Load/create `yasweb_config.json` under executable directory.
2. Launch a headless Chrome browser.
3. Attach Chrome DevTools Protocol network listeners to log events.
4. Navigate to the configured Yasweb URL.
5. Identify and fill the username.
6. Wait briefly for external data to load, then fill the password.
7. Submit the login form.
8. Verify login success by checking if the username appears in the header.
9. Click the pinned menu button and select the MIS module.
10. Logs are written to the `yasweblog` file.

## Primary Outputs

- `runner.log` for runner executable.
- `crm.log` for crm executable.
- `yasweblog` for yasweb executable containing network request events.
- `Downloads/*.csv` in executable directory.

## Modules

- `src/lib.rs`
- `src/bin/runner.rs`
- `src/bin/crm.rs`
- `src/bin/yasweb.rs`
- `src/runner/config.rs`
- `src/runner/engine.rs`
- `src/runner/gui.rs`
- `src/crm/mod.rs`
- `src/crm/auth.rs`
- `src/crm/config.rs`
- `src/crm/fetcher.rs`
- `src/crm/downloader.rs`
