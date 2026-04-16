# Claude Code Usage Guide - CRM Tool

This document guides Claude Code agents working on the CRM Tool project.

## Project Overview

The **CRM Tool** is a Rust-based application consisting of two separate executables:

- **runner** (`src/bin/runner.rs`): A tray-based scheduler with GUI dashboard that runs tasks and manages the scheduler
- **crm** (`src/bin/crm.rs`): A one-shot executable that fetches CRM reports via AWS Cognito authentication and downloads CSV files

Both applications are optimized for Windows deployment (cross-compiled from Linux) and use Tokio async runtime for concurrent operations.

## Core Architecture

### Executables

```
src/bin/
├── runner.rs - Tray app + scheduler + GUI server
└── crm.rs    - One-shot CRM fetch/download
```

### Shared Library Modules

```
src/
├── lib.rs           - Shared module exports
├── runner/
│   ├── config.rs    - Task definitions and runner configuration
│   ├── engine.rs    - Scheduler loop and task execution
│   └── gui.rs       - HTTP dashboard server
└── crm/
├── auth.rs          - Cognito SRP authentication
├── config.rs        - CRM configuration and token caching
├── fetcher.rs       - Report API requests with range splitting
├── downloader.rs    - CSV download streams
├── types.rs         - Shared types
└── mod.rs           - Main CRM run logic
```

### Key Design Patterns

- **Single-instance lock**: Runner binds to TCP `127.0.0.1:14592` to prevent multiple instances
- **External process invocation**: Runner spawns CRM executable for each fetch task
- **Async concurrency**: Scheduler and GUI run concurrently in Tokio runtime
- **Report parallelism**: Fetcher uses `tokio::spawn` + `join_all` for concurrent report requests
- **Signed-URL failure handling**: Failed report ranges are automatically halved on `Failed to generate signed url`
- **Shell command groups**: Commands can run sequentially or in parallel with per-command error handling

## Configuration System

### Two-Config Architecture

**runner_config.json** (runner behavior & tasks)
- GUI bind settings (`gui_host`, `gui_port`)
- Scheduler (`poll_interval_seconds`)
- Task definitions with multi-schedule support
- External CRM executable path
- Shell task controls (`allow_shell_tasks`, `shell_timeout_seconds`)

**config.json** (CRM authentication & API settings)
- AWS Cognito credentials (`region`, `user_pool_id`, `client_id`, `username`, `password`)
- API endpoints (`base_url`, `account_id`, `application_id`)
- Token cache (`access_token`, `refresh_token`, `id_token`)
- Report parameters (`from_date`, `to_date`, `download_csv`)

### Multi-Schedule System

Tasks support multiple schedule types (replaces legacy `repetition`/`frequency_seconds`):
- **once**: Single run at RFC3339 timestamp
- **interval**: Repeat every N seconds (15m to 7d supported)
- **daily_times**: Multiple times per day (HH:MM format)
- **weekly**: Specific day of week with optional time
- **monthly**: Specific day of month (1-31) with optional time

### Shell Command Groups

Shell tasks support command groups with sequential or parallel execution:
- **sequential**: Commands run in order, stop on error (unless `continue_on_error=true`)
- **parallel**: All commands spawn concurrently, fail group if any non-continued command fails
- Per-command error handling via `continue_on_error` flag

## Documentation Requirements (MANDATORY - CRITICAL)

### ⚠️ YOU MUST UPDATE DOCUMENTATION AFTER EVERY MODIFICATION ⚠️

**This is not optional. Every code change requires corresponding documentation updates.**

### Rules from AGENTS.md (ENFORCED)

**Rule 1: Docs Must Track Code**

After **EVERY** code/config/build command that changes behavior, update relevant docs in `md/` within the same change set. No exceptions.

**Rule 2: Required File Mapping**

When you modify a file, you **MUST** update these corresponding documentation files:

#### Code Changes → Required Documentation Updates

