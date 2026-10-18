# Yasweb and CRM Fetcher Report Reliability

## Scope

Address the report failures evidenced by the supplied Runner and Yasweb logs. Exclude the external `Sat Booking.vbs` Post-Run failure because that flow is moving into Tasker. Keep changes focused on V1 production correctness.

## Changes

1. Give each Yasweb report execution an isolated Chrome tab. Serialize tab creation and retry transient failures up to three times; preserve existing network-log redaction.
2. Verify recursive inclusive date splitting by comparing the complete ordered date sequence against the original range, including odd/even spans, leap-day boundaries, and a multi-month range.
3. Replace date-ranged report suppression based on the presence of any recent CSV with a completion marker for the exact requested range. The marker lists files from the successful run and is invalid if any listed file is missing. A failed fetch/download pipeline cannot write a completion marker.
4. Preserve rejection of inverted date ranges. The logs do not include the task's original date expressions, so the source of the 2026-10-18 to 2026-10-17 range cannot be safely fixed from logs alone; do not silently swap the dates.
5. Update `md/FETCHER.md`, `md/CONFIG.md`, and `md/YASWEB_DEV.md` to reflect actual behavior.

## Validation required

- Run `cargo fmt --check`.
- Run focused tests for report completion markers and recursive date splitting, then the relevant crate test suite and Clippy.
- CI and a live Yasweb run are still required to verify browser behavior against the deployed application.

## Exclusions

- No changes to the external `Sat Booking.vbs` Post-Run flow.
- No broad architecture changes, new generic retry framework, or unrelated cleanup.
- Follow the repository workflow: use an existing open PR if one exists; otherwise create one PR for this change set.
