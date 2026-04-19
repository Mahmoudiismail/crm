use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use num_bigint::BigUint;
use rand::Rng;
use sha2::{Digest, Sha256};
use tracing::{debug, info};

use crate::crm::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

// ──────────────────────────────────────────────────────────────
// SRP-6a constants (RFC 5054, 2048-bit group #14 — used by Cognito)
// ──────────────────────────────────────────────────────────────

const N_HEX: &str = "\
FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD1\
29024E088A67CC74020BBEA63B139B22514A08798E3404DD\
EF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245\
E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED\
EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3D\
C2007CB8A163BF0598DA48361C55D39A69163FA8FD24CF5F\
83655D23DCA3AD961C62F356208552BB9ED529077096966D\
670C354E4ABC9804F1746C08CA18217C32905E462E36CE3B\
E39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9\
DE2BCBF6955817183995497CEA956AE515D2261898FA0510\
15728E5A8AAAC42DAD33170D04507A33A85521ABDF1CBA64\
ECFB850458DBEF0A8AEA71575D060C7DB3970F85A6E1E4C7\
ABF5AE8CDB0933D71E8C94E04A25619DCEE3D2261AD2EE6B\
F12FFA06D98A0864D87602733EC86A64521F2B18177B200CB\
BE117577A615D6C770988C0BAD946E208E24FA074E5AB3143\
DB5BFCE0FD108E4B82D120A93AD2CAFFFFFFFFFFFFFFFF";

const G_HEX: &str = "2";

const INFO_BITS: &[u8] = b"Caldera Derived Key";

// ──────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────

/// Ensure we have a valid token in `config`, performing login if needed.
pub async fn ensure_authenticated(
    config: &mut AppConfig,
    client: &reqwest::Client,
    skip_login: bool,
) -> Result<String> {
    // 1. --skip-login: use cached token
    if skip_login {
        if !config.id_token.is_empty() {
            info!("Using cached id_token (--skip-login)");
            return Ok(config.id_token.clone());
        }
        if !config.access_token.is_empty() {
            info!("Using cached access_token (--skip-login)");
            return Ok(config.access_token.clone());
        }
        bail!("--skip-login specified but no cached token found in config");
    }

    // 2. Check cached token expiry
    if !config.access_token.is_empty() && !config.access_token_expiry.is_empty() {
        if let Ok(expiry) = DateTime::parse_from_rfc3339(&config.access_token_expiry) {
            if expiry > Utc::now() {
                info!("Cached token still valid (expires {})", expiry);
                let token = if !config.id_token.is_empty() {
                    config.id_token.clone()
                } else {
                    config.access_token.clone()
                };
                return Ok(token);
            }
            debug!("Cached token expired at {}", expiry);
        }
    }

    // 3. Fresh login
    info!("Performing Cognito SRP authentication...");
    let tokens = cognito_srp_login(config, client).await?;

    config.access_token = tokens.access_token.clone();
    config.id_token = tokens.id_token.clone();
    config.refresh_token = tokens.refresh_token.clone();

    let now = Utc::now();
    let expiry = now + chrono::TimeDelta::seconds(tokens.expires_in as i64);
    config.access_token_expiry = expiry.to_rfc3339();
    config.token_timestamp = now.to_rfc3339();

    info!("Authentication successful, token expires at {}", expiry);

    let token = if !config.id_token.is_empty() {
        config.id_token.clone()
    } else {
        config.access_token.clone()
    };
    Ok(token)
}

// ──────────────────────────────────────────────────────────────
// SRP Implementation
// ──────────────────────────────────────────────────────────────

struct AuthTokens {
    access_token: String,
    id_token: String,
    refresh_token: String,
    expires_in: u64,
}

