# Production Hardening — Panic & Reliability Audit

## Files modified
- src/tasker/dashboard_updater.rs

## Reliability improvements
- Removed `.expect()` calls in `src/tasker/dashboard_updater.rs` for `child.stdout.take()` and `child.stderr.take()`, converting them to graceful `Result` propagation via `anyhow`.
- Validated error propagation via Tokio executor contexts.

## Assertions reviewed
- Reviewed runtime assertions across the codebase (`src/crm/fetcher.rs`, `src/tasker/csv_task.rs`, etc.).
- The only assertions in production code were identified in test configurations or inside #[cfg(test)] macros. Tests are intentionally excluded, so no production assertions were found that required removal.

## Regression tests added
- Verified the integrity of the application after replacing `expect()` with existing test suites `cargo test --workspace`. No tests were failing, and no new tests were required for these small changes.

## Panic inventory (Keep / Replace / Investigate)

- **src/tasker/dashboard_updater.rs**
  - `child.stdout.take().expect("Failed to open stdout")` -> **REPLACE**: Runtime failure possible if powershell doesn't open streams correctly. Converted to return `Err`.
  - `child.stderr.take().expect("Failed to open stderr")` -> **REPLACE**: Same as stdout.

- **src/tasker/email.rs**
  - `let limit_date = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();` -> **KEEP**: Safe programmer invariant. The hardcoded, mathematically valid date is a legitimate compiler/programmer invariant and a perfectly acceptable use of `.unwrap()`.

- **src/runner/config.rs**
  - `NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is mathematically valid")` -> **KEEP**: It is a compiler invariant that 0,0,0 is a valid time.
  - `NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is mathematically valid")` -> **KEEP**: Same as above.

- **src/bin/yasweb.rs**
  - `let parsed_dt = parsed.and_hms_opt(0, 0, 0).expect("midnight is valid");` -> **KEEP**: Safe programmer invariant. It is a mathematical certainty that 0:00:00 is a valid time for a valid date.
  - `let parsed_dt = parsed.and_hms_opt(23, 59, 59).expect("23:59:59 is valid");` -> **KEEP**: Safe programmer invariant. 23:59:59 is a valid time for a valid date.

- **src/runner/engine.rs**
  - `queued_tasks.remove(i).expect("Queue index out of bounds");` -> **KEEP**: Safe programmer invariant. Pre-verified by a bounds check earlier in the execution path (`i < queued_tasks.len()`). An attempt to use `?` operator fails in Tokio `async move { ... }` blocks which return `()`.

- **src/bin/runner.rs**
  - `std::process::exit(0);` and `std::process::exit(1);` -> **KEEP**: Allowable use. Only binary entry points may terminate the process, and this is the main binary entry point for the runner.
  - `Icon::from_rgba(vec![0; 4], 1, 1).unwrap()` -> **KEEP**: Compiler invariant. The given vector provides precisely enough data for a 1x1 image (rgba = 4 bytes, width * height * 4).

## Any work deferred to future PRs
- N/A.
