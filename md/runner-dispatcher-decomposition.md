# Runner Dispatcher Decomposition

## 1. Executive Summary
This document outlines the structural decomposition of the monolithic `src/runner/engine/dispatcher.rs` file into a modular, highly cohesive directory structure (`src/runner/engine/dispatcher/`). The goal was to improve adherence to the Single Responsibility Principle (SRP) and Don't Repeat Yourself (DRY) principles while strictly preserving the existing public API, concurrency semantics, and business logic.

## 2. Original Architecture
The original `dispatcher.rs` was a large, monolithic file (approx. 750 lines) that contained mixed responsibilities:
- Runner lifecycle coordination (spawning the execution manager, scheduler loop).
- Dynamic working hours profile logic (CRUD operations).
- Task execution logic (CRUD operations, running specific tasks, running due tasks).
- Schedule evaluation and chron-calculation (advancing intervals, calculating relative dates).
- Configuration I/O boilerplates repeated heavily across command handlers via `tokio::task::spawn_blocking`.

## 3. New Architecture
The file was decomposed into the following directory structure:
```
src/runner/engine/dispatcher/
├── mod.rs (API Façade & Router)
├── helpers.rs (Config I/O DRY Wrappers)
├── schedule.rs (Cron & Next Run Calculations)
├── profile_commands.rs (Working Hours Profile CRUD)
├── task_commands.rs (Task CRUD & Execution Logic)
└── lifecycle.rs (Daemon Lifecycle & Execution Queues)
```

## 4. Module Responsibilities
- **`mod.rs`**: Acts strictly as the coordination layer routing incoming `RunnerCommand` enums to the appropriate submodules. It re-exports necessary public functions (like `start_scheduler`, `create_task`) to guarantee total backward compatibility for external consumers (like `gui/handlers.rs` and `runner.rs`).
- **`helpers.rs`**: Provides pure, asynchronous file-I/O helpers (`load_config`, `save_config`) that safely encapsulate `RunnerConfig` file operations within `tokio::task::spawn_blocking`.
- **`schedule.rs`**: Isolates all calculations regarding time. Functions like `advance_schedule`, `schedule_is_due`, and `policy_from_config` live here independently of task execution logic.
- **`profile_commands.rs`**: Focuses exclusively on the validation, persistence, and cascade-deletion rules surrounding `WorkingHoursProfile` management.
- **`task_commands.rs`**: Focuses exclusively on creating, updating, deleting, and manually/automatically executing instances of `RunnerTask`.
- **`lifecycle.rs`**: Owns the long-running application state. It contains the async `start_scheduler` tick loop and the `spawn_execution_manager` pipeline queue logic.

## 5. Dependency Direction
Dependencies strictly flow downwards toward domain models, avoiding circular imports:
`mod.rs` -> `{task_commands, profile_commands, schedule, lifecycle}` -> `{helpers}` -> `crate::runner::config::*`

## 6. DRY Improvements
The primary DRY improvement was standardizing configuration parsing. Previously, almost every `RunnerCommand` arm repeated a heavy `tokio::task::spawn_blocking` block to load, modify, and save the JSON configuration. This was abstracted into `load_config` and `save_config` within `helpers.rs`, vastly improving code legibility.

## 7. SRP Improvements
Responsibilities are now strongly decoupled. Changes to scheduling arithmetic no longer touch the file containing execution queues. Changes to task updates no longer conflict with working hours profile updates.

## 8. Concurrency Review
Existing concurrency models were preserved flawlessly. The `lifecycle.rs` loop continues to use Tokio channels (`mpsc`) with existing capacities (64 for commands, 128 for exec). Block I/O operations strictly remain within `spawn_blocking` via `helpers.rs`. No arbitrary sleeps or timing hacks were added to force consistency.

## 9. Public API Compatibility
Because `mod.rs` uses `pub use` statements for functions that were formerly exported directly by `dispatcher.rs` (e.g. `start_scheduler`), callers outside the engine (such as the GUI routing) required zero changes and compiled successfully immediately.

## 10. Behavior-Preservation Verification
The queue logic (`ExecutionManagerCommand`), task schedule alignment (`advance_schedule`), and configuration normalization rules (`normalize_and_validate_task`) were retained character-for-character. Missing fields are still handled identically via fallback. Error reporting into `RunnerStatus` state remains the same.

## 11. Tests Added/Updated
Existing tests from `dispatcher.rs` were successfully segregated:
- `tests_queue` (verifying channel capacity blocks) moved directly to `lifecycle.rs`.
- `legacy_repeat_task_is_due_without_next_run` and `daily_local_schedule_gets_future_next_run` moved directly to `schedule.rs`.
All internal module tests continue to pass.

## 12. Validation Results
- `cargo fmt` passed natively.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed with 0 warnings.
- `cargo test --workspace` passed 100%.

## 13. Remaining Technical Debt
The execution manager's internal loop iterates over active tasks via manual index management (`while i < queued_tasks.len()`). While robust, in the future this queue might be better served by standard Tokio semaphores or `FuturesUnordered` if task concurrency becomes a bottleneck. However, as it currently functions flawlessly, it was left untouched to honor behavior preservation.
