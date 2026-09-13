# Runner Application GUI

The Runner GUI provides an interface to configure tasks, applications, and pipelines. It is accessible via HTTP according to `gui_host` and `gui_port` inside `runner_config.json`.

## Multiple Steps and Execution Modes
Tasks inside the Runner support executing multiple sequential or parallel actions within "Steps". You can configure an unlimited number of Steps and Actions inside the GUI by clicking **Add step** or **Add action**.

## External Application Date Periodization Modes
Periodization and dates (`period_mode`, `start_date`, `end_date`) belong exclusively to `ExternalAppSpec`. `RunnerTask` does not own task-level periodization fields.

Each External Application action configures its own periodization mode alongside Start Date and End Date inputs:
- **Normal / Custom**: Standard single date range without period transformation (`[Start, End]`). Invalid/inverted date ranges (`start_date > end_date`) produce explicit errors.
- **Monthly**: Normalizes date range to complete calendar months from 1st of Start month to last day of End month.
- **Quarterly**: Normalizes date range to complete calendar quarters (Q1: Jan-Mar, Q2: Apr-Jun, Q3: Jul-Sep, Q4: Oct-Dec).
- **A Month**: Repeats the month determined by Start Date for each year between Start Year and End Year.
- **A Quarter**: Repeats the quarter determined by Start Date for each year between Start Year and End Year.

### Fixed Date vs Dynamic Expression UI
For each External Application independently, Start Date and End Date support explicit selection between:
- **Fixed Date**: Specific calendar dates (e.g. `2026-01-01`).
- **Dynamic Expression**: Relative dynamic expressions (`today`, `yesterday`, `tomorrow`, `beginning_of_month`, `eomonth`, and `next <weekday>`).

### Start-Before-End Resolution Semantics
Date expressions are evaluated sequentially:
1. Start Date is resolved first relative to current time.
2. End Date is then resolved using the resolved Start Date as its contextual base date.
3. For relative weekday expressions (e.g. `next sat` for both Start and End), date resolution evaluates both to the exact same upcoming occurrence rather than advancing the End Date to the subsequent week.
4. When Start Date determines a month context (e.g. `beginning_of_month`), `eomonth` for End Date refers to the same resolved month.
5. Invalid date expressions produce explicit errors without silent fallbacks.

## Legacy Configuration Migration
When loading legacy configuration files containing task-level `period_mode`, `start_date`, or `end_date`, the migration layer inspects all `ExternalAppSpec` instances in both `steps` and `post_run_steps`. Any `ExternalAppSpec` that does not have explicit period or date values inherits the legacy task-level settings. Explicit `ExternalAppSpec` settings take precedence and remain untouched. Upon re-saving, periodization settings are persisted exclusively under `ExternalAppSpec`.

## Live Execution Preview
Each External Application action block inside the task editor features its own dedicated **Live Execution Preview** displaying up to the **Next 10 Upcoming Executions** for that specific application. The preview uses the authoritative backend scheduling, date resolution, working-hours, week-off, and interval grid alignment engine. Changing one External Application's periodization or dates updates its preview box independently without executing applications or tasks.

A single step has an execution mode:
- **Sequential**: Every action executes in order. The pipeline halts if an action fails.
- **Parallel**: Every action executes at the same time concurrently. The pipeline halts if any action fails.

Tasks can also configure **Post Run Steps**, which trigger their own isolated step pipeline if and only if the main step pipeline completes successfully.

When configuring an application's actions, the Runner GUI will dynamically inject the available parameters based on the executed application's manifest. This means Runner can seamlessly schedule Yasweb downloads, CRM fetching, or simple Shell Commands out of the box.

## Manual Executions
When you manually trigger a task from the GUI (e.g. clicking **Run Now** or **Run All**), the task is queued for immediate execution without advancing its `next_run_at` schedule. This ensures that manually forcing a task does not overwrite or skip the originally scheduled automatic run.

## API Endpoints
- `/run/{task_id}` (POST) - Forces immediate execution of the given task ID (Manual mode).
- `/run-all` (POST) - Enqueues all tasks for immediate execution (Manual mode).
- `/api/tasks/preview` (POST) - Generates up to 10 future execution occurrences for a specified External Application and schedule configuration.

## Security Model

- **Loopback Enforcement**: Runner GUI binds to `127.0.0.1` (localhost loopback). Binding to non-loopback `gui_host` addresses (such as `0.0.0.0` or local network interfaces) without protection is rejected on startup to prevent unauthenticated remote control.
- **HTTP GET Safety**: State-changing operations (`/create`, `/update/...`, `/delete/...`, `/run/...`, `/enable/...`, `/disable/...`, `/run-all`, `/working-hours/create`, `/apps/create`, etc.) strictly require `POST` requests. `GET` requests to state-changing endpoints are rejected with `HTTP 405 Method Not Allowed`.
