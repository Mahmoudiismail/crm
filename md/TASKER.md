# Tasker Application

`tasker` is a lightweight, stateless task runner binary part of the CRM Tool suite. It is designed to be executed by the `runner` and can process various configured tasks in a single pass before exiting. Because it operates cleanly and exits, it can safely be invoked multiple times in parallel by the runner scheduler without deadlocks, provided tasks are configured with different targets.

## Note for AI Agents (AGENTS.md Addendum)

> **Important**: If any modifications are made to the `csv_analysis` task within `tasker`, you MUST run the validation test using the provided raw dataset (from the user's pastebin) and compare it to the expected output before submitting the PR to ensure regressions are avoided.

## Configuration

`tasker` looks for a `tasker_config.json` file in the same directory as the executable by default. You can override this by passing the path as the first argument, or using explicit flags. If the targeted configuration file does not exist, `tasker` will automatically generate a default configuration file with a template `csv_analysis` task and log a message to inform the user before attempting to run. Additionally, on each run, `tasker` will automatically scan the existing configuration and merge in any missing default configuration fields seamlessly to prevent breaking changes.

### CLI Arguments
- `--config <PATH>`: Overrides the default `tasker_config.json` path. (Legacy support also allows passing just the path without the flag, as long as it does not start with `-`).
- `--task <INDEX>`: Executes only a specific task from the configuration (1-based index). For example, `--task 1` runs only the first task in the config array.
- `--only-call-center`: When provided, the task skips generating per-team and per-branch emails, and *only* processes and sends the Call Center email logic for the target task.

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
        "send_per_team_branches": ["Dr. Soliman Fakeeh Hospital"],
        "send_per_branch_branches": ["dsfmc", "DSFMH"],
        "send_call_center": true
      }
    }
  ]
}
```

#### Fields Description
- `type`: Must be `"csv_analysis"`.
- `download_path`: Directory to search for downloaded CSV files. It matches files prefixed with `ticket_report` and ending with `.csv`.
- `users_file`: Path to the users mapping CSV (formerly PowerQuery Table11).
- `assignment_settings_file`: Path to the assignment settings CSV containing category, type, and subtype mappings.
- `minutes_ago`: Will only process ticket CSV files that have been modified within the last X minutes.
- `exclude_branches`: Array of strings. Tickets belonging to these branches will be excluded from the final output (case-insensitive).
- `exclude_categories`: Array of strings. Tickets belonging to these categories will be excluded from the final output (case-insensitive).
- `category_exceptions`: (Optional) List of objects specifying conditional inclusions for otherwise excluded categories.
  - `category`: The category string that normally would be excluded.
  - `branch`: (Optional) Only allow the exception if the ticket branch matches this.
  - `team`: (Optional) Only allow the exception if the assigned team matches this.
- `output_file`: Destination file path to write the combined, joined, and augmented CSV output.
- `email_config`: (Optional) Specifies automated email configuration via Microsoft Outlook.
  - `team_mapping_file`: Path to CSV configuring email recipients per Team or Branch Name (Requires headers `Team Name`, `To Emails`, `CC`).
  - `body_template_file`: (Optional) Path to an HTML file to use as the email template. If it does not exist, `tasker` will auto-generate it. It extracts the subject from the `<title>` tag and the content from the `<body>` tag, replacing `{bucket_name}`, `{from_date_str}`, `{today_str}`, and `{html_table}` dynamically.
  - `initial_cc` / `ending_cc`: Static CC emails appended to every sent mail.
  - `send_emails`: Boolean, if `false` emails are left open as drafts (using `.Display()`) for manual review. If `true` uses `.Send()`.
  - `default_to_email`: Fallback email if team mapped isn't found, and also used to send exception/error reports.
  - `send_per_team_branches`: List of branches that should send distinct emails for each *team* within the branch.
  - `send_per_branch_branches`: List of branches that will receive *one email for the entire branch* instead of separated by team.
  - `send_call_center`: Boolean, if true unifies the "Call Center" tickets from all allowed branches into a single email instead of being grouped with the others.

### `dashboard_updater` Task

This task is similar to `csv_analysis`, in that it processes raw ticket CSVs based on configuration mapping, but instead of emailing multiple pivot reports, it surgically injects the resulting CSV data into a specified Microsoft Excel file's Table (ListObject), refreshes the document's Pivot Tables, and emails the updated dashboard to specified stakeholders via Outlook.

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
      "exclude_branches": [],
      "exclude_categories": [],
      "output_file": "./results.csv",
      "dashboard_file": "./dashboard.xlsx",
      "dashboard_table_name": "table2",
      "email_to": "stakeholder@example.com",
      "email_cc": "cc@example.com"
    }
  ]
}
```

#### Fields Description
- Shares all core CSV generation fields with `csv_analysis` (`download_path`, `users_file`, `minutes_ago`, `exclude_branches`, etc.).
- `dashboard_file`: Path to the existing `.xlsx` dashboard file you want to update.
- `dashboard_table_name`: The name of the Excel Table (ListObject) inside the workbook that should be cleared and filled with the new CSV data (e.g., `"table2"`).
- `email_to`: (Optional) Email address to send the final updated dashboard to.
- `email_cc`: (Optional) CC email address for the final report.

#### Processing Logic
1. **User Maps:** Loads the user mapping file. Looks for columns matching `cognito_username` and `UserDepartmentName / Team Name` to create a `Position` list and define the primary assignee team.
2. **Assignments:** Loads the assignment settings, matching `(Category, Type, Subtype)` to `Auto agent/team assignment`.
3. **Tickets Join:** Iterates dynamically over dynamically identified ticket reports modified in the last `minutes_ago` interval.
    - Resolves and standardizes names (e.g., replaces underscores with spaces).
    - Removes duplicates based on `Ticket Id` (deduplicates globally across all files processed).
    - Parses Excel serial timestamps or `dd/mm/yyyy hh:mm:ss` timestamps into proper strings.
    - Joins data to calculate the exact `Position` and `team`.
    - Adds `Day` and `Month` tracking columns.
4. **Sort and Output:** Sorts numerically by `Ticket Id` and streams everything efficiently to the `output_file`.
5. **Email Automation:** (If `email_config` is defined)
    - Re-reads the generated output. Filters out any "Closed" tickets.
    - Groups remaining tickets by either branch or team as defined in `send_per_team_branches` and `send_per_branch_branches`, separating "Call Center" tickets if configured.
    - Uses `rust_xlsxwriter` to create a `.xlsx` data file for each group.
    - Generates a heavily styled HTML Pivot Table counting occurrences by dynamically resolved Status per Subtype/Category.
    - Executes a background PowerShell script to automate Microsoft Outlook (`New-Object -ComObject Outlook.Application`), appending the Excel attachment and drafting/sending the result.

## Logging

`tasker` includes detailed logging for auditing and debugging. It leverages the `tracing` framework to output logs both to STDOUT and to a rolling log file `task_csv_analysis.log` situated in the same folder as the executable. Every step (config parsing, file reading, row counting, filtering, pivot creation, and Outlook automation) is rigorously tracked in this file.
