# Build & Run Guide

## Local Development Requirements
- **Rust Toolchain**: Stable version (e.g. 1.75+). Install via [rustup](https://rustup.rs).
- **Node.js/npm**: Used occasionally for frontend linting (e.g., Prettier).

## Compiling

The project consists of multiple bins. You can build all of them at once:

```bash
cargo build --release
```

Or a specific binary:

```bash
cargo build --release --bin runner
cargo build --release --bin crm
cargo build --release --bin yasweb
cargo build --release --bin wcxx
cargo build --release --bin tasker
```

Executables will be placed in `target/release/`.

## Running the Architecture

To run the full suite as a user would, you need to ensure the executables are placed together.

1. Start the runner:
   ```bash
   cargo run --bin runner
   ```
2. The Runner GUI will be available at `http://127.0.0.1:8787`.
3. You can configure your CRM, Yasweb, WCXX, and Tasker instances from the GUI, or add them via the **Apps** page for dynamic manifest orchestration.

### Dynamic App Execution

Executables supporting the `AppManifest` standard can be queried independently:

```bash
cargo run --bin crm -- --manifest
cargo run --bin yasweb -- --manifest
cargo run --bin wcxx -- --manifest
cargo run --bin tasker -- --manifest
```

They will output a JSON representation of their arguments and exit.

## Running Tests

Standard Rust cargo tests:
```bash
cargo test
```

## Cross-Compiling for Windows (from Linux/WSL)

Since the target environment is Windows and the runner uses Windows subsystem features:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
cargo build --target x86_64-pc-windows-gnu --release
```
