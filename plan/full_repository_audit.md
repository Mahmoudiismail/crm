# Full Repository Audit — CRM Tool

## 1. Executive Summary

A comprehensive engineering audit of the CRM tool repository was conducted, covering the CRM report fetcher, CRM Updater, Tasker background processing, Runner scheduling, and general repository practices.

The audit identified critical issues regarding error propagation in Tasker (where task failures are swallowed, resulting in false success states) and an Outlook COM error notification cascade that obscures original failures. Additionally, CRM report downloading suffers from missing HTTP retry logic, leading to silent data loss on transient network errors. Several items initially flagged (like the Runner's semaphore `.unwrap()` and Updater's `Unblock-File`) have been investigated and reclassified as safe or non-bugs based on actual control flow.

## 2. P0 Findings

### [P0] Task failures silently swallowed in Tasker execution loop

**Component:** Tasker
**Exact file(s):** `src/bin/tasker.rs`, `src/tasker/csv_task/mod.rs`, `src/tasker/dashboard_updater.rs`
**Function/module:** `run_app`
**Relevant line numbers:** `src/bin/tasker.rs` lines 186-189, 193-196.

**Current control flow:**
1. Tasker parses its config and iterates over a list of tasks.
2. For a `CsvAnalysis` task, it calls `csv_task::run(csv_config, ...)`.
3. If `csv_task::run` encounters an error (e.g., file not found, bad CSV), it returns `Err(e)`.
4. Inside `run_app`, this is caught by `if let Err(e) = csv_task::run(...)`.
5. The error is logged via `error!("Error running CsvAnalysis task...: {:?}", e);`.
6. The loop **continues** to the next task without preserving the error state.
7. After the loop, `run_app` unconditionally returns `Ok(())`.

**Exact failure mechanism:** Errors from `CsvAnalysis` and `DashboardUpdater` are matched, logged, and dropped. The CLI exits with code `0`.
**Concrete realistic failure scenario:** A user configures `CsvAnalysis` to read from a network drive. The drive goes offline. The task fails to read the CSV and returns an IO error. Tasker logs the error and exits with code `0`. The Runner sees the `0` exit code, updates `last_run_at`, and displays a green "Success" checkmark in the GUI.
**Actual user/system impact:** **False success.** The system operates under the assumption that reports were sent, but they were not. Post-run scripts are executed under false pretenses. If multiple tasks are chained, one can fail while others succeed, masking the partial failure.

**Error/result propagation:**
- Returned: `Err` from `csv_task::run`.
- Caught: `if let Err(e)` in `tasker.rs`.
- Logged: `error!` macro.
- Converted to `Ok(())`: By loop continuation dropping the error.

**Current behavior:** Tasker logs task errors and returns success (exit 0).
**Expected behavior:** Tasker should halt execution of subsequent chained tasks and return the error, resulting in a non-zero exit code so the Runner correctly registers a failure.

**Minimal recommended fix:**
Change `error!(...)` to `anyhow::bail!(...)` (or propagate the error via `?`) for `CsvAnalysis` and `DashboardUpdater` inside `run_app` in `tasker.rs`, matching the existing behavior of the `CrmOpenSohail` task block.

**Regression-test strategy:** Write an integration test where `run_app` is given a `csv_analysis` task pointing to a deliberately missing file. Assert that `run_app` returns an `Err`.
**Suggested commit/PR boundary:** PR 1 (Reliability).
**Classification:** MUST FIX.

---

### [P0] Outlook Error-Notification Cascade

**Component:** Tasker (Email Client)
**Exact file(s):** `src/tasker/email/client.rs`
**Function/module:** `process_emails` (specifically the internal `send_email_for_bucket` closure)
**Relevant line numbers:** 520-546

