# Testing Improvements

This document tracks testing improvements and technical debt identified during the Phase 3 Test Coverage & Regression Protection phase.

## Test Coverage Improvements
* `src/utils.rs`: Added coverage for edge cases like atomic writes, executable dir path resolution, date handling features (`eomonth`, `base_date`), and configuration merging functions.
* `src/manifest.rs`: Added deep coverage for deserialization errors, validation, and AppArg builder methods.
* `src/crm/config.rs`: Added regression test coverage for `finalize_runtime_fields` and config parsing serialization.
* Removed redundant, useless placeholder tests from the `tests/` directory and replaced integration tests in `tests/manifest.rs` and `tests/csv_processing.rs` with functional verifications.

## Regression Tests Added
* `tests/test_crm_startup.rs`: Added CLI intercept test for `--manifest` to ensure it continues to reliably return a 0 status code for the runner UI.
* `tests/csv_processing.rs`: Ensured strict reading (flexible=false) actually fails appropriately.
* `src/utils.rs`: Regression tested configuration JSON merge rules to ensure atomic arrays don't get merged inappropriately and unchanged configs don't report false-positives.

## Benchmark Improvements
* Re-audited `benches/csv_parsing.rs` for isolated benchmarking.
* Added `benches/config_merging.rs` to benchmark complex JSON merging routines utilizing Criterion.

## Remaining Testing Technical Debt
* Many tasks inside `tasker/email.rs` heavily rely on filesystem I/O and PowerShell execution. They are hard to unit test in an isolated manner. Refactoring those modules to use Dependency Injection or Traits for Command execution would significantly improve testability.
* Network and Webex specific APIs in `wcxx` need mocked server tests instead of testing against live/fake tokens.
* Yasweb's browser automation tests require a headless environment, currently not isolated nicely enough in standard CI. A robust page-object or isolated mocked Chrome test would bring better coverage without flakiness.

## CI Recommendations
* Ensure `cargo bench --workspace` is optionally run or gated for performance regressions. It's safe to run in CI but takes significant time. Consider adding it to a weekly cron or manual trigger rather than PR gating.
* The tests are highly deterministic and can run in parallel quickly, the `test.yml` workflow looks robust right now.