async fn cognito_srp_login(config: &AppConfig, client: &reqwest::Client) -> Result<AuthTokens> {
    let n = BigUint::parse_bytes(N_HEX.as_bytes(), 16).context("Failed to parse N_HEX")?;
    let g = BigUint::parse_bytes(G_HEX.as_bytes(), 16).context("Failed to parse G_HEX")?;

    // k = H('00' + N_hex + '0' + g_hex) — matches Python warrant exactly
    let k = compute_k()?;

    // Generate random `a` (128 random bytes, then mod N — matches Python warrant)
    let mut a_bytes = [0u8; 128];
    rand::rng().fill_bytes(&mut a_bytes);
    let a = BigUint::from_bytes_be(&a_bytes) % &n;

    // A = g^a mod N
    let big_a = g.modpow(&a, &n);

    // Validate A % N != 0
    if &big_a % &n == BigUint::ZERO {
        bail!("SRP A mod N is zero — invalid state");
    }

    // Python sends long_to_hex(A) which is lowercase hex, no leading zeros
    let big_a_hex = long_to_hex(&big_a);

    // Pool ID without the region prefix  (e.g. "ap-south-1_wjZE70ShT" → "wjZE70ShT")
    let pool_name = config
        .user_pool_id
        .split('_')
        .nth(1)
        .unwrap_or(&config.user_pool_id);

    // ──── Step 1: InitiateAuth ────
    let initiate_url = format!("https://cognito-idp.{}.amazonaws.com/", config.region);

    let initiate_body = serde_json::json!({
        "AuthFlow": "USER_SRP_AUTH",
        "ClientId": config.client_id,
        "AuthParameters": {
            "USERNAME": config.username,
            "SRP_A": big_a_hex
        }
    });

    debug!("InitiateAuth request URL: {}", initiate_url);
    debug!(
        "InitiateAuth body: {}",
        serde_json::to_string_pretty(&initiate_body)?
    );

    let resp = client
        .post(&initiate_url)
        .header("Content-Type", "application/x-amz-json-1.1")
        .header(
            "X-Amz-Target",
            "AWSCognitoIdentityProviderService.InitiateAuth",
        )
        .json(&initiate_body)
        .send()
        .await
        .context("InitiateAuth request failed")?;

    let status = resp.status();
    let resp_text = resp.text().await?;
    debug!("InitiateAuth response status: {}", status);
    debug!("InitiateAuth response body: {}", resp_text);

    if !status.is_success() {
        bail!("InitiateAuth failed (HTTP {}): {}", status, resp_text);
    }

    let init_resp: serde_json::Value = serde_json::from_str(&resp_text)?;
    let challenge_params = init_resp
        .get("ChallengeParameters")
        .context("Missing ChallengeParameters in InitiateAuth response")?;

    let srp_b_hex = challenge_params["SRP_B"]
        .as_str()
        .context("Missing SRP_B")?;
    let salt_hex = challenge_params["SALT"].as_str().context("Missing SALT")?;
    let secret_block_b64 = challenge_params["SECRET_BLOCK"]
        .as_str()
        .context("Missing SECRET_BLOCK")?;
    let user_id = challenge_params["USER_ID_FOR_SRP"]
        .as_str()
        .context("Missing USER_ID_FOR_SRP")?;

    let big_b = BigUint::parse_bytes(srp_b_hex.as_bytes(), 16).context("Failed to parse SRP_B")?;

    // Validate B % N != 0
    if &big_b % &n == BigUint::ZERO {
        bail!("SRP B mod N is zero — server sent invalid value");
    }

    // ──── Step 2: Compute the password claim ────

    // u = H(pad_hex(A) || pad_hex(B))  — operates on hex strings
    let u = compute_u(&big_a, &big_b)?;
    if u == BigUint::ZERO {
        bail!("SRP u is zero — invalid state");
    }

    // x = H(pad_hex(salt) || H(poolName || userId || ":" || password))
    let x = compute_x(pool_name, user_id, &config.password, salt_hex)?;

    // S = (B - k * g^x) ^ (a + u * x) mod N
    let s = compute_s(&big_b, &k, &g, &x, &a, &u, &n);

    // HKDF key — manual HMAC-based KDF matching Python warrant exactly
    let hkdf_key = compute_hkdf(&s, &u)?;

    // Timestamp (must match Cognito's expected format exactly)
    let now = Utc::now();
    // Python: re.sub(r" 0(\d) ", r" \1 ", datetime.utcnow().strftime("%a %b %d %H:%M:%S UTC %Y"))
    // This strips the leading zero from day — chrono's %-d does this
    let timestamp = now.format("%a %b %-d %H:%M:%S UTC %Y").to_string();

    // Signature = HMAC_SHA256(hkdf_key, poolName | userId | secretBlock | timestamp)
    let secret_block_bytes = B64.decode(secret_block_b64)?;
    let mut msg = Vec::new();
    msg.extend_from_slice(pool_name.as_bytes());
    msg.extend_from_slice(user_id.as_bytes());
    msg.extend_from_slice(&secret_block_bytes);
    msg.extend_from_slice(timestamp.as_bytes());

    let mut mac = HmacSha256::new_from_slice(&hkdf_key).context("Failed to create HMAC")?;
    mac.update(&msg);
    let signature = B64.encode(mac.finalize().into_bytes());

    // ──── Step 3: RespondToAuthChallenge ────
    let challenge_body = serde_json::json!({
        "ChallengeName": "PASSWORD_VERIFIER",
        "ClientId": config.client_id,
        "ChallengeResponses": {
            "USERNAME": user_id,
            "PASSWORD_CLAIM_SECRET_BLOCK": secret_block_b64,
            "PASSWORD_CLAIM_SIGNATURE": signature,
            "TIMESTAMP": timestamp
        }
    });

    debug!(
        "RespondToAuthChallenge body: {}",
        serde_json::to_string_pretty(&challenge_body)?
    );

    let resp = client
        .post(&initiate_url)
        .header("Content-Type", "application/x-amz-json-1.1")
        .header(
            "X-Amz-Target",
            "AWSCognitoIdentityProviderService.RespondToAuthChallenge",
        )
        .json(&challenge_body)
        .send()
        .await
        .context("RespondToAuthChallenge request failed")?;

    let status = resp.status();
    let resp_text = resp.text().await?;
    debug!("RespondToAuthChallenge response status: {}", status);
    debug!("RespondToAuthChallenge response body: {}", resp_text);

    if !status.is_success() {
        bail!(
            "RespondToAuthChallenge failed (HTTP {}): {}",
            status,
            resp_text
        );
    }

    let auth_resp: serde_json::Value = serde_json::from_str(&resp_text)?;
    let auth_result = auth_resp
        .get("AuthenticationResult")
        .context("Missing AuthenticationResult")?;

    let access_token = auth_result["AccessToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let id_token = auth_result["IdToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let refresh_token = auth_result["RefreshToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let expires_in = auth_result["ExpiresIn"].as_u64().unwrap_or(3600);

    Ok(AuthTokens {
        access_token,
        id_token,
        refresh_token,
        expires_in,
    })
}

// ──────────────────────────────────────────────────────────────
// SRP math helpers — faithfully ported from Python warrant
// https://github.com/capless/warrant/blob/master/warrant/aws_srp.py
// ──────────────────────────────────────────────────────────────

/// Python: `'%x' % long_num`  — lowercase hex, no prefix, no leading zeros
fn long_to_hex(val: &BigUint) -> String {
    val.to_str_radix(16)
}

/// Python: `hash_sha256(buf)` — SHA256 of raw bytes, returned as zero-padded 64-char hex string
fn hash_sha256(buf: &[u8]) -> String {
    let hash = Sha256::digest(buf);
    let hex_str = hex::encode(hash);
    // Pad to 64 chars (Python: `(64 - len(a)) * '0' + a`)
    format!("{:0>64}", hex_str)
}

/// Python: `hex_hash(hex_string)` — hash_sha256(bytearray.fromhex(hex_string))
fn hex_hash(hex_string: &str) -> Result<String> {
    let bytes = hex::decode(hex_string).context("hex_hash: invalid hex input")?;
    Ok(hash_sha256(&bytes))
}

/// Python `pad_hex(long_int)`:
/// Convert BigUint to hex string, ensure even length, prepend "00" if high nibble >= 8.
fn pad_hex(val: &BigUint) -> String {
    let mut hash_str = long_to_hex(val);
    if hash_str.len() % 2 == 1 {
        hash_str = format!("0{}", hash_str);
    } else if "89abcdef".contains(hash_str.chars().next().unwrap_or('0')) {
        hash_str = format!("00{}", hash_str);
    }
    hash_str
}

/// Overload: pad_hex for a hex string (Python passes either long or string)
fn pad_hex_str(hex_str: &str) -> String {
    let mut s = hex_str.to_lowercase();
    if s.len() % 2 == 1 {
        s = format!("0{}", s);
    } else if "89abcdef".contains(s.chars().next().unwrap_or('0')) {
        s = format!("00{}", s);
    }
    s
}

/// Python: `calculate_u(big_a, big_b)`
/// `u_hex_hash = hex_hash(pad_hex(big_a) + pad_hex(big_b))`
fn compute_u(a: &BigUint, b: &BigUint) -> Result<BigUint> {
    let u_hex_hash = hex_hash(&format!("{}{}", pad_hex(a), pad_hex(b)))?;
    BigUint::parse_bytes(u_hex_hash.as_bytes(), 16).context("Failed to parse BigUint for u")
}

/// Python: `self.k = hex_to_long(hex_hash('00' + n_hex + '0' + g_hex))`
fn compute_k() -> Result<BigUint> {
    let n_hex_lower = N_HEX.to_lowercase();
    let g_hex_lower = G_HEX.to_lowercase();
    let combined = format!("00{}0{}", n_hex_lower, g_hex_lower);
    let k_hex = hex_hash(&combined)?;
    BigUint::parse_bytes(k_hex.as_bytes(), 16).context("Failed to parse BigUint for k")
}

/// Python:
/// ```python
/// username_password = '%s%s:%s' % (pool_id_suffix, username, password)
/// username_password_hash = hash_sha256(username_password.encode('utf-8'))
/// x_value = hex_to_long(hex_hash(pad_hex(salt) + username_password_hash))
/// ```
/// Note: salt is passed as a hex string from ChallengeParameters.
fn compute_x(pool_name: &str, user_id: &str, password: &str, salt_hex: &str) -> Result<BigUint> {
    let username_password = format!("{}{}:{}", pool_name, user_id, password);
    let username_password_hash = hash_sha256(username_password.as_bytes());

    // pad_hex(salt) — salt_hex is already a hex string from the server
    let padded_salt = pad_hex_str(salt_hex);

    let x_hex = hex_hash(&format!("{}{}", padded_salt, username_password_hash))?;
    BigUint::parse_bytes(x_hex.as_bytes(), 16).context("Failed to parse BigUint for x")
}

/// S = (B - k * g^x mod N) ^ (a + u * x) mod N
/// Handle the subtraction carefully to avoid underflow.
fn compute_s(
    b: &BigUint,
    k: &BigUint,
    g: &BigUint,
    x: &BigUint,
    a: &BigUint,
    u: &BigUint,
    n: &BigUint,
) -> BigUint {
    let gx = g.modpow(x, n);
    let kgx = (k * &gx) % n;

    // Python: `int_value2 = server_b_value - self.k * g_mod_pow_xn`
    // This can go negative in Python (Python handles big negative ints natively).
    // `pow(int_value2, exp, n)` in Python handles negative bases correctly.
    // In Rust with BigUint we need: (B + N - kgx) mod N when B < kgx
    let base = if b >= &kgx {
        (b - &kgx) % n
    } else {
        (b + n - &kgx) % n
    };

    let exp = a + &(u * x);
    base.modpow(&exp, n)
}

/// Python `compute_hkdf(ikm, salt)`:
/// ```python
/// prk = hmac.new(salt, ikm, hashlib.sha256).digest()
/// info_bits_update = info_bits + bytearray(chr(1), 'utf-8')
/// hmac_hash = hmac.new(prk, info_bits_update, hashlib.sha256).digest()
/// return hmac_hash[:16]
/// ```
/// This is a manual HKDF extract+expand (single block), NOT the hkdf crate.
///
/// Called as: `compute_hkdf(bytearray.fromhex(pad_hex(s_value)),
///                          bytearray.fromhex(pad_hex(long_to_hex(u_value))))`
fn compute_hkdf(s: &BigUint, u: &BigUint) -> Result<Vec<u8>> {
    // ikm = bytearray.fromhex(pad_hex(s_value))
    let ikm = hex::decode(pad_hex(s)).context("Invalid hex in S for HKDF")?;
    // salt = bytearray.fromhex(pad_hex(long_to_hex(u_value)))
    // pad_hex(long_to_hex(u)) is the same as pad_hex(u) since pad_hex accepts a long
    let salt = hex::decode(pad_hex(u)).context("Invalid hex in U for HKDF")?;

    // HKDF-Extract: PRK = HMAC-SHA256(salt, ikm)
    let mut mac = HmacSha256::new_from_slice(&salt).context("HMAC key creation failed")?;
    mac.update(&ikm);
    let prk = mac.finalize().into_bytes();

    // HKDF-Expand (single block): HMAC-SHA256(PRK, info_bits || 0x01)[:16]
    let mut info_update = Vec::from(INFO_BITS);
    info_update.push(1u8); // chr(1)

    let mut mac2 = HmacSha256::new_from_slice(&prk).context("HMAC key creation failed")?;
    mac2.update(&info_update);
    let hmac_hash = mac2.finalize().into_bytes();

    Ok(hmac_hash[..16].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_s_b_greater_than_kgx() {
        // Small-number test vector where b > kgx
        // n=101, g=2, k=3, x=5, a=7, u=11, b=150
        // gx = 2^5 % 101 = 32
        // kgx = 3 * 32 = 96
        // b = 150. 150 > 96.
        // base = (150 - 96) % 101 = 54
        // exp = 7 + 11 * 5 = 62
        // s = 54^62 % 101 = 19 (Verified with Python: pow(150 - 96, 62, 101) == 19)
        let n = BigUint::from(101u32);
        let g = BigUint::from(2u32);
        let k = BigUint::from(3u32);
        let x = BigUint::from(5u32);
        let a = BigUint::from(7u32);
        let u = BigUint::from(11u32);
        let b = BigUint::from(150u32);

        let s = compute_s(&b, &k, &g, &x, &a, &u, &n);
        assert_eq!(s, BigUint::from(19u32));
    }

    #[test]
    fn test_compute_s_b_less_than_kgx() {
        // Small-number test vector where b < kgx
        // n=101, g=2, k=3, x=10, a=7, u=11, b=10
        // gx = 2^10 % 101 = 14
        // kgx = 3 * 14 = 42
        // b = 10. 10 < 42.
        // base = (10 + 101 - 42) % 101 = 69
        // exp = 7 + 11 * 10 = 117
        // s = 69^117 % 101 = 39 (Verified with Python: pow((10 - 42) % 101, 117, 101) == 39)
        let n = BigUint::from(101u32);
        let g = BigUint::from(2u32);
        let k = BigUint::from(3u32);
        let x = BigUint::from(10u32);
        let a = BigUint::from(7u32);
        let u = BigUint::from(11u32);
        let b = BigUint::from(10u32);

        let s = compute_s(&b, &k, &g, &x, &a, &u, &n);
        assert_eq!(s, BigUint::from(39u32));
    }

    #[test]
    fn test_compute_s_large_numbers() {
        let n = BigUint::parse_bytes(N_HEX.as_bytes(), 16).unwrap();
        let g = BigUint::from(2u32);
        let k = BigUint::parse_bytes(b"94781652145770828568693154820068118481071319461203609910235169525938894292833", 10).unwrap();
        let x = BigUint::parse_bytes(b"57342845408344078087409546890185131247557055518447571803285707850707119990624", 10).unwrap();
        let a = BigUint::parse_bytes(b"37996974895140942209423261877439902212126989024440769239204529345877668725403481274789123247893533273764280592749938825115726044399812824296566789993270290673251382596242866798594055198067504726615870620775698244291009468974236464827316958821759948517476000607580207061541247784174633462848551803113365093658", 10).unwrap();
        let u = BigUint::parse_bytes(b"44444098212040351355717692468363509183647834247620902213903532544853357575578", 10).unwrap();
        let b = BigUint::parse_bytes(b"b94cfcfcd510df263a041f83334e2789dd8fff6ab9bf1b530c76b2596d66a3c4ba8bc0c2e5cb980ba977f5c916c1757ac93d283c321778aa2f4708c908f1e1d5065ed7dd3a3827239c79cf8bd4feade9014393be909549bed99062e796080b68204370d356f3ab6c2047aebbca482dce7da67f19050533b17c61b3c21dbab9e843df28933b8727aceb8c57b2702a7897105ea5e201795f032afc54866c3151fb30a40a393195dc777b2fce4e8623fc2b751d6aa6f8898155b48e6409dd23fce9ffda6870042763395b380741ca92fd647f3381b5864d06acb49a4ac25ce159921f8cfd54126c4ee2809ac7e1e74d39086f6b2dbfb18045c75de614f89dcba090", 16).unwrap();
        let expected_s = BigUint::parse_bytes(b"1cd86a03cc3d9bfaf4f72ba600fe8d3c135d681556e0e15657783a44a7f913def250866718eb1d3ba97aa08851e3f86d13a6748a4976000f5b87a6dad256d2012562f632417f61231c5b403a5d519cbad94f77b483eedb30eade757c549a643809e2988b19acf14a5b714876e2f8f7ae00c85eebf5d3030a5a1d1bbdb1d1500f80eff2cdcc75d72dda9f1857fd32d137a6e2e922e8fe3769dd1359d5561423513663be792c61133b5f6f220c86a589c27c0a36906c1fc07f6f334c0e25fcc1f732a2672778a9781d4b45e43c9049e507516f79599694b8ad218fd6a8d02a8e405ae8f50941bd334b4343676124e1a4b2c676db76ee71618546039347d0a0df632cda28b720df96ca8aa6ded9dc30762de0958456d846642aa751004533586b595de729f3786810d79c3997c3de7c5960c45339ba9827a87d1d0f10d848460dddf35d7611d9ce9a01d218364d771d9369179642a07609e48e3f33a18cc54e105580334ca6036a76268383ce5f1b33a46fc35284c1bc49a65cbdd603de0bbb2dec", 16).unwrap();

        let s = compute_s(&b, &k, &g, &x, &a, &u, &n);
        assert_eq!(s, expected_s);
    }
}
