# Execution Plan: Documentation & Developer Experience Improvements

## Objective
Improve onboarding, maintainability, and contributor experience by enhancing repository documentation, inline Rustdoc, examples, and scripts without altering runtime behavior. The work is split into logical batches of under 15-25 files each.

## Batch 1: High-Level Documentation, Scripts & Examples
- **README.md**: Enhance project overview, prerequisites, architecture summary, build/run/test instructions, and developer workflow.
- **CONTRIBUTING.md**: Sync developer workflows, testing instructions, and ensure no unnecessary duplication with README.
- **Examples & Scripts**: Review `tasker_config.json.example`, `run.sh`, etc. Ensure they compile, use current arguments, and follow best practices.
- **md/ Folder Consistency**: Review key markdown files (e.g., `BUILD_AND_RUN.md`, `OPERATIONS.md`) to ensure commands and examples reflect the current state (like pure Cargo commands).

## Batch 2: Core Shared Modules Rustdoc
- **`src/lib.rs`**: Ensure clear high-level crate documentation.
- **`src/utils.rs`**: Document purpose, invariants, and side effects of public/shared utility functions (e.g., `executable_dir`, `setup_logging_with_levels`, config loaders, date parsers).
- **`src/manifest.rs`**: Document `AppManifest`, arguments, and validation invariants.
- Focus on `pub` and `pub(crate)` items that provide high value for a developer exploring the codebase.

## Batch 3: Configuration & Complex Modules Rustdoc
- **Configuration modules**: `src/crm/config.rs`, `src/runner/config.rs`, `src/tasker/config.rs`, `src/yasweb/config.rs`. Ensure struct invariants and serde/CLI rules are documented.
- **Complex logic modules**: `src/crm/auth.rs` (SRP implementation details), `src/crm/fetcher.rs`, and key Tasker modules (e.g., `csv_task.rs` validations).
- **Comment cleanup**: Remove redundant comments that just restate code; ensure comments explain "why" and trade-offs.

## Conclusion
- Finalize execution by verifying tests, clippy, and cargo doc.
- Produce a final summary detailing completed deliverables and cataloging the remaining documentation technical debt.
