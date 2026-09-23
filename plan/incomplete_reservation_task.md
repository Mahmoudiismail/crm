# Execution Plan: Add incomplete reservation task to CRM app

## Objective
Add a new task named `incomplete_reservation` to the CRM application. The task should fetch incomplete booking reservations and update them in bulk using a PATCH API request, instead of downloading a CSV file.

## Steps Completed

1. **Update `AppManifest` in `src/bin/crm.rs`**
   - Added `"incomplete_reservation"` to the list of acceptable options for the `--report` CLI argument to allow external clients and the runner GUI to trigger the task.

2. **Add API payload structs in `src/crm/fetcher.rs`**
   - Added `IncompleteReservationItem`, `IncompleteReservationData`, `IncompleteReservationResponse` for parsing the GET API responses.
   - Added `BulkTicketPayload` for serializing the bulk PATCH payload to the CRM backend.

3. **Implement logic for `incomplete_reservation`**
   - Defined the task inside `report_defs()`.
   - Created the `fetch_and_update_incomplete_reservations` function which:
     - Iterates over two specific `status_id`s representing incomplete states.
     - Fetches tickets via the GET `task/ticket` endpoint (with the required `event_name` and configurable `limit`).
     - Gathers all matching `id`s.
     - Performs a single PATCH request to `task/bulk-ticket` to update their status.
     - Returns the result directly as part of the overall application results map.

4. **Add configuration setting to `AppConfig`**
   - Added `incomplete_reservation_limit` (default: 10000) to `src/crm/config.rs`.

5. **Update Documentation**
   - Added `md/CRM_INCOMPLETE_RESERVATION.md` explaining the feature behavior.
   - Updated `md/CONFIG.md` and `md/FETCHER.md` to reference the new task type and config parameters.
   - Executed tests, lint, and formatting.
