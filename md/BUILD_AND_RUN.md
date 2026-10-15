# Build & Run Guide

## Local Development Requirements
- **Rust Toolchain**: Stable version. Install via [rustup](https://rustup.rs).
- **Node.js/npm**: Used occasionally for frontend linting (e.g., Prettier).

## Compiling

The project consists of multiple binaries. Build all of them at once:

```bash
cargo build --release
```

Or build a specific binary:

```bash
cargo build --release --bin runner
cargo build --release --bin crm
cargo build --release --bin yasweb
cargo build --release --bin tasker
cargo build --release --bin crm_updater
```

Executables will be placed in `target/release/`.

## Running the Architecture

To run the full suite as a user would, ensure the executables are placed together.

1. Start the runner:
   ```bash
   cargo run --bin runner
   ```
2. The Runner GUI will be available at `http://127.0.0.1:8787`.
3. Configure CRM, Yasweb, and Tasker instances from the GUI, or add them via the **Apps** page for dynamic manifest orchestration.

### Dynamic App Execution

Executables supporting the `AppManifest` standard can be queried independently:

```bash
cargo run --bin crm -- --manifest
cargo run --bin yasweb -- --manifest
cargo run --bin tasker -- --manifest
```

They output a JSON representation of their arguments and exit.

## CI Pipeline and Testing

The GitHub Actions test workflow runs on relevant pull requests and can be started manually. It enforces:

1. `cargo fmt -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-targets --all-features`

Run the test suite locally with:

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

## Windows Release Workflow

The manually triggered **Release Binaries** workflow accepts `all` or a comma-separated selection from this allowlist:

- `crm`
- `runner`
- `yasweb`
- `tasker`
- `crm_updater`

Unknown names, empty selections, and duplicates are rejected before building. The workflow builds only the selected targets and packages their Windows executables into the release ZIP.

## Cross-Compiling for Windows (from Linux/WSL)

For targets supported by the installed toolchain, install the Windows target and linker:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
cargo build --target x86_64-pc-windows-gnu --release
```
