# CRM Startup Lifecycle

This document describes the startup sequence and lifecycle of the `crm` application binary (`src/bin/crm.rs`).

## Startup Flow

1. **Manifest Interception:**
   The application begins by calling `intercept_manifest(get_manifest())`. If the `--manifest` flag is passed (used internally by the runner), it prints the `AppManifest` as JSON and exits with code 0.

2. **CLI Parsing:**
   The arguments are parsed using `clap::Parser`. This step validates input arguments (such as `--report`, `--start-date`, `--config`) and will fail early if invalid arguments are provided.

3. **Startup Extraction (`run_crm_startup`):**
   The `main` function is kept as a thin wrapper that immediately delegates the rest of the startup and execution sequence to `run_crm_startup`.

4. **Path Resolution:**
   The application resolves the configuration path using `resolve_config_path()`. If a relative path is passed via `--config`, it resolves it against the executable's directory. If no config path is passed, it defaults to `config.json` next to the executable.

5. **Configuration Loading:**
   The `AppConfig` is loaded into memory. If the configuration file doesn't exist, it is created using default values. If keys are missing, the file is updated dynamically to include them. All operations include contextual error handling using `anyhow::Context`.

6. **Logging Initialization:**
   The application initializes logging (`setup_logging_with_levels`) by reading the `log_stdout_level` and `log_file_level` values from the loaded configuration.

7. **Date Resolution:**
   The `--start-date` and `--end-date` variables (if passed) are parsed and dynamic string patterns (like "today", "eomonth") are normalized using `replace_date_vars`.

8. **Execution:**
   Finally, `crm::run_once` is executed. The already parsed and mutable configuration instance is passed into the function to prevent double parsing. After ensuring the client and authentication are valid, the configuration is saved back to disk to persist any refreshed tokens.

## Refactoring Notes

The CRM startup code was refactored to achieve the following:
- Kept `main()` as a very thin wrapper.
- Extracted core initializations to `run_crm_startup`.
- Eliminated redundant `String` parsing by utilizing borrowed strings and `PathBuf`.
- Addressed a double configuration-load bug by passing a single `&mut AppConfig` downwards.
- Enhanced fallible steps using `anyhow::Context`.
