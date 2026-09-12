# Runner Application GUI

The Runner GUI provides an interface to configure tasks, applications, and pipelines. It is accessible via HTTP according to `gui_host` and `gui_port` inside `runner_config.json`.

## Multiple Steps and Execution Modes
Tasks inside the Runner support executing multiple sequential or parallel actions within "Steps". You can configure an unlimited number of Steps and Actions inside the GUI by clicking **Add step** or **Add action**.

## Date Periodization Modes
Tasks can configure optional periodization modes alongside Start Date and End Date inputs:
- **Normal / Custom**: Standard single date range without period transformation (`[Start, End]`).
- **Monthly**: Normalizes date range to complete calendar months from 1st of Start month to last day of End month.
- **Quarterly**: Normalizes date range to complete calendar quarters (Q1: Jan-Mar, Q2: Apr-Jun, Q3: Jul-Sep, Q4: Oct-Dec).
- **A Month**: Repeats the month determined by Start Date for each year between Start Year and End Year.
- **A Quarter**: Repeats the quarter determined by Start Date for each year between Start Year and End Year.

Date expressions support `today`, `yesterday`, `tomorrow`, `beginning_of_month`, `eomonth`, and `next <weekday>`. If Start and End dates use identical "next weekday" expressions (e.g. `next sat`), both resolve to the exact same upcoming weekday occurrence.

## Live Execution Preview
The Task Create/Edit form features a real-time **Live Execution Preview** displaying the **Next 10 Upcoming Executions**. The preview calls the underlying Runner schedule/period backend engine directly, accurately reflecting schedule times, working hours, and week-off filtering without executing external applications or tasks.

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


## Security Model

- **Loopback Enforcement**: Runner GUI binds to `127.0.0.1` (localhost loopback). Binding to non-loopback `gui_host` addresses (such as `0.0.0.0` or local network interfaces) without protection is rejected on startup to prevent unauthenticated remote control.
- **HTTP GET Safety**: State-changing operations (`/create`, `/update/...`, `/delete/...`, `/run/...`, `/enable/...`, `/disable/...`, `/run-all`, `/working-hours/create`, `/apps/create`, etc.) strictly require `POST` requests. `GET` requests to state-changing endpoints are rejected with `HTTP 405 Method Not Allowed`.
