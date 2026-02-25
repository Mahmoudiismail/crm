# Docker Build Guide

## Build Options

### Option 1: Cross-compile for Windows + Linux

Uses MinGW cross-compilation toolchain to produce both a Windows `.exe` and a Linux binary.

```bash
# Build and extract binaries
./docker-build.sh

# Or manually:
docker build -t crm-tool-builder .
CONTAINER_ID=$(docker create crm-tool-builder)
docker cp "$CONTAINER_ID:/output/crm_tool.exe" ./crm_tool.exe
docker cp "$CONTAINER_ID:/output/crm_tool" ./crm_tool_linux
docker rm "$CONTAINER_ID"
```

### Option 2: Linux-only (faster)

```bash
docker build -f Dockerfile.linux -t crm-tool .
```

Run directly from Docker:
```bash
docker run --rm \
  -v $(pwd):/data \
  -w /data \
  crm-tool --config config.json
```

## Dockerfile Details

### Multi-stage Build

```
Stage 1 (builder):
  - rust:1.83-bookworm base
  - Install mingw-w64 for Windows cross-compilation
  - Add x86_64-pc-windows-gnu target
  - Build for both targets
  
Stage 2 (output):
  - debian:bookworm-slim
  - Copy both binaries
  - Small output image
```

### Layer Caching

Dependencies are cached by:
1. Copying `Cargo.toml` first
2. Building with a dummy `main.rs`
3. Then copying actual source and rebuilding

This means dependency changes trigger a full rebuild, but source-only changes are fast.

## Build Requirements

- Docker 20.10+
- ~2GB disk space for build layers
- ~5-10 minutes first build (downloading dependencies)
- ~1-2 minutes subsequent builds (cached dependencies)

## Troubleshooting

### Windows build fails with linking errors
The MinGW cross-compilation may have issues with some native dependencies. If so, use the Linux-only Dockerfile and build Windows binaries natively on Windows.

### Build is slow
- Ensure Docker BuildKit is enabled: `DOCKER_BUILDKIT=1 docker build ...`
- Use `docker build --progress=plain` to see detailed output

### Binary too large
The release profile includes LTO and stripping:
```toml
[profile.release]
opt-level = 3
lto = true
strip = true
```

Expected sizes:
- Linux: ~8-12 MB
- Windows: ~10-15 MB
