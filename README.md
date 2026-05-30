# CRM Tool

A comprehensive Rust-based CRM (Customer Relationship Management) system with task runner, scheduler, and web automation capabilities.

## 📋 Overview

This project consists of three main binaries:
- **runner**: Orchestrates and executes configured tasks
- **crm**: One-shot CRM execution with CLI argument support
- **yasweb**: Web automation and headless Chrome integration

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (edition 2021)
- Windows (primary target: `x86_64-pc-windows-msvc`)
- Cargo

### Build

```bash
# Build all binaries in release mode
cargo build --release

# Build specific binary
cargo build --release --bin crm
cargo build --release --bin runner
cargo build --release --bin yasweb
```

### Run

```bash
# Run CRM tool
./target/release/crm.exe [OPTIONS]

# Run Runner
./target/release/runner.exe

# Run Yasweb
./target/release/yasweb.exe
```

### CLI Arguments

The `crm` binary supports:
- `--report`: Generate a report
- `--config <path>`: Specify configuration file

## 📚 Documentation

For detailed information, see the documentation in the `md/` folder:

1. [`AGENTS.md`](./AGENTS.md) - AI agent guidelines
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

## 🏗️ Project Structure

```
.
├── src/
│   ├── bin/
│   │   ├── runner.rs      # Runner orchestration entry point
│   │   ├── crm.rs         # CRM one-shot execution entry point
│   │   └── yasweb.rs      # Web automation entry point
│   ├── runner/            # Runner modules
│   ├── crm/               # CRM domain logic
│   └── lib.rs             # Shared module exports
├── md/                    # Comprehensive documentation
├── runner_config.json     # Task configuration
├── Cargo.toml             # Project manifest
└── .devcontainer/         # Dev container setup
```

## 🔧 Configuration

Tasks are configured in `runner_config.json`. Example structure:

```json
{
  "tasks": [
    {
      "name": "task_name",
      "command": "crm",
      "args": ["--report", "--config", "config.json"]
    }
  ]
}
```

## 🔐 Authentication

The system uses AWS Cognito for authentication. See [`AUTH_FLOW.md`](./md/AUTH_FLOW.md) for details.

## 📦 Dependencies

Key dependencies:
- **tokio**: Async runtime with selective features
- **reqwest**: HTTP client with rustls and streaming
- **serde/serde_json**: Serialization framework
- **tracing/tracing-subscriber**: Logging
- **chrono**: Date/time handling
- **headless_chrome**: Web automation
- **tray-icon/muda/winit**: Windows system tray (Windows-only)

See `Cargo.toml` for the complete dependency list.

## 🏗️ Build Optimization

The release profile is optimized for performance:
- Link-Time Optimization (LTO): fat
- Optimization level: 3
- Codegen units: 1
- Symbols stripped from release binaries
- No overflow checks in release

## 🚀 Release & Deployment

Releases are automatically built and published via GitHub Actions:
- **release-crm.yml**: Builds and publishes the CRM binary
- **release-runner.yml**: Builds and publishes the Runner binary
- **release-yasweb.yml**: Builds and publishes the Yasweb binary

All binaries use the same version from `Cargo.toml`.

## 📝 Contributing

Before making changes:
1. Read [`AGENTS.md`](./AGENTS.md)
2. Update relevant documentation in `md/` in the same commit as code changes
3. Follow the [AI Documentation Policy](./md/AI_DOC_POLICY.md)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.

## 💬 Support

For issues, feature requests, or questions, please refer to the project documentation or contact the maintainers.

---

**Last Updated**: May 2026
**Version**: 1.1.0
