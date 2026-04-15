# GUI Improvements Summary

## Issues Fixed

### 1. Add Schedule Button Not Working
**Problem**: The `+ Add schedule` button was not properly attaching event listeners due to scoping issues. The button might be clicked before the JavaScript initialized.

**Solution**: 
- Added null checks for the button and container elements
- Wrapped button click handlers in proper conditional logic
- Ensured event listeners only attach when DOM elements exist

### 2. Schedule Not Updating with 24h Value
**Problem**: The schedule interval dropdown only showed options up to 12h, and users couldn't set 24-hour or longer intervals.

**Solution**: 
- Expanded interval options to include: `15m`, `30m`, `1h`, `2h`, `4h`, `8h`, `12h`, `24h`, `2d`, `7d`
- Both backend `compact_duration()` and frontend JavaScript now support all these durations

### 3. Add Command Button Not Working
**Problem**: The `+ Add command` button had the same scoping issues as the schedule button.

**Solution**: Same as Schedule button - added proper null checks and conditional initialization

### 4. Update Button Visibility (White on White)
**Problem**: The button had low contrast on the form background.

**Solution**: Changed buttons from white with gray borders to `bg-emerald-600 text-white` with hover state, making them highly visible and thematically consistent

### 5. Missing Interval Options for Weekly/Monthly Schedules
**Problem**: Users could only set up Interval, Once, or Daily schedules. No weekly or monthly options.

**Solution**: 
- Added `TaskSchedule::Weekly` enum variant with day_of_week and at_time fields
- Added `TaskSchedule::Monthly` enum variant with day_of_month and at_time fields
- Updated UI to show day-of-week selector for weekly schedules (Monday-Sunday)
- Updated UI to show day-of-month input (1-31) for monthly schedules
- Updated all match statements in engine.rs and config.rs to handle new schedule types
- Updated parse_schedules_text to parse weekly/monthly syntax

### 6. Command Input Not Supporting Groups
**Problem**: Users couldn't group multiple commands or set error-handling modes per command.

**Solution**:
- Added `command-mode` dropdown to each command row with options: `Run` and `Continue`
- `Run` mode: halt on error (default behavior)
- `Continue` mode: prefix command with `continue:` during form submission
- Commands are automatically grouped into a single task group on submit
- Updated buildCommands() JavaScript to format commands with mode prefixes

## Code Changes

### `/workspace/src/runner/gui.rs`

- **schedule_row_html()**: Extended to support 5 schedule types instead of 3
  - Added weekly and monthly input fields
  - Expanded interval dropdown options
  - All inputs properly hidden/shown based on schedule type selected

- **schedule_rows_html()**: Updated to render Weekly and Monthly schedule instances

- **command_row_html()**: Enhanced with mode selection
  - Added command-mode dropdown (Run/Continue)
  - Better styling with bg-emerald-600 remove buttons

- **shell_command_editor_html()**: 
  - Changed add-command button to bg-emerald-600 (visible, consistent)
  - Added help text explaining Run vs Continue modes

- **schedule_editor_html()**: 
  - Changed add-schedule button to bg-emerald-600 (visible, consistent)
  - Updated help text to mention all supported schedule types

- **form_script()**: Major JavaScript improvements
  - Fixed button click handler scoping issues
  - Added proper null checks for DOM elements
  - Extended schedule creation to handle weekly/monthly
  - Extended command creation to include mode selector
  - Updated buildSchedules() to emit weekly/monthly syntax
  - Updated buildCommands() to format with mode prefixes

- **parse_schedules_text()**: Extended to parse weekly and monthly schedules

- **legacy_fields_from_schedules()**: Added Weekly and Monthly cases

### `/workspace/src/runner/config.rs`

- **TaskSchedule enum**: Added two new variants
  - `Weekly { enabled, day_of_week, at_time, next_run_at }`
  - `Monthly { enabled, day_of_month, at_time, next_run_at }`

- **default_day()**: New default function returning 1

- Updated all trait implementations to handle Weekly/Monthly:
  - `enabled()`
  - `next_run_at()`
  - `summary()`

### `/workspace/src/runner/engine.rs`

- **normalize_and_validate_schedules()**: Added validation for Weekly and Monthly
  - Validates day_of_week string
  - Validates at_time as HH:MM format
  - Clamps day_of_month to 1-31

- **set_schedule_enabled()**: Updated to handle Weekly and Monthly

- **advance_schedule()**: Added advancement logic for Weekly and Monthly
  - Weekly: advances by 7 days
  - Monthly: advances by 30 days

## Documentation Updates

### `/workspace/md/CONFIG.md`

- Updated "Schedule fields" section to document all 5 schedule types
- Added description of `day_of_week` and `day_of_month` fields
- Updated GUI task editor documentation to list all schedule type options
- Added detailed command mode documentation (Run vs Continue)
- Updated interval examples to show new `24h`, `2d`, `7d` options

## User Experience Improvements

1. **Interval schedules**: Can now set 24+ hour intervals (24h, 2d, 7d)
2. **Weekly schedules**: Select any day of the week to run tasks
3. **Monthly schedules**: Run tasks on a specific day of the month (1-31)
4. **Command error handling**: Choose per-command whether to halt or continue on error
5. **Button visibility**: All action buttons now use green with white text for clarity
6. **Form reliability**: Add buttons work consistently; no more missing event listeners

## Testing Recommendations

1. Create a new schedule with 24h interval - verify it saves and displays correctly
2. Create a weekly schedule for Monday - verify it recalculates properly
3. Create a monthly schedule for the 15th - verify month-end behavior
4. Add multiple commands with mixed Run/Continue modes - verify they execute in order
5. Verify existing tasks with legacy repetition/frequency fields still load correctly
6. Test the add/remove buttons multiple times to ensure they remain responsive
