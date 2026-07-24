# CRM Tool

A comprehensive Rust-based CRM (Customer Relationship Management) system with a task runner, scheduler, headless web automation capabilities, and automated data processing.

## 📋 Overview

This project provides automated management, downloading, and reporting for CRM systems. It operates through multiple specialized binaries rather than a single monolithic executable.

### Architecture Overview

The system consists of five main binaries:
- **`runner`**: Orchestrates and executes configured tasks, featuring a system tray GUI and interval-based scheduling.
- **`crm`**: Handles CRM data fetching, downloading, and reporting via authenticated APIs.
- **`yasweb`**: Automates headless Chrome interactions for legacy web systems (MIS modules).
- **`tasker`**: Stateless worker for background processes like CSV processing, Excel automation (via PowerShell COM), and dispatching formatted emails.
- **`wcxx`**: Dedicated fetcher for Webex Contact Center metadata and metrics.

For a detailed architectural breakdown, see [`md/ARCHITECTURE.md`](./md/ARCHITECTURE.md).

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (edition 2021)
- Windows OS (primary target: `x86_64-pc-windows-msvc`)
- Cargo
- (Optional) Docker for containerized builds

### Installation & Build

You can build the project using standard Cargo commands. The release profile is heavily optimized (`lto=fat`, `opt-level=3`, symbols stripped).

```bash
# Build all binaries in release mode
cargo build --release

# Build specific binaries
cargo build --release --bin crm
cargo build --release --bin runner
cargo build --release --bin yasweb
cargo build --release --bin tasker
cargo build --release --bin wcxx
```

### Running

Execute the built binaries directly from the `target/release/` directory.

```bash
# Run CRM tool (one-shot execution)
./target/release/crm.exe [OPTIONS]

# Run Runner (starts the background orchestrator and GUI)
./target/release/runner.exe

# Run Yasweb (headless web automation)
./target/release/yasweb.exe

# Run Tasker (background reporting tasks)
./target/release/tasker.exe
```

## 🔧 Developer Workflow

This project standardizes on pure Cargo commands for daily development.

### Testing

Run the test suite across the workspace.

```bash
cargo test --workspace --all-targets --all-features
```

### Linting & Formatting

Ensure your code meets the project's strict styling and safety guidelines before submitting changes.

```bash
cargo fmt
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Benchmarks

We use `criterion` for benchmarking performance-critical paths (like CSV parsing and manifest loading).

```bash
cargo bench
```

### Documentation Generation

Generate local documentation for the repository's modules and internal APIs:

```bash
cargo doc --no-deps --open
```

## 📦 Releases & Deployment

Releases are fully automated via GitHub Actions.

When code is merged to the main branch or tagged, individual workflows (`release-crm.yml`, `release-runner.yml`, `release-yasweb.yml`, `release-tasker.yml`) build optimized Windows binaries, zip them, and publish them directly to GitHub Releases.

## 📚 Documentation

Detailed documentation is stored in the `md/` directory.

1. [`AGENTS.md`](./AGENTS.md) - **Mandatory reading for AI agents**
2. [`APPLICATION_SUMMARY.md`](./md/APPLICATION_SUMMARY.md) - Application overview
3. [`ARCHITECTURE.md`](./md/ARCHITECTURE.md) - System architecture
4. [`BUILD_AND_RUN.md`](./md/BUILD_AND_RUN.md) - Build and deployment guide
5. [`CONFIG.md`](./md/CONFIG.md) - Configuration reference
6. [`AUTH_FLOW.md`](./md/AUTH_FLOW.md) - Authentication flow
7. [`FETCHER.md`](./md/FETCHER.md) - Data fetching module
8. [`DOWNLOADER.md`](./md/DOWNLOADER.md) - Download functionality
9. [`SCHEDULER_TRAY.md`](./md/SCHEDULER_TRAY.md) - Scheduler and system tray
10. [`DOCKER.md`](./md/DOCKER.md) - Docker setup
11. [`OPERATIONS.md`](./md/OPERATIONS.md) - Operations guide
12. [`AI_DOC_POLICY.md`](./md/AI_DOC_POLICY.md) - Documentation policy
13. [`TASKER.md`](./md/TASKER.md) - Tasker application

## 🛠 Troubleshooting

- **Missing Configurations**: All binaries auto-generate default configurations (e.g., `runner_config.json`, `tasker_config.json`) in their execution directory if missing. Check the logs if a binary exits unexpectedly.
- **Log Files**: Logs are written with `TRACE` verbosity to `.log` files in the binary's directory and `DEBUG` verbosity to `stdout`.
- **COM Exceptions (0x800A03EC)**: If `tasker` fails when interacting with Excel, ensure background Excel processes aren't hung. Tasker attempts to forcefully clean up orphaned Excel processes automatically.

## 📝 Contributing

We welcome contributions! Please read our [Contributing Guide](CONTRIBUTING.md) to get started. It covers repository structure, coding standards, pull request workflow, and mandatory documentation policies.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
