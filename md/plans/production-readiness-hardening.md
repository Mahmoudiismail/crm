# Production Readiness Hardening Plan

## Goal
Address the actionable production-readiness findings from the post-update repository audit with small, reviewable changes on one PR. Do not merge automatically.

## Scope
1. Harden Yasweb Chromium launch defaults by removing flags that disable certificate validation, web security, site isolation, and the browser sandbox.
2. Prevent authenticated page HTML from being written to routine logs or diagnostic files by default. Keep sensitive diagnostic capture available only through an explicit environment opt-in.
3. Stop logging response header values, which can contain credentials or session material.
4. Make CRM secret persistence opt-in by default; retain explicit opt-in compatibility and ensure missing config values resolve to the safer default.
5. Fix the release workflow's build-history path to the consolidated `md/plans/` location.
6. Update operational documentation and add focused regression tests where behavior can be tested locally in code.

## Explicitly not changed in this PR
- Do not change the existing update ZIP password because repository policy records that it was explicitly retained.
- Do not claim update-package authenticity is solved: there is no verified signing key/trust chain currently established in the repository. Document it as a production release blocker requiring a signing/verification design.
- Do not merge the PR. Keep all changes in this single PR.

## Validation
- Add/update unit tests for safer config defaults and safe URL/header logging behavior where feasible.
- Rely on GitHub Actions for formatting, Clippy, and the full test suite; report only observed results.
- Manually review the complete diff and ensure docs match the runtime behavior.
- Live compatibility testing of Yasweb against the actual portal is required before removing the security exceptions can be considered fully validated for production.
