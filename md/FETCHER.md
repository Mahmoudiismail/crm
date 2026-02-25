# Report Fetching

## Endpoints

| Report   | Endpoint                 | Extra Query Params      |
|----------|--------------------------|-------------------------|
| tickets  | `download-ticket-data`   | `type=ticket_report`    |
| calls    | `download-call-log-data` | *(none)*                |
| leads    | `download-lead-data`     | `type=lead_report`      |

## Common Query Parameters

All requests include:
- `from_date` — Start date
- `to_date` — End date
- `email` — User email

## Request Headers

```
account_id: <from config>
app-timezone-plus-minutes: <from config>
application_id: <from config>
auth-type: cognito
authorization: Bearer <token>
content-type: application/json
accept: */*
```

## Report Selection

| `--report` | Reports fetched              |
|------------|------------------------------|
| `all`      | tickets, calls, leads        |
| `tickets`  | tickets only                 |
| `calls`    | call logs only               |
| `leads`    | leads only                   |
| `none`     | no reports (skip fetching)   |

## Call Log Monthly Batching

Call logs are split into monthly batches to avoid API timeouts and large responses.

### Algorithm

```
Input: calls_from_date, to_date
Output: [(month_start, month_end), ...]

cursor = calls_from_date
while cursor <= to_date:
    month_end = last_day_of_month(cursor)
    batch_end = min(month_end, to_date)
    emit (cursor, batch_end)
    cursor = first_day_of_next_month(cursor)
```

### Example

`2026-01-01` to `2026-03-15` produces:
1. `2026-01-01` → `2026-01-31`
2. `2026-02-01` → `2026-02-28`
3. `2026-03-01` → `2026-03-15`

### Edge Cases

- **Leap year**: Feb 2024 → `2024-02-29`
- **Single day range**: `2026-01-15` to `2026-01-15` → 1 batch
- **Same month**: `2026-03-05` to `2026-03-20` → 1 batch

## Concurrency

All report requests run concurrently:
- Each report type is a `tokio::spawn` task
- All call log batches are individual concurrent tasks
- Results collected via `futures::join_all`

```
┌─ Tickets request ──────────────────────┐
├─ Leads request ────────────────────────┤  All concurrent
├─ Calls batch 1 (Jan) ─────────────────┤
├─ Calls batch 2 (Feb) ─────────────────┤
└─ Calls batch 3 (Mar) ─────────────────┘
```

## API Response Shape

```json
{
  "statusCode": 200,
  "message": "success",
  "data": {
    "url": "https://crm.fakeeh.care/crm-files/...",
    "progress": ["..."]
  },
  "error": false
}
```

## Output Format

```json
{
  "tickets": { "statusCode": 200, "data": { "url": "..." } },
  "calls": [
    { "statusCode": 200, "data": { "url": "..." } },
    { "statusCode": 200, "data": { "url": "..." } }
  ],
  "leads": { "statusCode": 200, "data": { "url": "..." } }
}
```

Call logs are always an array (one entry per monthly batch).

## Error Handling

- Per-report errors don't abort other reports
- Errors stored as: `{"error": "error message"}`
- Task join errors are logged and skipped

## CSV Download

When `download_csv = true`:
1. Extract `data.url` from each report response
2. URL-decode the filename from the URL path
3. Stream download with `accept-encoding: identity`
4. Save to current working directory
5. 60-second timeout per download
