# Audit Remediation Log

This document tracks the fixes applied based on `audit_report_1.md` and `audit_report_2.md`.

## Fixes Implemented

### 1. Workspace Configuration
- **Issue:** Missing `[workspace]` in `Cargo.toml`.
- **Status:** Already addressed in codebase (`Cargo.toml` lines 89-90).

### 2. Panic Hazards (`unwrap()`)
- **Issue:** Unhandled CLI parsing panic (`to_str().unwrap().to_string()`).
- **Location:** `src/bin/tasker.rs`
- **Status:** Already addressed in codebase. `config_path.to_string_lossy().to_string()` is used in tests.
- **Issue:** Unsafe date calculation `unwrap()`.
- **Location:** `src/bin/yasweb.rs`
- **Status:** Already addressed in codebase. `context("Invalid date math")?` is used for `pred_opt()` and other methods in `src/bin/yasweb.rs`.
- **Issue:** Unsafe `unwrap()` usage in date parsing.
- **Location:** `src/tasker/csv_task.rs`
- **Status:** Already addressed in codebase. `and_hms_opt` returning `Option` directly is correctly handled.

### 3. Concurrency, Async, & Runtime Efficiency
- **Issue:** Synchronous I/O in Async Executor Context (`std::fs::read_dir`, etc.) in `has_recent_download`.
- **Location:** `src/crm/fetcher.rs`
- **Status:** Already addressed in codebase. Uses `tokio::fs::read_dir` and `tokio::fs::metadata`.

### 4. Idiomatic Cleanliness
- **Issue:** Unnecessary Allocations via `.collect::<Vec<_>>().join("")`.
- **Location:** `src/runner/gui/templates.rs` (formerly `src/runner/gui.rs`)
- **Status:** Fixed. Modified `.collect::<Vec<_>>().join(...)` to `.fold(String::new(), ...)` to avoid unnecessary heap allocations of `Vec<String>`.

### 5. Data Structure & Algorithmic Efficiency
- **Issue:** O(N) Suboptimal Lookups in Hot Loops.
- **Location:** `src/tasker/csv_task.rs` and `src/tasker/email/reports.rs` (formerly `src/tasker/email.rs`)
- **Status:** Already addressed in codebase. `exclude_branches_lower` and `exclude_categories` are collected into a `HashSet<String>` before the hot loop.

### 6. Strict Compiler & Linting Pragmas
- **Issue:** Missing Strict Lints.
- **Location:** `src/lib.rs`
- **Status:** Already addressed in codebase. `#![forbid(unsafe_code)]` and other `#![warn(...)]` lints are present at the root of `src/lib.rs`.

### 7. Stringified Error Mapping
- **Issue:** Stringified Error Mapping (`.map_err(|e| e.to_string())` destroying error type).
- **Location:** `src/runner/gui/` module
- **Status:** Already addressed in codebase. It now uses `render_error_page(..., &format!("{e}"))`.
