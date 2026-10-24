# Phase 2 — Configuration & Serialization Audit

## Goal
Audit configuration loading, defaults/merging, migration, validation, serialization, and persistence across CRM, Runner, Tasker, Yasweb, CRM Updater, and AppManifest. Make only evidence-based, compatibility-preserving fixes.

## Scope and constraints
- Keep changes small and focused; no architecture redesign, new framework, or unnecessary public API.
- Preserve existing user values and legacy configuration shapes unless they are demonstrably invalid or unsafe.
- Keep task-array opt-in/default behavior intentional and covered by tests.
- Never overwrite malformed configuration with defaults; errors should remain visible.
- Every runtime behavior change must be reflected in the relevant `md/` documentation.
- Commit this plan and all changes in one Phase 2 PR. Do not modify `main` directly or create a second PR.

## Execution steps
1. Inspect repository instructions and configuration entry points on the post-PR-445 `main`; map all loaders, serializers, migration paths, validators, and writes.
2. Review `merge_json` behavior, especially the special `tasks` array path, for order sensitivity, preservation of user-defined tasks/settings, and intended opt-in defaults. Add focused regression tests before/with any fix.
3. Review atomic persistence and loader behavior for missing, valid, legacy, and malformed files. Review config validation for concrete runtime risks; do not add arbitrary constraints or break accepted legacy configurations.
4. Audit serde defaults/aliases/custom deserializers and Runner/AppManifest migrations for backward-compatible round trips. Correct documentation where actual behavior differs from the stated contract.
5. Add or update focused behavioral tests for every changed invariant, including recursive object merge, atomic arrays, task defaults/order behavior, legacy deserialization, malformed JSON, persistence, and validation where applicable.
6. Perform a manual diff review against this plan and repository instructions. Run formatting, Clippy with warnings denied, and the available Rust test suite; report any environment/CI-only checks honestly.

## Verification
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets`
- Review relevant GitHub Actions workflows and ensure docs under `md/` match implemented behavior.

## Completion criteria
- No untested behavior change.
- Legacy configuration shapes remain supported where intended.
- User-defined values are not silently lost or mixed with unrelated defaults.
- Persistence failures remain explicit and do not leave a partially written config.
- Documentation and implementation agree.
- One Phase 2 PR contains the plan, fixes, tests, and documentation; do not merge it automatically.
