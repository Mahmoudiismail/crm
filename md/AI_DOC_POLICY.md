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
