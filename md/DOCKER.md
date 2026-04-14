# Docker and Container Build

## Repository Files

- `Dockerfile`
- `Dockerfile.dev`
- `docker-compose.yml`
- `docker-build.sh`
- `run.sh`

## Dev Container

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

- `/workspace/crm_tool.exe`

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

## Common Issues

1. **Cache stale**: rebuild image with `--no-cache`.
2. **Permission mismatch**: repair ownership on host after container writes.
3. **Missing output**: verify `cargo build --release --target x86_64-pc-windows-gnu` succeeded.
