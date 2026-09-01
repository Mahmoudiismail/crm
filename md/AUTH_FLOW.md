# Authentication Flow (Cognito SRP)

Implementation: `src/crm/auth.rs`

## Entry Point

`ensure_authenticated(config, client, skip_login)`

Decision tree:

1. If cached token and `access_token_expiry` are valid -> reuse token.
2. Else perform fresh SRP login.

Current runtime policy: CRM execution always requires login flow (no user-facing skip-login option).

Report range splitting does not change authentication behavior. Split retries reuse the same bearer token and request headers as the original report fetch.

## Cognito SRP Sequence

### 1) Prepare SRP Values

- Parse constants `N` and `g`.
- Compute `k`.
- Generate random `a` (`rand::rng().fill_bytes`).
- Compute public `A = g^a mod N`.
- Validate `A mod N != 0`.

### 2) `InitiateAuth`

POST to:

- `https://cognito-idp.<region>.amazonaws.com/`

Headers:

- `Content-Type: application/x-amz-json-1.1`
- `X-Amz-Target: AWSCognitoIdentityProviderService.InitiateAuth`

Body includes:

- `AuthFlow = USER_SRP_AUTH`
- `ClientId`
- `AuthParameters.USERNAME`
- `AuthParameters.SRP_A`

### 3) Challenge Processing

Extract from `ChallengeParameters`:

- `SRP_B`
- `SALT`
- `SECRET_BLOCK`
- `USER_ID_FOR_SRP`

Validate `B mod N != 0`.

### 4) Compute Secret and Signature

- Compute `u`, `x`, and shared secret `S`.
- Derive HKDF key (`compute_hkdf`).
- Build Cognito-format timestamp (`%a %b %-d %H:%M:%S UTC %Y`).
- Sign payload with HMAC SHA-256 and base64-encode.

### 5) `RespondToAuthChallenge`

Headers include:

- `X-Amz-Target: AWSCognitoIdentityProviderService.RespondToAuthChallenge`

Body includes:

- `ChallengeName = PASSWORD_VERIFIER`
- `USERNAME`
- `PASSWORD_CLAIM_SECRET_BLOCK`
- `PASSWORD_CLAIM_SIGNATURE`
- `TIMESTAMP`

### 6) Token Extraction

From `AuthenticationResult`:

- `AccessToken`
- `IdToken`
- `RefreshToken`
- `ExpiresIn`

## Token Update in Config

On success:

- update tokens,
- set `access_token_expiry = now + expires_in`,
- set `token_timestamp = now`.

## Error Cases

- Cognito HTTP failure.
- Missing challenge fields.
- Invalid SRP math state (`u == 0`, `A mod N == 0`, `B mod N == 0`).
- JSON/decoding errors.

## Implementation Quality and Testing

The SRP-6a modular exponentiation math in `compute_s` is unit-tested against known test vectors to ensure reliability and handle edge cases like underflow. The tests cover:
- Standard scenarios where `b >= kgx`.
- Underflow scenarios where `b < kgx`, ensuring the logic `(b + N - kgx) mod N` correctly handles big integers.
- Realistic 2048-bit SRP-6a test vectors verified against reference Python implementations.

## Security Guidance

- Prefer default TLS verification (`no_verify_ssl = false`) in production.
- Use `remember_secrets = false` if config should not retain tokens/password.
- Do not log raw passwords.

## CRM API Requests & Auth Retry Logic
Added a proactive token expiration buffer: tokens within 5 minutes of their expiry are now forcefully refreshed prior to making API calls.
Implemented a 401 Unauthorized Interceptor: if an API call fails with 401, the system invalidates the cached token, requests a new one seamlessly via the AWS Cognito SRP login flow, and retries the request automatically exactly once to prevent infinite loops.

## Report Cleanup Policy
Added `retention_days` to `crm_config.json`. Setting it to `0` or omitting it leaves downloaded CSV files intact. If set to an integer value greater than `0`, the crm execution will automatically delete downloaded `ticket_report_*.csv` and `lead_report_*.csv` files that are older than the specified retention window in days.
