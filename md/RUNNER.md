# Runner Application GUI

The Runner GUI provides an interface to configure tasks, applications, and pipelines. It is accessible via HTTP according to `gui_host` and `gui_port` inside `runner_config.json`.

## Multiple Steps and Execution Modes
Tasks inside the Runner support executing multiple sequential or parallel actions within "Steps". You can configure an unlimited number of Steps and Actions inside the GUI by clicking **Add step** or **Add action**.

A single step has an execution mode:
- **Sequential**: Every action executes in order. The pipeline halts if an action fails.
- **Parallel**: Every action executes at the same time concurrently. The pipeline halts if any action fails.

Tasks can also configure **Post Run Steps**, which trigger their own isolated step pipeline if and only if the main step pipeline completes successfully.

## Application Concurrency & Lock Management
Step-level application locking is managed by `AppLockManager`:
- Non-concurrent applications (`allow_concurrent_tasks = false`) acquire an exclusive semaphore for the step duration.
- Multiple non-concurrent applications required within a step are sorted deterministically and acquired in order to prevent deadlocks.
- Semaphores are held for the step duration and automatically released on step completion, error, or cancellation.
- Tasks waiting for an application lock remain in `running_task_ids` with `waiting_for_app` status reported in the GUI.
- **Duplicate Task Prevention**: The Execution Manager strictly inspects `running_task_ids` during scheduling evaluation to definitively block duplicate concurrent instances of the identical task ID from being scheduled or pushed into execution pipelines.

## HTTP GUI Security Constraints
The Runner enforces strict application and network layer protections to secure its control plane:
- **Host Validation**: Binding to a non-loopback host interface (`0.0.0.0` or external IP) in `gui_host` without protection is actively rejected at startup, keeping access limited to local configurations (`127.0.0.1`, `localhost`).
- **Connection Timings**: Connections are subject to a strict `10`-second absolute parsing deadline via `tokio::time::timeout` to avoid unauthenticated connection hangs.
- **Memory & Parsing Limits**: The HTTP logic enforces maximum header parsing bounds (`64KB`) and strictly rejects request payloads exceeding `2MB` (`MAX_BODY_BYTES`) via exact `Content-Length` checks, rejecting fragmented or chunked uploads to prevent overflow.

## Scheduling Engine & Manual Execution
Tasks define their execution pattern through one or multiple schedules (Interval, Daily, Weekly, Monthly, Once). The scheduling engine (`generate_upcoming_executions_for_app`) actively evaluates constraints like working hours and week-offs before dispatching.

- **Manual Execution**: If a task is configured without any schedules (`schedules: []`), it represents a purely Manual execution mode. The task remains logically "enabled" in the configuration but will never be executed automatically by the background scheduling loop. It relies solely on manual triggers from the GUI or System Tray interface.

## External Application Date Periodization Modes
Periodization and dates (`period_mode`, `start_date`, `end_date`) belong exclusively to `ExternalAppSpec`. `RunnerTask` does not own task-level periodization fields.

Each External Application action configures its own periodization mode alongside Start Date and End Date inputs:
- **Normal / Custom**: Standard single date range without period transformation (`[Start, End]`). Invalid/inverted date ranges (`start_date > end_date`) produce explicit errors.
- **Monthly**: Normalizes date range to complete calendar months from 1st of Start month to last day of End month.
- **Quarterly**: Normalizes date range to complete calendar quarters (Q1: Jan-Mar, Q2: Apr-Jun, Q3: Jul-Sep, Q4: Oct-Dec).
- **A Month**: Repeats the month determined by Start Date for each year between Start Year and End Year.
- **A Quarter**: Repeats the quarter determined by Start Date for each year between Start Year and End Year.

### Date Input Visibility
The GUI dynamically inspects the returned application `AppManifest` when a task action is created or an app is selected.
If the manifest does not explicitly declare date/periodization arguments (e.g. `start_date`, `end_date`, `period_mode`, or generic `date_var` arguments), the Periodization Mode and Date inputs are completely hidden from the user interface to prevent confusion, preserving layout relevancy to the specific tool executed.

### Fixed Date vs Dynamic Expression UI
For each External Application independently, Start Date and End Date support explicit selection between:
- **Fixed Date**: Specific calendar dates (e.g. `2026-01-01`).
- **Dynamic Expression**: Relative dynamic expressions (`today`, `yesterday`, `tomorrow`, `beginning_of_month`, `eomonth`, and `next <weekday>`).

### Start-Before-End Resolution Semantics
Date expressions are evaluated sequentially:
1. Start Date is resolved first relative to current time.
2. End Date is then resolved using the resolved Start Date as its contextual base date.
3. For relative weekday expressions (e.g. `next sat` for both Start and End), date resolution evaluates both to the exact same upcoming occurrence rather than advancing the End Date to the subsequent week.

## Live Execution Preview
External Application sections feature a robust **Preview** button displaying a visual calendar of upcoming periodic invocations directly within the GUI.
The preview is strictly bounded to the upcoming 10 evaluations (preventing unbounded computational recursion for tightly-scheduled tasks).

The frontend relies **100%** on the Rust backend API (`/api/tasks/preview`) as the authoritative source of truth. The backend API uses the exact same `generate_upcoming_executions_for_app` scheduling engine that evaluates live background tasks, meaning the GUI prediction is functionally identical to the background behavior, accurately rendering working hours blockages, week-off eliminations, and interval compounding.
