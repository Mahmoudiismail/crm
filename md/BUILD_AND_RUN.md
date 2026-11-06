# Build & Run Guide

## Development requirements

- Rust stable toolchain installed through rustup. Cargo.toml does not declare an explicit MSRV.
- Windows is the primary runtime/build target (x86_64-pc-windows-msvc).
- Node.js is used by CI to syntax-check JavaScript files.
- PowerShell is used by Windows automation and CI to parse PowerShell scripts.

## Cargo workspace and independent executables

The repository is a Cargo workspace. `crm_tool` contains the shared manifest, utilities, and generic script engine; each process owns its implementation and direct dependencies in an independent package under `apps/`. No executable package depends on another executable package:

| Package | Executable | Source |
|---|---|---|
| `runner` | `runner.exe` | `apps/runner/src/main.rs` |
| `crm` | `crm.exe` | `apps/crm/src/main.rs` |
| `tasker` | `tasker.exe` | `apps/tasker/src/main.rs` |
| `yasweb` | `yasweb.exe` | `apps/yasweb/src/main.rs` |
| `crm_updater` | `crm_updater.exe` | `apps/crm_updater/src/main.rs` |

Build all executables with `cargo build --workspace --release`, or build only the executable being changed:

```bash
cargo build --release -p runner
cargo build --release -p crm
cargo build --release -p tasker
cargo build --release -p yasweb
cargo build --release -p crm_updater
```

Each command writes the selected executable to `target/release/`; shared dependencies are compiled as needed, but unrelated executable targets are not rebuilt.

### Enabling first-party plugin compatibility checks

CRM, Tasker, and Yasweb advertise process-plugin API version `1`. When an application is registered or updated through the Runner GUI, Runner probes its `--manifest` and automatically stores the advertised version in `runner_config.json`. Legacy executables that do not return a versioned manifest remain registered as legacy applications. For manually maintained configurations, `plugin_api_version` may still be set explicitly.

## Running

Run Runner to start the scheduler and local GUI:

```powershell
.\\target\\release\\runner.exe
```

The default GUI address is http://127.0.0.1:8787; the configured host and port are read from runner_config.json. Runner is a system-tray application on Windows and runs headless with Ctrl+C shutdown on non-Windows platforms.

Workers may also be invoked directly. See [CLI Reference](./CLI.md) and [Configuration Guide](./CONFIG.md).

## AppManifest integration

Runner can register supported applications dynamically. It invokes their hidden --manifest option to retrieve the argument schema:

```powershell
crm.exe --manifest
yasweb.exe --manifest
tasker.exe --manifest
crm_updater.exe --manifest
runner.exe --manifest
```

The manifest is JSON written to stdout; the process exits after printing it. See [Manifest Schema](./MANIFEST_SCHEMA.md).

## Local validation

Use the core Rust checks run by the Test workflow:

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
```

The workflow additionally checks JavaScript and PowerShell syntax, runs cargo audit and cargo deny check advisories, and builds all process packages in release mode with `--locked`.

## GitHub Actions

- .github/workflows/test.yml runs for relevant pull requests and pushes to main, and supports manual dispatch. It performs formatting, Clippy, dependency advisory checks, JavaScript/PowerShell syntax checks, and the test suite.
- .github/workflows/auto-format.yml is manual-dispatch-only. Its format job references the format-approval GitHub Environment; configure required reviewers in repository settings if an approval pause is desired.
- .github/workflows/release.yml is manually triggered. Its binary input accepts all or a comma-separated selection from crm, runner, yasweb, tasker, and crm_updater.

The Test workflow's format job can commit Rust formatting changes to the branch when permitted by the event/repository context. The workflow YAML is the source of truth for exact triggers and job dependencies.

## Windows cross-compilation from Linux/WSL

For the GNU Windows target (not the primary MSVC release target), install the target and linker:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
cargo build --target x86_64-pc-windows-gnu --release
```