**Current control flow:**
1. A PowerShell script (`ps_script`) is generated to send a report via Outlook COM.
2. It executes via `run_powershell(&ps_script)`.
3. If this fails (e.g., Outlook is closed), it returns `Err(e)`.
4. The code catches this, logs the error, and formats a **new** PowerShell script (`err_script`) using `New-Object -ComObject Outlook.Application` to send an error notification email.
5. It executes `run_powershell(&err_script)`.
6. If the second script fails (which it will, because Outlook is down), it logs a secondary error.
7. Finally, it bubbles the original error `e` via `anyhow::bail!`.

**Exact failure mechanism:** When the primary Outlook COM transport fails, the error handler attempts to use the exact same broken transport to send a notification.
**Concrete realistic failure scenario:** Outlook is closed. The primary script hangs and eventually throws `RPC_E_DISCONNECTED`. The Rust code catches this and spawns another PowerShell instance to connect to `Outlook.Application`, which hangs and fails for the same reason.
**Actual user/system impact:** **Delayed failure and Observability degradation.** The task takes twice as long to fail (timing out twice). The logs are cluttered with secondary COM errors, obscuring the original root cause. The original error *is* eventually bubbled, but the notification mechanism is fundamentally flawed.

**Error/result propagation:**
- Original error occurs in `run_powershell(&ps_script)`.
- Caught by `if let Err(e) = ...`.
- Notification triggered by formatting `err_script`.
- Creates a new Outlook COM object in the script.
- If it fails, logs `Failed to send error notification email: {}`.
- Original error is bubbled at the end via `anyhow::bail!`.

**Current behavior:** Tasker tries to use Outlook to email an alert that Outlook failed.
**Expected behavior:** Tasker should instantly bubble the error to the Runner/logs without attempting a secondary COM operation.

**Minimal recommended fix:**
Delete the `err_script` formatting and the `if let Err(e2) = run_powershell(&err_script)` block entirely.

**Regression-test strategy:** Mock `run_powershell` in a unit test to always fail, and verify that it only executes once per bucket and bubbles the error immediately.
**Suggested commit/PR boundary:** PR 1 (Reliability).
**Classification:** MUST FIX.

---

## 3. P1 Findings

### [P1] CRM signed-URL download failures are not retried

**Component:** CRM Fetcher
**Exact file(s):** `src/crm/fetcher.rs`, `src/crm/downloader.rs`
**Function/module:** `fetch_reports`, `fetch_recursive`, `download_csv`
**Relevant line numbers:** `fetcher.rs` 93-107 (download processor loop), `downloader.rs` 13-68.

**Current control flow:**
1. `fetch_recursive` calls `fetch_single` to request a report.
2. The server responds with JSON containing a signed S3 URL.
3. The URL is extracted and sent over an MPSC channel (`download_tx`).
4. A detached Tokio background task (`download_processor`) receives the URL and calls `downloader::download_csv`.
5. `download_csv` creates a local file, makes an HTTP GET to the signed URL, and streams the response.
6. If the stream fails (e.g., TCP reset), `download_csv` returns `Err`.
7. The `download_processor` loop catches the `Err`, logs `error!("Download failed for {}: {:#}", k, e);`, and continues to the next URL.

**Exact failure mechanism:** HTTP GET errors on valid signed URLs are logged but swallowed by the background Tokio task.
**Concrete realistic failure scenario:** The CRM API successfully generates a signed URL. The downloader starts fetching it but hits a transient network drop. The download fails halfway. The task finishes, reports success, but the resulting CSV file on disk is incomplete or missing.
**Actual user/system impact:** **Data loss / Partial operation.**

