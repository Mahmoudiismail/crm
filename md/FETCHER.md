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

- `from_date`
- `to_date`
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

`split_monthly(from, to)` breaks large call windows into monthly segments.

Example:

- Input: `2026-01-01` to `2026-03-15`
- Output batches:
  - `2026-01-01`..`2026-01-31`
  - `2026-02-01`..`2026-02-28`
  - `2026-03-01`..`2026-03-15`

## Concurrency

- Each report or call batch runs in its own `tokio::spawn` task.
- Completion is coordinated with `join_all`.
- Partial failures do not block other tasks.

## Result Shape

- Non-call reports stored as direct object values under keys.
- Calls aggregated into a `calls` array.
- Failed tasks contribute `{"error": "..."}` payloads.

## URL Extraction

`extract_urls(results)` scans payloads for `data.url` and returns `(report_key, url)` tuples for downloader usage.

## Test Coverage

Unit tests validate:

- monthly split correctness,
- leap-year handling,
- month-end logic.
