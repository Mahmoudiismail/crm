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

- **`config.json` (CRM):** Cognito user pool, API endpoints, credentials. Can override the standard `Downloads` folder using `custom_download_folder`. The CLI argument `--custom-download-folder` overrides this config key for a single run without persisting to `config.json`.
- **`yasweb_config.json`:** Browser automation configurations, cached filter mappings. `start_date_key` and `end_date_key` are dictionaries containing `key` and `format` fields to specify both the web UI filter name and its exact expected date/time format (e.g. `{"key": "FromDate", "format": "%d-%m-%Y 00:00"}`).
- **`wcxx_config.json`:** Webex CC token and organization endpoints.
- **`tasker_config.json`:** Tasker tasks like CSV pivoting, team mappings, Outlook configuration, and leads reporting for the Call Center. Includes `send_exceptions` to dynamically read from `category_exceptions` and skip standard team branch logic for exception tickets.

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
