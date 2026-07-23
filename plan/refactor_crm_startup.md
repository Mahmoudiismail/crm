# CRM Startup Refactoring Plan

## 1. Current startup flow
1. `main` intercepts manifest and exits if requested.
2. CLI options are parsed (`CrmCliOptions::parse()`).
3. Configuration path is resolved (`resolve_config_path` using `String`).
4. Configuration is loaded (`AppConfig::load`) sequentially.
5. Logging is initialized using logging levels from the loaded configuration.
6. Startup messages are printed.
7. Date variables (`start_date`, `end_date`) are resolved.
8. `crm::run_once` is called.
9. `crm::run_once` re-loads the configuration (or takes the path), normalizes dates, sets up auth/client, creates download folder, and initiates fetching.

## 2. Proposed architecture
- Keep `main()` extremely thin, mostly delegating to a `run_crm_startup` function that returns `anyhow::Result<()>`.
- `CrmCliOptions` will be modified slightly if necessary, but CLI backward compatibility must be strictly preserved.
- `resolve_config_path` will be refactored to take `Option<&str>` and `&Path` and return a `PathBuf` instead of `String` to avoid unnecessary heap allocations.
- Improve error context in `main` (or `run_crm_startup`) by using `.context()` on steps like `setup_logging_with_levels`.
- Refactor `crm::run_once`: Currently `run_once` re-loads the `AppConfig` from `crm_config_path`. We can either pass the already-loaded `AppConfig` into `run_once`, or at least avoid re-loading it if possible, though since the token needs to be saved back, `AppConfig` is currently mutable. Wait, `AppConfig::load` is called twice: once in `main` to get logging levels, and once in `run_once`. Let's pass the already loaded `AppConfig` or just load it once.
- Using borrowed types: change `report: Vec<String>` to `report: &[String]` or similar if possible without massive breakage, or change `resolve_config_path` return type.
- Add descriptive `anyhow::Context` to initialization steps.

## 3. Files to modify
- `src/bin/crm.rs`: Extract `run_crm_startup`, refactor `resolve_config_path`, improve `main`.
- `src/crm/mod.rs`: Update `run_once` signature and body to accept `AppConfig` instead of the path, avoiding a double load. Or if it needs the path for saving, pass `AppConfig` and the path. Add error contexts to folder creation and fetching setup.
- `tests/crm_startup_integration_test.rs`: New file to cover startup integration tests (valid config, missing config, invalid config, invalid CLI, logging init failures).
- `md/CRM_STARTUP.md` or similar: Add a concise document describing the startup lifecycle.

## 4. Risks
- Changes in CLI parsing or configuration merging could break backward compatibility. We will ensure the tests cover these cases.
- Altering the `AppConfig` passing logic might break the `save()` behavior if not careful.

## 5. Testing strategy
- Integration tests in `tests/test_crm_startup.rs`.
- We will mock the external calls or only test the startup sequence up to the point of invoking `run_once` (we can test the argument validation and config loading).
- Run `cargo test --workspace` to ensure no existing tests break.

## 6. Rollback considerations
- If complexity increases without benefit, we revert specific changes to `crm.rs` or `crm/mod.rs`. The changes are isolated to these two files primarily.

## 7. Estimated implementation order
1. Create this plan document.
2. Refactor `resolve_config_path` in `src/bin/crm.rs` to return `PathBuf`.
3. Refactor `crm::run_once` in `src/crm/mod.rs` to accept an already loaded `mut AppConfig` to prevent double-parsing the config file.
4. Extract `run_crm_startup` in `src/bin/crm.rs`.
5. Add `anyhow::Context` to all fallible operations.
6. Write integration tests in `tests/test_crm_startup.rs`.
7. Write documentation (e.g. `md/crm_architecture.md` or update existing docs).
8. Ensure pre-commit steps (formatting, clippy, tests) pass.
