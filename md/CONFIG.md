# Configuration

The application now uses three config files:

- `runner_config.json` (runner behavior, task storage, GUI host/port)
- `config.json` (CRM auth/report settings and token cache)
- `yasweb_config.json` (Yasweb browser automation target and credentials)

Runner behavior is config-driven. CRM executable behavior is controlled via CLI args. Yasweb runs headlessly according to its config file.

## 1) Runner Config (`runner_config.json`)

Implementation: `src/runner/config.rs`

### Top-level fields

- `gui_host`: GUI bind host (default `127.0.0.1`)
- `gui_port`: GUI bind port (default `8787`)
- `poll_interval_seconds`: scheduler tick interval (default `30`)
- `crm_config_path`: path to CRM config file (default `config.json`)
- `crm_executable_path`: crm executable file/path used by runner (default `crm.exe` on Windows, `crm` on non-Windows)
- `allow_shell_tasks`: allow `shell_command` tasks (default `false`)
- `shell_timeout_seconds`: max runtime per shell task (default `900`). A value of `0` means unlimited.
- `post_run_timeout_seconds`: max runtime for post-run scripts (default `900`). A value of `0` means unlimited.
- `min_task_interval_seconds`: clamp for repeat task minimum interval (default `5`)
- `tasks`: list of runnable task definitions

### Task fields

- `id`: unique task id
- `name`: display name
- `enabled`: whether task is active
- `repetition`: `once` or `repeat`
- `frequency_seconds`: interval used when `repetition=repeat`
- `next_run_at`: RFC3339 timestamp, empty means run immediately
- `schedules`: optional list of schedules. When present, this replaces the legacy `repetition`/`frequency_seconds`/`next_run_at` behavior for due-task detection.
- `kind`: tagged task payload
  - `crm_fetch` with `report` (`all`, `tickets`, `calls`, `leads`, `none`)
  - `shell_command` with execution `mode` and `commands`
- `post_run_script`: path to an optional script that executes only upon a successful task completion (`.vbs`, `.txt`, `.bat`, `.cmd`, `.ps1`, or direct executable).
- `last_run_at`: last run timestamp
- `last_status`: last run result message

### Schedule fields and cron-based evaluation

Each `schedules` item has a `type`:

- `once`: runs once at `next_run_at`; empty `next_run_at` means immediate. After execution, `enabled` is set to `false`.
- `interval`: runs every `every_seconds`; `next_run_at` stores the next due time. After execution, `next_run_at` advances by `every_seconds`.
- `daily_times`: runs at one or more local machine times in `times` (`HH:MM`); `next_run_at` stores the next calculated due time. After execution, `next_run_at` is recalculated for the next matching time.
- `weekly`: runs on a specific day of the week; `day_of_week` is day name (Monday, Tuesday, etc.), `at_time` is optional (`HH:MM` default 09:00). After execution, `next_run_at` advances to the next week.
- `monthly`: runs on a specific day of the month; `day_of_month` is 1-31, `at_time` is optional (`HH:MM` default 09:00). After execution, `next_run_at` advances to the next month.

**Cron Evaluation**: The scheduler polls on `poll_interval_seconds` and checks each enabled schedule:
- For each schedule, compare current UTC time with its `next_run_at` RFC3339 timestamp
- If current time >= `next_run_at`, the schedule is due and the task executes
- After task execution, the `advance_schedule()` function computes the next `next_run_at` based on schedule type

All persisted `next_run_at` and `last_run_at` values remain RFC3339 UTC. The GUI renders these values as local human-readable time with relative text.

### Shell commands

`shell_command` supports execution of multiple commands with an execution `mode`:

- `mode`: `sequential` or `parallel`.
- `commands`: list of command specs.
- `commands[].command`: shell text executed with `bash -lc`.
- `commands[].continue_on_error`: when `true`, a failed command does not fail the task.

A sequential task stops at the first failed command unless that command has `continue_on_error=true`. A parallel task starts all commands together, waits for all commands, and fails when any non-continued command fails.

### Task validation and normalization

When tasks are created/updated through the runner GUI CRUD endpoints:

- `id` is required and must contain only letters, numbers, `-`, `_`
- `name` is required
- `next_run_at` must be empty or valid RFC3339
- for `repetition=repeat`, `frequency_seconds` is clamped to at least `min_task_interval_seconds`
- `schedules` entries must use valid intervals, RFC3339 once timestamps, or `HH:MM` daily local times
- `shell_command.commands` must contain at least one non-empty command

`id` uniqueness is enforced across all tasks. Updates preserve `last_run_at` and `last_status` when these fields are not explicitly provided.

