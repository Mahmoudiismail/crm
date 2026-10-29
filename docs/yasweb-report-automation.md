# Yasweb Report Automation — Technical Reference

> Purpose: this document is the first-stop troubleshooting and maintenance reference for the Rust Yasweb automation. It records selectors and behaviors found in the code and the supplied failure evidence. Do not assume a selector is still valid just because it is documented: confirm it against a fresh TRACE snapshot when the portal changes.

## 1. Source of truth

| Area | Implementation |
|---|---|
| Browser orchestration and download wait | `src/yasweb/browser/mod.rs` |
| Login and account verification | `src/yasweb/browser/login.rs` |
| Navigation, report selection, filters, export | `src/yasweb/browser/reports.rs` |
| JavaScript step evaluation and JSON status parsing | `src/yasweb/browser/javascript.rs` |
| HTML snapshots and failure diagnostics | `src/yasweb/browser/debug.rs` |
| Download directory, file snapshot and completion | `src/yasweb/browser/download.rs` |

The report pipeline is sequential. A stage must return `status=SUCCESS` before the next stage runs. A successful JavaScript `click()` is not itself proof that the portal changed state; use the stage's postcondition.

## 2. Origin and frame rules (critical)

### Observed portal layout

The supplied failure HTML/logs showed:

- Parent SPA route: `https://yasweb.fakeeh.care:8030/home/mis-report/module/H5EB133C7D86D035BE06364BB010A801F`
- Embedded frame source: `https://yasweb.fakeeh.care:8065/auto-login`
- Parent and frame share a hostname but use different ports. Different ports mean different origins.
- The browser therefore blocks parent-page JavaScript from reading `iframe.contentWindow.document` under the Same-Origin Policy.

### Required execution model

- Use the parent page context only for the SPA navigation/menu and for reading the iframe element's `src`.
- Use Chrome DevTools Protocol (CDP) `Page.getFrameTree` to locate the report frame and `Page.createIsolatedWorld` to obtain an execution context for that frame.
- Run report readiness, selection, filtering, and export scripts through `Runtime.evaluate` in that frame context.
- Keep `grant_univeral_access=false`. Do not disable web security, bypass site isolation, or use parent-context `iframe.contentWindow.document`.
- The report frame can redirect after `/auto-login`; select it from the current CDP frame tree and current iframe source rather than assuming its URL never changes.
- If no frame/context can be found, fail with a clear stage error. Do not continue with the parent document and report false success.

Relevant implementation: `evaluate_report_frame_step` and `collect_child_frames` in `reports.rs`; `evaluate_automation_step_in_context` in `javascript.rs`.

## 3. End-to-end stage sequence

| Order | Stage/log name | Preconditions | Success condition | Failure response |
|---:|---|---|---|---|
| 1 | `login` | Login page loaded | Username/password entry, login submit, dashboard account ID verified | Stop; do not navigate to reports |
| 2 | `menu_button_click` / MIS menu | Dashboard authenticated | Navigation menu opens and MIS module is visible | Retry menu opening up to 3 attempts, then fail |
| 3 | `click_mis_module` | MIS menu open | MIS module clicked | Fail |
| 4 | `Wait for MIS Reports` | Parent route is `/home/mis-report/module/` | Report frame context contains visible type controls and Search; no visible loader | If route is absent, retry navigation once; if route exists, retry readiness without clicking navigation again |
| 5 | `Step 1 (Report Type)` | Report frame ready | Requested report type control is found and clicked | Fail if not found |
| 6 | `Step 2 (Select Report)` | Type selected | Report list appears and requested report entry is clicked | Fail on list/report timeout |
| 7 | `Step 3 (Binding)` | Report selected | Expected report name appears in a bound `mat-select` or a bold/semibold header | Fail on binding timeout |
| 8 | `Step 4 (Fill Filters)` | Report form bound | Every non-empty requested filter was found and applied | Fail if any requested filter cannot be applied; do not run Search |
| 9 | `Step 5 (Search Click)` | Filters applied | Search button found and clicked | Fail if Search is not found |
| 10 | `Step 6 (Wait & Export)` | Search submitted | Export control is visible and enabled after loader disappears; transient state rechecked before click | Fail if Export is missing/disabled until timeout |
| 11 | `Step 7 (XLSX)` | Export menu opened | XLSX option found and clicked | Fail if XLSX option is not found |
| 12 | `wait_for_download` | Export requested; download directory configured | A new/changed spreadsheet file is stable for 3 polls | Fail on timeout; an old report must not satisfy the wait |

The outer timeout comes from `config.timeout_minutes`. MIS UI readiness currently uses 30 seconds and the CDP frame-context discovery retries for up to 10 seconds. Avoid adding a retry that repeats a click unless the observed state proves the click did not take effect.

## 4. Selector inventory

Selectors below are from the current implementation. Angular/DevExtreme markup can change; inspect the current frame before replacing selectors.

### 4.1 Login — `login.rs`

