# Operations and Troubleshooting

## Logs

- Primary runner log file: `<exe_dir>/runner.log`
- Primary crm one-shot log file: `<exe_dir>/crm.log`
- Stdout logs at INFO level.

Key startup messages indicate:

- logging initialized,
- runner scheduler started,
- runner GUI bind address,
- tray initialized,
- task run activity.

## Health Checklist

1. Process is running once (single-instance lock not failing unexpectedly).
2. Runner GUI is reachable on configured `gui_host:gui_port`.
3. Task scheduler executes due tasks and updates `last_status`.
4. Authentication either reuses valid token or logs successful fresh login.
5. Fetch tasks complete with expected endpoints.
6. CSV files appear in `<exe_dir>/Downloads`.

## Common Failure Scenarios

### 1) Authentication fails

Check:

- `region`, `user_pool_id`, `client_id`
- `username` and `password`
- local clock drift
- network access to Cognito endpoint

### 2) Report fetch errors

Check:

- `base_url`
- `account_id`, `application_id`, `email`
- date format (`YYYY-MM-DD`)
- token validity

If logs show `Failed to generate signed url`, the CRM fetcher automatically retries smaller date ranges by halving the failing range. A remaining failure after the range reaches one day means the backend cannot produce a signed URL for that single-day export.

### 3) Download errors

Check:

- URL validity and expiry
- write permissions in executable/Downloads directory
- network timeout conditions

### 4) Tasks not triggering

Check:

- task `enabled=true`
- legacy task `next_run_at` format (RFC3339) or empty for immediate run
- `schedules` entries, especially enabled state, per-schedule `next_run_at`, interval seconds, and daily `HH:MM` local times
- legacy `repetition`/`frequency_seconds` values when `schedules` is absent or empty
- `poll_interval_seconds`

For `crm_fetch` tasks, verify `crm_executable_path` and `crm_config_path` in `runner_config.json` are correct.

### 5) Runner GUI unavailable

Check:

- `gui_host` and `gui_port` in `runner_config.json`
- firewall/local bind restrictions
- logs for GUI bind failures

### 6) Shell task blocked or timing out

Check:

- `allow_shell_tasks` in `runner_config.json`
- `shell_timeout_seconds` value
- `last_status` and runner `last_error` for timeout details
- command correctness under `bash -lc`
- command group `mode` (`sequential` or `parallel`)
- per-command `continue_on_error` when a failure should not fail the group

### 7) Task create/update fails from GUI

Check:

- task `id` format (letters/numbers/`-`/`_` only)
- `id` uniqueness (no duplicate IDs)
- non-empty task `name`
- valid RFC3339 `next_run_at` when provided
- valid schedule editor lines, for example `interval: every 1h`, `daily: 09:00, 13:00`, or `once: 2026-04-15T09:30:00-05:00`
- non-empty shell command text for `shell_command` tasks
- valid command group headers such as `@group Reports parallel`

Use `GET /tasks` to confirm persisted task state after edits.

The GUI shows schedule, next-run, and last-run values in local human-readable time. Use `GET /tasks` when exact RFC3339 timestamps are needed for troubleshooting.

### 8) Runner cannot execute CRM

Check:

- `crm_executable_path` points to valid `crm` binary (or default sibling executable)
- execution permission for `crm` binary
- runner timeout (`shell_timeout_seconds`) is sufficient
- `crm` command works manually with same args

Manual check example:

- `crm --config <path> --report tickets`
- `crm --config <path> --report none`

## Safe Recovery Steps

1. Stop app.
2. Backup `runner_config.json`, `config.json`, `runner.log`, and `crm.log`.
3. Clear token fields in CRM config if you need a full re-authentication.
4. Restart and validate auth + fetch flow.

## Release Validation (Minimal)

- `cargo check --target x86_64-pc-windows-gnu`
- `cargo test --target x86_64-pc-windows-gnu --no-run`
- `cargo build --release --bin runner`
- `cargo build --release --bin crm`
- one manual fetch run
- verify logs and output artifacts

GitHub release publishing is split by application:

- Run `Release Runner` to publish `runner_windows.zip`.
- Run `Release CRM` to publish `crm_windows.zip`.
- Both workflows use the `v<version>` tag from `Cargo.toml`; run both when a release should contain both applications.