**Detailed tracing answers:**
- *What means "range too large"?* A 400/500 HTTP status where the body contains "failed to generate signed url". Checked by `is_signed_url_generation_failure(&err)`.
- *What triggers recursive split?* The `Err` returned by `fetch_single` matching the condition above.
- *What means successful URL?* A valid JSON payload containing `{"data": {"url": "..."}}`.
- *Where are download errors swallowed?* Inside the `download_processor` channel receiver in `fetch_reports`.
- *Are partial files created/removed?* Yes, `tokio::fs::File::create` creates the file before streaming. If the stream fails, the partial file is left on disk (stale/corrupt).
- *Can a failed download report success?* Yes, the background task swallows the error, and the main task returns `Ok(Value::Object)`.
- *Downloading sequential/concurrent?* Concurrent (`stream.for_each_concurrent(6)`).
- *Do Tickets/Calls/Leads/Users share mechanism?* Tickets, Calls, and Leads use signed URLs. Users uses Base64.
- *Does retry apply to Base64?* No, Base64 is processed entirely in memory via `process_base64_payload`. The retry requirement applies to the secondary HTTP GET for signed URLs.

**Implementation of Desired Behavior (Without changing recursive semantics):**
To retry the *exact same signed URL 3 times*, and if it fails, *request the same report range again* (without splitting), the architecture must pull the download await out of the detached channel and into the `fetch_recursive` execution path.
1. `fetch_recursive` calls `fetch_single`.
2. Extracts URL.
3. Awaits `download_csv_with_retry` (which loops 3 times on the HTTP GET, deleting partial files on failure).
4. If `download_csv_with_retry` fails all 3 times, it returns a custom `Err` (e.g., "Download retry exhausted").
5. `fetch_recursive` catches this custom `Err`. Because it does *not* match `is_signed_url_generation_failure`, it does *not* split the date range. Instead, it loops back and calls `fetch_single` again for the exact same date range to get a new URL.

**Current behavior:** Logs download error, leaves partial file, returns success.
**Expected behavior:** Retry download 3 times. Clean up partial files. If 3 failures, get new URL for same range. Do not split.

**Minimal recommended fix:** Modify `fetch_recursive` to await downloads synchronously. Implement a 3-attempt loop in `downloader.rs`. If 3 attempts fail, bubble a specific error that instructs `fetch_recursive` to re-run `fetch_single` without splitting.
**Regression-test strategy:** Mock `fetch_single` to return a URL, and mock the HTTP client to fail 3 times on the GET request. Assert `fetch_single` is called twice for the exact same range.
**Suggested commit/PR boundary:** PR 2 (Network Resiliency).
**Classification:** MUST FIX.

---

## 4. P2 Findings

### [P2] RUNNER `std::fs::metadata` inside async loop

**Component:** Runner
**Exact file(s):** `src/runner/engine/dispatcher/lifecycle.rs`
**Function/module:** `start_scheduler`
**Relevant line numbers:** 165

**Current control flow:**
```rust
let get_mod_time = |p: &str| -> Option<SystemTime> { fs::metadata(p).ok()?.modified().ok() };
```
This is called inside `tokio::spawn(async move { loop { ... get_mod_time(&config_path); ... tokio::time::sleep(5).await; } })`.

**Exact failure mechanism:** A synchronous filesystem call is executed on a Tokio worker thread.
**Concrete realistic failure scenario:** If `runner_config.json` is located on a slow, contended, or disconnected network drive, the synchronous `fs::metadata` call blocks the Tokio worker thread for several seconds.
**Actual user/system impact:** **Performance degradation / Executor starvation.** Other concurrent async tasks on that worker thread (e.g., HTTP server handling GUI requests) will stall.

**Current behavior:** Uses blocking `std::fs::metadata`.
**Expected behavior:** Should yield to the executor using `tokio::fs::metadata`.

**Minimal recommended fix:**
Change the closure to an async block/function using `tokio::fs::metadata(path).await`.
**Classification:** SHOULD FIX.

---

## 5. P3 / Security Findings

### [P3] Hard-coded ZIP Password

**Component:** CRM Updater
**Exact file(s):** `src/crm_updater/update.rs`
**Function/module:** `process_update_pipeline`
**Relevant line numbers:** 41

**Exact location:** `let extracted_files = extract_zip(&zip_path, downloads_dir, b"123456")?;`

