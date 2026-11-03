# Build and Release

## Process packages

The repository contains five independently selectable executable packages:

| Executable | Cargo package |
|---|---|
| crm.exe | crm |
| runner.exe | runner |
| yasweb.exe | yasweb |
| tasker.exe | tasker |
| crm_updater.exe | crm_updater |

Build one executable from the repository root:

```powershell
cargo build --release -p runner --locked
```

Replace runner with crm, yasweb, tasker, or crm_updater as needed. The shared crm_tool package is compiled only as a dependency of the selected application.

## Pull request CI selection

The Test workflow determines which app packages are affected by the changed paths:

- Changes under apps/<package>/ run Clippy, tests, and release compilation for that package.
- The shared selector in `.github/scripts/detect_affected_packages.py` is covered by Python unit tests in `.github/scripts/test_detect_affected_packages.py`; CI runs these tests before creating the package matrix.
- Pull request detection compares the event's exact base SHA with its exact head SHA, so it includes every change in the PR rather than only the latest commit. Push events compare the before/after SHAs.
- Changes to shared root Rust code, root tests, Cargo manifests/lockfile, dependency policy, or anything under `.github/` select all five packages. Unknown app paths and unknown root-level code/config paths also fail closed and select all five.
- Formatting and dependency security auditing remain repository-wide.
- Changes limited to `md/` documentation do not start Rust package jobs. App-local files select their owning package. The workflow's JavaScript and PowerShell syntax checks still cover the app source trees when the Test workflow is triggered.

This is a build/test optimization only; it does not change runtime package dependencies or release contents.

## Windows release workflow

Run Release Binaries from GitHub Actions with one of:

- all
- one package name, such as runner
- comma-separated package names, such as runner,tasker

The workflow builds only the selected packages, preserves the executable filenames, creates the existing password-protected ZIP naming format, verifies the archive and extracted executable files, and invokes each selected executable with --help as a bounded startup/CLI smoke check. The ZIP is then published as a GitHub Release asset and uploaded to paste.c-net.org.

The --help check verifies that the Windows executable starts and reaches CLI argument handling. It is not an interactive GUI, browser, Outlook, or end-to-end operational test.

## CRM Updater selection

CRM Updater applies only source files present in the incoming ZIP or staged downloads directory that exactly match configured file_replacement_map entries. Unknown archive files and ambiguous mappings are rejected before any application is stopped. Applications omitted from the package are not stopped or restarted. The detached update script retains preflight, staged replacement, hash verification, backups, rollback, and restart behavior for the selected applications.
