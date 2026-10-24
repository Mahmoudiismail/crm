# Execution Plan: CRM Binary Production Fixes (Auth, Error Propagation, Date Propagation, Security)

## 1. Objective
Perform root-cause inspection and implement a plan to resolve four critical production issues in the `crm` binary.

## 2. Issues to Address

### Issue A: Silent Failure / Swallowed Errors on Report Failure
*   **Fix:** Propagate the error out of the async blocks inside `fetch_reports` instead of converting it to a `Value`. Change the future signature to return `Result<(String, Value)>` and use `try_collect::<Vec<_>>().await` so that if any report future fails, the entire fetch process bubbles up `Err(e)` to `run_once`, and `main()` exits with a non-zero exit code.

### Issue B: Cognito Token Expiration Buffer & 401 Cache Invalidation
*   **Fix (Buffer):** In `auth.rs`, update the expiry check to include a 5-minute buffer: `if expiry > Utc::now() + chrono::TimeDelta::try_minutes(5).unwrap_or_default()`.
*   **Fix (Cache Invalidation):** In `fetcher.rs`'s `get_valid_token_or_refresh`, forcefully clear `cfg.access_token`, `cfg.id_token`, and `cfg.access_token_expiry` before calling `ensure_authenticated` to enforce a fresh login on 401.

### Issue C: End-to-End Date Propagation (Enforce Local Timezone)
*   **Fix:** Ensure that date resolution for dynamic variables ("today", "yesterday") and defaults in `AppConfig::finalize_runtime_fields` strictly use `chrono::Local::now()`.

### Issue D: Auth Secret Leak in Debug Logs & Log Masking
*   **Fix:** Add a `mask_sensitive_json` helper function in `auth.rs` to clone payloads and recursively replace sensitive values with `***REDACTED***` before logging.

## 3. Required Deliverables
1.  **Modify `src/crm/fetcher.rs`:** Ensure futures return `Result<(String, Value)>` and use `try_collect`. Update `get_valid_token_or_refresh` to clear tokens.
2.  **Modify `src/crm/auth.rs`:** Update `ensure_authenticated` with a 5-minute buffer. Add `mask_sensitive_json` and apply it to logs.
3.  **Review/Modify `src/utils.rs` & `src/crm/config.rs`:** Ensure `chrono::Local::now()` is used correctly for date variables and defaults.
4.  **Add Regression Tests:** Add tests for token expiry buffer, 401 cache eviction, local timezone date resolution, and log redaction.
5.  **Update Documentation:** Update `md/ARCHITECTURE.md` and `md/AUTH_FLOW.md` (canonical source for auth logic) with the 5-minute buffer, 401 eviction policy, and log redaction mechanism.
