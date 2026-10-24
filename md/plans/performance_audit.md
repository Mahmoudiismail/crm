# Performance Audit Execution Plan

## Objective
Execute a comprehensive performance and allocation audit focusing on I/O, serialization, string allocations, and collection efficiency across the codebase without altering public APIs or observable behavior.

## Actions Performed
1. **I/O Refactoring (Phase 4):**
   - Transformed `std::fs::read` followed by string conversion into buffered reading (`std::io::BufReader`) across `src/tasker/csv_task.rs`, `src/tasker/email.rs`, and `src/tasker/crm_open_sohail.rs`.
   - Result: Greatly reduces peak memory usage on huge CSV files, relying on `csv::Reader` streaming capabilities while selectively falling back to full string reads only during error contexts for diagnostic logging.

2. **Allocation Optimizations (Phase 1 & 2):**
   - Eliminated new object creation inside hot CSV loops in `src/tasker/csv_task.rs` by reusing a single `StringRecord` memory buffer per file scan via `new_record.clear()` and `std::mem::swap`.
   - Prevented unnecessary key cloning inside inner loops by directly utilizing owned variables or references in sets/vectors without duplicated closures.

3. **String Concatenation Refactoring:**
   - Modified vectors building HTML to use optimal approaches where separator logic wasn't required, notably migrating `.collect::<Vec<_>>().join("")` down to simply `.collect::<String>()` inside UI generating scripts like `src/runner/gui.rs`.

4. **Serialization Optimization (Phase 6):**
   - Fixed `src/runner/engine.rs` logging blocks that unnecessarily performed expensive serialization mapping (`serde_json::to_string`) strictly for logs. Integrated `tracing`'s native deferred debug logic via `?task.schedules`.

5. **Benchmark Readiness (Phase 8):**
   - Scrubbed useless placeholder benchmark scripts that artificially padded CI.
   - Restructured the critical `csv_parsing` bench script to leverage `criterion::Throughput::Bytes` to properly track stream throughput scaling, accurately measuring isolated record buffering functionality instead of memory reallocation logic.

## Validation
- `cargo fmt` executed successfully.
- `cargo clippy --all-targets --all-features -- -D warnings` executed successfully.
- `cargo test --workspace` executed successfully, including tasker data validations.
- Benchmarks verified via `cargo bench --bench csv_parsing`.
