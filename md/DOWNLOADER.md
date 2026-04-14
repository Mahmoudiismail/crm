# Downloader Service

Implementation: `src/crm/downloader.rs`

## Purpose

Downloads CSV files from signed URLs returned by report APIs.

## Public Function

`download_csv(client, url, report_key, target_dir) -> Result<String>`

Behavior:

1. Extract readable filename from URL path.
2. URL-decode filename.
3. Ensure `target_dir` exists.
4. GET request with timeout and identity encoding.
5. Stream bytes to file asynchronously.
6. Flush and return filename.

## HTTP Details

- Method: GET
- Header: `accept-encoding: identity`
- Timeout: 60 seconds

## File Placement

- Path is provided by caller.
- In main workflow, target is `<exe_dir>/Downloads`.

## Error Conditions

- Invalid URL-encoded filename.
- HTTP non-success status.
- Stream read/write failures.
- Directory/file creation failures.

## Utility Function

`extract_filename(url)`:

- strips query string,
- takes trailing path segment,
- decodes `%xx` sequences,
- falls back to `download.csv` when empty.

## Tests

Covers:

- encoded filename extraction,
- no-query URLs.
