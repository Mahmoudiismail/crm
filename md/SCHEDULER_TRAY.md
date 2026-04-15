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

- Re-load `runner_config.json` each cycle.
- Sleep for `poll_interval_seconds` (minimum 5s in engine loop).
- Find due tasks using the enabled flag and either legacy `next_run_at` or the task `schedules` list.
- Execute tasks one-by-one.
- For `crm_fetch`, invoke external `crm` executable with CLI args (`--config`, `--report`).
- For `shell_command`, run configured command groups in order. Groups can execute commands sequentially or in parallel.
- Update task state:
	- `last_run_at`
	- `last_status`
	- legacy `next_run_at` for repeat tasks
	- per-schedule `next_run_at` for interval and daily local-time schedules
	- `enabled=false` for one-time tasks or one-time schedules after run

If multiple schedules on the same task are due in one scheduler tick, the task runs once and all due schedules advance or disable together.

At startup, runner ensures config files exist under executable directory:

- `runner_config.json` via runner config loader.
- `config.json` by invoking `crm --config <path> --report none` when missing.

## Run Serialization

Runner status lock prevents overlap:

- if task is already running -> new run is skipped.
- else run task and clear running flag when complete.

This prevents overlap from startup + scheduler + tray actions.

## GUI Trigger Endpoints

- `GET /`: HTML dashboard
- `GET /status`: runner status JSON
- `GET /tasks`: configured tasks JSON
- `GET /new-task`: create-task HTML form
- `GET /edit/<task_id>`: edit-task HTML form
- `POST /create`: create task from form fields, including multi-line schedule and command-group editors
- `POST /update/<task_id>`: update task from form fields, including multi-line schedule and command-group editors
- `GET /create?...`: legacy create task from query-string fields
- `GET /update/<task_id>?...`: legacy update task from query-string fields
- `GET /delete/<task_id>`: delete task
- `GET /run-all`: trigger run-all
- `GET /run-tickets`: trigger ad-hoc CRM tickets task
- `GET /run/<task_id>`: trigger specific configured task
- `GET /enable/<task_id>`: enable task
- `GET /disable/<task_id>`: disable task

CRUD operations persist directly to `runner_config.json` and return validation errors immediately when payload values are invalid.

## Safety Controls

- `allow_shell_tasks=false` blocks all `shell_command` task execution.
- `shell_timeout_seconds` terminates long-running shell tasks by timeout.
- `min_task_interval_seconds` prevents repeat tasks from running too frequently.
- `shell_command` groups use `bash -lc` for each command. A command with `continue_on_error=true` does not fail its group.
