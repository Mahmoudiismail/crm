# Contributing to CRM Tool

First off, thank you for considering contributing to the CRM Tool! It's people like you that make this tool great.

## Development Environment Setup

1. **Install Rust:** Ensure you have Rust installed. We require Rust 1.70+ (Edition 2021). You can install it via [rustup](https://rustup.rs/).
   - The primary target for this project is Windows (`x86_64-pc-windows-msvc`), but development can largely be done on other OSes.
2. **Clone the repository:**
   ```bash
   git clone https://github.com/your-repo/crm_tool.git
   cd crm_tool
   ```

## Repository Structure

- `src/bin/`: Entry points for the executable binaries (`crm`, `runner`, `yasweb`, `tasker`, `wcxx`).
- `src/crm/`: CRM domain logic (fetching, authentication).
- `src/runner/`: Task runner orchestration and GUI scheduler.
- `src/tasker/`: Background task processing (CSV analysis, emailing, Excel automation).
- `src/yasweb/`: Headless browser automation.
- `src/utils.rs`: Shared utilities across binaries.
- `md/`: Extensive architecture and operational documentation.

## Coding Standards

- **Rust Version:** 1.70+
- **Safety:** `#![forbid(unsafe_code)]` is enforced at the crate root. Do not use `unsafe`.
- **Error Handling:** Use `anyhow` for applications. Return explicit `Result`s for any function that parses parameters or resolves paths. Avoid lazy defaults (`unwrap_or_default`) unless semantically correct. Avoid `.unwrap()` and `.expect()` in domain logic.
- **Concurrency:** Prefer `tokio::spawn`. Manage state safely with tight `tokio::sync::Mutex` scopes. Do not use blocking I/O (e.g., `std::fs`) inside async contexts; use `tokio::fs`.

## Developer Workflow

Use standard `cargo` commands for development:

- **Build:** `cargo build` (use `--release` for optimized builds).
- **Format:** `cargo fmt` (enforced by CI).
- **Lint:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` (enforced by CI).
- **Test:** `cargo test --workspace --all-targets --all-features`.
- **Benchmark:** `cargo bench`.
- **Documentation:** `cargo doc --no-deps`.

## How to Add Tests

When a bug is fixed or a new feature is added, a corresponding test case **MUST** be created to prevent future regressions.

- **Unit tests:** Place them in the same file as the code they test in a `#[cfg(test)] mod tests` block.
- **Integration tests:** Place them in the `tests/` directory.
- Use mock datasets located in `TestingDownloads/` instead of downloading remote files during tests.
- Prefer testing externally visible behavior over internal implementation.

## How to Update Documentation

**Documentation is Mandatory:** After every code, configuration, dependency, or script change that impacts behavior, you MUST update the relevant markdown files in the `md/` directory in the *same pull request*.

- Read `md/AI_DOC_POLICY.md` for specific mapping rules (e.g., changing `src/bin/runner.rs` requires updating `APPLICATION_SUMMARY.md`, `ARCHITECTURE.md`, etc.).
- Ensure examples and commands in documentation remain runnable and accurate.
- AI Agents must always consult `AGENTS.md` before operating on this repository.

## Pull Request Workflow

1. Create a new branch for your feature or fix.
2. Implement your changes.
3. Write relevant tests.
4. Run `cargo fmt`, `cargo clippy`, and `cargo test`.
5. **Update all relevant documentation in `md/`.**
6. Push your branch and open a Pull Request.
7. Ensure CI checks pass.

## Commit Expectations

- Keep commits logically grouped and incremental.
- Use clear, descriptive commit messages (e.g., "Fix XYZ bug in runner" or "Add new --flag to tasker").
- Include both code changes and corresponding documentation updates in the same commit where possible.
