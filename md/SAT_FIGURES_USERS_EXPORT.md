# Sat Figures and Users Export Tasker Jobs

These jobs replace the Excel/VBS processing paths with native Tasker processing. They are intentionally not enabled in Tasker's generated default configuration because Sat Figures sends an email when it runs.

Add the following entries to the `tasks` array in the existing `tasker_config.json`. Keep the existing `opd_analysis` entry as well; Sat Figures reads its exclusion lists so both jobs use the same OPD filters.

```json
{
  "type": "sat_figures",
  "download_path": "../yasweb_windows/downloads/Sat figures",
  "output_file": "./sat_figures.csv",
  "email_to": "dd@merrywillow.mailk.us",
  "email_cc": "",
  "email_subject": "201009977888-1576573266@g.us"
},
{
  "type": "users_export",
  "download_path": "../crm_windows/Downloads",
  "mapping_workbook": "../../users.xlsm",
  "output_file": "./task1/users.csv",
  "minutes_ago": 10
}
```

## Sat Figures

- Reuses OPD's exact row-filter implementation and the workbook's filename prefix and supported extensions.
- Processes only the newest-created `OPD- Average Patients seen No of Schedule_` workbook in the configured folder.
- Calculates Slots Booked from the source `Slots Booked` column, Total Slots from `Total Slot`, Vacant Slots as Total Slots minus Slots Booked, and the percentage as Slots Booked divided by Total Slots.
- Writes a one-row CSV summary, creates a report image using PowerShell, and sends it with Outlook using the configured recipient and subject.
- Fails instead of sending a report if the source workbook is missing, filtering leaves no rows, or Total Slots is zero.

## Users Export

- Reads `users_*.csv` files created within the configured recent window (10 minutes by default), then combines only the file(s) with the latest modification timestamp, matching the workbook query.
- Reads the manual user-to-department mapping from `Sheet2`, columns H:I, in `users.xlsm`.
- Writes the five query output columns in their existing order to `users.csv`; unmatched names keep an empty mapped department in the expanded mapping column and use `Teams` for the final `UserDepartmentName / Team Name` column, matching the query.
- Does not open Excel or execute the old `ExportSheet2ToCSV` macro.

Run one task by its 1-based position in the `tasks` array using `tasker --task N`. The Sat Figures task requires an `opd_analysis` configuration in the same file so its filters are not duplicated.
