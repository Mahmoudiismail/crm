# Process-Based Architecture — Phase 3 & 4

## Scope

Continue the original roadmap without renaming or replacing its phases:

- **Phase 3 — Plugin Management:** complete Manifest Registration and API Versioning.
- **Phase 4 — Lifecycle & Isolation:** review process startup, shutdown, failure handling, logs, cancellation, and process isolation.

This work extends the existing Phase 1 & 2 changes in PR #464. It must remain in that PR and must not create a parallel PR.

## Phase 3 — Plugin Management

1. Audit all first-party executable manifests (Runner, CRM, Tasker, Yasweb, CRM Updater) against their actual Clap CLI options and runtime behavior.
2. Establish one explicit compatibility policy:
   - First-party plugins that implement the process-plugin contract declare the supported API version.
   - Legacy third-party executables with no configured API version remain launchable without a manifest probe.
   - A configured version requires a valid manifest with the same version; malformed, missing, and unsupported versions fail before process launch with an actionable error.
3. Ensure manifest registration/probing does not accidentally execute normal application startup, mutate configuration, initialize logging, or leave child processes running.
4. Validate manifest structure and argument metadata before the Runner consumes it; preserve existing schema fields and CLI compatibility.
5. Add behavioral tests for manifest output, JSON decoding, version match/mismatch, missing version, malformed output, and legacy external-app compatibility.

## Phase 4 — Lifecycle & Isolation

1. Trace Runner's real child-process lifecycle from admission through spawn, output collection, timeout/cancellation, exit handling, and shutdown.
2. Verify cancellation and timeout terminate the intended child process (and descendants where the current platform implementation supports it), reap the child, and finalize task state/logging.
3. Verify one plugin's startup/runtime failure does not corrupt Runner state or prevent unrelated plugins from running.
4. Verify stdout/stderr and exit status are associated with the correct task/process, including non-zero exits and early termination.
5. Add runtime-oriented tests for independent child processes, non-zero exit, timeout/cancellation, output capture, and continued operation after a sibling process fails. Prefer small fixture executables or existing test harnesses; avoid source-text assertions.
6. Preserve support for arbitrary legacy external executables and shell commands.

## Documentation

Update the relevant files under `md/` in the same change, including the manifest/API contract, CLI/architecture behavior, and this plan's completion notes. Do not introduce undocumented flags or config fields.

## Verification

- Use GitHub Actions only; do not run Cargo locally.
- Run focused tests for each changed behavior, then the complete required CI suite (format, Clippy with warnings denied, tests, and Windows release build).
- After any fix, rerun affected tests and the full required verification.
- Manually review the complete final diff against this plan and the original roadmap.
- Do not merge the PR.

## Constraints

- Smallest correct change; no unnecessary abstractions or frameworks.
- No breaking change to existing CLI contracts or legacy external-app behavior.
- Keep the work on PR #464's existing branch.
- Do not expand into unrelated Runtime Paths work or Phase 5 CI/CD changes, except documentation/CI adjustments strictly required to verify Phases 3 and 4.
