# Docker and Container Build

## Repository Files

- `Dockerfile`
- `Dockerfile.dev`
- `docker-compose.yml`
- `docker-build.sh`
- `run.sh`

## Dev Container

The VS Code devcontainer configuration lives in `.devcontainer/devcontainer.json` and uses the `dev` service from `docker-compose.yml`. It installs the Rust/editor extension set used by the project, including `rust-lang.rust-analyzer`, `vadimcn.vscode-lldb`, `serayuzgur.crates`, `tamasfe.even-better-toml`, and `openai.chatgpt`.

Start persistent dev environment:

```bash
docker compose up -d dev
```

Open shell:

```bash
docker compose exec dev bash
```

Run builds/tests inside:

```bash
cargo check
cargo test
```

## One-shot Windows Build

```bash
docker compose run --rm build-windows
```

Expected binary:

- `/workspace/runner.exe`
- `/workspace/crm.exe`

## Scripted Build

```bash
./run.sh
```

or

```bash
./docker-build.sh
```

## Toolchain Details

- Base image: Rust on Debian bookworm.
- Windows target: `x86_64-pc-windows-gnu`.
- Linker: `x86_64-w64-mingw32-gcc`.

## Release Optimization

Docker and local release builds use the same `Cargo.toml` release profile:

- maximum runtime optimization with `opt-level = 3`,
- fat LTO,
- one codegen unit,
- stripped symbols,
- aborting panics.

The scripted Windows build still builds both executables. GitHub release publishing is split into one workflow for `runner` and one workflow for `crm`.

## Common Issues

1. **Cache stale**: rebuild image with `--no-cache`.
2. **Permission mismatch**: repair ownership on host after container writes.
3. **Missing output**: verify `cargo build --release --target x86_64-pc-windows-gnu` succeeded.
