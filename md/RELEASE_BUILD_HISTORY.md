# Release Build History

The **Release Binaries** GitHub Actions workflow maintains `plan/v1-release-build-history.csv`.

Each row records the UTC start time, workflow run ID and attempt, source commit, package version, requested binary selection, build outcome, release outcome, and a sanitized failure reason. The workflow writes an initial `in_progress` row before checking whether the version tag already exists, then updates that same row when the release job finishes.

Status meanings:
- `success`: the corresponding step completed successfully.
- `failure`: the corresponding step failed.
- `not_started`: the corresponding stage did not run (for example, an existing version tag stopped the workflow before compilation).
- `in_progress`: the attempt was recorded but has not reached the final status update yet.

The release workflow serializes runs so concurrent executions cannot race while updating the shared history file. The workflow token needs repository contents write permission for this history commit. A hard cancellation or runner loss before the finalizer executes can leave a row marked `in_progress`.