**Security implications:** The password encrypts the update binaries distributed via Outlook Drafts. If an attacker knows the password, they can craft a malicious ZIP payload, place it in the Drafts folder, and the Updater will decrypt and execute it with the user's privileges.
**Who knows it:** Anyone with source code access.
**Internal vs External:** Consumed internally for extraction, but the ZIP itself is generated outside this repository (in a deployment/build pipeline).
**Remediation options:**
1. `env!("UPDATER_ZIP_PASSWORD")` at compile time.
2. Read from `updater_config.json`.
Both require coordinating with the external build pipeline that zips the files.

**Classification:** MUST FIX (Security), but deferred to a coordinated deployment ticket.

---

## 6. Findings Reclassified / Not Actually Bugs

### CRM Updater — `Unblock-File` ignores failures
**Component:** CRM Updater (`src/crm_updater/update.rs`, line 215)
**Finding:** Spawns `Unblock-File` and ignores the `.status()` result.
**Analysis:** The ZIP extraction is performed locally by pure Rust (`zip` crate). Files created directly via `fs::File::create` in Rust do **not** receive a "Mark of the Web" (MotW) NTFS alternate data stream in Windows (unlike files saved via a browser). Therefore, the files are already unblocked. Running `Unblock-File` is redundant. If it fails (e.g., PowerShell execution policies restrict it), ignoring the failure is actually the safest and correct behavior, as making it fatal would cause false update failures.
**Classification:** NOT A BUG.

### Runner Semaphore `.unwrap()`
**Component:** Runner Engine (`src/runner/engine/pipeline.rs`, line 104)
**Finding:** `permit_result.unwrap()` is called after acquiring a semaphore.
**Analysis:** The code immediately preceding the unwrap is:
```rust
let permit_result = sem.acquire_owned().await;
if permit_result.is_err() {
    return Err(...);
}
let permit = permit_result.unwrap();
```
`tokio::sync::Semaphore::acquire_owned()` only returns an `Err` if the semaphore is explicitly closed. Because of the `if permit_result.is_err()` check, reaching the `.unwrap()` line mathematically guarantees the result is `Ok`. It will never panic. While it is unidiomatic Rust (and should ideally be replaced with `?` or `context()`), it is not a correctness bug or panic risk.
**Classification:** NOT A BUG (Tech Debt).

---

## 7. Cross-cutting Issues

- **Inconsistent Error Bubbling:** Frequent use of `let _ = ...` to swallow cleanup errors (e.g., `std::fs::remove_file`). This masks file locking/permission issues, though typically acceptable for temp files.
- **Unidiomatic Rust:** Extensive safe but unidiomatic `.unwrap()` usage protected by explicit `if` checks, rather than idiomatic pattern matching or `?` propagation.

---

## 8. Recommended Fix Order

1. **[P0] Tasker Error Propagation** (Fixes observability and false success reporting).
2. **[P0] Outlook COM Cascade** (Removes noise and delayed failures).
3. **[P1] CRM Signed-URL Download Retry** (Guarantees data integrity on transient network errors).
4. **[P2] Runner Async Blocking** (Improves executor health).
5. **[P3] Security: ZIP Password** (Coordinate with DevOps for extraction).

---

## 9. Proposed PR/Commit Breakdown

- **PR 1: Core Reliability (P0)**
  - Commit: Fix Tasker error bubbling (`tasker.rs`, `csv_task/mod.rs`, `dashboard_updater.rs`).
  - Commit: Remove cascading Outlook error notification script (`email/client.rs`).
- **PR 2: Network Resiliency (P1)**
  - Commit: Implement 3-attempt HTTP retry loop for CSV downloads and adjust recursive splitting logic to support same-range re-requests (`fetcher.rs`, `downloader.rs`).
- **PR 3: Engine Refactoring (P2 & Tech Debt)**
  - Commit: Switch to `tokio::fs::metadata` for config polling (`lifecycle.rs`).
  - Commit: Clean up semaphore `.unwrap()` usage (`pipeline.rs`).

