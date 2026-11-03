# Execution Plan: Yasweb Standard Report Dates and Empty Text Values

## Objective

Implement the two approved configuration behaviors in the existing Runner serialization PR:

1. Use `%-d-%-m-%Y` as the default start/end date format for Yasweb `Standard Report` entries.
2. Allow editable optional text/filter values in Yasweb and Runner external-app manifest forms to be deliberately cleared and persisted as an empty string, without silently restoring a manifest/config default on reload.

## Scope and invariants

- Yasweb `Report Manager` keeps its existing `%d-%b-%Y` defaults.
- Yasweb `Standard Report` receives `%-d-%-m-%Y` for both start and end date keys when the configured format is absent/empty.
- Existing non-empty custom date formats remain unchanged. Other report types retain their current date/time defaults.
- An explicitly submitted empty filter value remains present as `key: ""` in `yasweb_config.json`; it is not treated as an omitted update.
- Runner external-app manifest string arguments preserve an explicit empty value in the serialized `args` object. On subsequent form render, the existing empty key takes precedence over `default_value`.
- Fields hidden by unsatisfied `depends_on` conditions and period-mapped date arguments remain excluded as required by the existing PR #468 change.
- Structural fields that must identify a valid record (for example Task ID / profile ID) and invalid empty shell commands are not made optional by this change.
- No change to public CLI arguments, config schema, or default behavior beyond the requested report date format and deliberate clearing of optional values.

## Implementation

### Yasweb
- Update `YaswebConfig::finalize_runtime_fields` to select the Standard Report default format explicitly while preserving Report Manager and fallback behavior.
- Add unit tests proving Standard Report start/end defaults, Report Manager defaults, preservation of explicit custom formats, and idempotent finalization.
- Verify the CLI/config update path keeps a submitted filter key whose value is an empty string and persists it through JSON serialization/reload.
- Update `md/CONFIG.md` with the report-type-specific defaults and the empty-filter persistence behavior.

### Runner
- Preserve the existing PR #468 serialization logic for hidden/conditional and mapped date arguments.
- Add a focused regression test proving an available optional string manifest argument with an explicit empty value is serialized as `""` (and that a supplied default is not substituted once the empty key is present).
- Verify the form's existing `currentArgs[arg.name] !== undefined` precedence continues to distinguish an explicitly empty string from a missing key.
- Update `md/RUNNER.md` to document clearing optional manifest text/filter values and the fact that an explicit empty string is persisted.
- Keep required structural form validation unchanged.

## Verification

- Run the focused Node test through the existing GitHub Actions Test workflow.
- Run the affected Yasweb package tests, Runner package tests, formatting, and Clippy through GitHub Actions only.
- Review the complete PR diff manually, including the existing #468 behavior, and verify that all original requirements remain covered.
- Do not merge the PR without a separate explicit merge request.
