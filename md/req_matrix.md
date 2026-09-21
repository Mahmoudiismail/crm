# Requirements Matrix

This document tracks fulfillment of requirements.

| ID | Requirement | Status | Notes |
|---|---|---|---|
| REQ-1 | Support Multiple Task Steps and Parallel Execution | Implemented | `ExecutionMode::Parallel` |
| REQ-2 | Strict HTTP parsing limits | Implemented | 64KB max header, 2MB max body, strict Content-Length checking |
| REQ-3 | Empty Schedules represent strictly Manual tasks | Implemented | Schedules evaluated as `[]` don't trigger background polling or default intervals. |
| REQ-4 | AppManifest drives date parameter UI visibility | Implemented | Front-end evaluates manifest args for date needs. |
| REQ-5 | Live Preview leverages Backend logic | Implemented | GUI requests `/api/tasks/preview` and eliminates JS-side math. |
| REQ-6 | Proper Working Hours binding to GUI | Implemented | UI maps selection to data-profiles for dynamic loading. |
