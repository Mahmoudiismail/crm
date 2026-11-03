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
- scopes: Array of approved Webex Contact Center OAuth scopes required by enabled operations. Data Collection requires `cjp:config_read`.
- org_id: Webex Contact Center organization ID.
- api_base_url: Regional Webex Contact Center API base URL (for example, `https://api.wxcc-us1.cisco.com` for US1; use the base URL for your tenant's actual region).
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

## API version policy

All Webex Contact Center configuration requests made by WXCC Handler must use the documented `v2` resource endpoints. The HTTP client rejects any Contact Center request path that does not begin with `v2/`, preventing accidental use of legacy unversioned or v1 resource paths. The OAuth authorization and token exchange endpoints remain `https://webexapis.com/v1/...` because they belong to the separate Webex Platform OAuth API; they are not Contact Center configuration requests.

## Data Collection

Configure `org_id` and `api_base_url`, then run:

```powershell
wxcc_handler.exe data collect
wxcc_handler.exe data collect --output C:\Exports\wxcc_data.csv
```

The default output is `wxcc_data.csv` beside the executable. Relative output paths are also resolved beside the executable. The command refreshes an access token when it is missing or expires within five minutes.

Collection uses the current v2 list resources for Sites, Teams, Skill Profiles, Users, and Contact Service Queues. It retrieves full Queue and Team resources because the June 1, 2026 API change removed relationship arrays from list responses. User and skill-profile details are retrieved where an ID is available. Skill-based queue membership is taken from Cisco's per-user skill-queue mapping endpoint rather than reimplementing skill eligibility.

The CSV is flat, UTF-8 with BOM, and contains one row per member-per-queue relationship. Source IDs are retained. Skill sets are serialized into one semicolon-delimited column. Last-login and creation timestamps are included when the API returns them; otherwise the corresponding field is blank. A queue with no members is still exported as one row, with all member columns blank.

CSV columns: `site_id`, `site_name`, `queue_id`, `queue_name`, `queue_type`, `member_id`, `member_email`, `member_name`, `member_skill_profile_id`, `member_skill_profile_name`, `member_skill_sets`, `member_last_login`, `member_created_at`.

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

Phase 1 implements the process package, local config initialization/validation, OAuth URL generation, authorization-code exchange, refresh-token handling, and token status. Phase 2 adds Data Collection and CSV export. Analyzer and User Provisioning are not yet implemented.


## API version and OAuth scope rules

- Use the current v2 configuration endpoints for Sites, Teams, Skill Profiles, Users, and Contact Service Queues where the v2 resource is documented. Cisco's changelog states that the listed v1 List APIs were removed on 15 October 2026; do not build new collection logic against those retired routes.
- Configuration read operations require cjp:config_read. Configuration write operations require cjp:config_write. Search GraphQL requires cjp:config or cjp:config_read and an Administrator or Supervisor role. Request only the scopes required by the enabled commands.
- For Contact Service Queues, the list response no longer includes the full agents array; retrieve each queue's detail resource when the collection requires that relationship.
