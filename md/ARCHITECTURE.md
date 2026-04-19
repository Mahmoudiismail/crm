# Architecture

## Source Tree

```text
src/
├── lib.rs
├── bin/
│   ├── runner.rs
│   └── crm.rs
├── crm/
│   ├── auth.rs
│   ├── config.rs
│   ├── fetcher.rs
│   ├── downloader.rs
│   ├── mod.rs
│   └── types.rs
├── runner/
│   ├── mod.rs
│   ├── config.rs
│   ├── engine.rs
│   ├── gui.rs
│   └── form_script.js
```

## Responsibilities

### `src/bin/runner.rs`

- Runner/tray application startup and shutdown.
- Single-instance lock.
- Logging initialization.
- Resolves config paths under executable directory.
- Ensures `runner_config.json` exists (auto-created with defaults).
- Ensures CRM `config.json` exists by invoking external `crm` with `--config <path> --report none` when missing.
- Tray icon/menu setup and event handling.
- Starts runner scheduler engine.
- Starts runner GUI server on configured host/port.

### `src/bin/crm.rs`

- CRM one-shot executable entrypoint.
- Logging initialization.
- Accepts runtime arguments (`--report`, `--config`).
- Executes one CRM cycle then exits.

### `src/lib.rs`

- Exposes shared modules for both executables.

### `src/runner/config.rs`

- Runner configuration loading/saving.
- Runner-local report enum for `crm_fetch` tasks.
- Task definitions (`crm_fetch`, `shell_command`).
- Legacy repetition/frequency scheduling fields and multi-schedule task definitions.
- Shell command group definitions for sequential or parallel execution.
- External CRM executable path configuration.

### `src/runner/engine.rs`

- **Cron-based polling scheduler** (no external job queue; uses standard Tokio + Chrono).
- **Poll interval**: configurable `poll_interval_seconds` (default 30s, minimum 5s), set in `runner_config.json`.
- **Schedule evaluation**: `schedule_is_due()` function compares current UTC time against `next_run_at` (RFC3339 timestamp).
- **Supported schedule types**:
  - **Interval**: runs every N seconds (next runtime = `last_run_at + interval_seconds`)
  - **Once**: runs at specified RFC3339 timestamp (cleared after execution)
  - **Daily**: runs at one or more local times each day (e.g., 09:00, 13:00); next run calculated via `find_next_daily_run()`
  - **Weekly**: runs on specified day of week at optional time (defaults 09:00); wraps to next week if needed
  - **Monthly**: runs on specified day of month at optional time (defaults 09:00); handles month-end edge cases
- **Task execution**:
  - **CRM tasks**: fork external `crm` executable with CLI args (report names, config path)
  - **Shell commands**: execute in configured group order; `parallel` groups spawn concurrently, `sequential` groups run one-at-a-time
  - **Per-command control**: `continue_on_error` flag determines if group halts on first failure
  - **Multi-schedule advancement**: after execution, *all* due schedules call `advance_schedule()` to compute next `next_run_at`
- **Metadata updates**: sets `last_run_at` (current UTC), `last_status` (success/error code), and per-schedule `next_run_at`
- **Safety**: child process spawning with timeout/error handling; status lock prevents concurrent task execution

### `src/runner/gui.rs`

- Lightweight HTTP GUI server.
- Tailwind-styled dashboard loaded from cdnjs.
- Status endpoint.
- Trigger endpoints: run-all, run-by-id, tickets-only.
- POST-based create/update forms with simpler schedule rows and command rows.

### `src/crm/*`

- `auth`: token reuse + Cognito SRP sequence.
- `config`: CRM configuration and token persistence.
- `fetcher`: report API requests, call-log monthly batching, and signed-URL failure range splitting.
- `downloader`: CSV stream download.
- CSV files are written under `<crm_exe_dir>/Downloads`.
- `mod.rs`: shared `run_once` API used by `crm` executable.

## Concurrency Design

- Runner-level status lock prevents overlapping execution cycles.
- Shell command groups run in configured group order. Commands inside a `parallel` group are spawned concurrently and joined before the next group starts.
- Report-level parallelism remains in fetcher (`tokio::spawn` + `join_all`).
- Scheduler and GUI run concurrently in Tokio runtime.

## Error Design

- Fatal startup issues: process exits.
- Runtime operation issues: logged with context.
- Per-report fetch errors: captured as JSON error entries, not full-run abort.
