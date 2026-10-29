# Runner Dispatch and Configuration Reliability Fix

## Goal

Correct the Runner GUI's false-positive run confirmations and improve the consistency of displayed and persisted `runner_config.json` data, without introducing a new framework or changing the config schema.

## Scope

1. Add result-bearing GUI commands for manual Run Task and Run All requests. Return an error when the scheduler channel is closed or the dispatcher rejects the request.
2. Stop ignoring execution-manager queue-send failures. Treat an already running/queued task as a rejected manual admission instead of reporting a new trigger.
3. Log scheduled dispatch success only after the scheduler command channel accepts the command; log channel failures explicitly.
4. Route Runner GUI/config API reads through the same in-process lock used by config mutations.
5. Verify each config mutation by reloading the atomically saved file and comparing the reloaded JSON to the expected configuration.
6. After task edits, compare the normalized submitted task with the persisted editable fields; exclude runtime-owned timestamps/status fields. Return an HTTP error if read-back fails or mismatches.
7. Update `md/RUNNER.md` and `md/CONFIG.md` to document the behavior.

## Constraints and decisions

- Keep the existing config format and atomic-write implementation.
- Keep the Execution Manager as the duplicate-admission authority; a status snapshot can be stale when concurrent requests arrive.
- Do not add generic logging infrastructure or broad refactors.
- Do not merge until CI checks pass.

## Verification plan

- Review the complete changed-file diff and verify all GUI config reads use the shared lock.
- Verify all ignored queue sends in the dispatcher paths touched by this change are removed.
- Verify manual Run Task and Run All wait for dispatcher results and render an error on rejection.
- Let the existing PR CI run formatting, Clippy, tests, and Windows build checks. No local test suite was run during this implementation session; report CI status separately and do not claim unobserved checks passed.

## Implementation status

Code and documentation updates are committed to the existing working branch/PR. CI verification remains pending.
