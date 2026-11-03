# Fix Release CLI Help Smoke Checks

## Context
Release workflow run 41612872501 built all five Windows executables and created/tested/extracted the ZIP successfully. The archive smoke check failed at `yasweb.exe --help`: Clap printed the help text, but the process returned exit code 1. Earlier checks for `crm.exe` and `runner.exe` passed. Publication and uploads were consequently skipped.

## Scope
1. Correct YASWEB CLI handling so Clap's intentional help/version display outcomes terminate successfully while genuine parse/config/runtime errors remain failures.
2. Add focused regression coverage for the CLI help/version exit behavior, using the actual parser/error handling path rather than asserting source text.
3. Make the release workflow smoke-check command reliably capture each executable's exit status under `set -euo pipefail`, and require both a zero exit code and non-empty help output.
4. Update the relevant operational documentation under `md/`.
5. Keep all changes in one PR and verify through GitHub Actions only; do not run Cargo locally.

## Implementation approach
- Inspect the CLI entry points for the binaries included in the release archive to determine whether the observed Clap handling defect is isolated or shared.
- Prefer the smallest targeted change. Handle Clap's `DisplayHelp` and `DisplayVersion` as successful exits; propagate all other errors unchanged.
- Add tests that exercise the same parsing/error classification used by the executable and prove help/version are success outcomes while invalid arguments remain errors.
- Replace the fragile post-command `$?` check in the workflow with an explicit conditional command invocation that works with Bash `errexit`.
- Document the CLI exit behavior and release smoke-check expectations in the appropriate existing guide.

## Verification
- Review the complete diff manually against this plan and repository rules.
- Run the focused YASWEB tests, then the full required CI checks via the PR's GitHub Actions.
- Run the Release Binaries workflow for all binaries after the fix; confirm build, ZIP test, extraction, all five `--help` checks, GitHub Release publication, artifact upload, and paste.c-net.org response.
- Do not merge without a separate explicit user instruction.

## Constraints
- No new dependencies or frameworks.
- No unrelated refactoring.
- No local Cargo execution.
- One PR only; do not create parallel PRs.
