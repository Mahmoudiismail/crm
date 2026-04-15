# Configuration

The application now uses two config files:

- `runner_config.json` (runner behavior, task storage, GUI host/port)
- `config.json` (CRM auth/report settings and token cache)

Runner behavior is config-driven. CRM executable behavior is controlled via CLI args.

## 1) Runner Config (`runner_config.json`)

Implementation: `src/runner/config.rs`

### Top-level fields

- `gui_host`: GUI bind host (default `127.0.0.1`)
- `gui_port`: GUI bind port (default `8787`)
- `poll_interval_seconds`: scheduler tick interval (default `30`)
- `crm_config_path`: path to CRM config file (default `config.json`)
- `crm_executable_path`: crm executable file/path used by runner (default `crm.exe` on Windows, `crm` on non-Windows)
- `allow_shell_tasks`: allow `shell_command` tasks (default `false`)
- `shell_timeout_seconds`: max runtime per shell task (default `300`)
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
  - `shell_command` with legacy `command` or grouped `groups`
- `last_run_at`: last run timestamp
- `last_status`: last run result message

### Schedule fields

Each `schedules` item has a `type`:

- `once`: runs once at `next_run_at`; empty `next_run_at` means immediate.
- `interval`: runs every `every_seconds`; `next_run_at` stores the next due time.
- `daily_times`: runs at one or more local machine times in `times` (`HH:MM`); `next_run_at` stores the next calculated due time.

All persisted `next_run_at` and `last_run_at` values remain RFC3339. The GUI renders these values as local human-readable time with relative text.

### Shell command groups

`shell_command` supports the legacy single `command` field and the newer `groups` list:

- `name`: display name for the command group.
- `mode`: `sequential` or `parallel`.
- `commands`: list of command specs.
- `commands[].command`: shell text executed with `bash -lc`.
- `commands[].continue_on_error`: when `true`, a failed command does not fail the group.

Groups run in order. A sequential group stops at the first failed command unless that command has `continue_on_error=true`. A parallel group starts all commands together, waits for all commands, and fails when any non-continued command fails.

### Task validation and normalization

When tasks are created/updated through the runner GUI CRUD endpoints:

- `id` is required and must contain only letters, numbers, `-`, `_`
- `name` is required
- `next_run_at` must be empty or valid RFC3339
- for `repetition=repeat`, `frequency_seconds` is clamped to at least `min_task_interval_seconds`
- `schedules` entries must use valid intervals, RFC3339 once timestamps, or `HH:MM` daily local times
- `shell_command.command` or `shell_command.groups` must contain at least one non-empty command

`id` uniqueness is enforced across all tasks. Updates preserve `last_run_at` and `last_status` when these fields are not explicitly provided.

The GUI create/update forms accept multi-line schedule text:

```text
interval: every 1h
daily: 09:00, 13:00, 18:30
once: 2026-04-15T09:30:00-05:00
```

The GUI shell command editor accepts multi-line group text:

```text
@group Setup sequential
run: echo prepare
continue: cleanup-if-present

@group Reports parallel
run: ./fetch-a.sh
run: ./fetch-b.sh
```

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
  "shell_timeout_seconds": 300,
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

Shell command group example:

```json
{
  "type": "shell_command",
  "groups": [
    {
      "name": "Reports",
      "mode": "parallel",
      "commands": [
        { "command": "./fetch-a.sh", "continue_on_error": false },
        { "command": "./fetch-b.sh", "continue_on_error": true }
      ]
    }
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
