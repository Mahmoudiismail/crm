# Authentication Flow (Cognito SRP)

Implementation: `src/crm/auth.rs`

## Entry Point

`ensure_authenticated(config, client, skip_login)`

Decision tree:

1. If cached token and `access_token_expiry` are valid -> reuse token.
2. Else perform fresh SRP login.

Current runtime policy: CRM execution always requires login flow (no user-facing skip-login option).

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

## Security Guidance

- Prefer default TLS verification (`no_verify_ssl = false`) in production.
- Use `remember_secrets = false` if config should not retain tokens/password.
- Do not log raw passwords.
