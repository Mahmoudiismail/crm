# WCXX Component

The `wcxx` component retrieves contact center data from Webex Contact Center (WXCC).

## Configuration

The component uses a configuration file named `wcxx_config.json`. If it does not exist, it will be created with default values.

```json
{
  "base_url": "https://webexapis.com/v1",
  "token": "YOUR_BEARER_TOKEN_HERE",
  "org_id": "",
  "client_id": "",
  "client_secret": "",
  "refresh_token": ""
}
```

- **`base_url`**: Base URL for the Webex API. Defaults to `https://webexapis.com/v1`.
- **`token`**: Your personal or application bearer token.
- **`org_id`**: (Optional) Organization ID appended to some endpoints if required.
- **`client_id`**: (Optional) Client ID for OAuth flows.
- **`client_secret`**: (Optional) Client Secret for OAuth flows.
- **`refresh_token`**: (Optional) Refresh token used to retrieve a new access token when the current token expires.

See `BUILD_AND_RUN.md` and `CONFIG.md` for more usage details.
