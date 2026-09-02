# Runner Scheduling Configuration

## Schedule Default (Manual vs Interval)
By default, whenever a new Runner task is instantiated, its explicit `schedules` array begins empty.
When `schedules` is empty, this acts as a **Manual** status.
The system will **never** silently assume a default `1h` (3600 seconds) interval just because an explicit configuration isn't provided.
Interval schedules are entirely explicit; if users want tasks to run regularly, they must select "Interval" and supply a frequency.

## Dynamic Dates

Dynamic relative date configurations are fully supported across Runner execution schemas (for properties such as `start_date` and `end_date`). These variables calculate relative to the current local timezone. If a specific `base_date` context is provided, they evaluate against that offset.

Currently supported tags:
- `today`, `yesterday`, `tomorrow`
- `beginning_of_month` (first day of the contextual month)
    - *Note: `this_month` operates identically as a backward compatible alias for existing task states.*
- `next <weekday>` (e.g. `next mon`, `next sat`). Resolves to the next matched occurrence *strictly after* the provided base date.
- `eomonth`: Represents the end of the month containing the contextual base date.

## Sequential Date Resolution

Dynamic schedules perform sequential, cascading dependency resolutions. If both a start date and an end date are derived dynamically (e.g. `start_date = next sat` and `end_date = eomonth`), the system will securely evaluate `start_date` first, and cascade its concrete evaluated resolution as the `base_date` parameter backing the `end_date`.

*(e.g., if evaluated from the current date, `next sat` evaluates to next Saturday, and `eomonth` is assigned to whichever month next Saturday lands in).*

Range validation verifies the constraint `start_date <= end_date` post-resolution, triggering fatal pipeline errors if configuration evaluations mismatch linearly.
