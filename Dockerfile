# Build stage: Use Rust with cross-compilation to Windows
FROM rust:1.86-bookworm AS builder

# Install cross-compilation toolchain for Windows
RUN apt-get update && apt-get install -y \
    gcc-mingw-w64-x86-64 \
    g++-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

# Add Windows target
RUN rustup target add x86_64-pc-windows-gnu

# Configure cargo to use mingw linker for Windows target
RUN mkdir -p /root/.cargo && \
    printf '[target.x86_64-pc-windows-gnu]\nlinker = "x86_64-w64-mingw32-gcc"\n' > /root/.cargo/config.toml

WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy lib/bin files to pre-build dependencies
RUN mkdir -p src/bin && \
    echo 'pub mod crm; pub mod runner;' > src/lib.rs && \
    echo 'fn main() { println!("runner dummy"); }' > src/bin/runner.rs && \
    echo 'fn main() { println!("crm dummy"); }' > src/bin/crm.rs && \
    cargo build --release --target x86_64-pc-windows-gnu 2>/dev/null || true && \
    rm -rf src

# Copy actual source code
COPY src/ src/

# Build the real binary for Windows
RUN cargo build --release --target x86_64-pc-windows-gnu

# Output stage: Copy the built binary out
FROM debian:bookworm-slim AS output

WORKDIR /output

COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/runner.exe ./runner.exe
COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/crm.exe ./crm.exe

# Default: just list the output
CMD ["ls", "-la", "/output/"]
