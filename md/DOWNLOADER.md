# Downloader Service

Implementation: `src/crm/downloader.rs`

## Purpose

Downloads CSV files from signed URLs returned by report APIs.

When the fetcher splits a report after a `Failed to generate signed url` API response, each successful split payload contributes its own signed URL. The downloader treats each URL independently and writes each CSV to the target directory. To optimize performance, multiple CSV files are downloaded concurrently.

## Public Functions

`download_csv(client, url, report_key, target_dir) -> Result<String>`

Behavior:

1. Extract readable filename from URL path.
2. URL-decode filename.
3. Ensure `target_dir` exists.
4. GET request with timeout and identity encoding.
5. Stream bytes to a uniquely-created temporary file asynchronously using a `BufWriter` (128 KiB buffer) to reduce write syscalls on large downloads.
6. Remove the temporary file on HTTP, stream, write, flush, or rename failure.
7. Rename the verified completed temporary file to its final destination and return the filename.

## HTTP Details

- Method: GET
- Header: `accept-encoding: identity`
- Timeout: 60 seconds per HTTP attempt.
- The fetcher retries a signed-URL download up to three times, then requests a fresh URL for the same report range if those attempts fail.
- A failed download is propagated to the overall fetch operation; partial temporary files are removed rather than left as report CSVs.

## File Placement

- Path is provided by caller.
- In main workflow, target is `<exe_dir>/Downloads`.

## Error Conditions

- Invalid URL-encoded filename.
- HTTP non-success status.
- Stream read/write failures.
- Directory/file creation failures.

## Base64 Payload Processing

`process_base64_payload(payload: &str, report_key: &str, target_dir: &Path) -> Result<String>`

This method processes raw base64 CSV data instead of signed URLs. It:
1. Strips any surrounding JSON quotes.
2. Decodes the Base64 string into bytes.
3. Removes the UTF-8 Byte Order Mark (BOM) if present.
4. Validates that the payload is valid CSV (extracting row/col counts for logging).
5. Ensures `target_dir` exists.
6. Writes the file to disk using the format `<report_key>_<timestamp>.csv`.
7. Returns the generated filename.

This method is primarily utilized by the `users` report endpoint.

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
