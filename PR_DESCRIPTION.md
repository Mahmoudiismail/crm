# Runner: Pipeline Execution

## Summary
This session successfully implements the new Pipeline Execution Model using the canonical configuration introduced in Session 2 and the modular engine created in Session 3. The execution engine now processes tasks as a deterministic pipeline consisting of Sequential and Parallel `TaskStep` elements containing one or more `ActionSpec` actions.

## Pipeline Execution Design
- `execute_pipeline` is the core orchestrator handling arrays of `TaskStep`s.
- `execute_step` orchestrates individual steps dynamically based on their `ExecutionMode` (`Sequential` or `Parallel`).
- `execute_action` directs internal execution boundaries transparently between `shell_command`s and `external_app`s, mapping the action parameters efficiently into `tokio::process::Command`.
- Cleanly eliminates technical debt (like old `legacy_kind` lookups inside execution engine) and obsolete sequential/parallel routines from `shell.rs`.

## Sequential Execution Behaviour
- Iterates synchronously through all actions.
- Automatically halts the execution of subsequent actions and the step entirely if any action fails (fail-fast logic).

## Parallel Execution Behaviour
- Iterates across all actions using `tokio::spawn` internally, wrapping execution in asynchronous futures natively joined using `join_all`/loops.
- Waits for all actions in the barrier to complete.
- Accumulates failures. If one or more parallel actions fail, the entire step is considered failed after all actions resolve.

## Post-Run Pipeline Behaviour
- Maintained exact semantic compatibility with existing Runner models.
- Only executes the `post_run_steps` pipeline if the main pipeline (`steps`) completed entirely with success (`Ok(_)`).

## Backward Compatibility Approach
- Legacy legacy-compatible tasks natively transition through the JSON migration serialization layer into the Canonical model (Session 2), keeping this execution layer completely agnostic to any structural evolution requirements.

## Files Modified
- `src/runner/engine/pipeline.rs`: Completely refactored `run_task_inner` into `execute_pipeline`, `execute_step`, and `execute_action` blocks natively driving execution through `TaskStep` structures. Added 4 comprehensive pipeline tests.
- `src/runner/engine/shell.rs`: Removed obsolete, unused `run_shell_sequential`, `run_shell_parallel`, and accompanying tests which were ported into the unified `pipeline.rs`.
- `md/ARCHITECTURE.md`, `md/APPLICATION_SUMMARY.md`, `md/AI_DOC_POLICY.md`: Updated definitions and terminology around `runner` task pipeline executions.

## Self-Review Summary
- **Architecture**: The `pipeline.rs` now properly encapsulates its orchestration logic, cleanly decoupling from the legacy legacy-dependent logic and reducing the burden on underlying executors (`shell.rs`, `application.rs`).
- **Code Quality**: Functions are small, DRY, properly borrow arguments, and idiomatic.
- **Tests**: 100% of the regression test suite passes. Additional execution scenarios natively cover mixed, parallel, and sequential barriers.
- **Risks**: Timeout semantics apply purely per action natively, not globally across pipelines. This was explicitly requested but may need adjustments in the future if global step timeouts are requested.
