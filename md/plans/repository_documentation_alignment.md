# Repository Documentation Alignment Plan

## Goal
Audit the repository documentation against the current source tree, Cargo metadata, CLI definitions, and GitHub Actions workflows, then correct the operational documentation without changing runtime code.

## Scope
- Root documentation: README.md and repository contribution/agent entry points only where references are demonstrably stale.
- Core guides: APPLICATION_SUMMARY, ARCHITECTURE, BUILD_AND_RUN, CLI, CONFIG, OPERATIONS, README index, AI_DOC_POLICY, RUNNER, TASKER, CRM_UPDATER, MANIFEST_SCHEMA, TESTING, and relevant Yasweb/CRM guides when source comparison identifies drift.
- Add this plan to md/plans as required by AGENTS.md.

## Audit checks
1. Inventory all tracked Markdown files and identify the current canonical guides versus historical plans/audits.
2. Verify binary names and Cargo package/workspace structure against Cargo.toml.
3. Verify each binary's CLI flags, config path defaults/overrides, manifest exposure, and date arguments against src/bin entry points.
4. Verify Runner pipeline, task admission/locking, GUI security, logging, date argument visibility, and persistence claims against implementation.
5. Verify Tasker task types, CSV result paths, lead assignee/team mapping, Call Center attachment filters, and supported CLI flags against implementation.
6. Verify CRM Updater behavior and flags against its CLI/config implementation.
7. Verify CI and release commands, triggers, artifacts, and publication behavior against .github/workflows.
8. Check internal Markdown links and remove statements that assert unsupported or obsolete behavior.

## Implementation rules
- Documentation-only change; no Rust, JavaScript, PowerShell, workflow, or configuration behavior changes.
- Preserve historical audit/plan documents as historical records; do not rewrite history or mark old findings as current behavior.
- Keep examples executable and distinguish Cargo package targets from a multi-package workspace.
- Keep documentation aligned with actual code even where existing docs or memories disagree.

## Verification
- Re-fetch each changed file from the PR branch and confirm the committed content.
- Check all changed Markdown links and search for stale CLI/workflow claims.
- Do not claim Rust tests or CI passed for a documentation-only update unless actually run.
