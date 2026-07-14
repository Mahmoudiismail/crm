# CSV Parsing Fixes

- Centralized `csv::ReaderBuilder` configuration in `src/utils.rs` via `build_csv_reader`.
- Added `.flexible(true)` to gracefully handle varying-length column records.
- Replaced the full-file diagnostic logging loop with `generate_csv_diagnostic_context` (also in `src/utils.rs`), limiting the context window to ±20 lines around the error line to reduce log size while retaining relevant debugging data.
