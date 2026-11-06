# Sat Figures and Users Export Tasker Jobs

These jobs replace the Excel/VBS processing paths with native Tasker processing. Tasker automatically appends their default entries to an existing `tasker_config.json` during startup without overwriting existing task settings. Both entries start with `enabled: false`; this prevents Sat Figures from sending an email unexpectedly. Set `enabled` to `true` for each job you want to run. Keep the existing `opd_analysis` entry; Sat Figures reuses its exclusion lists.

```json
{
  "type": "sat_figures",
  "enabled": true,
  "download_path": "../yasweb_windows/downloads/Sat figures",
  "minutes_ago": 10,
  "output_file": "./sat_figures.csv",
  "email_to": "dd@merrywillow.mailk.us",
  "email_cc": "",
  "email_subject": "201009977888-1576573266@g.us"
},
{
  "type": "users_export",
  "enabled": true,
  "download_path": "../crm_windows/Downloads",
  "mapping_workbook": "../../users.xlsm",
  "output_file": "./task1/users.csv",
  "minutes_ago": 10
}
```

## Sat Figures

- Reuses OPD's exact row-filter implementation and the workbook's filename prefix and supported extensions.
- Processes matching workbooks whose filesystem modification time is within `minutes_ago` (10 minutes by default), selecting the newest file when more than one workbook has the same report date.
- Uses the `DD-MM-YYYY` date segment in each report filename as the row's report date.
- Calculates Slots Booked from the source `Slots Booked` column, Total Slots from `Total Slot`, Vacant Slots as Total Slots minus Slots Booked, and the percentage as Slots Booked divided by Total Slots.
- Writes one CSV row and one image row per report date, with columns `Date`, `Slots Booked`, `Total Slots`, `Vacant Slots`, and `%`. No grand-total row is added. The image is sent with Outlook using the configured recipient and subject.
- Includes the existing booking date in the email body only when exactly one report date is processed; multi-day emails have no date in the body.
- Fails instead of sending a report if the source workbook is missing, filtering leaves no rows, or Total Slots is zero.

## Users Export

- Reads `users_*.csv` files created within the configured recent window (10 minutes by default), then combines only the file(s) with the latest modification timestamp, matching the workbook query.
- Reads the manual user-to-department mapping from `Sheet2`, columns H:I, in `users.xlsm`.
- Optionally reads `overrides_file` (default `./user_team_overrides.csv`) with `Email` and `Team` columns. Email matching trims whitespace and ignores case.
- An override replaces the final team; users not listed retain the existing workbook mapping. The mapped-department output column remains unchanged.
- A missing overrides file means no overrides. Duplicate emails or incomplete rows fail the task. If overrides are present, the CRM export must contain an `Email` column.
- Writes the five query output columns in their existing order to `users.csv`; unmatched names keep an empty mapped department in the expanded mapping column and use `Teams` for the final `UserDepartmentName / Team Name` column, matching the query.
- Does not open Excel or execute the old `ExportSheet2ToCSV` macro.

Run one enabled task by its 1-based position in the `tasks` array using `tasker --task N`. Disabled new jobs are skipped with a log entry. The Sat Figures task requires an `opd_analysis` configuration in the same file so its filters are not duplicated.
