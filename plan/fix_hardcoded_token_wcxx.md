# Plan: Fix Hardcoded Token Vulnerability in `wcxx.rs`

## Understanding the Issue
The default configuration object in `src/bin/wcxx.rs` initializes a `token` field with a placeholder string `"YOUR_BEARER_TOKEN_HERE"`. Hardcoding potential secret formats or placeholder strings in code can trigger static code analysis tools (SAST) and is an anti-pattern. While this specific string isn't a *real* token, the practice is a risk and is flagged as a vulnerability.

## Execution Steps

1. **Modify `src/bin/wcxx.rs`**:
   - Locate the `default_config` instantiation around line 87.
   - Update `token: "YOUR_BEARER_TOKEN_HERE".to_string()` to use `std::env::var("WCXX_TOKEN").unwrap_or_default()`. This allows loading a token from the environment securely, falling back to an empty string `""` if not set.
   - We will keep the check for `"YOUR_BEARER_TOKEN_HERE"` in `Config::validate()` to ensure any existing users who haven't updated their `wcxx_config.json` don't mysteriously start getting authentication errors.

2. **Add Tests to `src/bin/wcxx.rs`**:
   - Add a `#[cfg(test)]` block to test the `validate()` method of `Config`.
   - Test that empty token (`""`) fails.
   - Test that legacy placeholder token (`"YOUR_BEARER_TOKEN_HERE"`) fails.
   - Test that a valid token passes.

3. **Update Documentation**:
   - Update `md/WCXX.md` configuration example to show an empty string `""` for the token instead of `"YOUR_BEARER_TOKEN_HERE"`.

4. **Verify**:
   - Run `cargo fmt`, `cargo clippy`, and `cargo test`.
   - Ensure the tests pass.

5. **Commit and Submit**:
   - Run `pre_commit_instructions`.
   - Submit the PR with the required security title and description formatting.