daily: 09:00, 13:00, 18:30
The GUI create/update forms now provide a simpler task editor:

- schedule rows with `Interval`, `Once`, `Daily`, `Weekly`, or `Monthly` options
- interval dropdown of common durations: `15m`, `30m`, `1h`, `2h`, `4h`, `8h`, `12h`, `24h`, `2d`, `7d`
- a date/time picker for one-time schedules
- day-of-week selector for weekly schedules
- day-of-month selector (1-31) for monthly schedules
- a `+ Add schedule` button for multiple entries

Shell commands can be added as separate command rows:

- `Execution Mode` dropdown for the task: `Sequential` or `Parallel`
- `Command` input field for the shell command
- `Mode` dropdown: `Run` (halt on error, default) or `Continue` (ignore errors and proceed)
- a `+ Add command` button to add more commands

### Runner -> CRM invocation contract

For each `crm_fetch` task, runner executes external CRM binary with args:

- `--config <crm_config_path>`
- `--report <all|tickets|calls|leads|none>`

CRM execution always requires login.

Runner resolves relative `crm_config_path` and `crm_executable_path` from executable directory.

### Runner config example

```json
{
  "gui_host": "127.0.0.1",
  "gui_port": 8787,
  "poll_interval_seconds": 30,
  "crm_config_path": "config.json",
  "crm_executable_path": "crm",
  "allow_shell_tasks": false,
  "shell_timeout_seconds": 900,
  "post_run_timeout_seconds": 900,
  "min_task_interval_seconds": 5,
  "tasks": [
    {
      "id": "daily_all_reports",
      "name": "Daily CRM Fetch (All Reports)",
      "enabled": true,
      "repetition": "repeat",
      "frequency_seconds": 86400,
      "next_run_at": "",
      "schedules": [
        {
          "type": "daily_times",
          "enabled": true,
          "times": ["09:00", "13:00"],
          "next_run_at": "2026-04-15T09:00:00Z"
        }
      ],
      "kind": {
        "type": "crm_fetch",
        "report": "all"
      },
      "last_run_at": "",
      "last_status": ""
    }
  ]
}
```

Shell commands example:

```json
{
  "type": "shell_command",
    "mode": "parallel",
    "commands": [
      { "command": "./fetch-a.sh", "continue_on_error": false },
      { "command": "echo done", "continue_on_error": true }
  ]
}
```

## 2) CRM Config (`config.json`)

Implementation: `src/crm/config.rs`

### Authentication fields

- `region`
- `user_pool_id`
- `client_id`
- `username`
- `password`
- `no_verify_ssl`
- `remember_secrets`

### Report request fields

- `email`
- `from_date`
- `calls_from_date`
- `to_date`
- `download_csv`
- `account_id`
- `application_id`
- `app_timezone_plus_minutes`
- `base_url`

### Token cache fields

- `access_token`
- `access_token_expiry`
- `id_token`
- `refresh_token`
- `token_timestamp`

### CRM runtime behavior

- Empty `to_date` is finalized to local current date.
- Empty `calls_from_date` falls back to `from_date`.
- If `remember_secrets=false`, secret/token fields are removed before saving.
- Authentication credentials (`username` and `password`) must match the configured Cognito user pool.
- No config field is required for signed-URL failure splitting; the fetcher automatically halves a report date range when the CRM API returns `Failed to generate signed url`.

## 3) Yasweb Config (`yasweb_config.json`)

The `yasweb` binary stores its target URL and automation credentials in `yasweb_config.json`, expected to be located next to the executable. If it does not exist, the binary will auto-create a default file:

```json
{
  "url": "https://yasweb.fakeeh.care:8030/",
  "username": "",
  "password": null,
  "headless": false,
  "report_type": "",
  "report_name": ""
}
```

This configuration file is used by the headless browser automation tool to navigate to the target application and fill in the login form fields (`input[name='username']`, `input[type='password']`).

The `report_type` and `report_name` can also be supplied dynamically via the CLI, which will automatically save them to the configuration file for future runs. The CLI supports both space-separated and equals-separated values.
Example: `yasweb --type="Report Manager" --name="My Daily Report" --headless`

Short flags are also supported:
`yasweb -t="Report Manager" -n="Standard"`

Note: If a `report_type` is provided, a `report_name` MUST also be provided. The script automatically searches inside the report iframe to find and click the radio button or button corresponding to `report_type`.

### `yasweb_chrome_data/` Directory
The `yasweb` executable creates and manages this directory alongside the executable to persist Chrome profile data and caching. This directory speeds up repeated headless automation tasks.
