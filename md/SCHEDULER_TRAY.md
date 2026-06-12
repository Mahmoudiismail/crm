# Scheduler and Tray Runtime

Primary implementation: `src/bin/runner.rs`, `src/runner/engine.rs`, `src/runner/config.rs`, `src/runner/gui.rs`

## Single Instance Guard

At startup, app binds `127.0.0.1:14592`.

- bind success -> continue.
- bind fail -> assume already running and exit gracefully.

## Tray Setup

Menu items:

- `Run All Tasks Now`
- `Run CRM (Tickets Only)`
- `Open Runner GUI`
- `View Logs`
- `Exit`

Behavior:

- On `Run All Tasks Now`: trigger all enabled tasks in runner config.
- On `Run CRM (Tickets Only)`: trigger ad-hoc CRM fetch with report `tickets`.
- On `Open Runner GUI`: open configured runner GUI URL.
- On `View Logs`: open `<exe_dir>/runner.log`.
- On `Exit`: stop event loop.

## Scheduler Loop

The runner uses **cron-based polling** to evaluate task schedules:

1. **Config Reload**: Re-load `runner_config.json` each cycle.
2. **Poll Interval**: Sleep for `poll_interval_seconds` (minimum 5s, configurable in engine loop).
3. **Cron Evaluation**: For each enabled task, check all schedules using the `schedule_is_due()` function:
   - **Interval**: Check if current time >= `next_run_at`
   - **Once**: Check if current time >= `next_run_at` timestamp
   - **Daily**: Check if current time >= calculated `next_run_at` for today's times
   - **Weekly**: Check if current time >= calculated `next_run_at` for the target day
   - **Monthly**: Check if current time >= calculated `next_run_at` for the target date
4. **Task Execution**: If any schedule is due, execute the task:
   - For `crm_fetch`, invoke external `crm` executable with CLI args (`--config`, `--report`)
   - For `shell_command`, run configured commands. Commands can execute sequentially or in parallel
5. **Task State Update**:
   - Update `last_run_at` timestamp
   - Update `last_status` with execution result
   - For legacy repeat tasks: update `next_run_at` based on `frequency_seconds`
   - For cron schedules: call `advance_schedule()` to compute next execution time
   - For one-time schedules: set `enabled=false` after execution

If multiple schedules on the same task are due in one scheduler tick, the task runs once and all due schedules advance or disable together.

At startup, runner ensures config files exist under executable directory:

- `runner_config.json` via runner config loader.
- `config.json` by invoking `crm --config <path> --report none` when missing.
 - If `runner_config.json` already exists, runner does not perform an immediate task run; tasks will begin on the next scheduler cycle.

## Run Serialization

Runner status lock prevents overlap:

- if task is already running -> new run is skipped.
- else run task and clear running flag when complete.

This prevents overlap from startup + scheduler + tray actions.

## GUI Trigger Endpoints

- `GET /`: HTML dashboard
- `GET /status`: runner status JSON
- `GET /tasks`: configured tasks JSON
- `GET /new-task`: create-task HTML form with multi-schedule and multi-command support
- `GET /edit/<task_id>`: edit-task HTML form with multi-schedule and multi-command support
- `POST /create`: create task from form fields using the simplified schedule and command row editor
- `POST /update/<task_id>`: update task from form fields using the simplified schedule and command row editor
- `GET /create?...`: legacy create task from query-string fields
- `GET /update/<task_id>?...`: legacy update task from query-string fields
- `GET /delete/<task_id>`: delete task
- `GET /run-all`: trigger run-all
- `GET /run-tickets`: trigger ad-hoc CRM tickets task
- `GET /run/<task_id>`: trigger specific configured task

Note: action routes redirect back to the dashboard and display a toast notification on success. The GUI form editor supports multiple schedule types (Interval, Once, Daily at times, Weekly on day, Monthly on day) and multiple command rows with per-command error handling (Run/Continue modes).

CRUD operations persist directly to `runner_config.json` and return validation errors immediately when payload values are invalid.

## Safety Controls

- `allow_shell_tasks=false` blocks all `shell_command` task execution.
- `shell_timeout_seconds` terminates long-running shell tasks by timeout (this can also be overridden per task).
- `min_task_interval_seconds` prevents repeat tasks from running too frequently.
- `shell_command` tasks use `bash -lc` for each command. A command with `continue_on_error=true` does not fail its task.