| Selector | Purpose / expected state |
|---|---|
| `input[formcontrolname='username'], #mat-input-0` | Username input |
| `input[formcontrolname='password'], #passFocus` | Password input when a password is configured |
| `button#submitFocus, button.pmry` | Submit login |
| `.alert-danger.fade.show` | Visible login failure alert; its text is included in the error |
| `span.usr-id` | Logged-in account ID used to verify the configured username |

Numeric portal IDs may be zero-padded. `username_matches` trims whitespace and compares numeric IDs after removing leading zeroes; non-numeric usernames compare case-insensitively. An account mismatch is a hard failure.

### 4.2 Parent navigation — `reports.rs`

| Selector / route | Purpose / expected state |
|---|---|
| `#menuPinnedBtn:not(.d-none), #pinButton:not(.d-none)` | Visible menu button |
| `.menuModules` with class `show-modules` | Navigation menu is open |
| `.misManagement` | MIS module entry in the menu |
| `/home/mis-report/module/` | Parent SPA route pattern indicating MIS module navigation reached |
| `iframe` | Embedded report frame element; read `src` only from parent context |

A changed URL alone is not proof that the report UI is ready. Conversely, if this route has been reached, do not click the MIS module again just because the embedded frame is still loading.

### 4.3 Frame readiness and report type — `reports.rs`

| Selector / text | Purpose |
|---|---|
| `mat-radio-button` | Common report-type radio control |
| `[role="radio"]` | ARIA radio fallback used by readiness inspection |
| `input[type="radio"]` | Native radio fallback used by readiness inspection |
| `label` | Report-type fallback; selection logic also searches labels |
| `button` | Readiness fallback when the control text is rendered on a button |
| `input[formcontrolname="searchInput"]` | Angular report search input |
| `input[placeholder="Search"]` | Search placeholder fallback |
| `input[aria-label*="Search" i]` | Accessible-name fallback used during readiness |
| `#loader_svg` | Known loader selector |
| `.loading-screen-wrapper` | Known loader selector |
| `mat-progress-bar` | Known loader selector |
| `.dx-loadpanel` | DevExtreme loading panel |

Readiness requires both visible type labels (`Standard Report` and `Report Manager`) plus a visible Search input, and no visible known loader. Text is normalized by trimming, lowercasing, and collapsing whitespace. A missing control is not treated as a successful selection.

### 4.4 Report list and selection — `reports.rs`

| Selector | Purpose |
|---|---|
| `.tree-view` | Report list/tree container; must exist before searching |
| `.tree-view span` | Candidate report names |
| `.sub-list-items` | Preferred clickable parent of a matching report span |
| `input[formcontrolname="searchInput"], input[placeholder="Search"]` | Search field for the report list |

The current search logic prefers an exact normalized text match, then permits a partial match. If multiple report names share a substring, verify the chosen entry from the logs/snapshot before changing this matching rule. After typing, the script dispatches `input`, `change`, and Enter key events.

### 4.5 Binding and filters — `reports.rs`

| Selector / attribute | Purpose |
|---|---|
| `mat-select` | Evidence that the selected report form is bound; also a filter control type |
| `.fw-semibold, .fw-bold` | Alternative report-name/header binding evidence |
| `mat-label` | Filter labels discovered from the form |
| `label[for]` (located through `mat-label.closest('label')`) | Associates a visible label with its input ID |
| `input` | Text/date filter input selected by the associated ID |
| `MAT-SELECT` | Dropdown filter selected by the associated ID |
| `mat-option` | Dropdown choices |
| `.cdk-overlay-container` | Angular Material overlay fallback for options |
| `.cdk-overlay-backdrop` | Clicked to close a dropdown when the requested option is missing |

Filter keys are matched to `mat-label` text after lowercasing and replacing underscores with spaces. Empty filter values are skipped. Text inputs are filled by dispatching `input`, `change`, and `blur` events. For keys containing `date`, the current code pads the first date segment to two characters and preserves the segment order; do not assume it converts date formats without checking the caller's input and portal expectations.

A requested filter that cannot be located or whose dropdown option is missing must fail Step 4. Do not allow the pipeline to click Search with an incomplete filter set.

### 4.6 Search and report execution — `reports.rs`

| Selector | Purpose |
|---|---|
| `button[mattooltip="Search"]` | Primary Search button |
| `i.bi-search` | Search icon fallback; click its closest button |
| `#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel` | Known loaders checked before execution/export |

Step 5 confirms the Search control was found and clicked. Step 6 separately waits for Export to become enabled; a missing loader alone is not proof that report generation finished.

### 4.7 Export — `reports.rs`

| Selector / value | Purpose |
|---|---|
| `div[aria-label="Export"], button[aria-label="Export"]` | Accessible Export control |
| `.dx-button-text` with exact text `Export` | DevExtreme Export text; closest `.dx-button`, `[role="button"]`, or `button` is used |
| `button, div[role="button"], .dx-button` with exact text `Export` | Fallback export candidates |
| `.dx-list-item-content` with exact text `XLSX` | Preferred XLSX menu entry |
| `span, div` with exact text `XLSX` | Fallback XLSX entry |

