# Policy

Any operational changes to the codebase MUST be reflected in the markdown files in the `md/` directory.

- Features, CLI flags, configuration arguments must be kept in sync.
- Obsolete instructions must be updated or deleted.

## Audit Remediation (Issue 1 & 2)
- Cleaned up obsolete CI workflows (`.gitlab-ci.yml`, `.circleci/`)
- Updated `gui/templates.rs` to replace memory fragmenting allocations (`.collect::<Vec<_>>().join`) with efficient string `.fold` iterations.
- Added `audit_remediation_log.md` detailing previously addressed audit patterns (like `[workspace]`, `HashMap` lookups, and removed `.unwrap()` assertions).

## Audit Remediation (crm_updater)
- Created `md/CRM_UPDATER.md` detailing the operational behavior, CLI arguments, and config structure of the new `crm_updater` tool.

## PowerShell Source Management and Outlook

- Tasker PowerShell templates touched by the script-extraction work are maintained as standalone `.ps1` files under the corresponding `src/tasker/` directories and embedded with `include_str!`.
- CRM Updater scripts are maintained under `src/crm_updater/scripts/` and run through `ScriptManager`, including the detached self-update script path.
- Outlook recipient pre-validation via `Recipients.ResolveAll()` is intentionally disabled; do not reintroduce a custom recipient-resolution gate without an explicit requirement.
- Yasweb reviewed sources did not contain PowerShell scripts for this extraction change.
- Documentation for these behaviors is maintained in `md/TASKER.md`, `md/CRM_UPDATER.md`, and `md/ARCHITECTURE.md`.
