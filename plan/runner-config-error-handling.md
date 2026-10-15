# Runner Config Load Error Handling — V1 Plan

## Scope
Make the Runner GUI task create/edit handlers consistent when loading the Runner configuration fails. Reconcile the historical security/reliability audit so it does not present already-mitigated findings as current confirmed defects.

## Constraints
- Smallest correct V1 change; no broad refactor or new abstraction.
- Do not change public APIs.
- Keep the download queue deferred; do not introduce bounded-channel backpressure without a safe consumer failure/drain design.
- Do not claim tests or CI passed unless their results are observed.

## Implementation
1. In `src/runner/gui/handlers.rs`, propagate `RunnerConfig::load` errors in task form/create/update handlers instead of silently replacing the configuration with defaults.
2. Reuse the already-loaded config in `handle_edit_task_page` instead of loading it twice.
3. Update `md/security-reliability-audit.md` to distinguish historical findings from items verified against current code; correct stale statements about HTTP bounds and CSV reader configuration, and avoid claiming unverified issues are confirmed.
4. Keep the queue as a deferred risk and preserve the existing audit plan’s post-merge CI qualification unless a separate run is actually found.

## Verification
- Review the final diff and check that only intended handler config loads changed.
- Run the repository’s relevant format, Clippy, and test checks through GitHub Actions after opening the PR; report the actual results.
- A focused regression test should be added if the handler test setup can verify that a malformed config produces an error rather than silently using defaults without requiring disproportionate test scaffolding.

## Stop conditions
Do not merge the PR into `main` without separate explicit authorization.
