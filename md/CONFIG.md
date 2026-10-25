# Configuration Guide

All applications manage their own specific configuration files but work in tandem with the central `runner`. By default, configurations are placed in the same directory as the executable.

## `runner_config.json`

This file controls the GUI, global timing, and task scheduling. It has a `registered_apps` array that manages dynamically registered `AppManifest` applications.

```json
{
  "gui_host": "127.0.0.1", // strictly validated. Must be loopback (127.0.0.1 or localhost)
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

## CRM Fetch Range Behavior

Ticket exports are automatically split into requests covering at most three calendar months each, with up to three initial ranges processed concurrently. This behavior does not require a configuration option. If ticket date values cannot be parsed for batching, CRM attempts the original range directly; non-ticket endpoints keep their existing range behavior.

## Application Configurations

Executables spawned via the runner or manually have their own configurations.

- **`config.json` (CRM):** Cognito user pool, API endpoints, credentials. Can override the standard `Downloads` folder using `custom_download_folder`. The CLI argument `--custom-download-folder` overrides this config key for a single run without persisting to `config.json`. Includes CRM-specific download configuration properties:
  - `recent_download_window_seconds`: (Default: 30) Recent completed-report suppression window in seconds. Set to `0` to always fetch. For date-ranged tickets, calls, and leads, suppression requires a completion marker for the same requested range and all files recorded by that marker to still exist; a lone recent chunk file is not proof of completion. Users are fetched as one direct Base64 payload and retain the recent-file check. This is not a general CRM execution cooldown.
  - `remember_secrets`: (Default: `false`) Controls where CRM secrets are persisted. With `false` (default), passwords and tokens are omitted from the JSON config; token cache is protected separately with Windows DPAPI and passwords are never cached. With `true`, the password and tokens are stored in JSON for compatibility.
  - `retention_days`: (Optional) If configured with a value greater than `0`, the CRM process will clean up older downloaded CRM reports (`ticket_report_*`, `call_logs_*`, `lead_report_*`, `users_*`) modified longer than `retention_days` days ago. Missing, `null` or `0` disables cleanup. Unrelated files in the directory are safely preserved.

  ```json
  {
    "username": "my_user",
    "recent_download_window_seconds": 60,
    "retention_days": 14
  }
  ```
- **`yasweb_config.json`:** Browser automation configurations, cached filter mappings, and report timeout limits. The `timeout_minutes` field independently controls the maximum wait time for report generation (Step 6 loader) and the subsequent XLSX file download. It does not act as a global UI timeout for navigation or filter readiness. `start_date_key` and `end_date_key` are dictionaries containing `key` and `format` fields to specify both the web UI filter name and its exact expected date/time format (e.g. `{"key": "FromDate", "format": "%d-%m-%Y 00:00"}`).
- **`tasker_config.json`:** Tasker tasks like CSV pivoting, team mappings, Outlook configuration, and leads reporting for the Call Center. Includes `send_exceptions` to dynamically read from `category_exceptions` and skip standard team branch logic for exception tickets.

### Merging and Persistence Behavior
- Across all configurations, missing properties in files are systematically injected with application defaults using a recursive object merge via `utils::merge_json`.
- Arrays are normally treated as *atomic*: an existing array is preserved as-is, and defaults are applied only when the field is absent. The `tasks` field is a deliberate exception for configuration auto-healing: task objects are recursively merged, and Tasker tasks that have a `type` are matched by that type rather than list position so reordering tasks does not mix settings between task types. New opt-in task defaults with `enabled: false` are appended only when that task type is not already configured. An empty `tasks` array is populated from defaults.
- File saves leverage `atomic_write`, which writes and syncs a temporary file beside the destination before replacing it; a bare relative filename uses the current working directory. This prevents partially written JSON from an interrupted write, and malformed existing JSON is reported rather than replaced with defaults. Optional unconfigured strings and arrays are suppressed natively from the JSON output.
- Runner mutations are serialized with GUI/API reads through a shared in-process lock. After each mutation, Runner reloads `runner_config.json` and verifies that the saved JSON matches the intended in-memory configuration; task edits also verify the editable task fields after normalization. Read-back failures are surfaced to the caller instead of returning a successful save response.

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


## Security Model

- **Loopback Enforcement**: Runner GUI binds to `127.0.0.1` (localhost loopback). Binding to non-loopback `gui_host` addresses (such as `0.0.0.0` or local network interfaces) without protection is rejected on startup to prevent unauthenticated remote control.
- **HTTP GET Safety**: State-changing operations (`/create`, `/update/...`, `/delete/...`, `/run/...`, `/enable/...`, `/disable/...`, `/run-all`, `/working-hours/create`, `/apps/create`, etc.) strictly require `POST` requests. `GET` requests to state-changing endpoints are rejected with `HTTP 405 Method Not Allowed`.
