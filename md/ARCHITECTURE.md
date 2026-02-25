# Architecture

## Project Structure

```
crm-rust/
├── Cargo.toml              # Dependencies and build config
├── Dockerfile              # Multi-target build (Windows + Linux)
├── Dockerfile.linux        # Linux-only build (faster)
├── run.sh                  # Local build & run script
├── docker-build.sh         # Docker build & extract binaries
├── md/                     # Documentation
│   ├── README.md           # Main documentation
│   ├── ARCHITECTURE.md     # This file
│   ├── AUTH_FLOW.md        # Authentication details
│   ├── CONFIG.md           # Configuration reference
│   ├── FETCHER.md          # Report fetching details
│   └── DOCKER.md           # Docker build instructions
└── src/
    ├── main.rs             # Entry point, orchestration, logging
    ├── lib.rs              # Module declarations
    ├── cli.rs              # Argument parsing (clap derive)
    ├── config.rs           # Config loading/saving/merging
    ├── auth.rs             # Cognito SRP authentication
    ├── fetcher.rs          # Report fetching, monthly batching
    └── downloader.rs       # Streaming CSV download
```

## Module Responsibilities

### `main.rs` — Orchestration
- Parses CLI arguments
- Sets up dual logging (file + stdout)
- Loads and merges config
- Builds shared `reqwest::Client`
- Orchestrates: authenticate → fetch reports → download CSVs → output
- Handles fatal errors (exit code 1)

### `cli.rs` — Argument Parsing
- Uses `clap` derive API
- Defines `CliArgs` struct with all CLI arguments
- Defines `ReportType` enum (`All`, `Tickets`, `Calls`, `Leads`, `None`)

### `config.rs` — Configuration Management
- `AppConfig` struct with all config fields
- `Default` implementation with built-in defaults
- `load()` — Read JSON, merge missing keys from defaults
- `apply_cli_overrides()` — CLI args override config values
- `save()` — Write to disk, strip secrets if needed, strip nulls

### `auth.rs` — Cognito SRP Authentication
- `ensure_authenticated()` — Public API, checks cache first
- Full SRP-6a implementation (2048-bit group)
- `InitiateAuth` → `RespondToAuthChallenge` flow
- HMAC-SHA256 signature generation
- HKDF key derivation
- Token caching with expiry checking

### `fetcher.rs` — Report Fetching
- `fetch_reports()` — Concurrent fetch of all requested reports
- Monthly batching for call logs
- `split_monthly()` — Date range splitting with correct month-end handling
- `extract_urls()` — Extract download URLs from results
- Per-report error handling (errors stored in results, don't abort)

### `downloader.rs` — CSV Download
- `download_csv()` — Streaming download with 60s timeout
- URL-decoded filename extraction
- Progress logging

## Data Flow

```
CLI Args → Load Config → Merge → Authenticate → Fetch Reports → Download CSVs → Output
              ↓                      ↓                                             ↓
         config.json            Cognito SRP                                   stdout / file
                                     ↓
                              Token cached in config
```

## Concurrency Model

- All report fetches run concurrently via `tokio::spawn` + `join_all`
- Call log monthly batches are individual concurrent tasks
- Single shared `reqwest::Client` across all requests
- Errors are captured per-task, not propagated to abort other tasks

## Error Handling

- `anyhow::Result` used throughout
- Fatal errors: logged to file + stderr, exit code 1
- Per-report errors: logged, stored as `{"error": "message"}` in results
- Task join errors: logged and skipped

## Logging

- File (`crm_tool.log`): DEBUG level, append mode, includes all HTTP details
- Stdout: INFO level, clean output
- Start/completion banners with timestamps
