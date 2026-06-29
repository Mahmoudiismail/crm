# AI Documentation Update Policy

This policy is mandatory for any AI assistant or automation modifying this repository.

## Rule 1: Docs Must Track Code

After **every code/config/build command that changes behavior**, update relevant docs in `md/` within the same change set.

## Rule 2: Required Mapping

When a file changes, update docs as follows:

- `src/bin/runner.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md`
- `src/bin/crm.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md`
- `src/bin/tasker.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `OPERATIONS.md`
- `src/lib.rs` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`
- `src/runner/*` -> `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `CONFIG.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`
- `src/crm/*` -> `ARCHITECTURE.md`, `CONFIG.md`, `AUTH_FLOW.md`, `FETCHER.md`, `DOWNLOADER.md`, `OPERATIONS.md`
- `src/tasker/*` -> `ARCHITECTURE.md`, `CONFIG.md`
- `Cargo.toml` / dependency changes -> `BUILD_AND_RUN.md`, `DOCKER.md`, `APPLICATION_SUMMARY.md`
- `.github/workflows/*` -> `BUILD_AND_RUN.md`, `OPERATIONS.md`
- `.devcontainer/*` -> `DOCKER.md`, `BUILD_AND_RUN.md`
- `Dockerfile*`, scripts -> `DOCKER.md`, `BUILD_AND_RUN.md`
- `AGENTS.md` -> `README.md`, `AI_DOC_POLICY.md`

## Rule 3: Command-Level Discipline

For each engineering command/session:

1. Identify impacted behavior.
2. Update corresponding `md/*.md` files.
3. Verify docs still reflect actual code paths.
4. Do not defer documentation updates.

## Rule 4: Pull Request Gate

A change is incomplete if behavior changed and no matching doc update exists.

## Rule 5: Accuracy Standard

- Prefer exact function/file names.
- Document defaults, edge cases, and failure modes.
- Keep examples runnable and aligned with current CLI.

## Enforcement Suggestion

Before commit, run a manual checklist:

- [ ] Code changed?
- [ ] Matching docs updated in `md/`?
- [ ] `AGENTS.md` read before making agent-authored changes?
- [ ] Examples still valid?
- [ ] New config fields documented?
- [ ] New CLI flags documented?

If any answer is `no`, update docs before finalizing.
