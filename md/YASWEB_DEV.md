Yasweb Application (Downloads): Custom download paths are supported and evaluated in priority order: CLI argument (`--download-path`), per-report config (`ReportConfig.download_path`), global config (`YaswebConfig.download_path`), falling back to a local `downloads` directory relative to the executable. Missing directories are created automatically using `std::fs::create_dir_all`.

Yasweb Automation (Browser Waits & Loaders): When injecting JavaScript blocks (such as clicking the search button or exporting reports), the scripts must dynamically utilize `timeout_minutes` from `YaswebConfig` inside polling wait loops instead of hardcoded iterative timeouts. The polling loops dynamically wait up to `timeoutMinutes * 60 * 1000` via loops with `await sleep(...)` breaking instantly upon condition satisfaction. Furthermore, `#loader_svg` must be evaluated alongside `.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel` to handle all possible loader variants on legacy UI elements in Step 09.

Yasweb Automation (Step Logging): JavaScript evaluations are refactored into distinct micro-steps (e.g., "Step 6 (Wait & Export)") mapping fine-grained step executions and logging back to the Rust host. This guarantees high observability inside browser interaction workflows.

## Yasweb Process Cleanup
* To prevent headless browser instances from persisting as zombie processes in the background (especially on Windows) when tasks timeout or fail, the `browser` process `Arc<Browser>` is explicitly dropped (`drop(browser);`) immediately after `run_browser_tab()` finishes execution in `src/bin/yasweb.rs`. This relies on `headless_chrome`'s internal Drop implementation which issues process termination commands (e.g. `.kill()`).
Yasweb Automation (Error Handling): JavaScript micro-step evaluations must return a valid JSON string with an explicit `status` of `SUCCESS` or `ERROR`. Invalid JSON, a missing/invalid status, and unsupported status values must return a Rust `Err`; `ERROR` must also return `Err` through `evaluate_automation_step` so browser logic cannot silently continue on a broken DOM state.


## Browser failure diagnostics

Yasweb writes its normal application log according to `log_stdout_level` and
`log_file_level` in `yasweb_config.json`. During browser automation, look for
the `YASWEB RUN`, `YASWEB STAGE`, and `AUTOMATION STEP` entries. Each stage
records its start/end, result, elapsed milliseconds, and the error chain when it
fails.

On a browser automation failure, Yasweb also writes a redacted diagnostic JSON
file under `debug_html/` next to the executable. The file includes the current
URL without query/fragment, page title/readiness, iframe metadata, and a DOM
snapshot with form values removed. Existing numbered `step_*.html` snapshots
are still available for the automation checkpoints; treat those HTML files as
potentially sensitive because they can contain application data.

Set `keep_open` to `true` in `yasweb_config.json` when you need the visible
browser to remain open for up to 60 seconds after a failure. With
`keep_open: false`, the browser closes after the failure, but diagnostic files
are still saved. Never share logs or HTML snapshots publicly without reviewing
them for usernames, patient/customer data, and other sensitive information.

A failed menu lookup, MIS click, MIS Reports transition, report-type selection,
report search, filter application, or download wait must return an error rather
than being treated as a successful run with an empty filter list.


## Concurrent report tabs

Each report execution uses its own Chrome tab. Tab creation is serialized and retried up to three times when Chrome fails to deliver the target-created event. This prevents concurrent report jobs from navigating or modifying the same first tab. A live Yasweb run is still required to confirm behavior against the deployed Angular application.

## Filesystem safety and async responsiveness

Yasweb sanitizes report-name and date-derived filename components before creating temporary download folders, Chrome profile folders, or final report filenames. Path separators, control characters, and Windows-invalid filename characters are replaced, and traversal sequences are neutralized. The temporary download directory is created with checked async filesystem I/O; final report moves/copies run in `spawn_blocking` so large files do not block Tokio's async executor.

When concurrent reports discover new filter names, their configuration updates are serialized within the Yasweb process. Each update reloads and validates the current JSON while holding the update lock, adds only missing filters, and writes atomically. If the config cannot be read or parsed, Yasweb logs a warning and preserves the existing file rather than replacing it with a stale in-memory copy. This lock coordinates report tasks in one Yasweb process; it is not a cross-process file lock.
