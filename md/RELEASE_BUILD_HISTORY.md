# Release Build History

The **Release Binaries** GitHub Actions workflow appends one row to `plan/v1-release-build-history.csv` for each workflow invocation. The row is written by a separate history job after the release job completes or is skipped, so early failures in build preparation are recorded as well.

Each row contains the UTC start timestamp, workflow run ID and attempt, source commit, package version, requested binary selection, build outcome, release outcome, and a failure reason when available.

Status meanings:
- `success`: the corresponding stage completed successfully.
- `failed`: the corresponding stage failed.
- `blocked`: publication was blocked by the release-tag guard; this is not a compilation failure.
- `not_started`: the corresponding stage did not run.
- `unknown`: version or source metadata could not be determined during preparation.

The release workflow serializes runs so concurrent executions cannot race while updating the shared history file. The history job uses the workflow token to commit the CSV update to `main`, so repository Actions settings must permit write access to repository contents. If the history job itself cannot push (for example, due to repository rules), the run may finish without a committed history row.