Export is considered enabled only if the control is visible and none of these conditions is true: a disabled class (`disabled`, `disable`, `dx-state-disabled`, `mat-button-disabled`), a `disabled` attribute/property, `aria-disabled="true"`, or CSS `pointer-events: none`. After finding an enabled button, Step 6 waits 500 ms and checks it again before clicking.

### 4.8 Download completion — `download.rs`

There are no DOM selectors in the download completion check. The code snapshots existing files before export, then accepts a new or changed `.xlsx`, `.xls`, or `.csv` file only after its size/modified-time tuple remains stable for 3 consecutive polls. Poll interval is 200 ms. A `.crdownload` or `.tmp` file marks the download as incomplete. Existing stale reports are deliberately excluded.

## 5. Stage logs and diagnosis

Search logs for the first `YASWEB STAGE FAILED` or `AUTOMATION STEP END: ... status=ERROR`; later outer errors may only wrap the original failure.

| Symptom/log | Interpretation | Next inspection |
|---|---|---|
| `MIS module ... not found` | Menu did not expose the MIS item | Confirm menu button visibility, `.menuModules.show-modules`, and `.misManagement` in parent snapshot |
| `route was not reached` | SPA navigation did not reach the MIS route | Check parent URL and menu click result; retry navigation once only |
| `Could not locate embedded Yasweb report frame` | CDP frame tree did not expose a matching child frame | Compare current parent `iframe.src` with CDP frame URLs/origins; inspect frame navigation/network failure |
| `Blocked a frame with an origin ...` | A script tried to access cross-origin iframe DOM from parent context | Treat as a regression: all report steps must run in CDP's report-frame execution context |
| `Report frame did not expose visible ... controls` | Frame exists but report UI is not ready or markup changed | Inspect `frameUrl`, `documentReadyState`, `reportTypes`, `searchVisible`, and `visibleLoaders` in readiness diagnostics |
| `Report type not found` | Report-type text/control changed or wrong frame selected | Check exact visible text and the control's current tag/ARIA attributes |
| `Report list timeout` / `Report name not found` | List did not load, search didn't filter, or name matching changed | Check `.tree-view`, search input, and rendered report spans |
| `Binding timeout` | Report selection did not bind the expected form | Check selected report identity, `mat-select`, and `.fw-semibold/.fw-bold` text |
| `One or more requested filters could not be applied` | A configured non-empty filter was not filled or dropdown option missing | Inspect failed filter keys, matching `mat-label`, associated `for` ID, and available `mat-option` values |
| `Search button not found` | Search selector/tooltip/icon changed or the form isn't ready | Inspect visible buttons and `i.bi-search` inside the report frame |
| `Export button did not become enabled` | Report may still be running, failed, or Export markup/state changed | Inspect loader visibility, export candidate class, disabled properties and ARIA state |
| `XLSX` option not found | Export menu didn't open or its markup/text changed | Inspect `.dx-list-item-content` and exact visible `XLSX` text |
| `Timed out waiting for a new or updated report download` | Browser didn't download a changed stable spreadsheet | Check configured directory, download behavior, partial files, and whether Step 7 clicked XLSX |

Network logs are emitted by `client::enable_network_logging`; request URLs are stripped of query strings and fragments. When diagnosing the portal, correlate the first failed stage with request/response timing instead of increasing timeouts without evidence.

## 6. HTML snapshots and sensitive data

- `debug::save_html_state` and `debug::save_failure_diagnostics` persist HTML only when TRACE logging is enabled.
- Files are saved under `debug_html/` next to the executable. Filenames include step number/name and timestamp; unsafe filename characters are sanitized.
- HTML snapshots currently capture the top-level tab content. They do **not** automatically serialize the cross-origin report frame's DOM.
- Authenticated HTML can contain user/session/customer data. Keep TRACE disabled during routine production runs; sanitize snapshots before sharing them.
- On a frame-specific failure, prefer the structured frame-readiness logs first. If a frame snapshot is later added, it must use CDP frame context and follow the same TRACE-only privacy gate.

## 7. Change checklist

Before changing selectors or timing:

1. Identify the first failed stage from logs.
2. Confirm the parent route and report-frame URL separately.
3. Inspect the report frame in its CDP execution context; never test cross-origin selectors from the parent page.
4. Confirm selector visibility and exact rendered text in a fresh snapshot.
5. Change only the smallest selector/state check needed.
6. Add or update a focused regression test for the failure contract (for example, readiness script contains no parent-frame DOM access, or a failed requested filter returns `ERROR`).
7. Run `cargo fmt --check`, Clippy, and the relevant tests through CI.
8. Test the actual Yasweb portal on a representative Windows workstation before treating portal-specific behavior as verified.

## 8. Known limits and cautions

- The selectors describe the current observed portal implementation, not a guaranteed external API.
- A partial report-name match can select a similarly named item; prefer exact match and verify ambiguous cases.
- Filter labels are currently matched by normalized visible text. If the portal changes its label/ID structure, do not silently skip a configured filter.
- Search click success is not equivalent to successful report generation; export readiness and final download are separate gates.
- The current download check validates new/changed stable spreadsheet file presence, not the semantic correctness of report contents.
