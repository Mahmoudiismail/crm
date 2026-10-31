# Repository Reliability and Performance Remediation — Progress

Branch: `fix/repository-reliability-optimization`
Base: `86806fbd07f61b794d1a819c24f165c501a1792f`

## Applied changes

### Yasweb
- Replaced process-local-only serialization for discovered-filter persistence with an OS file lock shared by independent processes.
- Applied the same inter-process lock and a fresh read-modify-write merge to CLI-driven report configuration updates, so discovered filters are not overwritten by a stale startup snapshot.
- Changed manifest filter-option handling: a dropdown is emitted only when all reports sharing that filter have the same non-empty option set; conflicting or missing option sets fall back to a string argument.
- Moved the finalization lock to a stable temp-directory path derived from the canonical output directory. The lock file is intentionally not unlinked.
- Added/updated regression coverage for concurrent filter persistence, conflicting manifest options, invalid config preservation, and absence of lock artifacts in report output directories.

### Runner
- Monthly schedule normalization now preserves a valid stored occurrence (including due/overdue timestamps) and initializes only a missing occurrence.
- The local GUI HTTP parser rejects cross-origin POST requests when an Origin header is present. It accepts same-origin loopback requests and clients that omit Origin, while requiring an allowed loopback Host and valid port when Origin is supplied.
- Scheduler polling now loads the Runner config once per poll and reuses it for due-task dispatch and daily log cleanup.
- Added regression coverage for monthly schedule normalization and GUI POST Origin validation.

### Tasker
- Department Split resolves its configured workbook/dashboard/output paths relative to the executable directory, consistent with other Tasker jobs.
- Sat Figures validates total/booked slot arithmetic and rejects negative totals, negative bookings, zero totals, and bookings exceeding capacity.
- Tasker config validation rejects negative `minutes_ago` values in CSV Analysis, Dashboard Updater, and CRM Open Sohail configs.
- Leads report lookback arithmetic now uses checked conversion and multiplication, rejecting negative or overflowing values.
- CSV ticket diagnostics retain record numbers and parser errors but no longer log raw ticket row contents.
- Team mapping trace logs no longer include To/CC recipient addresses.

## Deferred / not implemented in this change set
- Tasker users-export and leads-report streaming rewrites were not applied: preserving the existing concatenation, BOM, lossy-decoding, delimiter, and diagnostic semantics needs a dedicated behavior-preserving implementation and test coverage. Avoid claiming a performance gain for those paths.
- Runner manifest-discovery subprocess reuse was not applied; it requires tracing the external-app execution call graph and proving cache lifetime/invalidation.
- CRM Updater package authenticity remains a release-security decision. A checksum is not authenticity unless the expected checksum is anchored in a trusted source. Signing/key management is intentionally not invented in this PR.

## Verification status
- Changes are committed to the branch and focused regression tests were added.
- Full `cargo fmt`, `cargo test`, Clippy, Windows CI, and runtime validation have not yet been executed in this environment. Do not treat this progress document as a passing test report.
- GUI CSRF protection is limited to Origin-bearing POST requests; non-browser clients without Origin remain supported. Review this compatibility boundary before release.
