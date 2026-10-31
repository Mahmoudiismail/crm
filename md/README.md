# CRM Tool Documentation Index

This directory contains current operational guides, technical references, historical plans, issue analyses and audit records.

## Start here

1. [Repository instructions](../AGENTS.md) — mandatory workflow for coding agents.
2. [Application summary](./APPLICATION_SUMMARY.md) — binary roles and core capabilities.
3. [Architecture](./ARCHITECTURE.md) — runtime boundaries and execution flow.
4. [Build and Run](./BUILD_AND_RUN.md) — local build, validation and release workflows.
5. [CLI Reference](./CLI.md) — supported command-line options by binary.
6. [Configuration](./CONFIG.md) — configuration files, path resolution and persistence.
7. [Operations](./OPERATIONS.md) — daily operations, CI, troubleshooting and release cautions.

## Application guides

- [Runner](./RUNNER.md)
- [Scheduler and Tray](./SCHEDULER_TRAY.md)
- [CRM authentication](./AUTH_FLOW.md)
- [CRM Fetcher](./FETCHER.md)
- [CRM Downloader](./DOWNLOADER.md)
- [CRM startup](./CRM_STARTUP.md)
- [CRM Updater](./CRM_UPDATER.md)
- [Tasker](./TASKER.md)
- [Yasweb development](./YASWEB_DEV.md)
- [Yasweb report automation](../docs/yasweb-report-automation.md)
- [AppManifest schema](./MANIFEST_SCHEMA.md)

## Engineering and governance

- [Testing](./TESTING.md)
- [AI documentation policy](./AI_DOC_POLICY.md)
- [Dependency audit](./DEPENDENCY_AUDIT.md)
- [Production readiness](./PRODUCTION_READINESS.md)
- [Rebuild guide](./REBUILD_GUIDE.md)

## Historical material

- plans/ contains implementation plans and completed planning records.
- issues/ contains issue reports and remediation notes.
- audits/ contains repository assessments and audit history.

Historical documents describe findings and project state at the time they were written. They are not necessarily statements of current runtime behavior; use the current application guides and source code for operational truth.

## Documentation maintenance

Any change to Rust, scripts, configuration, dependencies or workflows must update the relevant current guides in the same change. If a historical audit is retained unchanged, label it as historical rather than treating it as current implementation documentation.
