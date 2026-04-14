# Operations and Troubleshooting

## Logs

- Primary log file: `<exe_dir>/crm_tool.log`
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
6. CSV files appear in `<exe_dir>/download`.

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

### 3) Download errors

Check:

- URL validity and expiry
- write permissions in executable/download directory
- network timeout conditions

### 4) Tasks not triggering

Check:

- task `enabled=true`
- task `next_run_at` format (RFC3339) or empty for immediate run
- `repetition`/`frequency_seconds` values
- `poll_interval_seconds`

### 5) Runner GUI unavailable

Check:

- `gui_host` and `gui_port` in `runner_config.json`
- firewall/local bind restrictions
- logs for GUI bind failures

## Safe Recovery Steps

1. Stop app.
2. Backup `runner_config.json`, `config.json`, and `crm_tool.log`.
3. Clear token fields in CRM config (or disable `skip_login` on relevant task).
4. Restart and validate auth + fetch flow.

## Release Validation (Minimal)

- `cargo check --target x86_64-pc-windows-gnu`
- `cargo test --target x86_64-pc-windows-gnu --no-run`
- one manual fetch run
- verify logs and output artifacts
