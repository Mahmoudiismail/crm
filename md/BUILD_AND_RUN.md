# Build and Run

## Requirements

- Rust toolchain installed.
- Access to Cognito + CRM endpoints.
- Valid `runner_config.json` and `config.json` values (with correct authentication credentials).

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

Release builds use the `Cargo.toml` release profile tuned for maximum runtime optimization and minimal file size:

- `opt-level = 3` — maximize runtime optimization
- `lto = "fat"` — whole-program link-time optimization
- `codegen-units = 1` — maximize optimization across crate
- `strip = "symbols"` — strip symbols from release binaries
- `panic = "abort"` — smaller than unwinding
- `debug = false` — omit debug info in release artifacts
- `incremental = false` — keep release builds fully optimized
- `overflow-checks = false` — disable overflow checks

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
- The runner GUI loads Tailwind CSS from cdnjs at runtime. The scheduler and JSON endpoints still work if that stylesheet cannot be reached, but the dashboard will render without Tailwind styling.

CRM CLI arguments:

- `--report all|tickets|calls|leads|none`
- `--config <path>`

CRM always performs login when running.

## Dependencies

Dependencies are maintained at latest stable versions. Current pinned versions (as of April 2026):

- `tokio` 1.52.0 — async runtime
- `reqwest` 0.13.2 — HTTP client with rustls
- `serde` / `serde_json` 1.0.228 / 1.0.149 — serialization
- `chrono` / `chrono-tz` 0.4.44 / 0.10.4 — date/time
- `tracing` / `tracing-subscriber` 0.1.44 / 0.3.23 — logging
- `hmac` / `sha2` / `hex` / `base64` / `rand` — cryptography
- `tray-icon` 0.22 — system tray
- `winit` 0.30.12 — windowing (for tray integration)

See `Cargo.toml` for the complete dependency list.

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
3. Dashboard shows human-readable schedule, next-run, and last-run values.
4. Task scheduler runs configured legacy tasks and multi-schedule tasks.
5. Shell command groups run sequentially or in parallel when `allow_shell_tasks=true`.
6. CRM auth/fetch/download succeeds for CRM tasks.
7. CSV files are created under `Downloads/` beside `crm` executable.
