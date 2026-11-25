Yasweb Application (Downloads): Custom download paths are supported and evaluated in priority order: CLI argument (`--download-path`), per-report config (`ReportConfig.download_path`), global config (`YaswebConfig.download_path`), falling back to a local `downloads` directory relative to the executable. Missing directories are created automatically using `std::fs::create_dir_all`.

Yasweb Automation (Browser Waits & Loaders): When injecting JavaScript blocks (such as clicking the search button or exporting reports), the scripts must dynamically utilize `timeout_minutes` from `YaswebConfig` inside polling wait loops instead of hardcoded iterative timeouts. The polling loops dynamically wait up to `timeoutMinutes * 60 * 1000` via loops with `await sleep(...)` breaking instantly upon condition satisfaction. Furthermore, `#loader_svg` must be evaluated alongside `.loading-screen-wrapper, mat-progress-bar, .dx-loadpanel` to handle all possible loader variants on legacy UI elements in Step 09.

Yasweb Automation (Step Logging): JavaScript evaluations are refactored into distinct micro-steps (e.g., "Step 6 (Wait & Export)") mapping fine-grained step executions and logging back to the Rust host. This guarantees high observability inside browser interaction workflows.

## Yasweb Process Cleanup
* To prevent headless browser instances from persisting as zombie processes in the background (especially on Windows) when tasks timeout or fail, the `browser` process `Arc<Browser>` is explicitly dropped (`drop(browser);`) immediately after `run_browser_tab()` finishes execution in `apps/yasweb/src/main.rs`. This relies on `headless_chrome`'s internal Drop implementation which issues process termination commands (e.g. `.kill()`).
Yasweb Automation (Error Handling): JavaScript micro-step evaluations must return a valid JSON string with an explicit `status` of `SUCCESS` or `ERROR`. Invalid JSON, a missing/invalid status, and unsupported status values must return a Rust `Err`; `ERROR` must also return `Err` through `evaluate_automation_step` so browser logic cannot silently continue on a broken DOM state.


## Browser failure diagnostics

Yasweb writes its normal application log according to `log_stdout_level` and
`log_file_level` in `yasweb_config.json`. During browser automation, look for
the `YASWEB RUN`, `YASWEB STAGE`, and `AUTOMATION STEP` entries. Each stage
records its start/end, result, elapsed milliseconds, and the error chain when it
fails.

For production privacy, authenticated page HTML and iframe DOM snapshots are persisted locally only when `save_html_diagnostics: true` is explicitly set in `yasweb_config.json` (default `false`). The file log level is TRACE by default, but TRACE alone does not enable HTML persistence. Each browser-step snapshot attempts to capture the top-level document and the current document frames through CDP. Snapshot files are written through unique temporary paths and atomically renamed; frame URLs in logs have query strings and fragments redacted. Frame-capture failures are logged with the redacted frame URL and reason. Diagnostic HTML artifacts can contain credentials, session data, and customer information; keep `save_html_diagnostics` false during routine production runs and sanitize artifacts before sharing them. The option is independent of the log levels.

The diagnostic option can be enabled temporarily by setting `save_html_diagnostics` to `true` in `yasweb_config.json`; restore it to `false` after collecting the required evidence.

During `Step 4 (Fill Filters)`, TRACE logs include the visible filter-field inventory before and after filling (label, control type, current value, placeholder, accessible label, form-control name, and element ID). The inventory is included even when one or more requested filters fail. An explicitly supplied empty filter value clears only a matching `input[type="text"]`; other control types, including required OwlDateTime controls, are left unchanged. OwlDateTime values are inserted through Chrome CDP native text input and must be accepted by the Angular control before Search can run. A filter omitted from the request remains unspecified.

For Standard Reports, date formats are enforced as `DD-MMM-YYYY 00:00` for the start boundary and `DD-MMM-YYYY 23:59` for the end boundary (for example, `08-Nov-2026 00:00` and `08-Nov-2026 23:59`). Report Manager date formatting and behavior remain unchanged.

Set `keep_open` to `true` in `yasweb_config.json` when you need the visible
browser to remain open for up to 60 seconds after a failure. This keeps the
browser available for manual inspection without writing page contents to disk.

A failed menu lookup, MIS click, MIS Reports transition, report-type selection,
report search, filter application, or download wait must return an error rather
than being treated as a successful run with an empty filter list.


## Concurrent report tabs

