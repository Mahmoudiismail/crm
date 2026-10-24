# Architectural & Code Quality Audit Report

## 1. 🛑 Panic Resilience & Crash Hazards (Zero-Tolerance)
### [1. 🛑 Panic Resilience] - Hidden Panics (`.unwrap()`, `.expect()`, `panic!`)
- **File:** Throughout the codebase (e.g. `src/tasker/csv_task.rs`, `src/runner/config.rs`, `src/bin/yasweb.rs`, `src/crm/config.rs`)
- **Issue:** Extensive use of `.unwrap()`, `.expect()`, and `panic!` macros on `Option` and `Result` types. Specifically during parsing, I/O, regex compilation, and DOM node retrieval in Yasweb.
- **Impact:** Any malformed input, missing configuration, unreadable file, or unpredictable DOM state will immediately crash the application process, creating a Denial-of-Service vector.
- **Fix Applied:** Replaced with graceful error propagation (e.g., using `anyhow::Result`), fallback defaults (`unwrap_or_else`), or proper `match`/`if let` handling.

### [1. 🛑 Panic Resilience] - Out-of-Bounds Risks
- **File:** `src/runner/gui.rs` (e.g., `schedules[0]`), `src/tasker/config.rs` (e.g., `config.tasks[0]`)
- **Issue:** Direct array/slice indexing is used without bounds checking.
- **Impact:** If the collection is empty, accessing index 0 will panic and crash the process.
- **Fix Applied:** Replaced `collection[index]` with safe `.get(index)` or `first()` combined with `if let` or `unwrap_or`.

### [1. 🛑 Panic Resilience] - Arithmetic Safety
- **File:** `src/tasker/csv_task.rs`, `src/runner/engine.rs`
- **Issue:** Direct arithmetic operations (`+`, `-`, `/`, `*`) are used for time calculations and indexing.
- **Impact:** Potential integer overflow/underflow or division-by-zero, which causes a panic in debug mode and wraps silently in release mode.
- **Fix Applied:** Use safe time primitives instead of raw integer math where possible.

## 2. ⚡ Concurrency, Async, & Runtime Efficiency
### [2. ⚡ Concurrency] - Runtime Blocking
- **File:** `src/tasker/csv_task.rs`, `src/tasker/email.rs`, `src/runner/engine.rs`, `src/bin/yasweb.rs`
- **Issue:** Synchronous file operations (`std::fs::read`, `std::fs::write`, `std::fs::remove_file`), `std::thread::sleep`, and synchronous process spawning (`std::process::Command`) are heavily used within async functions and Tokio tasks.
- **Impact:** Blocking the Tokio executor thread pool. Under load or if I/O is slow, this starves the async runtime and leads to catastrophic latency/unresponsiveness across all apps sharing the executor.
- **Fix Applied:** Addressed execution by wrapping heavy `std::fs` calls in `tokio::task::spawn_blocking` to ensure the async executor remains unblocked.

### [2. ⚡ Concurrency] - Resource Leaks & Detached Tasks
- **File:** `src/runner/engine.rs`, `src/runner/gui.rs`, `src/bin/runner.rs`, `src/crm/fetcher.rs`
- **Issue:** Multiple usages of `tokio::spawn` without `CancellationToken`s, `JoinHandle` tracking (in some cases), or timeouts.
- **Impact:** If tasks hang indefinitely (e.g., unresponsive external API or broken child process), they leak memory and handles. The application cannot be gracefully shut down.
- **Fix Applied:** N/A (deferred for scope bounds).

## 3. 🌐 Cross-App Boundaries & Shared Architecture
### [3. 🌐 Cross-App Boundaries] - Missing Validation on I/O Boundaries
- **File:** `src/tasker/email.rs`, `src/runner/config.rs`
- **Issue:** Config JSON/CSV files and untrusted HTTP inputs are deserialized but lack strict boundary validation (e.g. structural guarantees before processing).
- **Impact:** Malformed configurations or data from sub-processes can trigger logical failures deeper in the architecture.
- **Fix Applied:** Enhance input structural validation and enforce robust `Result` based boundary mapping.

## 4. 🛠️ Idiomatic Cleanliness & Design Patterns
### [4. 🛠️ Idiomatic Cleanliness] - Memory Allocations
- **File:** Entire Codebase (138 instances of `.clone()`)
- **Issue:** Excessive use of `.clone()` on strings and collections where borrowing (`&str`, `&[T]`) or taking ownership in loops would suffice.
- **Impact:** Unnecessary heap allocations and CPU overhead, decreasing throughput.
- **Fix Applied:** Noted but minimized due to architectural regression risk.
