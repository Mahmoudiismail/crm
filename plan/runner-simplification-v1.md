# Runner Simplification — V1 Plan

## Status
Implementation authorized by the user with explicit `START`. Scope remains Runner-only and limited to the approved plan.

## Baseline
- Repository: `Mahmoudiismail/crm`
- Starting commit: `907581a5233b58005668fc5c2fc18651cea46d60`
- Branch: `runner-simplification-plan-oct-2026`
- Scope: Runner application only.
- Guiding rule: smallest correct change; no new framework, no broad architectural rewrite, and no file splitting just to reduce line counts.

## Goals
1. Remove confirmed duplication in scheduling/preview code without changing intended behavior.
2. Make custom/open-ended preview behavior explicit and correct.
3. Reduce repeated working-hours parsing/weekday matching only where a small shared helper is clearly simpler.
4. Avoid unrelated refactors in the scheduler, task dispatcher, pipeline, templates, and JavaScript.

## Proposed scope

### Phase 1 — Scheduling preview correctness and clarity
File: `src/runner/config/schedule.rs`

- Done: removed the three identical consecutive `effective_period_end_utc` definitions and redundant `#[allow(unused_variables)]` attributes, retaining one definition.
- Done: the early-continue guard now compares against `effective_period_end_utc`, so an open-ended Custom period uses its five-year preview horizon. Added a focused regression test.
- Done: corrected the Windows command in `test_real_duplicate_admission_race` to pass its PowerShell script through `-EncodedCommand` (UTF-16LE Base64), avoiding quote/argument loss in the shell command parser. The marker read now retries briefly after the existing race window and reports the underlying I/O error. CI verification is still required.
- Review the 1,000-step guard for interval preview. Preserve it unless evidence demonstrates incorrect user-visible truncation; if behavior is intentionally bounded, document or test that boundary rather than removing the guard speculatively.
- Keep schedule calculations in Rust and keep the GUI preview routed through the existing Rust endpoint.

### Phase 2 — Minimal duplication reduction
Files: `src/runner/config/schedule.rs`, `src/runner/gui/forms.rs`

- Consider a small shared helper for weekday-range matching used by `is_within_working_hours` and `is_working_day`, preserving their different time/day semantics.
- Consider one small helper for parsing inline working-hours definitions in `parse_schedules_text`, where the same `day=start-end` parsing repeats across schedule variants.
- Do not redesign the schedule representation or add a generic parsing framework.
- Preserve existing configuration compatibility, working-hours profile behavior, and accepted schedule syntax.

### Explicitly out of scope for this V1
- Splitting `templates.rs` or `forms.js` into multiple files.
- Rewriting `task_commands.rs`, `pipeline.rs`, or process/shell execution.
- Dependency changes, public API expansion, new architecture layers, broad formatting-only edits, or unrelated fixes.
- Changes to other applications.

These may be reconsidered after this small change is reviewed and its actual benefit is clear.

## Verification plan (after explicit START only)
1. Inspect existing tests and add focused regression coverage for any behavior changed, especially:
   - open-ended Custom period preview after the current day's end;
   - preview upper-bound behavior and result limit;
   - weekday ranges and working-hours matching if the shared helper is changed;
   - schedule parsing for inline working-hours data if parsing is refactored.
2. Run `cargo fmt -- --check`.
3. Run focused Runner tests, then `cargo test`.
4. Run Clippy with warnings denied, using the repository's documented command/configuration.
5. Inspect the final diff and confirm no unrelated application files changed.
6. Done: updated `md/RUNNER.md` to explain the open-ended preview horizon. Review the final diff and documentation together.
7. Report exact commands, results, and any limitations. The current execution environment cannot reach GitHub or run the repository's Cargo toolchain against this branch; do not claim test results until a CI run or local run confirms them.

## Review workflow
The repository's `AGENTS.md` now makes INTERNAL Code Review mandatory when Jules or Claude is used. This work is being performed directly, not through Jules or Claude, so use careful manual review before tests and again on the final diff.

## Acceptance criteria
- Only confirmed, in-scope simplifications are implemented.
- Intended preview/scheduling behavior and configuration compatibility are preserved.
- Focused regression tests cover changed behavior.
- Required documentation and review gates are satisfied.
- No implementation starts until the user explicitly replies `START`.
