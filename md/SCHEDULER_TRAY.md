# Runner Scheduler and System Tray

Primary entry points: src/bin/runner.rs, with implementation under src/runner/.

## Startup

Runner accepts --config PATH. Relative paths are resolved from the executable directory; an absolute path is used unchanged. If omitted, Runner uses runner_config.json beside the executable.

An existing malformed configuration is treated as a startup error; Runner does not silently replace it with defaults. If the configuration file is missing, Runner starts with defaults and schedules a non-manual Run All request after startup.

Runner uses a single-instance guard by binding the local port 127.0.0.1:14592. If the bind fails, it reports that Runner may already be running and exits.

## Windows tray

The current tray menu contains:

- Run All Tasks Now — requests a manual run of all eligible tasks.
- Open Runner GUI — opens the configured local GUI URL.
- View Logs — opens runner.log beside the executable.
- Exit — requests shutdown and exits the tray event loop.

The tray does not currently expose a separate Run CRM (Tickets Only) item.

On non-Windows platforms Runner runs headless and listens for Ctrl+C to initiate shutdown.

## Scheduler and manual tasks

Runner evaluates configured schedules (including Interval, Daily, Weekly, Monthly and Once schedules) and applies configured working-hours/week-off constraints where supported. A task with an explicitly empty schedules list is manual-only and is not automatically dispatched by the scheduler.

Manual Run Task and Run All requests report queue-admission failures instead of treating a failed queue send as a successful trigger. The execution manager is the authoritative duplicate-admission point for queued/running task IDs.

## Pipelines and concurrency

A task pipeline consists of steps with one or more actions:

- Sequential steps execute actions in order and stop on failure.
- Parallel steps start actions concurrently and wait for the step to complete; a failed action makes the step fail.
- Post Run Steps execute as a separate pipeline only after the main pipeline succeeds.

Application locks are acquired for a step and released on completion, error or cancellation. Tasks waiting for a non-concurrent application are surfaced as waiting in Runner status. The execution manager prevents duplicate admission of a task ID already queued or running.

## GUI and HTTP safety

The GUI is served on configured gui_host and gui_port; the default address is http://127.0.0.1:8787. Startup validation rejects non-loopback binding. Read-only pages use GET. State-changing actions (create/update/delete, enable/disable, run and similar mutations) require POST; GET requests to mutation endpoints are rejected with HTTP 405.

Request parsing is bounded by connection deadlines and request-size/header limits. See [Runner](./RUNNER.md) for implementation constraints.

## Configuration persistence

Runner serializes GUI/API configuration access with a shared lock. After a mutation is saved, it reloads the file and compares the persisted configuration with the expected state. A read-back mismatch is surfaced as an error. See [Configuration](./CONFIG.md).
