# Incomplete Reservation Task (CRM)

The `crm` binary supports a task for handling incomplete reservations. This is triggered by including `incomplete_reservation` in the `--report` argument.

## Execution Flow

When triggered, the task does not download a CSV report. Instead, it performs bulk ticket updates directly via the CRM API:

1. **Fetch Incomplete Reservations**:
   It makes two parallel-like GET requests to `task/ticket` targeting two specific `status_id`s representing incomplete reservation states:
   - `f38fafc3-bd3f-4dd0-ac6a-09dd7d51b6a0` (Follow-up)
   - `275688a4-df41-4725-ae56-fd2deab9c1c9` (Other/Pending)
   Both requests are filtered with `event_name=cc_booking_incomplete_reservation`. The limit for these fetches is defined by `incomplete_reservation_limit` (default: 10,000).

2. **Extract Identifiers**:
   It extracts the `id` from all matching ticket items across both requests.

3. **Bulk Update**:
   If IDs are found, it dispatches a single PATCH request to `task/bulk-ticket` using a payload containing the gathered IDs and a hardcoded target `status_id` (`46282444-7951-42eb-a27e-b2bc65c53727`).

4. **Result Logging**:
   The API response from the PATCH request is returned and logged within the standard report fetcher JSON output.

## Configuration Options

- `incomplete_reservation_limit`: (Type: `u32`, Default: `10000`). Specifies the `limit` query parameter passed to the `GET` API requests when discovering incomplete tickets. If the number of incomplete reservations exceeds this limit, they will not be fetched in a single run.
