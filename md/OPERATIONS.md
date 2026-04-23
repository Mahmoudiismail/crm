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
- shell command `mode` (`sequential` or `parallel`)
- per-command `continue_on_error` when a failure should not fail the task

### 7) Task create/update fails from GUI

Check:

- task `id` format (letters/numbers/`-`/`_` only)
- `id` uniqueness (no duplicate IDs)
- non-empty task `name`
- valid RFC3339 `next_run_at` when provided
- use the runner GUI schedule editor with interval/once rows and the + button to add more schedules
- non-empty shell command text for `shell_command` tasks

Note: As of the GUI improvements, the `+ Add schedule` and `+ Add command` buttons now have proper event listener scoping and will reliably add new rows to the form.

Use `GET /tasks` to confirm persisted task state after edits.

- Successful create/update/run/enable/disable/delete actions redirect back to the dashboard and show a toast notification.

The GUI shows schedule, next-run, and last-run values in local human-readable time. Use `GET /tasks` when exact RFC3339 timestamps are needed for troubleshooting.

### 8) GUI button visibility issues

If action buttons appear invisible or have low contrast:

- Update to a version with GUI improvements applied (buttons use `bg-emerald-600` with white text)
- Check Tailwind CSS is loading from CDN (styles will degrade gracefully if unavailable)
- Verify browser isn't applying custom CSS extensions that might override button styling

### 8) Yasweb browser automation issues

If the browser is unable to render elements, `yasweb` currently implements long wait times, loops, and aggressive HTML extraction. Inspect `yasweblog` to view the page HTML before/after steps if elements fail to appear. Certificate errors are ignored by default via launch options. The menu button (`#menuPinnedBtn`) interaction is optimized to avoid double-clicking and provides diagnostic class list logging on failure. The menu button (`#menuPinnedBtn`) interaction is optimized to avoid double-clicking and provides diagnostic class list logging on failure.

Check:

- `yasweb_config.json` for target URL and credentials (username/password)
- headless/visible mode setting
- whether the page layout matches expected selectors (`span.usr-id`, `#menuPinnedBtn`, `.misManagement`) and iframe internal selectors (`#mat-input-0`)
- the report selection logic requires navigating into an iframe; it searches for mat-radio-buttons matching the text, enters the report name into the search box (`#mat-input-0`), and simulates an "Enter" key press. If the iframe path or inner structure changes significantly, the JS evaluation block may need updates.
- the application includes a 2-second sleep after typing the username, to accommodate the login page fetching external data before the password field becomes available.

### 9) Runner cannot execute CRM

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
- Release workflow uses `actions/checkout@v6`, `actions/cache@v5`, and `softprops/action-gh-release@v3` for improved build performance.

### Yasweb Browser Automation

When executing the `yasweb` headless browser:
- The browser now starts maximized by default for consistent element rendering and visibility during debug runs.
- Chrome's user data and cache are persisted to a `yasweb_chrome_data` directory alongside the executable, significantly reducing load times on subsequent runs.
- The automation re-uses the initially launched browser tab instead of opening new ones to conserve memory.
- Wait loops dynamically poll for specific success (`.usr-id`) or failure (`.alert-danger`) elements, allowing the tool to fail-fast upon invalid logins without arbitrary delays.
- A mandatory 60-second delay is enforced at the end of the script before exiting. This pause occurs regardless of whether the run was successful or failed, allowing operators time to visually inspect the final browser state when running in non-headless mode.
