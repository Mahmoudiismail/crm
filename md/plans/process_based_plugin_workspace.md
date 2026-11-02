# Process-Based Plugin System and Cargo Workspace

## Goal

Turn Runner into the host/orchestrator for independently built process-based applications while preserving the existing executable CLI contract, AppManifest behavior, configuration files, task scheduling, process supervision, logs, and external executable support.

## Architecture decisions

- Use process isolation: each first-party application remains a separate executable; no Rust dynamic-library ABI or in-process plugin loading.
- Runner remains the host and continues to own scheduling, task admission, concurrency policy, timeout/cancellation, and per-task output capture.
- CRM, Tasker, and Yasweb remain independently buildable executables. CRM Updater remains a separate utility because its detached self-replacement lifecycle must not be hosted by the process it replaces.
- Keep the existing AppManifest argument schema and `--manifest` discovery contract backward-compatible. Add only the minimum plugin identity/version metadata needed for managed first-party plugins; do not duplicate the argument schema.
- Preserve manually registered external executables and existing `runner_config.json` records. Any new config fields must have serde defaults and be optional.
- Convert the repository to a Cargo workspace with separate executable packages. Keep the existing `crm_tool` implementation library as the shared core in this migration so process isolation can be introduced without a risky mass relocation of domain modules. Keep executable filenames unchanged.

## Scope

1. Audit all source, integration tests, benches, scripts, documentation, and workflows that refer to the current `crm_tool` package and binary targets.
2. Define and validate the process-plugin metadata/compatibility contract. Invalid, duplicate, incompatible, or missing executable registrations must fail safely and explain the reason.
3. Integrate managed plugin registration with Runner while retaining the existing registered external-app path and config migration behavior.
4. Preserve the child-process execution guarantees: bounded stdout/stderr capture, task logging, timeout, process-tree termination, duplicate admission, app-level concurrency, and working-directory/config resolution.
5. Add independent executable packages as Cargo workspace members without changing business logic or public CLI behavior.
6. Update CI, release selection/build/package logic, VS Code tasks, and all live operational documentation.
7. Add focused regression tests for plugin metadata validation, backward-compatible config loading, manifest discovery, executable resolution, and process lifecycle behavior where changed.

The executable packages are intentionally thin launchers over `crm_tool` in this iteration. This is a deliberate boundary: process-level independence is the requirement; moving all domain source trees into additional library crates is not required to deploy or update each executable independently and would enlarge the migration risk.

## Compatibility requirements

- Keep binary names: `runner.exe`, `crm.exe`, `tasker.exe`, `yasweb.exe`, and `crm_updater.exe`.
- Keep config names and default locations: `runner_config.json`, `config.json`, `tasker_config.json`, `yasweb_config.json`, and `updater_config.json`.
- Preserve `--manifest`, CLI flags, manifest JSON shape for existing fields, and current registered app/task references.
- Preserve PowerShell, Excel COM, Chrome, file layout, logging, and updater behavior.
- Do not remove support for arbitrary registered EXEs or shell commands.
- Avoid dynamic libraries, unsafe code, new frameworks, or unnecessary abstractions.

## Verification

- Run `cargo fmt -- --check`.
- Run Clippy and all tests for the full workspace on Windows, matching repository CI.
- Build each executable independently with `cargo build --release -p <package>`.
- Verify each executable's `--manifest` output and exit code.
- Exercise legacy `runner_config.json` loading, the opt-in plugin compatibility gate, and the existing process supervisor path.
- Validate release workflow selections and ZIP contents for each executable.
- Review the complete diff against this plan and update relevant `md/` documentation in the same PR.

## Delivery

- Keep the work on one branch and one coherent PR; do not create overlapping PRs.
- Commit this plan under `md/plans/` together with the implementation.
- If the available execution environment cannot run Windows tests/builds, report that limitation explicitly and use the repository CI as the verification gate; never claim unexecuted checks passed.
