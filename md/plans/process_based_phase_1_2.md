# Process-Based Architecture: Phase 1 and Phase 2

## Objective

Move application-owned implementation out of the root `crm_tool` library and establish dependency boundaries for the five executable processes without changing runtime behavior.

## Approved scope

- Phase 1: move CRM, Tasker, Yasweb, Runner, and CRM Updater implementation into their owning application packages.
- Keep only genuinely shared contracts and utilities in the root shared package.
- Remove cross-process implementation dependencies, including CRM Updater's dependency on Tasker's ScriptManager.
- Keep CLI behavior, manifest output, configuration formats and paths, log names, executable names, and external-process compatibility unchanged.
- Phase 2: make each application package declare only the dependencies its own implementation uses; remove unused dependencies and broad feature sets where safe.
- Keep package-local tests with their owning package and retain integration coverage for cross-process contracts.

## Repository findings

- The root package currently owns all five implementation modules under `src/` and exposes them through `src/lib.rs`.
- Each executable under `apps/` currently depends on the root `crm_tool` package.
- The root manifest declares a broad dependency set, while app manifests add further dependencies. This means the executables are separate OS processes but still compile against a central implementation crate.
- CRM Updater calls `crate::tasker::script_manager::ScriptManager` to execute its own PowerShell scripts. That is a direct implementation dependency across process boundaries and must be removed without losing updater script behavior.
- Tasker ScriptManager embeds both Tasker and CRM Updater PowerShell templates. Its ownership and embedded-script responsibilities must be split so Tasker and Updater do not depend on one another.
- Root-level integration tests and benchmarks reference `crm_tool::runner`, `crm_tool::tasker`, and `crm_tool::yasweb`; these must be reassigned or adapted as part of the move.
- `src/utils.rs` contains shared runtime helpers, but every exported helper must be checked for actual consumers before it remains in the shared package.

## Target boundaries

| Package | Owns |
| --- | --- |
| `crm_tool` (shared root package) | Manifest contract and utilities genuinely used by multiple applications |
| `apps/crm` | CRM authentication, configuration, fetcher, downloader, and CRM CLI |
| `apps/tasker` | Tasker configuration, report/task implementations, email, and Tasker script management |
| `apps/yasweb` | Yasweb configuration and browser automation |
| `apps/runner` | Runner configuration, scheduler, process execution, and GUI |
| `apps/crm_updater` | Updater configuration, update pipeline, diagnostics, and updater-specific script management |

The shared package must not re-export or depend on application-owned modules. An application may depend on the shared package, but no application package may depend on another executable package merely to reuse implementation.

## Implementation sequence

1. Inventory all source files, tests, benchmarks, `include_str!` paths, module references, and direct/transitive dependency use.
2. Move each application module and its unit tests into the owning app package; add a package library target where needed so the existing binary remains a thin entry point.
3. Reduce the root library to the shared manifest and utility surface. Keep compatibility only where required by actual external consumers; do not retain application modules as forwarding shims.
4. Split Tasker and Updater script management. Updater must install and execute only updater-owned scripts; Tasker retains its own built-in scripts.
5. Reassign root integration tests to package-local tests or rewrite the remaining cross-process tests to exercise public CLI/manifest behavior.
6. Prune root and app Cargo dependencies based on source use and package-specific `cargo tree` / dependency audit results. Do not remove a dependency solely because it is not referenced by the binary entry-point file.
7. Update the architecture, build, dependency, and application documentation in `md/`.
8. Review the complete diff for accidental behavior/config/CLI changes.

## Verification

Required before merge:

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo test --workspace --all-targets --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- Windows release build for all five executables.
- Verify each package's dependency tree and ensure app packages do not depend on one another.
- Verify `--manifest` output, CLI help, configuration path behavior, logging, and updater/Tasker script installation behavior.
- Inspect CI results and the final PR diff; do not claim a check passed unless its actual run succeeded.

## Constraints

- Preserve existing runtime behavior and public CLI/configuration contracts.
- Smallest correct changes; no new framework, plugin runtime, or unnecessary abstraction.
- Do not change unrelated functionality.
- Do not merge until review and required CI checks pass.
