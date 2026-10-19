# CRM Updater

`crm_updater` is a standalone binary in the CRM tool workspace designed to automate application updates and log rotation via Microsoft Outlook COM automation.

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

### 2. Log Rotation & Sending
The application collects application-level `.log` files beside the executable, task logs recursively from the configured `runner_logs_dir`, and Yasweb HTML/JSON diagnostics from the sibling `debug_html` directory.
- It compresses the collected files into ZIP archives using standard Deflate compression, preserving directory paths so files with the same name do not overwrite each other.
- It splits archives based on a 20MB uncompressed-input size estimate per chunk.
- It connects to the active Outlook COM session and sends an email to the configured `log_recipient_email` with all generated ZIP chunks attached.
- Only after successful email delivery, it deletes collected original `.log` files and temporary ZIP archives. HTML/JSON diagnostics remain on disk for future investigation.

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

**Note:** For security, the `log_recipient_email` field is fully redacted with `***REDACTED***` in the application's runtime debug logs.

## CLI Options

By default, executing `crm_updater` runs the update pipeline first, followed immediately by the log rotation pipeline.
Granular execution can be triggered using the following CLI flags:

- `--update-only`: Executes **only** the application update pipeline.
- `--logs-only`: Executes **only** the log rotation and sending pipeline.

Example:
```bash
cargo run --bin crm_updater -- --logs-only
```
