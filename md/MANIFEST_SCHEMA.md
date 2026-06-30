# AppManifest Standard

This document details the JSON schema for the `AppManifest`, used by the runner application to dynamically configure and execute external applications.

## Schema Overview

The manifest defines the application's metadata and the arguments it accepts via the command line interface (CLI). By exposing an `AppManifest`, any external application can natively integrate with the runner's web dashboard.

### `AppManifest` Object

| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | String | The human-readable name of the application. |
| `description` | String | A brief description of what the application does. |
| `arguments` | Array of `AppArg` | The list of command-line arguments the application accepts. |

### `AppArg` Object

| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | String | The name of the argument as passed to the CLI (e.g., `--config`, `--dry-run`). |
| `arg_type` | String | The data type of the argument. Allowed values: `"string"`, `"number"`, `"list"`, `"boolean"`. |
| `required` | Boolean | Whether the argument must be provided by the user. |
| `default_value` | String (Optional) | The default value used if the user provides no input. |
| `options` | Array of String (Optional) | Required if `arg_type` is `"list"`. Specifies the valid choices for the dropdown. |

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
      "options": ["daily", "weekly", "monthly"]
    },
    {
      "name": "--timeout",
      "arg_type": "number",
      "required": false,
      "default_value": "300"
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
