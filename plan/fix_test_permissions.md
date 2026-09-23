1. **Analyze the Issue**:
   - The test failures are due to access denied errors (`os error 5`) when attempting to atomically write to `NamedTempFile`s.
   - `NamedTempFile` files are open, so `temp_file.persist()` fails on Windows when used by `atomic_write` since it tries to rename a file that is in use.
   - Using `tempfile::tempdir` instead of `tempfile::NamedTempFile` creates a temp directory where we can create our own file, preventing the file lock from `NamedTempFile` and resolving the `os error 5`.
2. **Apply the Fix**:
   - In `src/crm/config.rs` inside `test_load_merges_partial_config`, replace the use of `NamedTempFile` with `tempfile::tempdir()` and write directly to a `config.json` inside it.
   - In `src/runner/gui.rs` inside `test_start_gui_server_routing`, perform the same replacement, writing to a file inside `tempfile::tempdir()`.
3. **Run Pre-Commit Checks**: Ensure tests pass and the changes are documented as per `AGENTS.md`.
4. **Submit**: Submit the changes.
