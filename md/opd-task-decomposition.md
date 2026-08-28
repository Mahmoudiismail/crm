# OPD Task Decomposition

## 1. Executive Summary

The `opd_task.rs` module has been successfully decomposed into a responsibility-driven architecture within the `src/tasker/opd_task/` directory. This refactoring improves the Single Responsibility Principle (SRP) and the Don't Repeat Yourself (DRY) principle, enhancing the code's readability and testability while strictly preserving existing business rules, public API, and application behavior.

## 2. Old Structure

The previous implementation resided in a single file: `src/tasker/opd_task.rs` (630 lines). This monolithic file mixed orchestration, file system traversal, CSV parsing, Excel extraction, complex domain logic for merging overlapping rows, CSV serialization, and generating/executing a complex PowerShell script for Outlook automation.

## 3. New Structure

The logic has been divided into a cohesive module hierarchy:

```
src/tasker/opd_task/
├── mod.rs                (High-level orchestration workflow)
├── models.rs             (Shared domain structs like `CusRow`)
├── csv_history.rs        (Reads and parses the historical `cus_input` CSV file)
├── file_discovery.rs     (Scans directories for relevant new Excel files and parses timestamps)
├── data_extraction.rs    (Reads Excel files, applies business filters, and aggregates new data)
├── merging.rs            (Pure domain logic grouping rows, checking overlaps, and merging)
├── csv_export.rs         (Serializes and writes the merged `CusRow`s back to a CSV)
└── powershell_email.rs   (Generates and executes the PowerShell COM automation script)
```

## 4. Responsibilities of Each Module

*   **`mod.rs`**: Acts as the coordinator. It maps configuration paths and calls the respective specialized modules in sequence. It implements no direct processing logic.
*   **`models.rs`**: Houses the `CusRow` struct, making the core data representation available to parsing, merging, and exporting layers without circular dependencies.
*   **`csv_history.rs`**: Dedicated strictly to reading `cus_input`, parsing its legacy headers, converting KSA date strings into strongly-typed `NaiveDate` structs, and identifying the latest archived timestamp to determine processing boundaries.
*   **`file_discovery.rs`**: Navigates the filesystem using `WalkDir`, isolates target files based on naming conventions and extensions, and parses file modification timestamps directly from file names.
*   **`data_extraction.rs`**: Uses the `calamine` crate to load Excel workbooks, identify specific worksheets, validate row structures, apply all business exclusion logic (specialties, employees, departments), and extract/aggregate the target metrics.
*   **`merging.rs`**: Houses pure business logic. It takes the historical CSV representations and the newly extracted aggregate data, aligns them chronologically, checks for time-column overlaps, and decides whether to merge or append rows.
*   **`csv_export.rs`**: Translates the merged domain models back into CSV records using the `csv` crate and writes them to the output path.
*   **`powershell_email.rs`**: Isolates the string templating of the complex PowerShell COM script and the `tokio::process::Command` execution block, preventing script generation logic from muddying data operations.

## 5. Dependency Flow

Dependencies strictly flow downwards from the orchestration layer to the specialized modules. The specialized modules (`csv_history`, `data_extraction`, `merging`, etc.) depend only on standard library components, shared utilities, and the shared `models.rs` types. There are no circular dependencies.

## 6. DRY Improvements

- Separated pure logic from I/O boundaries, allowing future extraction of common patterns.
- Date parsing logic specific to historical CUS input is centralized in `csv_history.rs`.
- Type-casting and extraction of strings/numbers from Excel is cleanly scoped inside the `data_extraction.rs` boundary.

## 7. SRP Improvements

- Business filtering rules are clearly isolated in `data_extraction.rs`.
- Domain merging decisions are decoupled from CSV parsing, located in `merging.rs`.
- The PowerShell script generation is no longer tangled with the file operations.
- The `mod.rs` clearly exposes *what* the module does sequentially without bogging the reader down in *how* it does it.

## 8. Public API Compatibility

The public API was successfully preserved. The signature remains:

```rust
pub fn run(config: &OpdAnalysisConfig) -> Result<()>
```

All callers referencing `crm_tool::tasker::opd_task::run` will continue to function without modification.

## 9. Behavior-Preservation Verification

No features, behavior, output formats, or filename generation logic were altered. The exact sequence of PowerShell execution, error logging formats, CSV header generation, and date format strings matches the original implementation.

## 10. Tests Added/Updated

- Verified all existing workspace tests continue to pass.
- Added `test_merge_no_overlap` in `merging.rs` to validate the pure domain logic of time-column grouping and collision detection.

## 11. Validation Results

- `cargo fmt` executed successfully.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed with no warnings. Added `#[derive(Debug)]` to new structures to satisfy existing strict lint rules (`missing_debug_implementations`).
- `cargo test --workspace` passed 112 main crate tests, 0 failures.

## 12. Dependencies Added

No new dependencies were added.

## 13. Remaining Technical Debt

- The PowerShell script embedded as a format string in `powershell_email.rs` is large. Consider migrating it to a standalone `.ps1` template file shipped alongside the binary in the future for easier maintenance.
- `file_discovery.rs` manually parses timestamps from strings; depending on future filename variations, a more robust regex-based extraction might be preferred.

## 14. Follow-up Recommendations

- Extract `file_discovery` logic into a generic utility if other Tasker reports need to parse dates from downloaded filenames.
- Implement more extensive unit tests in `data_extraction.rs` and `merging.rs` leveraging mock data to further lock in business rule behaviors.
