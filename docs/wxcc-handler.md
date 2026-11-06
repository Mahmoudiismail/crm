# WXCC Handler

## OAuth authorization

For automatic browser-based authorization, register this exact redirect URI in the Webex integration and set the same value in `wxcc_config.json`:

```text
http://127.0.0.1:8765/callback
```

The WXCC Handler binds only to `127.0.0.1:8765`. It does not expose the callback listener to the local network. The configured redirect URI must match the URI registered in the Webex Developer Portal exactly.

Run the automatic flow from Runner with:

```text
--action auth-login
```

Or from the command line:

```powershell
wxcc_handler.exe auth login
```

The handler starts the localhost callback listener, opens the system browser, validates the returned OAuth `state`, exchanges the authorization code, and saves the access and refresh tokens to the existing config file. The callback listener waits for up to three minutes and then exits. The browser displays a completion page after the token exchange succeeds.

The manual `auth-authorize-url` and `auth-exchange-code` actions remain available for troubleshooting and environments where automatic browser launch is unavailable.

## Other Runner actions

- `config-init`, `config-check`
- `auth-login`, `auth-authorize-url`, `auth-exchange-code`, `auth-refresh`, `auth-status`
- `data-collect`

V1 stores client secret and tokens in the local JSON config. Protect that file with Windows account and filesystem permissions, and never commit a populated config.
