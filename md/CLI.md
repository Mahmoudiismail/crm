# CLI Reference

This document describes the command-line options declared by the current binary entry points. Run an executable with --help for the parser help for the exact build.

## Runner

```powershell
runner.exe [--config <PATH>]
```

- --config PATH selects Runner's JSON configuration. Relative paths are resolved from the executable directory; absolute paths are used unchanged.
- --manifest is a hidden integration option that prints Runner's AppManifest and exits.

## CRM

```powershell
crm.exe [--report <REPORTS>] [--config <PATH>] [--start-date <DATE>] [--end-date <DATE>] [--custom-download-folder <PATH>]
```

- --report REPORTS accepts comma-separated report names and defaults to tickets.
- --config PATH selects the CRM configuration file.
- --start-date DATE and --end-date DATE override configured date boundaries. Flexible formats and dynamic expressions are described in [Fetcher](./FETCHER.md).
- --custom-download-folder PATH overrides the download folder for this run; it does not persist the override.
- --manifest is a hidden integration option.

## Yasweb

```powershell
yasweb.exe --name <REPORT[,REPORT...]> [OPTIONS]
```

- --name selects one or more configured report names.
- --type selects the report type when applicable.
- --config selects the config file; default is yasweb_config.json.
- --filters supplies filter values in the configured CLI format.
- --start-date and --end-date supply date boundaries mapped through each report's start_date_key and end_date_key. Missing date-key mappings mean the corresponding date filter is skipped.
- --download-path overrides the output directory.
- --add-time-to-file adds a time suffix to output filenames.
- --monthly is a hidden compatibility option.
- --manifest is a hidden integration option.

## Tasker

```powershell
tasker.exe [--config <PATH>] [--task <INDEX>] [--only-call-center] [--send-exceptions] [--start-date <DATE>]
```

- --config PATH selects tasker_config.json. Relative paths are resolved from the executable directory.
- A legacy positional config path is still accepted and is interpreted relative to the current working directory.
- --task INDEX runs one configured task using a 1-based index.
- --only-call-center runs the Call Center-specific email path for the selected task and skips standard team/branch email generation.
- --send-exceptions selects the configured exception-ticket reporting path.
- --start-date DATE overrides the selected task's start date and requires --task.
- --manifest is a hidden integration option.

Tasker executes configured tasks in order and returns a non-zero result when a configured task fails. See [Tasker](./TASKER.md).

## CRM Updater

```powershell
crm_updater.exe [--config <PATH>] [--update-only] [--logs-only]
```

- --config PATH selects updater_config.json. Relative paths are resolved from the executable directory.
- --update-only runs only the update pipeline.
- --logs-only runs only local report/log collection.
- With neither selector, collection runs before the update pipeline.
- --manifest prints the AppManifest and exits.

## AppManifest

Runner-compatible applications expose a hidden --manifest option. It prints the argument schema as JSON and exits successfully. See [Manifest Schema](./MANIFEST_SCHEMA.md).
