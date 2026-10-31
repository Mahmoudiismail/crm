# Tasker Application

`tasker` is a lightweight, stateless task runner binary part of the CRM Tool suite. It is designed to be executed by the `runner` and can process various configured tasks in a single pass before exiting. Because it operates cleanly and exits, it can safely be invoked multiple times in parallel by the runner scheduler without deadlocks, provided tasks are configured with different targets.

## Failure propagation

Configured tasks are run in order. If a task fails, Tasker logs the error and returns a non-zero failure result immediately instead of continuing to later tasks and exiting successfully. This allows Runner to record a failed execution rather than showing a false success. Outlook email failures are propagated directly; Tasker does not attempt to send an error notification through the same Outlook COM transport that just failed.

## Note for AI Agents (AGENTS.md Addendum)

> **Important**: If any modifications are made to the `csv_analysis` task within `tasker`, you MUST run the validation test using the provided raw dataset (from the user's pastebin) and compare it to the expected output before submitting the PR to ensure regressions are avoided.

## Configuration

`tasker` looks for `tasker_config.json` beside the executable by default. `--config <PATH>` selects another file; relative paths for this option are resolved from the executable directory. The legacy positional path is still accepted and retains current-working-directory path semantics. Startup logging-level detection and task execution use the same selected path. If the targeted configuration file does not exist, `tasker` automatically generates a default configuration file with a template `csv_analysis` task and logs a message before attempting to run. On each run, it also merges missing default configuration fields without overwriting user values.

### CLI Arguments
- `--config <PATH>`: Overrides the default `tasker_config.json` path. (Legacy support also allows passing just the path without the flag, as long as it does not start with `-`).
- `--task <INDEX>`: Executes only a specific task from the configuration (1-based index). For example, `--task 1` runs only the first task in the config array.
- `--only-call-center`: When provided, the task skips standard per-team/per-branch email generation and runs the Call Center email path for the selected task.
- `--send-exceptions`: Runs the configured exception-ticket reporting path. Exception mode bypasses standard team/branch routing and the specialized Call Center lead attachment path.
- `--start-date <DATE>`: Overrides the configured task start date; this option requires `--task`.

**Example: Scheduling in runner_config.json**
To schedule these separately in the `runner`, you can configure two different tasks in `runner_config.json`:
```json
{
  "tasks": [
    {
      "name": "Standard Reports",
      "command": "tasker.exe",
      "args": ["--task", "1"]
    },
    {
      "name": "Call Center Report Only",
      "command": "tasker.exe",
      "args": ["--task", "1", "--only-call-center"]
    }
  ]
}
```

The configuration file is a JSON object with a `tasks` array.

### `csv_analysis` Task

This task is designed to process multiple ticket report CSV files and augment them with assignment data and team configurations.

#### Example Configuration
```json
{
  "tasks": [
    {
      "type": "csv_analysis",
      "download_path": "./downloads",
      "users_file": "./data/users.csv",
      "assignment_settings_file": "./data/assignments.csv",
      "minutes_ago": 15,
      "start_date": "01-May-2026",
      "exclude_branches": [
        "Dr. Soliman Fakeeh Hospital Madinah",
        "Medical Fakeeh"
      ],
      "exclude_categories": [
        "incomplete reservation"
      ],
      "category_exceptions": [
        {
          "category": "incomplete reservation",
          "branch": "DSFH Jeddah",
          "team": "Specific Team Name"
        }
      ],
      "output_file": "./results.csv",
      "email_config": {
        "team_mapping_file": "./teams.csv",
        "body_template_file": "./task1/email_template.html",
        "initial_cc": "initial@example.com",
        "ending_cc": "ending@example.com",
        "send_emails": false,
        "default_to_email": "fallback@example.com",
        "send_per_team_all_branches": ["PRE-AUTHORIZATION"],
        "send_per_branch_branches": ["dsfmc", "DSFMH"],
        "send_per_team_branches": ["Dr. Soliman Fakeeh Hospital Jeddah"],
        "send_call_center": true,
        "indentation_spaces": 4
      }
    }
  ]
}
```

#### Fields Description
- `type`: Must be `"csv_analysis"`.
- `download_path`: Directory to search for recent CRM exports. Ticket processing reads `ticket_report*.csv`; lead enrichment reads `lead_report*.csv`.
- `users_file`: Path to the users mapping CSV (formerly PowerQuery Table11).
- `assignment_settings_file`: Path to the assignment settings CSV containing category, type, and subtype mappings.
- `minutes_ago`: Limits both ticket and lead source CSVs to files modified within the last X minutes.
- `start_date`: (Optional) String representing a date (e.g. "01-May-2026") to filter out tickets created before this date.
- `exclude_branches`: Array of strings. Tickets belonging to these branches will be excluded from the final output (case-insensitive).
- `exclude_categories`: Array of strings. Tickets belonging to these categories will be excluded from the final output (case-insensitive).
- `category_exceptions`: (Optional) List of objects specifying conditional inclusions for otherwise excluded categories.
  - `category`: The category string that normally would be excluded.
  - `branch`: (Optional) Only allow the exception if the ticket branch matches this.
  - `team`: (Optional) If specified and the exception matches, this overrides and assigns the ticket to this team.
- `output_file`: Destination path for the enriched ticket result. Tasker also writes `lead_result.csv` beside this file whenever recent lead exports are available. The lead result preserves the lead columns and adds/updates `team` using the assignee's primary team from `users_file`; an empty or unknown assignee leaves `team` blank. The lead result includes all recent source lead statuses and branches for review. Source files are limited by the configured minutes_ago lookback. Rows are deduplicated by Lead Id across matching recent exports, with the newest source file considered first. The existing ticket output is not replaced by the lead output.
- `email_config`: (Optional) Specifies automated email configuration via Microsoft Outlook.
  - `team_mapping_file`: Path to CSV configuring email recipients per Team or Branch Name (Requires headers `Team Name`, `To Emails`, `CC`).
  - `body_template_file`: (Optional) Path to an HTML file to use as the email template. If it does not exist, `tasker` will auto-generate it. It extracts the subject from the `<title>` tag and the content from the `<body>` tag, replacing `{bucket_name}`, `{from_date_str}`, `{today_str}`, and `{html_table}` dynamically.
  - `initial_cc` / `ending_cc`: Static CC emails appended to every sent mail.
  - `send_emails`: Boolean, if `false` emails are left open as drafts (using `.Display()`) for manual review. If `true` uses `.Send()`.
  - `default_to_email`: Fallback email if team mapped isn't found, and also used to send exception/error reports.
  - `send_per_team_all_branches`: List of teams (e.g., "PRE-AUTHORIZATION") whose tickets should be aggregated into a single email spanning across all allowed branches, rather than partitioned by individual branch rules.
  - `send_per_branch_branches`: List of branches that will receive *one email for the entire branch* instead of separated by team.
  - `send_per_team_branches`: List of branches that should have their emails sent per team.
  - `send_call_center`: Boolean, if true unifies the "Call Center" tickets from all allowed branches into a single email instead of being grouped with the others. It also builds the leads attachment from `lead_result.csv`, including only rows whose `team` is `Call Center`, whose status is `New` or `Follow-up`, and whose branch is not excluded. Leads assigned to other teams, plus blank/unknown assignees, remain in the durable `lead_result.csv` for review and are not included in the Call Center attachment.
  - `indentation_spaces`: (Optional) Customizable number of non-breaking spaces before the email body content for indentation. Defaults to 4.

### `dashboard_updater` Task

This task is similar to `csv_analysis`, in that it processes raw ticket CSVs to generate an `output_file` (`results.csv`), but instead of emailing multiple pivot reports, it relies on Data Model connections inside an existing Microsoft Excel dashboard file. It uses COM automation to open the Excel file, refresh its PowerQuery data connections and Pivot Tables, save the workbook, and email the updated dashboard to specified stakeholders via Outlook.

#### Example Configuration
```json
{
  "tasks": [
    {
      "type": "dashboard_updater",
      "download_path": "./downloads",
      "users_file": "./data/users.csv",
      "assignment_settings_file": "./data/assignments.csv",
      "minutes_ago": 15,
      "start_date": "01-May-2026",
      "exclude_branches": [],
      "exclude_categories": [],
      "output_file": "./results.csv",
      "dashboard_file": "./dashboard.xlsx",
      "email_to": "stakeholder@example.com",
      "email_cc": "cc@example.com"
    }
  ]
}
```

### `department_split` Task

The `department_split` task takes a master macro-enabled Excel dashboard and automatically splits it into individual departmental copies based on a target mapping file, while fully preserving all formatting, sheets, and VBA macro modules.

## Pivot HTML performance

The email pivot HTML renderer sorts borrowed ticket-row references instead of cloning the full input rows. This reduces avoidable row/string copying while preserving the existing team/assignee/subtype/category sort order and rendered totals.

## Logging

`tasker` includes detailed logging for auditing and debugging. It leverages the `tracing` framework to output logs both to STDOUT and to a rolling log file `task_csv_analysis.log` situated in the same folder as the executable. Every step (config parsing, file reading, row counting, filtering, pivot creation, and Outlook automation) is rigorously tracked in this file.

## `crm_open_sohail` Task

The `crm_open_sohail` task automates the generation and delivery of Branch & Month dashboard email summaries based on Excel slicer iterations. It acts as an orchestrator around the standard `dashboard_updater`.

### Workflow
1. Executes the standard `dashboard_updater` task using its nested `dashboard_config`.
2. Generates and executes a PowerShell COM script to open the target Dashboard Excel workbook.
3. Automatically maps and iterates over Slicer Items for "Branch" and "Month" slicers.
4. Extracts tabular data dynamically from `PivotTable1` in the `TKT - Dashboard` worksheet.
5. Loads data from `team_mapping_file` (a CSV) to associate team names with owners and target email addresses, mapping these to an `OUL` HTML hyperlink (`<a href="mailto:...">@Owner</a>`) or falling back to plain text mention or default string.
6. Aggregates data and dispatches an Outlook HTML Email populated with nested summary tables mimicking the required visual identity (Blue headers, Red "Grand Total" row). The ReplyAll draft appends `miismail@fakeeh.care` to To unless that address is already present.

### Example Configuration
```json
{
  "type": "crm_open_sohail",
  "download_path": "./downloads",
  "users_file": "./users.csv",
  "assignment_settings_file": "./assignment_settings.csv",
  "minutes_ago": 15,
  "output_file": "./results.csv",
  "exclude_branches": [],
  "exclude_categories": [],
  "dashboard_file": "./dashboard.xlsx",
  "team_mapping_file": "./teams.csv",
  "email_to": "sohail@example.com",
  "email_cc": "reports@example.com",
  "sender_account_email": "sender@example.com",
  "reply_subject_prefix": "[CRM-TEST]",
  "fallback_oul": "N/A",
  "dashboard_sheet_name": "Sheet1",
  "dashboard_pivot_name": "PivotTable2",
  "table_column_widths": ["15%", "10%", "10%", "15%", "15%", "15%", "20%"]
}
```

## Persistent Versioned PowerShell Scripts & Injection Safety

Tasker manages PowerShell scripts strictly through `ScriptManager`, storing scripts persistently in `<exe_dir>/scripts/<Task Name>/`.

### Core Guarantees
1. **Static Templates & CLI Parameterization**: All production PowerShell scripts use static canonical script templates. Runtime-controlled data (paths, subjects, HTML bodies, emails, PIDs) is passed strictly through CLI parameter flags (`param(...)`), eliminating dynamic script string interpolation.
2. **Fingerprinting**: Script SHA-256 fingerprints cover canonical static generated script templates only. Changes in runtime parameter values do not alter fingerprints or create unnecessary script versions.
3. **User Edit Preservation**: When the canonical generator fingerprint is unchanged, `ScriptManager` reuses existing active script files on disk, strictly preserving any manual user edits or debugging adjustments.
4. **Automatic Versioning & Collision Resolution**: Changes to generator source code generate new timestamped script versions (`script_YYYY-MM-DD_HH-MM-SS.ps1`) with deterministic collision suffixes (`_1`, `_2`). Old versions are preserved and never automatically deleted.
5. **Cross-Process File Locking**: All metadata and script operations are protected by OS-level exclusive file locks (`fs2::FileExt::lock_exclusive` on `.task.lock`) covering the complete critical section across processes.
6. **Path Traversal Safety & Metadata Recovery**: Logical script names and active script metadata are strictly validated to prevent path traversal (`..`, `/`, `\`). Corrupted metadata JSON files are automatically backed up to `.metadata.json.corrupted_<timestamp>` and recovered cleanly.

## ScriptManager Versioning & Locking
PowerShell integration relies on `ScriptManager` persistence. Script generators define static templates parameterized solely by run-time arguments.
- Generating the script computes a strict SHA-256 fingerprint ignoring runtime parameters.
- When generating files across concurrent processes, an OS-level lock via `fs2` ensures metadata file updates (`.metadata.json`) are perfectly coordinated.
- In the event of a crash during write, `.metadata.json.corrupted_*` backups are generated and state is safely auto-healed.
- Manual user modifications to active scripts are intentionally kept preserved if the source generator algorithm hash remains identical to the metadata.

## PowerShell Script Files

The PowerShell logic for the following Tasker operations is maintained in standalone `.ps1` files and embedded into the executable at build time with Rust `include_str!`:

- Outlook email execution: `src/tasker/email/outlook_mail.ps1` is the single script for creating, replying, sending, displaying, and saving draft messages.
- Dashboard refresh: `src/tasker/scripts/dashboard_updater.ps1`.
- Department workbook split: `src/tasker/scripts/department_split.ps1`.
- CRM Open Sohail slicer extraction: `src/tasker/crm_open_sohail/slicer_extract.ps1`.
- OPD analysis email: `src/tasker/opd_task/opd_analysis_email.ps1`.
- SAT figures email: `src/tasker/sat_figures_email.ps1`.

At startup, Tasker calls `ScriptManager::install_builtin_scripts()` to materialize all built-in PowerShell templates under `<exe_dir>/scripts/<Task Name>/`, including the shared Outlook helper and CRM Updater templates. Each script is therefore present on disk before a configured task starts, and execution continues through the managed files. The source files are also embedded in the executable at build time so they can be restored if missing.

### Outlook recipient handling

The Outlook script does not call `Recipients.ResolveAll()` and does not run custom recipient pre-validation. `CRM Open Sohail` creates a ReplyAll message and saves it as a Draft; it does not send or schedule that message. Each Outlook operation logs its execution stages through the Tasker log without logging full recipient addresses or message content.
