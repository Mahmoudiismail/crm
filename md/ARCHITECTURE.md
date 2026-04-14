# Architecture

## Source Tree

```text
src/
├── main.rs
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

### `src/main.rs`

- Application startup and shutdown.
- Single-instance lock.
- Logging initialization.
- Tray icon/menu setup and event handling.
- Starts runner scheduler engine.
- Starts runner GUI server on configured host/port.

### `src/runner/config.rs`

- Runner configuration loading/saving.
- Task definitions (`crm_fetch`, `shell_command`).
- Repetition/frequency scheduling fields.

### `src/runner/engine.rs`

- Scheduler polling loop.
- Immediate/manual task triggers.
- Task execution and task metadata updates.
- CRM task execution bridge.

### `src/runner/gui.rs`

- Lightweight HTTP GUI server.
- Status endpoint.
- Trigger endpoints: run-all, run-by-id, tickets-only.

### `src/crm/*`

- `auth`: token reuse + Cognito SRP sequence.
- `config`: CRM configuration and token persistence.
- `fetcher`: report API requests and monthly batching.
- `downloader`: CSV stream download.

## Concurrency Design

- Runner-level status lock prevents overlapping execution cycles.
- Report-level parallelism remains in fetcher (`tokio::spawn` + `join_all`).
- Scheduler and GUI run concurrently in Tokio runtime.

## Error Design

- Fatal startup issues: process exits.
- Runtime operation issues: logged with context.
- Per-report fetch errors: captured as JSON error entries, not full-run abort.
