# CRM Tool — Rust CLI Application

## Overview

A production-ready CLI tool built in Rust that:

1. **Authenticates** with AWS Cognito using the SRP (Secure Remote Password) flow
2. **Fetches** CRM reports from a REST API using the resulting JWT token
3. **Downloads** report CSVs from signed S3/MinIO URLs
4. **Persists** configuration and tokens to a local JSON file

---

## Quick Start

### Local Build & Run

```bash
# Build and run
./run.sh

# Or manually:
cargo build --release
./target/release/crm_tool
```

### Docker Build (Windows .exe + Linux binary)

```bash
./docker-build.sh
# Outputs: crm_tool.exe (Windows) and crm_tool_linux (Linux)
```

### Docker Build (Linux-only, faster)

```bash
docker build -f Dockerfile.linux -t crm-tool .
docker run --rm -v $(pwd):/app crm-tool --config /app/config.json
```

---

## CLI Arguments

| Argument             | Type           | Default       | Description                            |
|----------------------|----------------|---------------|----------------------------------------|
| `--config`           | String         | `config.json` | Path to JSON config file               |
| `--region`           | Option<String> | —             | AWS region                             |
| `--user-pool-id`     | Option<String> | —             | Cognito User Pool ID                   |
| `--client-id`        | Option<String> | —             | Cognito App Client ID                  |
| `--username`         | Option<String> | —             | Cognito username / phone               |
| `--password`         | Option<String> | —             | Cognito password                       |
| `--email`            | Option<String> | —             | Email for CRM report requests          |
| `--from-date`        | Option<String> | —             | Start date for tickets/leads           |
| `--calls-from-date`  | Option<String> | —             | Start date for call logs               |
| `--to-date`          | Option<String> | —             | End date (defaults to today)           |
| `--report`           | Enum           | `all`         | `all`, `tickets`, `calls`, `leads`, `none` |
| `--output`           | Option<String> | —             | Save JSON output to file               |
| `--no-verify-ssl`    | bool flag      | false         | Disable TLS cert verification          |
| `--skip-login`       | bool flag      | false         | Use cached token, skip login           |
| `--remember-secrets` | Option<bool>   | —             | Persist password/tokens to config      |

### Examples

```bash
# Fetch all reports with defaults
./target/release/crm_tool

# Fetch only tickets
./target/release/crm_tool --report tickets

# Save output to file
./target/release/crm_tool --output results.json

# Use cached token
./target/release/crm_tool --skip-login

# Override config values
./target/release/crm_tool --email "user@example.com" --from-date "2026-01-01"
```

---

## Configuration

The tool uses a JSON config file (`config.json` by default). It is created automatically on first run with sensible defaults.

### Precedence

1. CLI arguments (highest priority)
2. Config file values
3. Built-in defaults (lowest priority)

### Secret Management

When `remember_secrets = false`, these fields are stripped before saving:
- `password`
- `access_token`, `access_token_expiry`
- `id_token`, `refresh_token`
- `token_timestamp`

---

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed module documentation.
