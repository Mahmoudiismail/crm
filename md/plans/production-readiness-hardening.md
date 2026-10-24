# Production Readiness Hardening Plan

## Goal
Address the actionable production-readiness findings from the post-update repository audit with small, reviewable changes on one PR. Do not merge automatically.

## Scope
1. Harden Yasweb Chromium launch defaults by removing flags that disable certificate validation, web security, site isolation, and the browser sandbox.
2. Prevent authenticated page HTML from being written to routine logs or diagnostic files. Keep HTML snapshots and failure diagnostics disabled without an environment-variable opt-in.
3. Stop logging response header values, which can contain credentials or session material.
4. Make CRM secret persistence opt-in by default; retain explicit opt-in compatibility and ensure missing config values resolve to the safer default.
5. Fix the release workflow's build-history path to the consolidated `md/plans/` location.
6. Add focused regression tests for safer configuration defaults and the diagnostics opt-in gate.
7. Add `cargo audit` and `cargo deny check advisories` to pull-request CI.
8. Verify the integrity and expected contents of each release ZIP before publishing it.
9. Update operational documentation and manually review the complete diff.

## Explicitly not changed in this PR
- Do not change the existing update ZIP password because repository policy records that it was explicitly retained.
- Do not claim update-package authenticity is solved: there is no verified signing key/trust chain currently established in the repository. Document it as a production release blocker requiring a signing/verification design.
- Do not merge the PR. Keep all changes in this single PR.

## Validation
- Add/update unit tests for safer config defaults and the diagnostics opt-in gate; ensure network logging no longer emits header values.
- Rely on GitHub Actions for formatting, Clippy, the full test suite, dependency audit, and release workflow syntax; report only observed results.
- Manually review the complete diff and ensure docs match the runtime behavior.
- Live compatibility testing of Yasweb against the actual portal is required before removing the security exceptions can be considered fully validated for production.
