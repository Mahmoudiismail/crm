# App-Level Concurrency Architecture

The Runner implementation resolves task concurrency using a robust, decoupled split between task-lifecycle management and resource-level locks via `tokio::sync::Semaphore`.

## Responsibilities

1. **`ExecutionManager`**: Responsible exclusively for the task lifecycle. It ensures that tasks queue correctly, transition to `Running`, and finish gracefully. It no longer prevents tasks from starting due to `RegisteredApp` locks (`allow_concurrent_tasks`).
2. **`AppLockManager`**: Implements resource allocation via synchronization primitives (owned `Semaphore` maps). This guarantees task-independent abstraction of application exclusivity.
3. **Pipeline (`execute_step`)**: Analyzes the specific runtime action bounds (`TaskStep`s) to identify the minimal required applications. It sorts them deterministically (to avoid deadlocks) and waits for exclusive locks only when the running task needs them (preventing future-step reservations). Wait states accurately map back to `RunnerStatus` via `waiting_for_app` keys.

## Determinism

Testing relies entirely on strict deterministic constructs like `tokio::sync::Barrier` and `tokio::sync::Notify` instead of relying on arbitrary timing mechanisms (`tokio::time::sleep`).
