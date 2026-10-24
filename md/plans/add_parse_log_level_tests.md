# Plan: Add Unit Tests for `parse_log_level`

## Overview
The `parse_log_level` function in `src/utils.rs` converts a string slice into a `tracing_subscriber::filter::LevelFilter` variant or returns an `anyhow::Error` for invalid strings. This plan outlines adding comprehensive unit tests to ensure all valid mappings, case-insensitive inputs, and invalid string error handling are thoroughly verified.

## Scope of Changes

1. **Test Coverage Additions in `src/utils.rs`**:
   - Add unit tests inside `mod tests` in `src/utils.rs`.
   - Test happy paths for all 6 supported log levels: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`, `"off"`.
   - Test case insensitivity (e.g., `"TRACE"`, `"Debug"`, `"InFo"`, `"OFF"`).
   - Test invalid strings (e.g., `"invalid"`, `"verbose"`, `""`, `" info "`, `"123"`), asserting that an error is returned containing the expected error message string.

2. **Verification**:
   - Run `cargo test utils::tests` and full `cargo test`.
   - Perform a intentional mutation check to verify test failure when logic is altered.

3. **Documentation**:
   - Keep this plan updated in `plan/add_parse_log_level_tests.md`.
