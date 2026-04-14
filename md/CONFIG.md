# Configuration

The application now uses two config files:

- `runner_config.json` (runner behavior, task storage, GUI host/port)
- `config.json` (CRM auth/report settings and token cache)

No CLI runtime overrides are required.

## 1) Runner Config (`runner_config.json`)

Implementation: `src/runner/config.rs`

### Top-level fields

- `gui_host`: GUI bind host (default `127.0.0.1`)
- `gui_port`: GUI bind port (default `8787`)
- `poll_interval_seconds`: scheduler tick interval (default `30`)
- `crm_config_path`: path to CRM config file (default `config.json`)
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
- `skip_login`: pass-through for CRM task authentication behavior
- `output`: optional JSON output path for CRM fetch task
- `kind`: tagged task payload
  - `crm_fetch` with `report` (`all`, `tickets`, `calls`, `leads`, `none`)
  - `shell_command` with `command`
- `last_run_at`: last run timestamp
- `last_status`: last run result message

### Runner config example

```json
{
  "gui_host": "127.0.0.1",
  "gui_port": 8787,
  "poll_interval_seconds": 30,
  "crm_config_path": "config.json",
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
      "skip_login": false,
      "output": null,
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
