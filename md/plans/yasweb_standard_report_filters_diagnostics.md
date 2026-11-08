# Yasweb Standard Report Filters, Diagnostics, and Date Handling

## Scope
Implement the approved Yasweb repair on an isolated branch. Keep Report Manager behavior unchanged.

## Changes
1. Capture the report iframe document through the existing CDP frame-resource mechanism at each browser automation step, with collision-resistant filenames and explicit capture failures in logs. Keep capture gated by TRACE because authenticated report HTML may contain session and customer data.
2. Record visible report-form fields before and after filter application, including labels, input types, current values, placeholders, and relevant attributes. Emit this inventory even when an individual filter fails; distinguish missing field, unresolved input, write failure, and verification failure.
3. Resolve configured filters against actual visible labels/controls robustly, including date controls whose displayed labels differ from configured keys. Continue collecting diagnostics for remaining fields after a per-field failure.
4. Preserve three filter states: unspecified (existing behavior), non-empty value (fill), and explicitly empty value (clear the control).
5. For Standard Report only, enforce date formats `%d-%b-%Y 00:00` for start and `%d-%b-%Y 23:59` for end, overriding legacy custom formats on Standard Reports (e.g. `08-Nov-2026 00:00` and `08-Nov-2026 23:59`). Keep Report Manager date formats and runtime behavior unchanged.
6. Add focused regression tests for Standard Report date formats (including overriding legacy formats), unchanged Report Manager formats, and preservation of explicit formats for other report types. Validate filter clearing, date-label matching, and iframe capture in a live portal run.
7. Update Yasweb operational documentation to describe TRACE-gated authenticated iframe snapshots, field inventories, explicit clearing, and Standard Report date formatting.

## Constraints
- No changes to Report Manager behavior.
- No changes to Runner/Tasker.
- No new framework or unnecessary abstraction.
- Do not submit raw HTML or customer data to external services. Diagnostic HTML is written locally only when TRACE is enabled.
- No PR merge as part of this task.

## Verification
Run focused Yasweb tests and the Yasweb package check/build if the available environment permits. Report exact commands and results; do not claim live portal verification without a live run.
