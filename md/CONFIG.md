# Configuration Reference

## Config File Location

Default: `config.json` in the current working directory.
Override with `--config <path>`.

## Default Configuration

```json
{
  "region": "ap-south-1",
  "user_pool_id": "ap-south-1_wjZE70ShT",
  "client_id": "i7g0t35boqicb1tdc4rgthk6",
  "username": "+201155520811",
  "password": "Thb@1234",
  "no_verify_ssl": true,
  "remember_secrets": true,
  "email": "Mahmoud_iismail@rayacx.com",
  "from_date": "2025-01-01",
  "calls_from_date": "2026-02-01",
  "to_date": "",
  "download_csv": true,
  "account_id": "233b5ff5-8aff-4445-815b-39d7916a1d46",
  "application_id": "83921976-97dd-4679-9b36-ee936ecf50d1",
  "app_timezone_plus_minutes": "180",
  "base_url": "https://crm.fakeeh.care/medi-crm/vault/v1/task"
}
```

## Field Reference

| Field                     | Type   | Description                                   |
|---------------------------|--------|-----------------------------------------------|
| `region`                  | String | AWS region for Cognito                        |
| `user_pool_id`            | String | Cognito User Pool ID                          |
| `client_id`               | String | Cognito App Client ID                         |
| `username`                | String | Cognito username (phone number)               |
| `password`                | String | Cognito password (secret)                     |
| `no_verify_ssl`           | bool   | Disable TLS certificate verification          |
| `remember_secrets`        | bool   | Persist secrets to config file                |
| `email`                   | String | Email for CRM report requests                 |
| `from_date`               | String | Start date for tickets/leads (YYYY-MM-DD)     |
| `calls_from_date`         | String | Start date for call logs (YYYY-MM-DD)         |
| `to_date`                 | String | End date (YYYY-MM-DD), empty = today          |
| `download_csv`            | bool   | Auto-download CSV files from report URLs      |
| `account_id`              | String | CRM account identifier                        |
| `application_id`          | String | CRM application identifier                    |
| `app_timezone_plus_minutes` | String | Timezone offset in minutes                  |
| `base_url`                | String | CRM API base URL                              |

### Token Fields (auto-managed)

| Field                  | Type   | Description                        |
|------------------------|--------|------------------------------------|
| `access_token`         | String | JWT access token                   |
| `access_token_expiry`  | String | Token expiry (ISO 8601 UTC)        |
| `id_token`             | String | JWT ID token                       |
| `refresh_token`        | String | Refresh token                      |
| `token_timestamp`      | String | When token was obtained (ISO 8601) |

## Precedence Rules

```
CLI Arguments  >  Config File  >  Built-in Defaults
```

## Special Behaviors

### `to_date`
- If empty after all merging, defaults to **today's date**

### `calls_from_date`
- If empty after all merging, defaults to `from_date`

### `remember_secrets = false`
Before saving, these fields are **removed** from the config file:
- `password`
- `access_token`
- `access_token_expiry`
- `id_token`
- `refresh_token`
- `token_timestamp`

### Null Stripping
All `null` JSON values are removed before saving.

## Config Lifecycle

```
1. Load config.json (or create with defaults)
2. Merge missing keys from defaults
3. Apply CLI overrides
4. Run tool (may update tokens)
5. Save config back to disk
```
