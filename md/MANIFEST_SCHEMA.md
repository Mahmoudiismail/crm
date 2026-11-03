# AppManifest Standard

This document details the JSON schema for the `AppManifest`, used by the runner application to dynamically configure and execute external applications.

## Schema Overview

The manifest defines the application's metadata and the arguments it accepts via the command line interface (CLI). Process plugins are separate OS processes; Runner invokes the executable with `--manifest` and validates `plugin_api_version` when the corresponding `registered_apps[]` entry opts into version enforcement. By exposing an `AppManifest`, any external application can natively integrate with the runner's web dashboard.

### `AppManifest` Object

| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | String | The human-readable name of the application. |
| `plugin_api_version` | Integer (Optional) | Process-plugin contract version. First-party CRM, Tasker, Yasweb, and CRM Updater workers declare `1`. Runner itself is the host, not a worker plugin. Omission means the manifest is legacy/unversioned. Runner enforces the version only when the corresponding `registered_apps[]` entry sets `plugin_api_version`. |
| `description` | String | A brief description of what the application does. |
| `arguments` | Array of `AppArg` | The list of command-line arguments the application accepts. |

### `AppArg` Object

| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | String | The name of the argument as passed to the CLI (e.g., `--config`, `--dry-run`). |
| `arg_type` | String | The data type of the argument. Allowed values: `"string"`, `"number"`, `"list"`, `"multi_list"`, `"boolean"`, `"date_var"`. |
| `required` | Boolean | Whether the argument must be provided by the user. |
| `default_value` | String (Optional) | The default value used if the user provides no input. |
| `options` | Array of String (Optional) | Required if `arg_type` is `"list"` or `"multi_list"`. Specifies the valid choices for the dropdown. |
| `depends_on` | Object (Optional) | A map where keys are parent argument names and values are arrays of strings (acceptable values). The argument will only be visible in the GUI if the parent argument currently matches one of the specified values. |
| `autofill` | Object (Optional) | A map where keys are parent argument names and values are nested maps linking parent argument values to the desired autofill value. Useful for automatically populating inputs when a related dropdown changes. |

### Validation

Manifests enforce lightweight validation to guarantee structural integrity:
- `name` cannot be empty.
- Every `AppArg` must have a non-empty `name`.
- `AppArg` names must be unique within an `AppManifest`.

## Example JSON Representation

```json
{
  "name": "Yasweb Reporting Tool",
  "description": "Fetches dynamic reports via headless browser automation.",
  "arguments": [
    {
      "name": "--report-type",
      "arg_type": "list",
      "required": true,
      "options": ["daily", "weekly"]
    },
    {
      "name": "--timeout",
      "arg_type": "number",
      "required": false,
      "default_value": "300"
    },
    {
      "name": "--filter-branch",
      "arg_type": "string",
      "required": false,
      "depends_on": {
        "--report-type": ["daily"]
      },
      "autofill": {
        "--report-type": {
          "daily": "main_branch"
        }
      }
    },
    {
      "name": "--headless",
      "arg_type": "boolean",
      "required": false,
      "default_value": "true"
    }
  ]
}
```

## Argument Handling

*   **Boolean Arguments**: Boolean arguments (e.g., `--headless`) operate as flags. If the user checks the box (evaluates to true), the flag is passed to the executable (`--headless`). If false, the argument is completely omitted.
*   **Other Arguments**: For string, number, or list types, the argument name and the user-provided value are passed as two distinct elements to the process execution (e.g., `["--report-type", "daily"]`).

## Implementation Requirement

Applications wishing to be orchestrated via this standard must implement a `--manifest` flag. When invoked with this flag, the application must:
1. Print the serialized JSON representation of its `AppManifest` to standard output (stdout).
2. Exit with a status code of `0`.

## Process plugin compatibility

Runner currently supports process-plugin API version `1`. Set `registered_apps[].plugin_api_version` to `1` to opt an executable into strict compatibility checks; Runner then requires `--manifest` to return valid JSON with the same version. Unsupported configured versions, missing/malformed manifest versions, and mismatches prevent launch. When the registry field is absent, Runner does not invoke `--manifest` for compatibility checks or automatic date-argument discovery. Explicit `start_date_arg` and `end_date_arg` mappings in the registered action remain available to legacy executables. This preserves compatibility and startup behavior for arbitrary external executables. This is a process contract version, not the Cargo package version.


## Registration validation

When a registered application opts into API version enforcement, Runner requires the `--manifest` command to exit successfully, emit valid JSON matching the `AppManifest` structure (`name`, `description`, and `arguments`), pass manifest validation (non-empty name and argument names, unique argument names), and declare the exact configured API version. Invalid manifests are rejected before the application is launched for its task. Applications without `registered_apps[].plugin_api_version` remain on the legacy path and are not probed.
