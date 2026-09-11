# Architectural and Code Quality Audit Report

## 1. 🛑 Panic Resilience & Crash Hazards (Zero-Tolerance)

**[CRITICAL] [Panic Resilience] - Unsafe `unwrap()` usage in date parsing**
* **Location:** `src/tasker/csv_task.rs : 37`
* **Anti-Pattern Found:** The `parse_created_at` function maps `NaiveDate::parse_from_str` outcomes by immediately calling `.unwrap()` on `and_hms_opt()`.
* **Architectural Impact:** If `and_hms_opt` fails (which shouldn't happen for `0,0,0` but the compiler doesn't guarantee this), the runner panics, causing denial-of-service for the background task loop.
* **Remediation:** Return the `Option` directly from `and_hms_opt()`.
```rust
// src/tasker/csv_task.rs
    if let Ok(dt) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return dt.and_hms_opt(0, 0, 0);
    }

    if let Ok(dt) = NaiveDate::parse_from_str(trimmed, "%d-%b-%Y") {
        return dt.and_hms_opt(0, 0, 0);
    }

    // e.g. "1-May" -> "1-May-2026" (append current year)
    let with_year = format!("{}-{}", trimmed, chrono::Local::now().year());
    if let Ok(dt) = NaiveDate::parse_from_str(&with_year, "%d-%b-%Y") {
        return dt.and_hms_opt(0, 0, 0);
    }

    // try d-b format
    let with_year2 = format!("{}-{}", trimmed, chrono::Local::now().year());
    if let Ok(dt) = NaiveDate::parse_from_str(&with_year2, "%e-%b-%Y") {
        return dt.and_hms_opt(0, 0, 0);
    }
```

**[CRITICAL] [Panic Resilience] - Unhandled CLI argument parsing panic**
* **Location:** `src/bin/tasker.rs : 313`
* **Anti-Pattern Found:** During tests or configuration path fallback execution, `config_path.to_str().unwrap().to_string()` is called.
* **Architectural Impact:** If the configuration path contains invalid UTF-8 bytes (which is possible on Windows/Linux file systems), this will panic and crash the application at boot.
* **Remediation:** Use `to_string_lossy()` which safely handles invalid UTF-8 by replacing characters.
```rust
// src/bin/tasker.rs
        let args = vec![
            "tasker".to_string(),
            "--config".to_string(),
            config_path.to_string_lossy().to_string(),
            "--only-call-center".to_string(),
            "--send-exceptions".to_string(),
        ];
```

**[CRITICAL] [Panic Resilience] - Unsafe date calculation `unwrap()`**
* **Location:** `src/bin/yasweb.rs : 1213`
* **Anti-Pattern Found:** Hardcoded `.unwrap()` is used when doing eomonth (end-of-month) date arithmetic.
* **Architectural Impact:** If the date math somehow underflows/overflows (e.g. extremely large year values from CLI args), it will crash the automation driver.
* **Remediation:** Use `.expect()` with a semantic message to handle the panic explicitly, satisfying static analysis requirements.
```rust
// src/bin/yasweb.rs
                let next_month = if dt.month() == 12 {
                    NaiveDate::from_ymd_opt(dt.year() + 1, 1, 1).expect("valid next year")
                } else {
                    NaiveDate::from_ymd_opt(dt.year(), dt.month() + 1, 1).expect("valid next month")
                };
                next_month
                    .pred_opt()
                    .expect("valid preceding day")
                    .format("%d-%m-%Y")
                    .to_string()
```

## 2. ⚡ Concurrency, Async, & Runtime Efficiency

**[HIGH] [Concurrency & Async] - Synchronous I/O in Async Executor Context**
* **Location:** `src/crm/fetcher.rs : 6`
* **Anti-Pattern Found:** The helper function `has_recent_download` relies heavily on `std::fs::read_dir`, `std::fs::metadata`, and other blocking `std::fs` operations, and it is called directly from the async `fetch_reports` loop.
* **Architectural Impact:** Calling blocking synchronous I/O from within an async task blocks the underlying Tokio worker thread. At scale or on slow network drives, this starves the executor and prevents other concurrent tasks from progressing.
* **Remediation:** Migrate `has_recent_download` to use `tokio::fs`.
```rust
// src/crm/fetcher.rs
use std::time::SystemTime;

async fn has_recent_download(download_dir: &std::path::Path, prefix: &str) -> bool {
    let threshold = SystemTime::now() - std::time::Duration::from_secs(30);

    if let Ok(mut entries) = tokio::fs::read_dir(download_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(prefix) && name.ends_with(".csv") {
                            if let Ok(modified) = metadata.modified() {
                                if modified >= threshold {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
// Note: You must also update the caller `fetch_reports` to `.await` this function:
// if !prefix.is_empty() && has_recent_download(download_dir, prefix).await {
```

## 5. 🛠️ Idiomatic Cleanliness & Design Patterns

**[MEDIUM] [Idiomatic Cleanliness] - Unnecessary Allocations via `.collect::<Vec<_>>().join("")`**
* **Location:** `src/runner/gui.rs : 1663`
* **Anti-Pattern Found:** Iterating over items, collecting them into an intermediate heap-allocated `Vec<String>`, and then immediately joining them.
* **Architectural Impact:** Unnecessary heap allocation and memory fragmentation, especially detrimental in high-throughput HTML generation logic.
* **Remediation:** Use `Iterator::collect::<String>()` to bypass the intermediate vector.
```rust
// src/runner/gui.rs
    }).collect::<String>();
```

## 9. 🧮 Data Structure & Algorithmic Efficiency

**[HIGH] [Data Structures] - O(N) Suboptimal Lookups in Hot Loops**
* **Location:** `src/tasker/email.rs : 452`
* **Anti-Pattern Found:** The application checks if a branch is excluded by calling `.contains()` on a `Vec<String>` during row-by-row iteration over the dataset.
* **Architectural Impact:** This results in O(N * M) time complexity where N is the number of rows and M is the number of excluded branches. For large datasets, this severely impacts performance.
* **Remediation:** Pre-compute a `HashSet<String>` outside the loop.
```rust
// src/tasker/email.rs
    // [Surrounding code context: near line 427]
    let exclude_branches_lower: std::collections::HashSet<String> = config
        .exclude_branches
        .as_deref()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|b| b.to_lowercase())
        .collect();

    let exclude_categories_lower: std::collections::HashSet<String> = config
        .exclude_categories
        .as_deref()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|c| c.to_lowercase())
        .collect();

    // [Inside the loop, near line 452]
            let is_excluded_branch = exclude_branches_lower.contains(&branch);
            let is_excluded_category = exclude_categories_lower.contains(&category);
```

## 15. 🦀 Strict Compiler & Linting Pragmas

**[MEDIUM] [Compiler Pragmas] - Missing Strict Lints**
* **Location:** `src/lib.rs`
* **Anti-Pattern Found:** The root library file lacks strict safety and linting pragmas.
* **Architectural Impact:** Misses automated code quality gates provided by rustc and clippy, allowing unsafe code or unidiomatic patterns to creep into the codebase.
* **Remediation:** Enforce `#![forbid(unsafe_code)]` at the root module.
```rust
// src/lib.rs
#![forbid(unsafe_code)]

pub mod crm;
pub mod manifest;
pub mod runner;
pub mod tasker;
```
