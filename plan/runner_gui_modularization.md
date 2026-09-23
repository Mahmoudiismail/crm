# Execution Plan: Runner GUI Modularization (Session 5)

## Objective
Refactor `src/runner/gui.rs` into a modular architecture within `src/runner/gui/` without changing behavior or modifying embedded JavaScript.

## 1. Create Module Structure
- `src/runner/gui/mod.rs` - Re-exports the public `start_gui_server` and handles the core server loop (`run_server`, `HttpRequest`, `read_http_request`).
- `src/runner/gui/routes.rs` - Houses `route_request` and acts as the entry point for routing, delegating to handlers.
- `src/runner/gui/handlers.rs` - Houses route handlers (e.g. `handle_dashboard`, `handle_status_api`, `handle_create_task`, etc.) to keep routes clean.
- `src/runner/gui/templates.rs` - Houses HTML generation functions (`render_dashboard`, `render_task_form`, `html_page`, `metric_card`, etc.).
- `src/runner/gui/forms.rs` - Houses form parsing and validation logic (`build_task_from_values`, `parse_schedules_text`, `parse_duration_text`, `parse_shell_commands_text`).
- `src/runner/gui/helpers.rs` - Houses shared utilities (`escape_html`, `js_escape`, `parse_query_string`, `url_decode`, `split_path_and_query`).

## 2. Refactor Steps
- Read the entire `gui.rs`.
- Extract functions to their respective new modules.
- Ensure correct `pub(crate)` visibility across the new modules.
- Move existing tests from `gui.rs` to the modules they belong to.
- Update `src/runner/mod.rs` to point to the new `gui` directory instead of the `gui.rs` file.
- Validate by running `cargo fmt`, `cargo clippy`, and `cargo test`.
- Remove the old `gui.rs` file.

## 3. Pre-commit Steps
- Run `pre_commit_instructions` tool to make sure proper testing, verification, review, and reflection are done.

## 4. Final Verification
- Re-run all cargo commands one last time to ensure compilation and correct behavior.
