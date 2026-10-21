# CRM Updater

`crm_updater` is a standalone binary in the CRM tool workspace designed to automate application updates and move reports, logs, and diagnostics into local collections.

## Features

### 1. Update Pipeline
The application checks the local Outlook client's **Drafts** folder for any email containing an attachment named `crm_tool_*.zip`.
If found, it:
- Downloads the ZIP file and deletes the Outlook draft.
- Extracts the ZIP archive using AES decryption (default password: `123456`).
- Unblocks the extracted files (`Unblock-File`).
- Dynamically generates and executes a detached PowerShell script (`.ps1`) to:
  - Securely injects the `file_replacement_map` configuration payload into the PowerShell context via a temporary JSON file passed to the script using the `-ReplacementMapPath` argument. This avoids unbounded Windows CLI limits and command injection vulnerabilities when deploying updates to large numbers of binaries.
  - Resolves source and target paths relative to the current executable's directory.
  - Check if target processes are currently running.
  - Gracefully stop any running target applications (e.g., `crm_updater.exe`, `runner.exe`) and robustly wait for actual process termination.
  - Overwrite the existing executables with the newly extracted ones as defined in `updater_config.json` and verify replacement success.
  - Restart the applications (with optional arguments), guided by the `autostart` configuration.
  - Generate a detailed, persistent log at `updater_detached.log` providing diagnostics on paths, running status, replacement, and success/failure outcome.
- Gracefully exits itself to allow the PowerShell script to perform the self-update, ensuring update logging continues robustly.

### 2. Local Reports, Logs, and Diagnostics Collection
The `--logs-only` option moves files locally into a timestamped subfolder under `collected_logs`. It does not send email or create ZIP archives.

- Moves application-level `.log` files beside the updater and per-task `.log` files under `runner_logs_dir`.
- Moves Yasweb HTML/JSON diagnostics from `debug_html`, excluding the `sent` subfolder.
- Searches application `Downloads` folders and moves report files with `.csv`, `.xls`, `.xlsx`, `.xlsm`, and `.pdf` extensions.
- Preserves relative folder paths within each collection and creates a new timestamped folder on every run so earlier collections remain intact.
- Moves rather than copies files. If a move crosses volumes, it falls back to copy followed by deleting the original. If any file cannot be moved, the pipeline reports failure rather than claiming complete success.

### 3. Update Pipeline
When an update ZIP is found in Outlook Drafts, the updater extracts it into `downloads` and validates that every file in `file_replacement_map` is present before launching the detached replacement script.

If no Outlook draft is found, the updater can apply a complete set of already-staged application files in `downloads`. It refuses to start a partial update when any configured source file is missing. The detached script stops target applications, replaces each executable, verifies SHA-256 hashes, restarts configured applications, and logs the result in `updater_detached.log`.

## Configuration

When first run, `crm_updater` generates a default `updater_config.json` configuration file:

```json
{
  "downloads_dir": "downloads",
  "runner_logs_dir": "logs",
  "log_recipient_email": "admin@example.com",
  "file_replacement_map": [
    {
      "source_file": "crm_updater.exe",
      "target_path": ".",
      "executable_name": "crm_updater.exe",
      "autostart": true
    },
    {
      "source_file": "runner.exe",
      "target_path": ".",
      "executable_name": "runner.exe",
      "autostart": true
    }
  ],
  "log_stdout_level": "DEBUG",
  "log_file_level": "TRACE"
}
```

**Compatibility note:** `log_recipient_email` remains in existing configuration files for backward compatibility, but local log collection does not use it and does not send email.

## CLI Options

By default, executing `crm_updater` runs the local collection pipeline first, followed by the update pipeline.
Granular execution can be triggered using the following CLI flags:

- `--update-only`: Executes **only** the application update pipeline.
- `--logs-only`: Executes **only** local report/log collection without compression.

Example:
```bash
cargo run --bin crm_updater -- --logs-only
```
