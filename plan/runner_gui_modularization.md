# Runner GUI Modularization Execution Plan

## Objective
Refactor the single, monolithic `src/runner/gui.rs` into a structured, modular `src/runner/gui/` directory without changing any functionality, behavior, execution logic, or frontend frameworks.

## Sub-Tasks
1.  **Preparation**
    -   Create new branch `feature/runner-gui-modularization`.
    -   Create `src/runner/gui/` directory structure.
    -   Move `form_script.js` to `src/runner/gui/`.

2.  **Module Creation & File Separation**
    -   **`mod.rs`**: Define server entry point (`start_gui_server`, `run_server`) and export submodules.
    -   **`routes.rs`**: Handle HTTP reading, connection loops, and route dispatching (`route_request`).
    -   **`handlers.rs`**: Functions handling domain actions for task creation/editing/running (split from route conditions).
    -   **`api.rs`**: Functions handling JSON endpoints.
    -   **`templates.rs`**: HTML rendering logic (`render_dashboard`, `render_task_form`, `html_page`, etc.).
    -   **`forms.rs`**: Request payload parsing, input conversion, schedule/command string parsing (`build_task_from_values`, etc.).
    -   **`assets.rs`**: Handle embedded JS (`form_script.js`).
    -   **`helpers.rs`**: Common string manipulation and data utilities (`escape_html`, `split_path_and_query`).
    -   **`response.rs`**: HTTP response builders and HTML fragments (`render_error_page`, `render_toast`).
    -   **`validation.rs`**: Isolated data validation (if any, extracted from forms logic).

3.  **Relocate Tests**
    -   Move existing routing tests to `routes.rs`.
    -   Move schedule/duration parsing tests to `forms.rs`.
    -   Move helper logic tests to `helpers.rs`.

4.  **Verification**
    -   Run `cargo fmt`.
    -   Run `cargo clippy --workspace --all-targets --all-features`.
    -   Run `cargo test --workspace`.

5.  **Submission**
    -   Commit changes.
    -   Create draft pull request.
