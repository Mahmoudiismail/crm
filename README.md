# CRM Tool

A Rust-based automation suite for CRM data retrieval, browser-based reporting, scheduled execution, and operational report processing. The repository is one Cargo package (crm_tool) that builds five executable targets.

## Applications

| Binary | Responsibility |
|---|---|
| runner | Windows system-tray scheduler, local HTTP GUI, task pipelines, process lifecycle and application concurrency |
| crm | Authenticated CRM API access and report downloads (tickets, calls, leads, users) |
| yasweb | Headless Chrome automation for configured Yasweb/MIS reports |
| tasker | CSV enrichment/reporting, Excel automation, Outlook email workflows and other configured operational tasks |
| crm_updater | Local diagnostics collection and staged application update workflow |

See [Architecture](./md/ARCHITECTURE.md) for component boundaries and execution flow.

## Requirements

- Rust stable toolchain (the repository does not currently declare a minimum supported Rust version).
- Windows is the primary runtime target. Some components support headless development/testing on other platforms.
- Cargo, Node.js for CI JavaScript syntax checks, and PowerShell for Windows automation.

## Build

Build all five binaries:

```bash
cargo build --release
```

Build a specific executable:

```bash
cargo build --release --bin runner
cargo build --release --bin crm
cargo build --release --bin yasweb
cargo build --release --bin tasker
cargo build --release --bin crm_updater
```

Artifacts are written to target/release/ (with .exe on Windows).

## Run

Run Runner for scheduled orchestration and its local GUI. Worker binaries can also be run directly:

```powershell
.\\target\\release\\runner.exe
.\\target\\release\\crm.exe --report tickets
.\\target\\release\\yasweb.exe --name "Report Name"
.\\target\\release\\tasker.exe --task 1
.\\target\\release\\crm_updater.exe --logs-only
```

Use --help on an executable for the parser help for that build. See [CLI](./md/CLI.md) and [Configuration](./md/CONFIG.md).

## Validation

The repository is one Cargo package with explicit binary targets, not a multi-package Cargo workspace. Existing CI commands use Cargo's --workspace selector against that package.

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

The Test workflow additionally validates JavaScript and PowerShell syntax and runs RustSec/cargo-deny dependency checks. See [Build and Run](./md/BUILD_AND_RUN.md) and [Operations](./md/OPERATIONS.md).

## Releases

The manually triggered [Release Binaries workflow](./.github/workflows/release.yml) builds the selected binaries (or all five), packages Windows executables into a timestamped ZIP, publishes it as a GitHub Release asset, uploads a short-lived workflow artifact, and prints the response from the configured paste upload endpoint. It builds from the main commit resolved by the prepare job.

## Documentation

Start with [the documentation index](./md/README.md). Core references include [Application Summary](./md/APPLICATION_SUMMARY.md), [Architecture](./md/ARCHITECTURE.md), [Build and Run](./md/BUILD_AND_RUN.md), [CLI](./md/CLI.md), [Configuration](./md/CONFIG.md), [Runner](./md/RUNNER.md), [Tasker](./md/TASKER.md), [CRM Updater](./md/CRM_UPDATER.md), and [Operations](./md/OPERATIONS.md).

AI coding agents must read [AGENTS.md](./AGENTS.md) before changing the repository. Documentation policy is described in [md/AI_DOC_POLICY.md](./md/AI_DOC_POLICY.md).

## Data and security

Do not commit operational exports, customer/employee data, credentials, or production configuration. Use synthetic fixtures for tests. Review [Operations](./md/OPERATIONS.md) and [Production Readiness](./md/PRODUCTION_READINESS.md) before using update or release flows in production.

## License

MIT — see [LICENSE](./LICENSE).
