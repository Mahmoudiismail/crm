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
│   └── gui.rs
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
- Repetition/frequency scheduling fields.
- External CRM executable path configuration.

### `src/runner/engine.rs`

- Scheduler polling loop.
- Immediate/manual task triggers.
- Task execution and task metadata updates.
- CRM task execution by launching external `crm` executable with CLI args.
- Timeout and failure handling for child process execution.

### `src/runner/gui.rs`

- Lightweight HTTP GUI server.
- Status endpoint.
- Trigger endpoints: run-all, run-by-id, tickets-only.

### `src/crm/*`

- `auth`: token reuse + Cognito SRP sequence.
- `config`: CRM configuration and token persistence.
- `fetcher`: report API requests, call-log monthly batching, and signed-URL failure range splitting.
- `downloader`: CSV stream download.
- CSV files are written under `<crm_exe_dir>/Downloads`.
- `mod.rs`: shared `run_once` API used by `crm` executable.

## Concurrency Design

- Runner-level status lock prevents overlapping execution cycles.
- Report-level parallelism remains in fetcher (`tokio::spawn` + `join_all`).
- Scheduler and GUI run concurrently in Tokio runtime.

## Error Design

- Fatal startup issues: process exits.
- Runtime operation issues: logged with context.
- Per-report fetch errors: captured as JSON error entries, not full-run abort.