---

## 10. Tests Required

- Integration test in Tasker asserting that `run_app` returns an `Err` when `CsvAnalysis` fails.
- Unit test in `email/client.rs` verifying that an initial PowerShell failure bubbles instantly without spawning a second process.
- Integration test in `crm` simulating a 500 HTTP response during a signed-URL download and asserting a 3-attempt retry, followed by a re-request of the same date range without splitting.

---

## 11. Documentation Updates Required

- Update `md/tasker.md` to reflect that task failures will now trigger Runner-level failure states rather than "Success".
- Update `md/crm.md` to document the new 3-attempt retry behavior for individual CSV downloads and the fallback behavior.

---

## 12. Design Decisions Required Before Implementation

- **MUST FIX:** P0 Tasker Error Propagation, P0 Outlook COM Cascade, P1 CRM Download Retry.
- **SHOULD FIX:** P2 Runner blocking I/O.
- **OPTIONAL / TECH DEBT:** Runner Semaphore `.unwrap()`.
- **DEFERRED / DESIGN DECISION:** P3 Hardcoded ZIP password (requires external pipeline changes).

## 13. Deep Architectural Analysis

### A. CRM Signed-URL Download Concurrency
**Current Architecture:**
- `fetch_reports` iterates over report types and spawns an API fetch future for each into a vector (`futures`).
- These futures execute concurrently via `futures_util::stream::iter(futures).buffer_unordered(4)`.
- Inside each API future, `fetch_recursive` calls `fetch_single`. If it succeeds, it extracts the signed URLs from the JSON and sends them via an unbounded MPSC channel (`download_tx`).
- If `fetch_single` returns an `Err` matching `is_signed_url_generation_failure`, `fetch_recursive` splits the date range and calls itself concurrently using `tokio::spawn`.
- A single, detached `download_processor` consumes the MPSC channel using `stream::unfold` and `.for_each_concurrent(6, ...)`.
- The `download_processor` is awaited at the end of `fetch_reports`.

**Design Options for Retry:**
To retry HTTP downloads without changing the S3 split behavior, there are two primary architectural options:

*Option 1: Synchronous Await in fetch_recursive (Simplest)*
- Drop the MPSC channel entirely.
- In `fetch_recursive`, after receiving the JSON from `fetch_single`, directly await the 3-attempt download loop.
- If the download fails 3 times, `fetch_recursive` recursively calls `fetch_single` on the *same* range (no splitting) and tries again.
- *Tradeoff:* We lose the global `.for_each_concurrent(6)` download pool. Downloads would be constrained by the top-level `.buffer_unordered(4)` API concurrency.

*Option 2: Channel with Oneshot Callbacks (Complex but preserves concurrency)*
- Send a `(URL, OneshotSender<Result<()>>)` across the MPSC channel.
- The `download_processor` still runs `.for_each_concurrent(6)`. Inside the loop, it attempts the 3-retry HTTP GET.
- It sends the final success/failure result back across the oneshot channel.
- `fetch_recursive` awaits the oneshot channel. If it receives a failure, it loops to request a new URL.
- *Tradeoff:* Preserves exact concurrency limits (4 API requests, 6 active downloads), but adds significant channel complexity.

*Recommendation:* Option 1 is highly recommended. The `.buffer_unordered(4)` provides more than enough concurrency for the low volume of report files, and it drastically simplifies the mental model, tying the download lifecycle directly to the API lifecycle.

### B. Outlook Error Propagation
**Verification:**
The tracing of `src/tasker/email/client.rs` confirms that the secondary notification script does *not* completely replace the original error. The original error is logged, the secondary script is launched, its error is logged, and then `anyhow::bail!` throws the original error up the stack.
However, because this occurs inside an iteration over `buckets`, and the secondary COM operation takes 30-60 seconds to time out itself, this cascade can delay task failure by minutes while cluttering logs with redundant RPC disconnect errors. It is fundamentally an observability and reliability issue.
