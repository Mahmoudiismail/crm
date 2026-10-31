# Build & Run Guide

## Development requirements

- Rust stable toolchain installed through rustup. Cargo.toml does not declare an explicit MSRV.
- Windows is the primary runtime/build target (x86_64-pc-windows-msvc).
- Node.js is used by CI to syntax-check JavaScript files.
- PowerShell is used by Windows automation and CI to parse PowerShell scripts.

## Cargo package and binaries

The repository is one Cargo package (crm_tool) with explicit binary targets; it is not a multi-package workspace. It declares five executables:

\`\`\`bash
cargo build --release
cargo build --release --bin runner
cargo build --release --bin crm
cargo build --release --bin yasweb
cargo build --release --bin tasker
cargo build --release --bin crm_updater
\`\`\`

Binaries are placed in target/release/.

## Running

Run Runner to start the scheduler and local GUI:

\`\`\`powershell
.\\target\\release\\runner.exe
\`\`\`

The default GUI address is http://127.0.0.1:8787; the configured host and port are read from runner_config.json. Runner is a system-tray application on Windows and runs headless with Ctrl+C shutdown on non-Windows platforms.

Workers may also be invoked directly. See [CLI Reference](./CLI.md) and [Configuration Guide](./CONFIG.md).

## AppManifest integration

Runner can register supported applications dynamically. It invokes their hidden --manifest option to retrieve the argument schema:

\`\`\`powershell
crm.exe --manifest
yasweb.exe --manifest
tasker.exe --manifest
crm_updater.exe --manifest
runner.exe --manifest
\`\`\`

The manifest is JSON written to stdout; the process exits after printing it. See [Manifest Schema](./MANIFEST_SCHEMA.md).

## Local validation

Use the core Rust checks run by the Test workflow:

\`\`\`bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
\`\`\`

The workflow additionally checks JavaScript syntax, parses PowerShell source files, and runs cargo audit and cargo deny check advisories.

## GitHub Actions

- .github/workflows/test.yml runs for relevant pull requests and pushes to main, and supports manual dispatch. It performs formatting, Clippy, dependency advisory checks, JavaScript/PowerShell syntax checks, and the test suite.
- .github/workflows/auto-format.yml is manual-dispatch-only. Its format job references the format-approval GitHub Environment; configure required reviewers in repository settings if an approval pause is desired.
- .github/workflows/release.yml is manually triggered. Its binary input accepts all or a comma-separated selection from crm, runner, yasweb, tasker, and crm_updater.

The Test workflow's format job can commit Rust formatting changes to the branch when permitted by the event/repository context. The workflow YAML is the source of truth for exact triggers and job dependencies.

## Windows cross-compilation from Linux/WSL

For the GNU Windows target (not the primary MSVC release target), install the target and linker:

\`\`\`bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
cargo build --target x86_64-pc-windows-gnu --release
\`\`\`
