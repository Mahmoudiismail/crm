# CRM Tool Documentation Index

This folder is the full rebuild and operations reference for the CRM Tool codebase.

## Read Order

1. `APPLICATION_SUMMARY.md`
2. `ARCHITECTURE.md`
3. `BUILD_AND_RUN.md`
4. `CONFIG.md`
5. `AUTH_FLOW.md`
6. `FETCHER.md`
7. `DOWNLOADER.md`
8. `SCHEDULER_TRAY.md`
9. `DOCKER.md`
10. `OPERATIONS.md`
11. `AI_DOC_POLICY.md`

## Documentation Rules

- Treat these docs as source-controlled system design.
- Any behavior change in `src/`, runtime scripts, or `Cargo.toml` must update relevant docs in this folder.
- If docs and code conflict, update docs immediately in the same change.

## Current Runtime Model

- Runner orchestration lives in `src/bin/runner.rs` + `src/runner/*`.
- CRM one-shot execution entrypoint lives in `src/bin/crm.rs`.
- Shared modules are exported from `src/lib.rs`.
- CRM domain logic lives in `src/crm/*` (via module paths to current implementation files).
- No command-line runtime arguments are used for normal operation.
- All runnable tasks are stored in `runner_config.json`.
