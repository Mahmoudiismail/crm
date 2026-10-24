# Audit Findings — Phases 4–8

## Scope
Reviewed current source and repository instructions, focusing on Yasweb filesystem handling and config persistence, Runner process lifecycle, CRM downloads, tests/benchmarks, CI, and affected documentation. Historical audit reports were treated as leads and checked against current code.

## Confirmed findings addressed

### Yasweb generated paths
Report names and CLI date strings were incorporated into generated temporary-directory, Chrome profile, and final output names without sanitizing path separators. Temporary directory creation errors were also ignored.

**Fix:** Sanitize generated filename components at their construction/finalization boundaries and propagate temporary-directory creation errors. The configured output directory remains user-configurable.

**Tests:** Added sanitization and filesystem tests to verify a traversal-shaped final filename remains inside the output directory.

### Blocking filesystem work in async report flow
Final report finalization performs directory enumeration, file moves/copies, metadata reads, and deletion. It was called directly from the async report flow.

**Fix:** Run finalization in `tokio::task::spawn_blocking`; create the temporary download directory with checked `tokio::fs` I/O. Browser automation waits already run in a blocking worker and were not rewritten unnecessarily.

### Concurrent config updates
Concurrent report futures could independently read, modify, and write the complete Yasweb config, losing each other's newly discovered filters. Read failures were converted to empty content, and parse failures could fall back to stale in-memory config.

**Fix:** Serialize the complete config read/validate/modify/write operation with a per-process async mutex. Read/parse failures now preserve the file and are logged as warnings; valid updates remain atomic.

**Tests:** Added concurrent update coverage and a malformed-config preservation test.

## Reviewed and retained
- Runner process handling already bounds captured output, drains pipes after the capture limit, uses `kill_on_drop(true)`, and supports timeouts. Its execution queue is bounded; no broad cancellation/queue rewrite was justified by the inspected paths.
- Yasweb browser automation and its polling/sleeps already run in `spawn_blocking`; final file finalization was the confirmed executor-blocking path.
- CRM signed-URL downloads already sanitize URL-derived filenames and use unique temporary files created with `create_new`.
- No public API expansion, new dependency, framework, or broad architecture refactor was introduced.

## Security follow-up requiring live compatibility validation
Yasweb's Chrome launch currently disables certificate checks and web security, disables site isolation, and disables the browser sandbox. These settings weaken browser security. They were not changed here because their operational purpose and compatibility with the deployed portal have not been established in this environment. Validate them against the real portal before deciding whether they can be removed; do not assume they are safe by default.

## Verification status
- Focused regression tests were added in `src/bin/yasweb.rs`.
- `md/YASWEB_DEV.md` was updated for generated path safety, async finalization, and concurrent config updates.
- Formatting, Clippy, tests, and live portal behavior have not been run in this environment. No passing result is claimed. Check the PR CI after the `format-approval` environment gate is approved.
- Review the complete diff and resolve CI failures before merging. Do not merge automatically.
