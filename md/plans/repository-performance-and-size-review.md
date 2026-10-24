# Repository Performance and Binary-Size Review

## Goal

Improve execution time, peak memory use, and release executable size with the smallest safe changes. Preserve report contents, ordering, filtering, duplicate handling, CLI behavior, and Windows compatibility.

## Initial findings

1. `src/tasker/email/reports.rs` reads each lead CSV into a byte vector, wraps it in a lossy UTF-8 view, retains every accepted CSV record in `all_records`, and then copies those records into an XLSX workbook. Removing the intermediate record vector is a low-risk first step. Full streaming of source bytes is deferred because the current lossy UTF-8 behavior should not change without an explicit compatibility decision.
2. `reqwest` enables its `blocking` feature in production dependencies, while repository source searches did not find `reqwest::blocking` use. Keep the feature in dev-dependencies for tests, and remove it from production features if the final source check remains clear.
3. Release settings already favor runtime speed and binary stripping (`opt-level = 3`, fat LTO, one codegen unit, stripped symbols, aborting panics). Do not change these defaults without measuring each binary's size and representative runtime.
4. The repository has Criterion benchmarks for CSV parsing and config merging. Extend or use these before making broad optimization claims; CI validation is not a substitute for representative performance measurements.

## V1 scope

- Write accepted lead rows directly to the XLSX worksheet while reading CSV records, retaining only the seen-ID set and small header metadata rather than a second full copy of accepted records.
- Preserve file ordering (newest first), first-file header/index behavior, deduplication order, excluded-branch behavior, accepted statuses (`new` and `follow-up`), delimiter detection, BOM handling, and current CSV error diagnostics.
- Remove the production-only `reqwest` blocking feature if no production source uses it. Keep the dev-dependency feature for test builds.
- Add/strengthen focused regression assertions where practical.
- Document measurements still required: Windows Release size for each binary and representative runtime/peak-memory comparisons before and after.

## Explicit non-goals

- No architecture rewrite, new runtime/framework, dependency replacement, or blanket refactor.
- No change to Release optimization settings without measured evidence.
- No behavioral change to CSV decoding, report filtering, output naming, sorting, or workbook format.
- No claims of speed or EXE-size improvements until the Windows CI/build artifacts or a reproducible local benchmark confirm them.

## Validation

1. Run `cargo fmt -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` through the repository's Windows CI.
2. Confirm production source has no `reqwest::blocking` usage.
3. Compare Windows Release binary sizes for `runner`, `crm`, `tasker`, `yasweb`, and `crm_updater` before and after. Record exact sizes and the same target/toolchain/settings.
4. Compare representative Tasker lead-report processing with a realistic large CSV set. If no stable baseline data exists, report that limitation rather than inventing speedup numbers.