Each report execution uses its own Chrome process and a unique user-data profile directory; the configured shared semaphore caps simultaneous browser operations (default 6, zero means unlimited). Profile names include report, date, process ID, and a per-invocation run ID. Tab creation is serialized and retried up to three times when Chrome fails to deliver the target-created event. This prevents concurrent report jobs from navigating or modifying the same first tab. Regression tests cover distinct profile paths and semaphore limits. A live Yasweb run is still required to confirm behavior against the deployed Angular application.

## Filesystem safety and async responsiveness

Yasweb sanitizes report-name and date-derived filename components before creating temporary download folders, Chrome profile folders, or final report filenames. Path separators, control characters, and Windows-invalid filename characters are replaced, and traversal sequences are neutralized. The temporary download directory is created with checked async filesystem I/O; final report moves/copies run in `spawn_blocking` so large files do not block Tokio's async executor.

When concurrent reports discover new filter names, their configuration updates are serialized within the Yasweb process. Each update reloads and validates the current JSON while holding the update lock, adds only missing filters, and writes atomically. If the config cannot be read or parsed, Yasweb logs a warning and preserves the existing file rather than replacing it with a stale in-memory copy. This lock coordinates report tasks in one Yasweb process; it is not a cross-process file lock.


## MIS SPA navigation and report-frame readiness

Clicking the MIS module starts an Angular client-side route transition; it does
not guarantee that the parent tab URL changes synchronously. The browser flow
must wait up to 30 seconds for `/home/mis-report/module/` before inspecting the
frame tree. Do not re-click MIS while this route wait is still active. Only
after the route is reached should the automation discover the current iframe,
create a fresh CDP execution context, and run the report-frame readiness check.
If the route does not appear within the initial bounded wait, attempt the
existing MIS navigation recovery once, then return a diagnostic error with the
last observed URL if the route is still absent. Once the route is reached,
iframe readiness waits
for the configured deadline without refreshing the parent tab; this avoids
restarting a healthy but slow iframe and prevents unnecessary beforeunload prompts.
See `md/yasweb-spa-race-condition.md` for the failure history and regression coverage.


## Runner filter argument compatibility

Runner builds external-app command lines from the arguments declared by the
Yasweb manifest. The manifest exposes dynamic `--filter-<key>` arguments so
Runner can render report-specific controls and cached dropdown options. The
Yasweb CLI accepts these dynamic flags at its process boundary and converts
them into the existing `--filters <JSON>` representation before Clap parsing.
The legacy `--filters` JSON argument remains supported and is merged with
dynamic flags; an explicitly supplied dynamic flag takes precedence for the
same key.

Filter option caches are aggregated across reports by filter key. Reports
without cached options do not erase options already known for another report.
The Runner form evaluates `depends_on` against every selected `--name`
checkbox, and optional dropdowns start with an empty selection. Do not add
report titles or placeholder labels to a filter's option list: options must be
actual values accepted by the corresponding MIS filter. If the live report
does not expose its option values in the available DOM snapshot, discover them
from the rendered filter control during a live run before editing the config.


## Report download recovery

If report navigation or a later SPA readiness step returns an error after export has started, Yasweb performs a short bounded recovery wait. The run is treated as recovered only when the current run creates or changes a spreadsheet file and that file remains stable with no temporary download in progress. Existing unchanged reports never satisfy recovery; when no new completed file appears, the original navigation/export error is returned.


## MIS report result readiness before export

After Search, automation waits for the rendered DevExtreme report grid to
contain rows or expose an explicit empty-results message. A transient zero-row
grid is not a completed report. Grid state, busy indicators, loaders, and the
enabled Export control must remain consistent for three consecutive polling
samples before export; state is checked again immediately before the click.
Timeout diagnostics include report name, visible row count, explicit-empty
state, grid busy state, loader state, and Export availability.

This DOM-readiness safeguard does not guarantee semantic completeness of
server-side report data. Validate the exported workbook against the source
report when investigating missing records. A live Yasweb run is required to
confirm the deployed report's exact empty-state markup.

Report-frame execution contexts are resolved again for every automation step.
The resolver reads all current parent-page iframe sources and matches them to
the current Chrome frame tree. It must not reuse a stale iframe reference or
select an arbitrary cross-origin child when several frames exist. If multiple
frames are present and none matches, the step fails with iframe-source and
frame-URL diagnostics; a single child frame may be used as a bounded fallback.

Completed spreadsheet downloads are also opened and structurally checked before
Yasweb reports success: XLSX/XLS workbooks must expose a readable worksheet,
and CSV exports must parse as records. This detects truncated or corrupt files;
it does not establish that every server-side matching record was exported.
