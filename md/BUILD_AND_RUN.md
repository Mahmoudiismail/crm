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

## Run

```bash
cargo run --bin runner
cargo run --bin crm
```

- `runner` starts tray + scheduler + GUI.
- `crm` runs one CRM cycle and exits.
- At first runner start, `runner_config.json` is created automatically if missing.

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
- `.github/workflows/release.yml`

All workflows use one shared cargo cache key strategy:

- `shared-cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}`

## Quick Smoke Verification

1. App starts and logs initialization banner.
2. Runner GUI starts on configured host/port.
3. Task scheduler runs configured tasks.
4. CRM auth/fetch/download succeeds for CRM tasks.
5. CSV files are created under `download/`.
