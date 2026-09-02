# Configuration Guide

All applications manage their own specific configuration files but work in tandem with the central `runner`. By default, configurations are placed in the same directory as the executable.

## `runner_config.json`

This file controls the GUI, global timing, and task scheduling. It has a `registered_apps` array that manages dynamically registered `AppManifest` applications.

```json
{
  "gui_host": "127.0.0.1",
  "gui_port": 8787,
  "poll_interval_seconds": 30,
  "allow_shell_tasks": false,
  "log_retention_days": 30,
  "registered_apps": [
    {
      "id": "my_crm",
      "name": "Local CRM Tool",
      "executable_path": "crm.exe",
      "config_path": "config.json"
    }
  ],
  "tasks": [
    {
      "id": "fetch_all",
      "name": "Daily Fetch",
      "enabled": true,
      "kind": {
        "type": "external_app",
        "app_id": "my_crm",
        "args": {
            "--report": "all"
        }
      }
    }
  ]
}
```

## Application Configurations

Executables spawned via the runner or manually have their own configurations.

- **`config.json` (CRM):** Cognito user pool, API endpoints, credentials. Can override the standard `Downloads` folder using `custom_download_folder`. The CLI argument `--custom-download-folder` overrides this config key for a single run without persisting to `config.json`. Includes CRM-specific download configuration properties:
  - `recent_download_window_seconds`: (Default: 30) Prevents redundant CRM downloads by skipping requests if a recently downloaded file for the same report already exists in the folder and its age is within this window in seconds. Set to `0` to always fetch. This is based on the presence/age of downloaded report files, and it is NOT a general CRM execution cooldown.
  - `retention_days`: (Optional) If configured with a value greater than `0`, the CRM process will clean up older downloaded CRM reports (`ticket_report_*`, `call_logs_*`, `lead_report_*`, `user_report_*`) modified longer than `retention_days` days ago. Missing, `null` or `0` disables cleanup. Unrelated files in the directory are safely preserved.

  ```json
  {
    "username": "my_user",
    "recent_download_window_seconds": 60,
    "retention_days": 14
  }
  ```
- **`yasweb_config.json`:** Browser automation configurations, cached filter mappings, and report timeout limits. The `timeout_minutes` field defines how long to wait for UI loaders and file downloads. `start_date_key` and `end_date_key` are dictionaries containing `key` and `format` fields to specify both the web UI filter name and its exact expected date/time format (e.g. `{"key": "FromDate", "format": "%d-%m-%Y 00:00"}`).
- **`wcxx_config.json`:** Webex CC token and organization endpoints.
- **`tasker_config.json`:** Tasker tasks like CSV pivoting, team mappings, Outlook configuration, and leads reporting for the Call Center. Includes `send_exceptions` to dynamically read from `category_exceptions` and skip standard team branch logic for exception tickets.

### Merging and Persistence Behavior
- Across all configurations, missing properties in files are systematically injected with application defaults using a recursive object merge via `utils::merge_json`.
- Arrays are treated as *atomic* during merges. That is, if a configuration contains an array, it strictly preserves the user's array content rather than performing element-by-element deep merges. Defaults are only applied if the array field is completely absent.
- File saves leverage `atomic_write` utilizing `tempfile` persist boundaries, preventing any JSON corruption from unexpected interrupts or I/O hangs. Optional unconfigured strings and arrays are suppressed natively from the JSON output.

*(See respective markdown files for detailed schemas of these components).*

### `CrmOpenSohailConfig`

Specific to the `crm_open_sohail` task. Inherits all fields from `DashboardUpdaterConfig` at the root object level, and introduces specific settings for generating enriched emails based on Excel slicers.

| Field | Type | Description |
|---|---|---|
| `team_mapping_file` | String | Path to the CSV mapping Team Names to Receiver Names (Owners) and emails. |
| `body_template_file` | Option<String> | Custom HTML template for the email body. |
| `subject_template` | Option<String> | The subject of the dispatched email. |
| `branch_filter` | Option<Vec<String>> | Filter applied to Slicers to restrict the parsed branches. |
| `month_filter` | Option<Vec<String>> | Filter applied to Slicers to restrict parsed months. |
| `fallback_oul` | Option<String> | Value mapped in the OUL column when a team is missing from the mapping file. |
