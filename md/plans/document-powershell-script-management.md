# Plan: Document PowerShell Script Management

## Scope
- Update `md/TASKER.md` to list extracted Tasker PowerShell templates and document the Outlook recipient-resolution behavior.
- Update `md/CRM_UPDATER.md` to document the managed updater scripts, result-file handoff, and detached self-update lifecycle.
- Update `md/ARCHITECTURE.md` with the source/runtime distinction and explain why the updater script runs detached.
- Update `md/AI_DOC_POLICY.md` to record the relevant documentation expectations for these changes.
- Fix the CI-discovered missing `anyhow::Context` import in `src/crm_updater/update.rs` before verification.

## Verification
- Confirm documentation reflects the current implementation without claiming Windows Outlook delivery has been tested.
- Recheck the existing PR workflow after the code and documentation commits.
- Keep all changes in existing PR #438; do not create another PR.

## Constraints
- Minimal changes only; no unrelated refactoring.
- Yasweb documentation must not imply PowerShell extraction where the reviewed source contained no PowerShell scripts.
