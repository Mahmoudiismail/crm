# Panic & Error Handling Audit Inventory

## Overview

This document provides a complete inventory of panics, `unwrap`, `expect`, `unreachable!`, and `todo!` macros found in the non-test production code.

## Category A — Acceptable

These occurrences have been verified as safe because they are practically impossible to hit in the given context (e.g. index bound checks that are already verified) or they correspond to poison error recovery.

1. **`src/yasweb/browser.rs:290`** (and others in same file)
   - Code: `browser.get_tabs().lock().unwrap_or_else(|e| e.into_inner());`
   - Justification: Standard Rust mechanism for poisoned Mutex recovery.

2. **`src/runner/engine.rs:219`**
   - Code: `let (task_to_run_box, policy) = queued_tasks.remove(i).expect("Queue index out of bounds");`
   - Justification: Safe because `i` is checked strictly within `i < queued_tasks.len()` inside a synchronous loop over a local vector.

3. **`src/bin/yasweb.rs:535`**, **`src/bin/yasweb.rs:555`**
   - Code: `let parsed_dt = parsed.and_hms_opt(0, 0, 0).expect("midnight is valid");`
   - Justification: `and_hms_opt(0, 0, 0)` on a mathematically valid `NaiveDate` is guaranteed to succeed.

4. **`src/tasker/dashboard_updater.rs:31`**, **`src/tasker/dashboard_updater.rs:32`**
   - Code: `let stdout = child.stdout.take().expect("Failed to open stdout");`
   - Justification: The process is spawned with explicit `Stdio::piped()`, meaning that it is mathematically guaranteed to be captured by `take()`.

5. **`src/runner/config.rs:783`**, **`src/runner/config.rs:834`**
   - Code: `NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is mathematically valid")`
   - Justification: 00:00:00 is always mathematically valid.

## Category B — Replace

These occurrences represent potentially recoverable errors that were converted to robust error propagation (e.g., using `anyhow`) or handled properly.

1. **`src/bin/runner.rs:229`**
   - Original Code: `Icon::from_rgba(rgba, width, height).unwrap_or_else(|_| panic!("Failed to create icon"))`
   - Action Taken: Modified `load_icon()` to return `anyhow::Result<Icon>`. Handled the error in `resumed()` by logging it and supplying a dummy icon so the application doesn't completely crash if icon rendering fails on a platform.

2. **`src/utils.rs:187`** and **`src/utils.rs:195`**
   - Original Code: `Local::now().date_naive() - chrono::TimeDelta::try_days(1).expect("valid days");`
   - Action Taken: Replaced with `.context("valid days")?` since `chrono::TimeDelta::try_days` could technically fail, and the function `resolve_date_var` already returns `Result`.

3. **`src/utils.rs:209`**, **`src/utils.rs:211`**, **`src/utils.rs:213`**
   - Original Code: `.expect("valid next year")` and `.expect("valid preceding day")` during EOMonth calculation.
   - Action Taken: Replaced with `.context("...")?` propagating the error back to the caller gracefully.

4. **`src/bin/yasweb.rs:455`**, **`src/bin/yasweb.rs:456`**
   - Original Code: `let start_dt_time = current_dt.and_hms_opt(0, 0, 0).unwrap();`
   - Action Taken: Replaced with `.context("Invalid start/end time")?` to propagate error gracefully.

5. **`src/utils.rs:126`**
   - Original Code: `changed |= merge_json(curr_map.get_mut(k).unwrap(), v);`
   - Action Taken: Converted to `if let Some(mut_val) = curr_map.get_mut(k) { ... }` so that if `get_mut` fails somehow, it's silently skipped, rather than panicking the app.

## Category C — Needs Investigation

None found. All identified `unwrap()` or `expect()` macros in production pathways were either easily fixable into propagated errors or were mathematically proven to be safe.

## Risk Assessment
- **Before:** Several unwrap statements could potentially panic if there were specific date math edge cases or un-renderable Window's UI icons, bringing down the entire `runner` interface or failing scheduled operations completely.
- **After:** The risk of unexpected panics in production logic is effectively mitigated. Safe `.unwrap()`s were converted to explicit `.expect("...")`s with math proofs, and failures are propagated securely as Result combinations using `anyhow`.
