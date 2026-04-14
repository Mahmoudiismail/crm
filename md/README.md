# CRM Tool Documentation Index

This folder is the full rebuild and operations reference for the CRM Tool codebase.

## Agent Entry Point

All AI coding agents must read repository-root `AGENTS.md` before making changes. That file requires agents to keep this `md/` documentation set updated in the same change as code, config, dependency, or script edits.

## Read Order

1. `AGENTS.md`
2. `APPLICATION_SUMMARY.md`
3. `ARCHITECTURE.md`
4. `BUILD_AND_RUN.md`
5. `CONFIG.md`
6. `AUTH_FLOW.md`
7. `FETCHER.md`
8. `DOWNLOADER.md`
9. `SCHEDULER_TRAY.md`
10. `DOCKER.md`
11. `OPERATIONS.md`
12. `AI_DOC_POLICY.md`

## Documentation Rules

- Treat these docs as source-controlled system design.
- Any behavior change in `src/`, runtime scripts, `.devcontainer/`, or `Cargo.toml` must update relevant docs in this folder.
- If docs and code conflict, update docs immediately in the same change.

## Current Runtime Model

- Runner orchestration lives in `src/bin/runner.rs` + `src/runner/*`.
- CRM one-shot execution entrypoint lives in `src/bin/crm.rs`.
- Shared modules are exported from `src/lib.rs`.
- CRM domain logic lives in `src/crm/*` (via module paths to current implementation files).
- Runner triggers CRM work by invoking external `crm` executable.
- `crm` supports CLI args for runtime behavior (`--report`, `--config`).
- All runnable tasks are stored in `runner_config.json`.
