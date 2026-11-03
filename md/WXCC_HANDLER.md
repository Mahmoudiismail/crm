# WXCC Handler

## Purpose and constraints

WXCC Handler is a Windows-only, command-based process application in the CRM Rust repository. It is Rust-first and may invoke package-owned PowerShell scripts for Windows-specific operations when required.

- Use documented Cisco Webex / Webex Contact Center APIs only.
- Do not use browser automation, undocumented endpoints, or UI scraping.
- Keep the implementation process-based; Runner launches it as an independent executable.
- OAuth credentials and tokens are stored in local JSON for V1. DPAPI is intentionally not used.
- Never print client secrets, access tokens, refresh tokens, or authorization codes in logs.

## Phase 1: process and OAuth foundation

The package produces wxcc_handler.exe and exposes a Runner-compatible --manifest.

Default configuration is wxcc_config.json beside the executable. Override it with --config PATH; relative paths resolve beside the executable.

Initialize the file with:

```powershell
wxcc_handler.exe config init
```

Set these values in wxcc_config.json:

- client_id: Webex Integration client ID.
- client_secret: Webex Integration client secret.
- redirect_uri: Exact URI registered for the Integration.
- scopes: Array of approved Webex Contact Center OAuth scopes required by enabled operations.
- access_token, refresh_token, access_token_expires_at, refresh_token_expires_at: Managed by the application.
- pending_oauth_state: Temporary OAuth state for callback validation.
- log_stdout_level and log_file_level: Independent tracing levels.

V1 stores client secret and tokens in plain local JSON by explicit project decision. Protect the file using Windows account and filesystem permissions. Never commit a populated config.

## OAuth commands

1. Run wxcc_handler.exe auth authorize-url.
2. Open the printed URL, sign in to Webex, and approve the configured scopes.
3. After redirect, copy the code and state query values.
4. Run wxcc_handler.exe auth exchange-code --code CODE --state STATE.
5. Check auth status; refresh with wxcc_handler.exe auth refresh.

The flow uses the official Webex OAuth authorization-code and token endpoints. A locally generated random state is validated before exchanging the authorization code. Tokens are never printed.

## Planned capabilities

### Data Collection

- Collect Site → Queue → Member relationships.
- Include member skill profile, skill sets, last login, and creation date where documented APIs expose them.
- Produce one flat CSV with one member-per-queue row, retaining source IDs; skill sets occupy one delimited column.
- CSV output uses UTF-8 with BOM.
- The queue-without-members row policy remains to be decided before that edge case is implemented.

### Analyzer

- Automatically discover reports and cache report IDs, parameters, and filters in local JSON.
- Provide a separate analyzer reports sync command.
- Execute reports for selected start/end dates and filters.
- CSV and XLSX output are required from the first release.
- If Saved Analyzer Reports cannot be discovered and executed through documented APIs, build equivalent reports through the official Search GraphQL API using locally stored report definitions. Do not automate the Analyzer UI or use undocumented endpoints.

### User Provisioning

- Input records include email and full name. First name is the first two words; last name is the remaining words.
- Create users through a supported provisioning API, then perform supported WxCC assignments for site, team, and one skill profile.
- If an email already exists, skip the user entirely, make no changes, and record an error that the email already exists.
- A failure in one step must not roll back successful earlier steps, prevent later steps for that user, or stop subsequent users. Preserve the original error details.
- Do not assume SCIM / Control Hub creation automatically completes WxCC provisioning or assignment; verify supported synchronization and assignment APIs first.

## Status

Phase 1 implements the process package, local config initialization/validation, OAuth URL generation, authorization-code exchange, refresh-token handling, and token status. Data Collection, Analyzer, and User Provisioning are not yet implemented.


## API version and OAuth scope rules

- Use the current v2 configuration endpoints for Sites, Teams, Skill Profiles, Users, and Contact Service Queues where the v2 resource is documented. Cisco's changelog states that the listed v1 List APIs were removed on 15 October 2026; do not build new collection logic against those retired routes.
- Configuration read operations require cjp:config_read. Configuration write operations require cjp:config_write. Search GraphQL requires cjp:config or cjp:config_read and an Administrator or Supervisor role. Request only the scopes required by the enabled commands.
- For Contact Service Queues, the list response no longer includes the full agents array; retrieve each queue's detail resource when the collection requires that relationship.
