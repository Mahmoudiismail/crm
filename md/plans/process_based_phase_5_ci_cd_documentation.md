# Process-Based Architecture — Phase 5: CI/CD and Documentation

## Objective
Complete the process-based rollout by aligning package selection across CI, release packaging, and CRM Updater while preserving executable names and legacy Runner compatibility.

## Scope
1. Keep each executable independently buildable with its Cargo package.
2. Make CI select affected app packages for package-local Clippy, tests, and release builds. Changes to shared root code, workspace manifests/lockfile, or anything under `.github/` fan out to all packages. Unknown app/root code paths fail closed to all packages.
3. Extract package classification into a small Python script and add unit tests for app-only, multi-app, shared, unknown, and documentation-only path sets. Run these tests in the detect-packages job before emitting the matrix.
4. Preserve manual release selection for crm, runner, yasweb, tasker, crm_updater, comma-separated selections, or all.
5. Make CRM Updater apply only configured executable files present in the update package or staging directory. Reject unknown files and ambiguous mappings before stopping processes. Omitted applications must not be stopped or restarted.
6. Preserve preflight, staging, hash verification, backup, rollback, and restart for the selected set.
7. Verify extracted Windows release binaries exist, are non-empty, and pass a bounded --help startup smoke check.
8. Update architecture, updater, CLI/build/release documentation and this plan.

## Compatibility and safety
- Full updates remain supported when all configured binaries are included.
- Keep the legacy ZIP password for compatibility; it is not publisher authentication. Retain the existing production trust gate.
- Preserve executable names and CLI/configuration contracts.
- Never stop an application omitted from the selected package.

## Verification
- GitHub Actions only; no local Cargo commands.
- Focused updater tests cover selective selection and unknown-file rejection.
- Python unit tests exercise the exact package path-classification function used by CI, including Runner-only selection and shared-path fan-out.
- Full Windows workspace CI covers formatting, Clippy, tests, and release compilation.
- Release packaging verifies the selected executable set after extraction and invokes --help on each binary.
- Manually inspect the complete PR diff and workflow conditions. Keep work in PR #464; do not merge.

## Findings
- Release workflow already supports binary selection and preserves .exe names.
- Previous updater behavior required every configured source, conflicting with selective releases; this phase changes selection to exact configured files present in the package.

## Selective CI detector follow-up
- Implemented `.github/scripts/detect_affected_packages.py` as the single source of truth for path classification.
- Added behavioral unit tests for Runner-only, Tasker-only, multiple apps, shared manifests/code/workflows, unknown paths, documentation-only changes, and path normalization.
- The detect-packages job runs the tests before producing its JSON matrix; `.github/scripts/**` is included in workflow path filters.
