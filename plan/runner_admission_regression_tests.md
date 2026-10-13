# Runner Admission and Regression Test Hardening

## Goal
Close the remaining V1 verification gaps identified after PRs #397 and #398 were merged, using the smallest changes consistent with the existing Runner design.

## Scope
1. **Duplicate task admission:** make admission authoritative at the execution manager boundary so concurrent callers cannot admit the same task while it is queued or running. Preserve legitimate subsequent reruns after completion and manual-run semantics.
2. **Behavioral race regression:** replace or strengthen the timing-sensitive race test so it verifies actual execution count under concurrent admission, rather than inferring correctness from a short-lived queue/status snapshot.
3. **TaskLogger retry regression:** exercise a real initialization failure, then make the log path writable and verify a later log call successfully initializes and writes the message. Keep test setup isolated and cross-platform.
4. **Corrupt configuration startup regression:** add a focused process-level test proving an existing malformed runner_config.json exits unsuccessfully and is not silently replaced with defaults, if the existing binary/test structure allows deterministic isolation without introducing a test framework.
5. **Documentation:** update the relevant Runner documentation under md/ to describe the admission invariant and focused verification coverage.
6. **Plan:** commit this plan with the code and documentation changes.

## Constraints
- Keep this a focused V1 fix: no new dependencies, E2E framework, architecture refactor, or unrelated cleanup.
- Preserve current CLI behavior, scheduler policies, and valid sequential reruns.
- Avoid timing-only assertions where observable behavior can be tested.
- Do not introduce blocking filesystem I/O in async production paths.
- Do not use git reset.
- Read root AGENTS.md, applicable nested AGENTS.md, and md/AI_DOC_POLICY.md; follow their review and verification requirements.

## Verification
- Mandatory INTERNAL Code Review before running tests, then fix valid findings.
- Run cargo fmt -- --check, focused Runner tests, cargo clippy --all-targets --all-features -- -D warnings, and cargo test --all-targets --all-features where the available runtime supports them.
- Perform the mandatory FINAL INTERNAL Code Review after fixes and rerun affected checks.
- Report only checks actually executed; explicitly state any unavailable required review capability or platform limitation.

## Out of scope
- Performance benchmarking or unrelated optimization.
- Changes to release workflow or the already-merged PRs #397/#398.
- Broad test architecture changes or adding dependencies.
