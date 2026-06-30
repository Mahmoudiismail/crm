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

- **`config.json` (CRM):** Cognito user pool, API endpoints, credentials.
- **`yasweb_config.json`:** Browser automation configurations, cached filter mappings.
- **`wcxx_config.json`:** Webex CC token and organization endpoints.
- **`tasker_config.json`:** Tasker tasks like CSV pivoting, team mappings, and Outlook configuration.

*(See respective markdown files for detailed schemas of these components).*
