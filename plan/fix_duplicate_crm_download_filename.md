# Execution Plan - Fix Duplicate CRM Download Filenames

## Context & Problem
When the CRM API fails to generate a signed URL for a large date range (HTTP 500), `crm.exe` recursively splits the date range in half and fetches the chunks concurrently.
When two or more parallel chunks complete at the exact same millisecond or return signed URLs with identical timestamps/filenames (e.g., `ticket_report_1788814899432.csv`), the downloads overwrite each other in the `Downloads` directory.
As a result, some report chunks are lost (e.g. only 3 files found instead of 4), causing missing ticket records in downstream processing (`csv_task`).

## Key Requirements
1. **Prevent File Overwrites**: Prevent concurrent or duplicate downloads from overwriting each other when filenames match.
2. **Maintain Compatibility**:
   - The file name MUST preserve its original starting prefix (e.g., `ticket_report_`, `call_logs_`, `lead_report_`, `users_`) so `tasker` (`csv_task`), retention cleanup (`cleanup_old_reports`), and `has_recent_download` continue working without any changes.
   - The extension MUST remain `.csv`.
3. **Collision Disambiguation**: When `extract_filename(url)` yields a filename that already exists in `target_dir` (or has an active `.tmp` download), append `_1`, `_2`, etc. before the `.csv` extension (e.g., `ticket_report_1788814899432_1.csv`).

## Proposed Changes

### `src/crm/downloader.rs`
- Update `download_csv` (and helper logic) to handle duplicate filenames safely:
  - Extract the raw filename from URL (e.g., `ticket_report_1788814899432.csv`).
  - Check if `target_dir.join(&filename)` or `target_dir.join(format!("{}.tmp", filename))` already exists.
  - If it exists, append a collision suffix `_1`, `_2`, ... before `.csv` until an unused filename is found.
  - To prevent race conditions between concurrent download tasks in the same process, synchronize filename reservation or handle atomic temp file creation with `tokio::fs::OpenOptions` (`create_new(true)`).

### Tests
- Add unit tests in `src/crm/downloader.rs` verifying that attempting to download/save files with identical names results in distinct, non-overlapping filenames preserving `ticket_report_...csv`.