| Code File Modified | Documentation Files That MUST Be Updated |
|-------------------|------------------------------------------|
| `src/bin/runner.rs` | `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md` |
| `src/bin/crm.rs` | `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md` |
| `src/lib.rs` | `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md` |
| `src/runner/config.rs` | `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `CONFIG.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md` |
| `src/runner/engine.rs` | `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `CONFIG.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md` |
| `src/runner/gui.rs` | `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md` |
| `src/crm/auth.rs` | `ARCHITECTURE.md`, `CONFIG.md`, `AUTH_FLOW.md`, `OPERATIONS.md` |
| `src/crm/fetcher.rs` | `ARCHITECTURE.md`, `CONFIG.md`, `FETCHER.md`, `OPERATIONS.md` |
| `src/crm/downloader.rs` | `ARCHITECTURE.md`, `CONFIG.md`, `DOWNLOADER.md`, `OPERATIONS.md` |
| `src/crm/config.rs` | `ARCHITECTURE.md`, `CONFIG.md`, `AUTH_FLOW.md`, `OPERATIONS.md` |
| `src/crm/types.rs` | `ARCHITECTURE.md`, `CONFIG.md` |
| `src/crm/mod.rs` | `ARCHITECTURE.md`, `CONFIG.md`, `OPERATIONS.md` |
| `Cargo.toml` (dependencies) | `BUILD_AND_RUN.md`, `DOCKER.md`, `APPLICATION_SUMMARY.md` |
| `.github/workflows/*` | `BUILD_AND_RUN.md`, `OPERATIONS.md` |
| `Dockerfile*`, scripts | `DOCKER.md`, `BUILD_AND_RUN.md` |
| `AGENTS.md` | `README.md`, `AI_DOC_POLICY.md` |

**Rule 3: Command-Level Discipline**

For each engineering command/session:

1. Identify impacted behavior
2. Update corresponding `md/*.md` files
3. Verify docs still reflect actual code paths
4. **Do not defer documentation updates**

**Rule 4: Pull Request Gate**

A change is **INCOMPLETE** if behavior changed and no matching doc update exists. PRs without doc updates will be rejected.

**Rule 5: Accuracy Standard**

