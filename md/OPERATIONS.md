# Operations

## Logging & Observability

### Log Levels
- By default, all applications write `TRACE` level logs to the file (`<app_name>.log`) and `DEBUG` level logs to stdout.
- Log levels are now controlled dynamically via `logging_config.json` in the binary directory.

### Log Rotation & Retention
- Log files automatically roll over **daily**.
- Logs older than **7 days** are automatically cleaned up on application startup.

### Cross-App Correlation
- When `runner` spawns external child tasks, it generates a unique UUID `execution_id`.
- This ID is passed to child apps via the `CRM_CORRELATION_ID` environment variable.
- You can filter logs by `Execution started with Correlation ID: <uuid>` to trace workflows end-to-end across `runner`, `crm`, and `yasweb`.

### Expected Errors
- `500 Internal Server Error: Failed to generate signed url...`: This error occurs when CRM hits the S3 file size limit. The fetcher intelligently catches this error and automatically chunks the download into smaller date ranges, so it is **expected behavior** and does not require manual intervention.

### Testing Rule
- **Mandatory Test Requirement:** Whenever an issue or bug is found and fixed, a corresponding test case MUST be created to ensure the issue is avoided in the future.
