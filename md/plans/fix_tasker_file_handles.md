1. **Analyze the Issue**:
   - The test failures `test_task_filtering_logic_out_of_bounds`, `test_empty_tasks_panics_on_start`, and `test_task_filtering_logic_valid_index` in `src/bin/tasker.rs` on Windows CI are also likely caused by keeping the `std::fs::File` open while running `run_app`. When `run_app` attempts to load and update the config (which involves an atomic write/rename on the same file), it fails on Windows because the file is still open by the test (due to the un-dropped `file` handle created via `std::fs::File::create`).
   - Using `drop(file)` explicitly after writing, or just not binding it and letting it drop, will free the lock before `run_app` is called.
2. **Apply the Fix**:
   - In the three tests, add `drop(file);` immediately after `file.sync_all().unwrap();`.
3. **Run Tests**: Verify tests pass using `cargo test --bin tasker`.
4. **Submit**: Submit the changes.