- Prefer exact function/file names
- Document defaults, edge cases, and failure modes  
- Keep examples runnable and aligned with current CLI
- Update version numbers, timestamps, and examples
   - `src/bin/runner.rs` → `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md`
   - `src/bin/crm.rs` → `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `OPERATIONS.md`, `BUILD_AND_RUN.md`
   - `src/lib.rs` → `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`
   - `src/runner/*` → `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, `CONFIG.md`, `SCHEDULER_TRAY.md`, `OPERATIONS.md`
   - `src/crm/*` → `ARCHITECTURE.md`, `CONFIG.md`, `AUTH_FLOW.md`, `FETCHER.md`, `DOWNLOADER.md`, `OPERATIONS.md`
   - `Cargo.toml` changes → `BUILD_AND_RUN.md`, `DOCKER.md`, `APPLICATION_SUMMARY.md`
   - `.github/workflows/*` → `BUILD_AND_RUN.md`, `OPERATIONS.md`
   - `Dockerfile*`, scripts → `DOCKER.md`, `BUILD_AND_RUN.md`
   - `AGENTS.md` → `README.md`, `AI_DOC_POLICY.md`

3. **Command-level discipline**: Before each engineering session:
   - Identify impacted behavior
   - Update corresponding `md/*.md` files
   - Verify docs reflect actual code paths
   - Do not defer documentation updates

4. **Pull request gate**: A change is incomplete without matching doc updates

5. **Accuracy standard**:
   - Use exact function/file names
   - Document defaults, edge cases, failure modes
   - Keep examples runnable and aligned with current CLI

### Pre-commit Checklist

Before committing any change, verify:
- [ ] Code changed? YES
- [ ] Matching docs updated in `md/`? YES
- [ ] `AGENTS.md` read before making agent-authored changes? YES
- [ ] Examples still valid? YES
- [ ] New config fields documented? YES
- [ ] New CLI flags documented? YES

If any answer is `no`, update docs before finalizing the change.

## Commands and Tasks

### Build Commands

Development build:
```bash
cargo check
cargo test
cargo build
```

Release build (optimized):
```bash
cargo build --release --bin runner
cargo build --release --bin crm
```

Windows cross-compile:
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu --bin runner
cargo build --release --target x86_64-pc-windows-gnu --bin crm
```

### Run Commands

```bash
# Run tray scheduler + GUI
cargo run --bin runner

# Run one-shot CRM fetch
cargo run --bin crm -- --report all
cargo run --bin crm -- --config custom_config.json --report tickets
```

### Test Tasks

1. **Verify runner starts**: Check logs for GUI bind address and tray initialization
2. **Verify GUI accessible**: Open http://localhost:8787 (default port)
3. **Verify crm executable**: Run manual CRM task and check csv files in Downloads/
4. **Verify authentication**: Check for successful Cognito login in crm.log
5. **Verify scheduler**: Create test task with 5-minute interval, verify it runs
6. **Verify shell tasks**: Enable `allow_shell_tasks=true`, test command execution
7. **Verify multi-schedule**: Create task with multiple daily times and verify execution

## Important Implementation Details

### Authentication Flow

CRM uses AWS Cognito SRP (Secure Remote Password) authentication:
1. Calculate SRP values using username, password, user pool ID
2. Initiate auth with Cognito `InitiateAuth` API
3. Verify server signature and calculate session proof
4. Finalize with `RespondToAuthChallenge`
5. Cache tokens locally in config.json
6. Reuse cached tokens if still valid

### Runner -> CRM Invocation

For each `crm_fetch` task:
1. Runner constructs CLI args: `--config <path> --report <type>`
2. Spawns external CRM executable as child process
3. Waits for completion with timeout
4. Updates task metadata: `last_run_at`, `last_status`
5. Returns result to scheduler loop

### GUI Architecture

- Embedded HTTP server on configurable host/port
- Tailwind CSS loaded from cdnjs (no local assets)
- Dashboard shows: status, task list, schedule info, next-run times
- RESTful endpoints: `GET /`, `GET /tasks`, `POST /run/all`, etc.
- Toast notifications for successful actions
- Human-readable time display for schedules and last-run

### Scheduler Engine

- Polls at `poll_interval_seconds` (default 30s)
- Checks each task for due schedules
- Atomic run guard prevents overlapping execution
- Upgrades legacy tasks to multi-schedule format internally
- Recalculates `next_run_at` after each execution

## Common Issues and Troubleshooting

### Authentication Fails

- Check `region`, `user_pool_id`, `client_id` in config.json
- Verify `username` and `password`
- Ensure local clock is accurate (SRP is time-sensitive)
- Check network access to Cognito endpoint

### CRM Executable Not Found

- Verify `crm_executable_path` in runner_config.json
- Check file permissions (should be executable)
- Test manually: `./crm --config config.json --report none`
- Logs show spawn errors if path is incorrect

### Tasks Not Triggering

- Verify `enabled=true` on task
- Check `next_run_at` format (RFC3339) or empty for immediate
- Verify legacy fields: `repetition` (once|repeat), `frequency_seconds`
- Check `schedules` array format and enabled flags
- Verify `poll_interval_seconds` is reasonable

### GUI Not Accessible

- Check runner logs for GUI bind address
- Verify `gui_host` and `gui_port` in config
- Check firewall rules
- Try localhost vs 127.0.0.1

### Shell Tasks Blocked

- Verify `allow_shell_tasks=true` in runner_config.json
- Check `shell_timeout_seconds` value
- Verify command syntax (runs via `bash -lc`)
- Check logs for timeout errors

### Signed-URL Failures

- CRM fetcher automatically handles this: splits date range in half
- Repeated until range is 1 day or succeeds
- If single-day range fails, backend cannot generate export
- Check API permissions and report size

## Release Process

### GitHub Actions Workflows

- `.github/workflows/ci.yml` - CI checks and tests
- `.github/workflows/build-windows.yml` - Cross-compiled Windows builds
- `.github/workflows/release-runner.yml` - Publishes runner_windows.zip
- `.github/workflows/release-crm.yml` - Publishes crm_windows.zip

### Version Tagging

Both release workflows use tag `v<version>` from `Cargo.toml`:

```toml
[package]
name = "crm_tool"
version = "1.1.0"
```

Run both workflows to publish a complete release with both executables.

## Documentation Files in `md/`

- **APPLICATION_SUMMARY.md** - High-level application overview
- **ARCHITECTURE.md** - Detailed module responsibilities
- **AUTH_FLOW.md** - Cognito SRP authentication sequence
- **BUILD_AND_RUN.md** - Build instructions and commands
- **CLI.md** - Runner GUI API endpoints
- **CONFIG.md** - Configuration file formats and fields
- **DOCKER.md** - Docker and dev container setup
- **DOWNLOADER.md** - CSV download implementation
- **FETCHER.md** - CRM API fetching with error handling
- **OPERATIONS.md** - Troubleshooting and operational procedures
- **README.md** - Central documentation index
- **SCHEDULER_TRAY.md** - Scheduler engine and tray integration

## Critical Rules for Development

1. **Never skip documentation**: Every code change requires doc updates in `md/`
2. **Test both executables**: Changes to shared modules (`src/lib.rs`, `src/crm/*`) require testing both runner and crm
3. **Validate Windows builds**: Use `cargo check --target x86_64-pc-windows-gnu` to validate cross-compilation
4. **Check log output**: Verify INFO-level logs are informative and ERROR-level logs include context
5. **Preserve backward compatibility**: Query parameter changes and config format changes must be backward compatible or include migration path
6. **Atomic operations**: Ensure task execution is atomic (no overlapping runs) via the run guard
7. **Error propagation**: CRM fetch errors should be per-report, not fail entire run
8. **Authentication safety**: Never log credentials; token cache is okay
9. **Path handling**: Resolve relative paths from executable directory, not current working directory
10. **Timeout handling**: Always use configured timeouts for external commands and network requests

## Working with Claude Code

When using Claude Code for this project:

1. **Read AGENTS.md first** - Understand mandatory documentation policy
2. **Read this CLAUDE.md** - Understand project architecture and patterns
3. **Read relevant `md/*.md` files** - Understand existing docs before modifying
4. **Update docs with code** - Never commit code changes without doc updates
5. **Run smoke tests** - Verify basic functionality after changes
6. **Check GitHub Actions** - Ensure workflows are updated if needed
7. **Ask for clarification** - Use AskUserQuestion if requirements are unclear

### ⚠️ MANDATORY DOCUMENTATION UPDATE PROCESS ⚠️

**For EVERY code modification, you MUST:**

1. **Identify all documentation files** that need updating (use mapping table above)
2. **Update documentation FIRST** before writing code
3. **Sync code with docs** - ensure they match exactly
4. **Verify examples** - all examples must be runnable
5. **Check accuracy** - function names, paths, and behavior must be correct
6. **Complete checklist** - answer all pre-commit questions with YES

**Failure to update documentation = INCOMPLETE TASK**

### Pre-Commit Safety Checklist

**⚠️ BEFORE EVERY COMMIT, VERIFY ALL OF THE FOLLOWING: ⚠️**

```markdown
- [ ] Code changed? → YES
- [ ] Matching docs updated in `md/`? → YES (MANDATORY)
- [ ] `AGENTS.md` read before making agent-authored changes? → YES (MANDATORY)
- [ ] Examples still valid? → YES (MANDATORY)
- [ ] New config fields documented? → YES (MANDATORY)
- [ ] New CLI flags documented? → YES (MANDATORY)
```

**🚨 WARNING: If ANY answer is `no`, DO NOT commit. Update docs first.**

---

### Enforcement & Consequences

**Non-compliance will result in:**
- ✅ PRs blocked by reviewers
- ✅ Changes reverted if docs are missing
- ✅ Build pipeline failures for missing documentation
- ✅ Tasks marked as incomplete

**Remember: A task is not complete until BOTH code AND docs are updated.**

---

## Getting Help

For operational issues: See `md/OPERATIONS.md`
For build problems: See `md/BUILD_AND_RUN.md`
For configuration: See `md/CONFIG.md`
For authentication flow: See `md/AUTH_FLOW.md`
For architecture details: See `md/ARCHITECTURE.md`
For API documentation: See `md/CLI.md`

When asking for help, include:
- Relevant log entries from `runner.log` or `crm.log`
- Config file contents (with secrets redacted)
- Error messages with context
- Steps to reproduce
