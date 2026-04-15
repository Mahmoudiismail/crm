# Build and Run

## Requirements

- Rust toolchain installed.
- Access to Cognito + CRM endpoints.
- Valid `runner_config.json` and `config.json` values.

## Local Build

```bash
cargo check
cargo test
cargo build
```

## Release Build

```bash
cargo build --release
```

Release builds use the `Cargo.toml` release profile tuned for maximum runtime optimization:

- `opt-level = 3`
- `lto = "fat"`
- `codegen-units = 1`
- `strip = "symbols"`
- `panic = "abort"`
- `debug = false`
- `incremental = false`
- `overflow-checks = false`

For one optimized application binary:

```bash
cargo build --release --bin runner
cargo build --release --bin crm
```

## Run

```bash
cargo run --bin runner
cargo run --bin crm
```

- `runner` starts tray + scheduler + GUI.
- `crm` runs one CRM cycle and exits.
- Both binaries resolve config files under their executable directory by default.
- At first runner start, `runner_config.json` is auto-created if missing.
- Runner also ensures CRM `config.json` exists if missing (by invoking `crm --config <path> --report none`).

CRM CLI arguments:

- `--report all|tickets|calls|leads|none`
- `--config <path>`

CRM always performs login when running.

## Devcontainer Workspace

When opened through VS Code dev containers, `.devcontainer/devcontainer.json` attaches to the `dev` compose service and installs the project editor extensions: Rust Analyzer, CodeLLDB, crates, Even Better TOML, and OpenAI ChatGPT. This affects only the developer workspace; build and runtime commands remain the same as the local commands above.

## Windows Cross-Compile

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Output binary:

- `target/x86_64-pc-windows-gnu/release/runner.exe`
- `target/x86_64-pc-windows-gnu/release/crm.exe`

## Windows-target validation in Linux dev container

Use these commands when Linux host lacks GTK tray system libraries:

```bash
cargo check --target x86_64-pc-windows-gnu
cargo test --target x86_64-pc-windows-gnu --no-run
```

## Workflows

GitHub Actions are split into:

- `.github/workflows/ci.yml`
- `.github/workflows/build-windows.yml`
- `.github/workflows/release-runner.yml`
- `.github/workflows/release-crm.yml`

Release workflow behavior:

- `release-runner.yml` builds `cargo build --release --bin runner` and uploads `runner_windows.zip`.
- `release-crm.yml` builds `cargo build --release --bin crm` and uploads `crm_windows.zip`.
- Both release workflows publish to tag `v<package version>` from `Cargo.toml` and can update the same GitHub release with separate assets.

All workflows use one shared cargo cache key strategy:

- `shared-cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}`

## Quick Smoke Verification

1. App starts and logs initialization banner.
2. Runner GUI starts on configured host/port.
3. Task scheduler runs configured tasks.
4. CRM auth/fetch/download succeeds for CRM tasks.
5. CSV files are created under `Downloads/` beside `crm` executable.
