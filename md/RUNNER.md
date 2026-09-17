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

## Live Execution Preview
Each External Application action block inside the task editor features its own dedicated **Live Execution Preview** displaying up to the **Next 10 Upcoming Executions** for that specific application. The preview uses the authoritative backend scheduling, date resolution, working-hours, week-off, and interval grid alignment engine. Changing one External Application's periodization or dates updates its preview box independently without executing applications or tasks.

## Process Execution, Timeout & Output Caps
- **Process Tree Cleanup**: Process timeouts terminate the entire process tree (`taskkill /F /T /PID` on Windows, `pkill -P` on Unix) to prevent orphan child processes.
- **Bounded Output**: Stdout and stderr byte streams are bounded at 10 MiB memory caps per stream to prevent unbounded memory growth during heavy output.

## HTTP Server Hardening & Security Model
- **Header & Body Limits**: Enforces `MAX_HEADER_BYTES = 64KB` and `MAX_BODY_BYTES = 2MB`. Requests exceeding boundaries return `HTTP 413 Payload Too Large`.
- **Read Timeout**: Socket read operations enforce a 10-second read timeout, returning `HTTP 408 Request Timeout` and closing stalled connections.
- **Line Ending Consistency**: Parses HTTP headers and body consistently across both CRLF (`\r\n\r\n`) and LF (`\n\n`) line-ending delimiters.
- **Chunked Transfer Encoding**: Rejects unsupported `Transfer-Encoding: chunked` with `HTTP 400 Bad Request`.
- **GET Mutation Protection**: All state-changing endpoints strictly reject `GET` requests with `HTTP 405 Method Not Allowed` and require `POST`.
- **Loopback Enforcement**: Runner GUI binds to `127.0.0.1` (localhost loopback). Non-loopback `gui_host` bindings (such as `0.0.0.0` or local network interfaces) are rejected on startup.
- **Non-blocking Async Handlers**: All async HTTP handlers offload blocking process calls using async process execution (`tokio::process::Command`) to keep the Tokio runtime unblocked.

## API Endpoints
- `/run/{task_id}` (POST) - Forces immediate execution of the given task ID (Manual mode).
- `/run-all` (POST) - Enqueues all tasks for immediate execution (Manual mode).
- `/api/tasks/preview` (POST) - Generates up to 10 future execution occurrences for a specified External Application and schedule configuration.
- `/api/apps/manifest` (GET) - Retrieves JSON manifest for registered application via non-blocking async process invocation.

## HTTP Security Model
The Runner GUI web server (`0.0.0.0` bindings rejected, limited strictly to loopback addresses like `127.0.0.1` unless configured otherwise intentionally) ensures safe execution of workflows on a local machine.
- Requests are bounded with a 10-second lifetime absolute timeout across all fragmented reads to prevent Slowloris attacks.
- Strict limit bounds are in place: `MAX_HEADER_BYTES = 64KB` and `MAX_BODY_BYTES = 2MB`. If limits are exceeded, HTTP 413 is raised.
- Malformed inputs, missing headers, or unexpected bytes beyond the declared Content-Length result in an immediate 400 Bad Request error.
- All non-read state mutation routes strictly require POST requests and enforce a 405 Method Not Allowed error on GET.
- A Tokio semaphore limits concurrent HTTP requests to a max of 100 in-flight connections to prevent resource starvation.
- Server logs sanitize URIs and only record standard metadata, masking sensitive body information.

## Manifest Execution
External apps are queried via `--manifest` through an async bounded execution environment (`src/runner/engine/process.rs`). Outputs are heavily capped (MAX 10 MB per stream stdout/stderr). Long-running manifests or hanging child processes are aggressively cleaned up natively across platforms via process tree termination (`taskkill /F /T /PID` or `pkill -P`).

## Date Periodization
The period engine is responsible for converting configurations (`PeriodMode::Monthly`, `AQuarter`, etc.) directly into Execution Periods, decoupled from the core lifecycle queue. Resolution occurs sequentially where the context of the `start_date` bounds the upcoming contextual resolution of the `end_date`.
