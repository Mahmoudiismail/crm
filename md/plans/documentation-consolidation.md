# Documentation Consolidation Plan

## Scope
- Consolidate Markdown documentation under `md/`.
- Move historical plans to `md/plans/`, issue analyses to `md/issues/`, and repository assessment to `md/audits/`.
- Remove the tracked `TestingDownloads/README.md` and remove guidance directing tests to that folder.
- Update repository guidance and documentation indexes.
- Retain `benches/`; Criterion benchmarks measure performance and are distinct from tests.

## Constraints
- Preserve historical document contents while moving them.
- Keep root entrypoint files in place.
- Use the existing open PR branch; do not open another PR or merge.

## Verification
- Inspect the resulting Git tree for the new paths and absence of old directories.
- Review changed paths and relevant repository guidance.
- Check CI after commit; no tests are assumed to pass merely because of file moves.
