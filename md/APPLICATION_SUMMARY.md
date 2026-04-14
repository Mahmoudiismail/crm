# CRM Tool — Application Summary

## What It Does

`crm_tool` is a Rust application that:

- Runs a `runner` layer for scheduling and task execution.
- Authenticates to AWS Cognito using SRP.
- Fetches CRM report payloads (tickets, calls, leads).
- Extracts signed download URLs from response JSON.
- Downloads CSV files to a local folder.
- Runs automatically on task schedule and manually from tray + GUI.

## Runtime Style

- Tray-oriented app (`#![windows_subsystem = "windows"]`).
- Single-instance lock via TCP bind on `127.0.0.1:14592`.
- Async orchestration with `tokio`.
- Non-blocking logs to file + stdout.
- Embedded runner GUI HTTP server bound from config (`gui_host`, `gui_port`).

## Main Workflow

1. Load `runner_config.json`.
2. Start scheduler loop and runner GUI server.
3. Run tasks from runner config (`crm_fetch` and optional shell commands).
4. For CRM tasks, load CRM config, authenticate, fetch, and download.
5. Persist task run metadata (`next_run_at`, `last_status`, `last_run_at`).

## Scheduler + Manual Triggers

- Scheduler polls at `poll_interval_seconds` from runner config.
- Task execution supports `repetition` (`once` or `repeat`) and `frequency_seconds`.
- Tray and GUI can trigger run-all, tickets-only CRM, or specific task by id.
- Atomic run guard prevents overlapping task execution.
- Shell tasks are controlled by runner safety policy (`allow_shell_tasks`, timeout, min interval).

## Primary Outputs

- `crm_tool.log` in executable directory.
- `download/*.csv` in executable directory.
- Optional JSON output path per task (`task.output`).

## Modules

- `src/main.rs`
- `src/runner/config.rs`
- `src/runner/engine.rs`
- `src/runner/gui.rs`
- `src/crm/mod.rs`
- `src/crm/auth.rs`
- `src/crm/config.rs`
- `src/crm/fetcher.rs`
- `src/crm/downloader.rs`
