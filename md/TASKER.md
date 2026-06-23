# Tasker Application

`tasker` is a lightweight, stateless task runner binary part of the CRM Tool suite. It is designed to be executed by the `runner` and can process various configured tasks in a single pass before exiting. Because it operates cleanly and exits, it can safely be invoked multiple times in parallel by the runner scheduler without deadlocks, provided tasks are configured with different targets.

## Note for AI Agents (AGENTS.md Addendum)

> **Important**: If any modifications are made to the `csv_analysis` task within `tasker`, you MUST run the validation test using the provided raw dataset (from the user's pastebin) and compare it to the expected output before submitting the PR to ensure regressions are avoided.

## Configuration

`tasker` looks for a `tasker_config.json` file in the current working directory by default. You can override this by passing the path as the first argument:
```bash
./tasker path/to/my_config.json
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
      "output_file": "./results.csv"
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
- `output_file`: Destination file path to write the combined, joined, and augmented CSV output.

#### Processing Logic
1. **User Maps:** Loads the user mapping file. Looks for columns matching `cognito_username` and `UserDepartmentName / Team Name` to create a `Position` list and define the primary assignee team.
2. **Assignments:** Loads the assignment settings, matching `(Category, Type, Subtype)` to `Auto agent/team assignment`.
3. **Tickets Join:** Iterates dynamically over dynamically identified ticket reports modified in the last `minutes_ago` interval.
    - Resolves and standardizes names (e.g., replaces underscores with spaces).
    - Removes duplicates based on `Ticket Id`.
    - Parses Excel serial timestamps or `dd/mm/yyyy hh:mm:ss` timestamps into proper strings.
    - Joins data to calculate the exact `Position` and `team`.
    - Adds `Day` and `Month` tracking columns.
4. **Sort and Output:** Sorts numerically by `Ticket Id` and streams everything efficiently to the `output_file`.
