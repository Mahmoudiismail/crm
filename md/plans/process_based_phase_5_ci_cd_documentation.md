# Process-Based Architecture — Phase 5: CI/CD and Documentation

## Objective
Complete the process-based rollout by aligning package selection across CI, release packaging, and CRM Updater while preserving executable names and legacy Runner compatibility.

## Scope
1. Keep each executable independently buildable with its Cargo package.
2. Make CI select affected app packages for package-local Clippy, tests, and release builds. Changes to shared root code, workspace manifests/lockfile, or workflow policy fan out to all packages.
3. Preserve manual release selection for crm, runner, yasweb, tasker, crm_updater, comma-separated selections, or all.
4. Make CRM Updater apply only configured executable files present in the update package or staging directory. Reject unknown files and ambiguous mappings before stopping processes. Omitted applications must not be stopped or restarted.
5. Preserve preflight, staging, hash verification, backup, rollback, and restart for the selected set.
6. Verify extracted Windows release binaries exist, are non-empty, and pass a bounded --help startup smoke check.
7. Update architecture, updater, CLI/build/release documentation and this plan.

## Compatibility and safety
- Full updates remain supported when all configured binaries are included.
- Keep the legacy ZIP password for compatibility; it is not publisher authentication. Retain the existing production trust gate.
- Preserve executable names and CLI/configuration contracts.
- Never stop an application omitted from the selected package.

## Verification
- GitHub Actions only; no local Cargo commands.
- Focused updater tests cover selective selection and unknown-file rejection.
- Full Windows workspace CI covers formatting, Clippy, tests, and release compilation.
- Release packaging verifies the selected executable set after extraction and invokes --help on each binary.
- Manually inspect the complete PR diff and workflow conditions. Keep work in PR #464; do not merge.

## Findings
- Release workflow already supports binary selection and preserves .exe names.
- Previous updater behavior required every configured source, conflicting with selective releases; this phase changes selection to exact configured files present in the package.
