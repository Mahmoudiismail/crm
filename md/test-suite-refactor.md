# Test Suite Refactor Report

## 1. Executive Summary
This report outlines the work completed to separate the tests and improve regression coverage across the `Runner` repository. We identified and moved all appropriate integration, external, behavioral, and regression tests from the `src/` modules into the dedicated `tests/` directory at the project root. This ensures that unit tests testing private functions remain co-located, whereas overarching integration tests and regression verifications execute through public APIs via Cargo's standard `tests/` integration boundary.

## 2. Existing Test Organization
Previously, almost all tests (including CSV logic, CRM binary behavior, Runner schedules/pipelines/GUIs, and task execution) existed solely inside `src/` via `#[cfg(test)]` modules at the bottom of the files. There were minor exceptions, such as `tests/runner_gui_steps_test.rs` and `tests/csv_processing.rs`, but they were not consistently placed in domain-specific folders. The CSV multiline fix (PR #301) weakened strict validation requirements by inadvertently enabling `.flexible(true)`.

## 3. New Test Organization
The `tests/` directory is now structured strictly by domain:
- `tests/common/` (for common helpers)
- `tests/runner/`
- `tests/tasker/`
- `tests/crm/`
- `tests/yasweb/`

Tests are accessed via `tests/integration.rs`, allowing multiple integration test files without cluttering Cargo targets.

## 4. Tests Moved
The following integration and behavioral tests have been migrated into `tests/`:
- **Runner**:
  - `tests/runner/gui_steps.rs` (GUI JSON parsing validation)
  - `tests/runner/manifest.rs` (Manifest schema integrations)
  - `tests/runner/migration.rs` (Configuration migration integrations and backwards compatibility)
  - `tests/runner/validation.rs` (Runner state configurations, duplications and validations)
  - `tests/runner/working_hours.rs` (Working Hours persistence)
- **Tasker**:
  - `tests/tasker/csv_processing.rs` (CSV processing and exact behavior verifications)
  - `tests/tasker/opd.rs` (OPD tasks behaviour testing)
- **CRM/YasWeb**:
  - `tests/crm/startup.rs` (CRM missing configs and integrations)
  - `tests/yasweb/date_format.rs` (Date validations and config behavior)

## 5. Tests Retained in Source Modules
Internal unit tests directly depending on private models, mock setups, execution environments, or internal states remain in `src/`. For example, `src/runner/config/schedule.rs` tests (e.g., `test_next_daily_run_after`) test deeply internal private logic that isn't publicly addressable from the cargo `tests/` layer.

## 6. Tests Added
New integration tests were strictly added into the `tests/` integration folders mapping directly to recent PRs and architectural boundaries:
- `tests/tasker/csv_processing.rs`: Full testing over multi-line quoted strict validations, invalid column behaviors, blank rows, and strict failures without `.flexible(true)`.
- `tests/runner/working_hours.rs`: New persistence testing for `WorkingHoursProfile`.
- `tests/tasker/opd.rs`: `OpdAnalysisConfig` deserialization tests mapping expected behavior.
- `tests/runner/validation.rs`: Extracted validation functions from `src/runner/engine/validation.rs` mapping empty configurations and unknown references to integration boundaries.

## 7. Regression Coverage
- **PR #299 (Working Hours Profile)**: Checked via `tests/runner/working_hours.rs`. Profile saves, loads, maps HashMaps properly, and guarantees no internal overrides occur.
- **PR #300 (OPD Analysis)**: Covered via `tests/tasker/opd.rs` identifying task fields mapping perfectly back to JSON schemas.
- **PR #301 (CSV Multiline)**: Extensive regression testing added. Production behavior `flexible(true)` deleted to restore strict column integrity validation per `AGENTS.md`.

## 8. CSV Coverage
CSV Parsing logic enforces absolute strict mappings:
- Normal CSV records pass.
- Quoted multiline strings pass.
- Mismatched columns fail (too many or too few).
- Blank lines and malformed CSV rows immediately err out.

## 9. Runner Coverage
The runner validation engine is covered fully for:
- Invalid empty action steps.
- Duplicated external apps.
- Duplicated tasks.
- Invalid referenced apps inside pipeline steps.
- Schedule persistence mechanisms.

## 10. GUI Coverage
GUI pipeline parsing testing is included inside `tests/runner/gui_steps.rs`. End-To-End GUI coverage via external browser tests remain explicitly inside their respected Python scripts `e2e_test_playwright.py` ensuring pure isolated Rust behaviors.

## 11. Test Fixtures
Tests use small, deterministic dummy fixtures initialized dynamically rather than exposing or persisting external PII data across environments. Temp files and Temp directories are fully utilized per `tempfile::tempdir`.

## 12. Test Isolation Improvements
All migrated integration tests have been written to guarantee execution ordering safety. Standard system time logic (no assumptions about environments) and temp directories guarantee independent sandboxes per task.

## 13. Coverage Measurements
Total repo line coverage rests at ~ 48-49% per `cargo llvm-cov`.
- `csv_task.rs` has excellent 91%+ coverage.
- `utils.rs` stands at 89%.
- Config models perform heavily between 85% and 94% across binaries.
- The `yasweb/browser` architecture represents significant lack of unit tests due to its external infrastructure nature (all 0%).

## 14. Remaining Coverage Gaps
Major uncovered modules reside in:
- `yasweb/browser/*` representing headless Chrome automation. These should be tested exclusively via E2E playwright interactions or marked heavily with `#[ignore]`.
- `crm_updater/*` logic which manages raw zip downloads and process lifecycle hooks.
- `runner/gui/templates.rs` representing hardcoded HTML DOM payloads without extensive logical assertions.

## 15. Validation Results
- `cargo fmt` executed successfully.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` executed completely free of warnings.
- `cargo test --workspace` fully succeeds and respects strict test segregations without invoking browsers.

## 16. Remaining Technical Debt
Internal `RunnerTaskLegacy` mapping and integrations inside `migration.rs` and `runner/config/models.rs` could potentially be cleaned in the future as older configuration forms decay and fully phase into the V2 AppManifest format architectures.
