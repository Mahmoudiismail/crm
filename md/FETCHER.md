# Fetcher Service

Implementation: `src/crm/fetcher.rs`

## Overview

Fetcher retrieves report metadata JSON from CRM endpoints using the Cognito bearer token.

## Report Definitions

- `tickets` -> endpoint `download-ticket-data` + `type=ticket_report`
- `calls` -> endpoint `download-call-log-data`
- `leads` -> endpoint `download-lead-data` + `type=lead_report`

## Selection Logic

Based on `ReportType`:

- `All`: fetch all
- `Tickets`: only tickets
- `Calls`: only calls
- `Leads`: only leads
- `None`: return empty object immediately

## Request Metadata

Query parameters:

- `from_date` (can be overridden via CLI `--start-date`). Supports dynamic variables: `today`, `yesterday`, `tomorrow`.
- `to_date` (can be overridden via CLI `--end-date`). Supports dynamic variables: `today`, `yesterday`, `tomorrow`, `eomonth` (end of month for start date).
- `email`
- plus report-specific extras

Headers:

- `account_id`
- `app-timezone-plus-minutes`
- `application_id`
- `auth-type: cognito`
- `authorization: Bearer <token>`
- `content-type: application/json`
- `accept: */*`

## Calls Monthly Batching

`split_monthly(from, to)` breaks large call windows into monthly segments before the first request.

Example:

- Input: `2026-01-01` to `2026-03-15`
- Output batches:
  - `2026-01-01`..`2026-01-31`
  - `2026-02-01`..`2026-02-28`
  - `2026-03-01`..`2026-03-15`

## Signed URL Failure Range Splitting

All report fetches use `fetch_with_signed_url_split(...)`.

Behavior:

1. Try the requested date range first.
2. If the CRM API error text contains `Failed to generate signed url`, split that date range in half.
3. Retry each half, recursively splitting again when the same signed-URL error occurs.
4. Stop splitting at a single-day range; if that still fails, return the API error with single-day context.

Operational impact:

- Large ticket or lead exports can still be fetched when the backend refuses to sign one oversized CSV.
- Call logs are still pre-split monthly, and any failing monthly batch can be split further.
- A split report returns an array of successful API payloads instead of a single payload.
- Non-signed-URL failures are not retried by this splitter.

## Concurrency

- Each report or call batch runs in its own `tokio::spawn` task.
- Completion is coordinated with `join_all`.
- Partial failures do not block other tasks.

## Result Shape

- Non-call reports stored as direct object values under keys.
- Calls aggregated into a `calls` array.
- Failed tasks contribute `{"error": "..."}` payloads.

## URL Extraction

`extract_urls(results)` scans payloads and arrays for `data.url` and returns `(report_key, url)` tuples for downloader usage.

For performance and memory safety, URL extraction leverages an iterative stack algorithm. The array iteration logic includes a pre-filtering mechanism that skips primitive values (numbers, booleans, strings, nulls) when traversing `Value::Array`, mitigating recursive function overhead and preventing performance regressions or stack overflows when dealing with deeply nested JSON structures or multi-megabyte scalar arrays.

This supports both normal single-payload reports and split reports such as:

```json
{
  "tickets": [
    {"data": {"url": "https://example.com/tickets-part-1.csv"}},
    {"data": {"url": "https://example.com/tickets-part-2.csv"}}
  ]
}
```

## Test Coverage

Unit tests validate:

- monthly split correctness,
- leap-year handling,
- month-end logic,
- recursive date-range halving,
- URL extraction from split report arrays.

## Performance Optimization

To minimize performance overhead from reference counting when setting up many asynchronous batch tasks, task context variables (`token`, `base_url`, `email`, `account_id`, `application_id`, `tz`) are consolidated into a single `FetchContext` struct, requiring only one `Arc` wrapper to be cloned per spawned task.
