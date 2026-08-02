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
  - Gracefully stop the target running applications (e.g., `crm_updater.exe`, `runner.exe`).
  - Wait for process handles to be released.
  - Overwrite the existing executables with the newly extracted ones as defined in `updater_config.json`.
  - Restart the applications (with optional arguments).
- Gracefully exits itself to allow the PowerShell script to perform the self-update.

### 2. Log Rotation & Sending
The application scans the configured `runner_logs_dir` for `.log` files.
- It compresses all `.log` files into ZIP archives using standard Deflate compression.
- It strictly enforces a 20MB maximum file size for each ZIP chunk. If the compressed archive exceeds 20MB, it rotates to a new `.zip` file (e.g., `logs_part1.zip`, `logs_part2.zip`).
- It connects to the active Outlook COM session and sends an email to the configured `log_recipient_email` with all generated ZIP chunks attached.
- After successful processing, it safely deletes the original uncompressed log files and the temporary ZIP archives.

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
      "executable_name": "crm_updater.exe"
    },
    {
      "source_file": "runner.exe",
      "target_path": ".",
      "executable_name": "runner.exe"
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
