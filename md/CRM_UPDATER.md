# CRM Updater

`crm_updater` is a standalone binary in the CRM Tool Cargo package designed to automate application updates and move reports, logs, and diagnostics into local collections.

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
The `--logs-only` option creates a timestamped diagnostic ZIP under `collected_logs`. It does not send email or generate business reports.

- Includes application-level `.log` files beside the updater and per-task `.log` files under `runner_logs_dir`.
- Includes Yasweb HTML/JSON browser diagnostics from `debug_html`, excluding the `sent` subfolder and files whose names identify them as configuration.
- Preserves relative paths inside the ZIP (for example, root-level task logs and `debug_html/<snapshot>.html`). It does not collect report files from Downloads or generate OPD reports.
- Does not include standalone application configuration files; configuration already represented in Runner runtime logs is not duplicated, and credentials are not packaged as diagnostic data.
- Writes a temporary ZIP, reopens it, checks the entry count and compares every archived file byte-for-byte with its source. Only after verification does it finalize the ZIP and remove the original files.
- If ZIP creation or verification fails, source files remain untouched. If cleanup fails after verification, the valid ZIP is retained and the updater reports which originals could not be removed.

### 3. Update Pipeline
When an update ZIP is found in Outlook Drafts, the updater extracts it into `downloads` and validates that every file in `file_replacement_map` is present before launching the detached replacement script.

If no Outlook draft is found, the updater can apply a complete set of already-staged application files in `downloads`. It refuses to start a partial update when any configured source file is missing. The detached script preflights all sources and destinations before stopping target applications. It stages each replacement beside its destination, verifies SHA-256 hashes, and keeps a backup of each existing destination until the complete replacement set has succeeded. If a replacement fails, it attempts to roll back already-replaced files in reverse order and logs any rollback failures. After all replacements are verified, it removes backups, restarts configured applications, and logs the result in `updater_detached.log`.

## Update Package Trust (Production Gate)

The updater currently decrypts incoming ZIP attachments with the legacy fixed password `123456` and validates file replacement consistency with SHA-256. These checks do **not** authenticate who created the incoming package. A hash calculated from the same untrusted package or email is not an authenticity check.

Do not use this updater for general production rollout until a trusted package-verification mechanism is implemented and enforced **before stopping applications or replacing binaries**. Preferred options are a signed manifest verified against a trusted public key embedded in the updater, or Authenticode verification with an enforced publisher identity. The signing key must not be embedded in the client. The existing ZIP password remains unchanged for compatibility and must not be treated as a security boundary.

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

The updater uses `updater_config.json` beside the executable by default. Pass `--config <PATH>` to select another configuration; relative paths are resolved from the executable directory and absolute paths are used unchanged. The selected file controls both updater settings and logging levels.

By default, executing `crm_updater` runs the local collection pipeline first, followed by the update pipeline.
Granular execution can be triggered using the following CLI flags:

- `--update-only`: Executes **only** the application update pipeline.
- `--logs-only`: Executes **only** diagnostic ZIP collection and verified source cleanup.
- `--config <PATH>`: Selects the updater configuration file.

Example:
```bash
cargo run --bin crm_updater -- --logs-only
```

## PowerShell Script Management

The updater PowerShell source is maintained in these standalone files and embedded into the binary at compile time:

- `src/crm_updater/scripts/scan_update_drafts.ps1` — scans Outlook Drafts and writes a small result marker to a caller-provided result file so Rust can read the result through the managed script runner.
- `src/crm_updater/scripts/apply_update.ps1` — applies the replacement map, waits for the parent updater to exit, replaces target files, restarts configured applications, and writes the detached update log.
- `src/crm_updater/scripts/unblock_file.ps1` — unblocks extracted update files.

At startup, `crm_updater` calls `ScriptManager::install_builtin_scripts()`, which materializes every built-in PowerShell template—including Tasker and Updater scripts—under `<exe_dir>/scripts/<Task Name>/` before the update or collection pipeline starts. The source templates remain embedded in the executable so missing files can be restored. The apply script is launched detached via `ScriptManager::execute_script_detached_with_args` so it can continue after `crm_updater` exits and replace the updater executable itself. Replacement data is passed through a temporary JSON file using `-ReplacementMapPath`, rather than interpolated into the PowerShell source. The detached script is managed persistently; it does not delete its own script file after running.

## Outlook Recipient Resolution

Tasker no longer runs a custom `Recipients.ResolveAll()` preflight gate before sending Outlook messages. Removing that gate avoids the application rejecting a message solely because its custom recipient-resolution check failed. Outlook can still reject a message at send time if recipient information is invalid or unavailable.
