# Authentication Flow

## Overview

The tool authenticates with AWS Cognito using the SRP (Secure Remote Password) protocol, specifically SRP-6a with a 2048-bit group.

## Token Cache Logic

```
1. --skip-login?
   ├── YES → Use id_token or access_token from config (error if neither)
   └── NO → Continue
2. access_token exists AND access_token_expiry > now?
   ├── YES → Use cached token
   └── NO → Perform fresh SRP login
```

## SRP Protocol Steps

### Step 1: Client Generates Ephemeral Key

```
a = random 128 bytes mod N  (matches Python warrant: get_random(128) % N)
A = g^a mod N
```

Where:
- `N` = 2048-bit safe prime (RFC 5054, Group 14)
- `g` = 2 (generator)

Validation: `A mod N != 0`

SRP_A is sent as lowercase hex (`long_to_hex(A)`) — no padding, no leading zeros.

### Step 2: InitiateAuth Request

```
POST https://cognito-idp.{region}.amazonaws.com/
Headers:
  Content-Type: application/x-amz-json-1.1
  X-Amz-Target: AWSCognitoIdentityProviderService.InitiateAuth
Body:
  {
    "AuthFlow": "USER_SRP_AUTH",
    "ClientId": "<client_id>",
    "AuthParameters": {
      "USERNAME": "<username>",
      "SRP_A": "<hex(A)>"
    }
  }
```

Response contains `ChallengeParameters`:
- `SRP_B` — Server's ephemeral public key (hex)
- `SALT` — Password salt (hex)
- `SECRET_BLOCK` — Base64-encoded secret
- `USER_ID_FOR_SRP` — Resolved username

### Step 3: Compute Password Claim

All operations use **hex-string level** functions matching Python warrant exactly:

```
# Helper functions (match warrant/aws_srp.py):
hash_sha256(buf)  → SHA256(buf) as 64-char zero-padded hex string
hex_hash(hex_str) → hash_sha256(bytes.fromhex(hex_str))
pad_hex(n)        → hex(n), pad to even length, prepend "00" if high nibble >= 0x8

# SRP computations:
k = hex_to_long(hex_hash("00" + N_HEX + "0" + G_HEX))
u = hex_to_long(hex_hash(pad_hex(A) + pad_hex(B)))
username_password_hash = hash_sha256(poolName + userId + ":" + password)
x = hex_to_long(hex_hash(pad_hex(salt) + username_password_hash))
S = (B - k * g^x mod N) ^ (a + u * x) mod N
```

### Step 4: HKDF Key Derivation (Manual HMAC, NOT hkdf crate)

```
ikm  = bytes.fromhex(pad_hex(S))
salt = bytes.fromhex(pad_hex(u))
info = b"Caldera Derived Key"

# HKDF-Extract:
prk = HMAC-SHA256(key=salt, msg=ikm)

# HKDF-Expand (single block):
key = HMAC-SHA256(key=prk, msg=info || 0x01)[:16]
```

This is a direct port of warrant's `compute_hkdf()` which does manual
HMAC-based extract+expand rather than using a standard HKDF library.

### Step 5: Signature

```
timestamp = "Mon Jan 2 03:04:05 UTC 2006" format
message = poolName || userId || secretBlock || timestamp
signature = Base64(HMAC-SHA256(key, message))
```

### Step 6: RespondToAuthChallenge

```
POST https://cognito-idp.{region}.amazonaws.com/
Headers:
  Content-Type: application/x-amz-json-1.1
  X-Amz-Target: AWSCognitoIdentityProviderService.RespondToAuthChallenge
Body:
  {
    "ChallengeName": "PASSWORD_VERIFIER",
    "ClientId": "<client_id>",
    "ChallengeResponses": {
      "USERNAME": "<user_id>",
      "PASSWORD_CLAIM_SECRET_BLOCK": "<secret_block>",
      "PASSWORD_CLAIM_SIGNATURE": "<signature>",
      "TIMESTAMP": "<timestamp>"
    }
  }
```

### Step 7: Token Extraction

Response `AuthenticationResult` contains:
- `AccessToken` — JWT for API access
- `IdToken` — JWT with user claims
- `RefreshToken` — For token renewal
- `ExpiresIn` — Token lifetime in seconds

## Token Storage

After successful authentication:
```json
{
  "access_token": "<jwt>",
  "id_token": "<jwt>",
  "refresh_token": "<jwt>",
  "access_token_expiry": "2026-02-25T13:00:00Z",
  "token_timestamp": "2026-02-25T12:00:00Z"
}
```

## SRP Math Details

### Hex-String Operations (Critical Difference)
The Python warrant library operates on **hex strings**, not raw bytes:
- `pad_hex(n)` — Convert to hex, ensure even length, prepend `"00"` if high nibble ≥ 8
- `hex_hash(s)` — Decode hex string to bytes, SHA256, return 64-char hex string
- `hash_sha256(b)` — SHA256 of raw bytes, return zero-padded 64-char hex string

All hashing inputs are hex-string concatenations decoded to bytes, NOT raw BigUint bytes.

### Big Integer Operations
- All arithmetic uses `num-bigint` (`BigUint`)
- Modular exponentiation via `.modpow()`
- Subtraction handled to avoid underflow: `(B + N - kgx) mod N`

### Cryptographic Primitives
- SHA-256 for all hashing (`sha2` crate)
- HMAC-SHA256 for signature and HKDF (`hmac` crate)
- Manual HKDF extract+expand (no `hkdf` crate — matches Python warrant's manual approach)
