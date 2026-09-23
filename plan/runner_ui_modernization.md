# Runner UI Modernization & UX Refactoring Plan

## Phase 1: Setup UI Foundations
- Create `src/runner/gui/icons.rs` to provide lightweight SVG icon helper functions (using Heroicons).
- Create `src/runner/gui/components.rs` to provide composable UI helpers (e.g., `layout`, `sidebar`, `top_nav`, `card`, `stat_card`, `button`, `badge`, `data_table`, `form_group`, `input`, `empty_state`).
- Update `src/runner/gui/mod.rs` to export these new modules.

## Phase 2: Refactor Layout & Views
- Refactor `src/runner/gui/templates.rs` to use the new composable components instead of large, duplicated `format!` macros.
- Implement the new modern admin dashboard layout (Sidebar + Main Content + Top Nav).
- Overhaul the Dashboard view to improve information density (active tasks, schedules, stats).
- Overhaul forms (New/Edit Task, Apps) for better grouping, labels, validation feedback, and readability.
- Overhaul tables to improve spacing, overflow handling, and empty states.
- Improve error pages and toast notification visuals.

## Phase 3: JavaScript Refactoring
- Clean up `src/runner/assets/js/` (api.js, common.js, forms.js, notifications.js, validation.js).
- Eliminate global variables where possible and rely on `DOMContentLoaded` initialization.
- Implement delegated event handlers.
- Add JS logic for the responsive mobile sidebar toggle.
- Ensure all inline `<script>` tags in Rust templates are migrated to external JS modules.

## Phase 4: Pre-commit Steps
- Ensure proper testing, verification, review, and reflection are done (cargo fmt, clippy, tests).

## Phase 5: Submission
- Commit and submit the changes to `feature/runner-ui-modernization`.
