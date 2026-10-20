# CRM Updater

`crm_updater` is a standalone binary in the CRM tool workspace designed to automate application updates and collect logs into local ZIP archives.

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

### 2. Local Log Collection
The application collects application-level `.log` files beside the executable, task logs recursively from the configured `runner_logs_dir`, and Yasweb HTML/JSON diagnostics from the sibling `debug_html` directory.
- It compresses the collected files into ZIP archives using standard Deflate compression, preserving directory paths so files with the same name do not overwrite each other.
- It splits archives based on a 20MB uncompressed-input size estimate per chunk.
- It writes persistent ZIP archives to `collected_logs/` beside the updater executable, using timestamped names such as `logs_20261020_120000_part1.zip`.
- Original `.log`, HTML, and JSON files are left unchanged. No email is sent, and the updater does not delete or move collected source files.

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

By default, executing `crm_updater` runs the log collection pipeline first, followed by the update pipeline.
Granular execution can be triggered using the following CLI flags:

- `--update-only`: Executes **only** the application update pipeline.
- `--logs-only`: Executes **only** local log collection and ZIP creation.

Example:
```bash
cargo run --bin crm_updater -- --logs-only
```
