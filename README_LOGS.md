# Runner Application Fixes and Logging

- Fixed an issue where Interval tasks would immediately run without respecting their configured `start_time`.
- Refactored `gui.rs` to add the missing `schedule-start-time` CSS class to allow frontend script parsing.
- Enforced new Interval tasks to configure `next_run_at: String::new()` on initialization.
- Re-architected schedule property mapping via `update_task()` to persist unmodified `next_run_at` configurations while users edit task properties (e.g. timeout limits).
- Plumbed out comprehensive telemetry (`tracing`) capturing exact metadata on task events:
    - Task Creation / Deletion / Enable / Disable
    - Task Run Statuses / Start Timestamps / Completion Durations / Failures
    - Scheduler Reload and Background Loop Initialization
