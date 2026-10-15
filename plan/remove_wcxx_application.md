# Remove WCXX Application

## Goal
Remove the unused WCXX binary and all repository-owned build, release, and documentation references without changing other applications.

## Scope
- Remove the `wcxx` binary declaration from `Cargo.toml`.
- Delete `src/bin/wcxx.rs` and `md/WCXX.md`.
- Remove WCXX from the release workflow's selectable binary list and validation.
- Update README, contributor guidance, repository agent instructions, and relevant `md/` documentation to describe only maintained applications.
- Audit repository references for remaining WCXX-specific build/run/config/log/documentation references.
- Keep shared dependencies unless repository-wide inspection proves they are exclusively used by WCXX.

## Out of Scope
- Do not delete user-local WCXX configuration files or previously released binaries.
- Do not alter unrelated application behavior or refactor shared code.
- Do not merge the pull request into `main`.

## Verification
- Search the repository for remaining WCXX-specific references.
- Run `cargo fmt -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` where execution environment permits.
- Validate release workflow's remaining binary allowlist and inspect the full diff.
- Report actual CI outcomes without claiming unrun checks passed.
