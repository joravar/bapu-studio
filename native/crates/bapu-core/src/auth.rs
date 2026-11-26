//! Clean-room cryptographic & token authentication helpers:
//! - AWS Signature Version 4 (`AWS4-HMAC-SHA256`)
//! - Local HS256 JWT generation (`Jwt`)
//! - OAuth 2.0 Token Exchange (`client_credentials` & `password` grants)

use crate::types::AuthConfig;
use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Signs an HTTP request using AWS Signature Version 4 (`AWS4-HMAC-SHA256`) and returns
/// the headers to attach (`Authorization`, `x-amz-date`, `x-amz-content-sha256`, and optional
/// `x-amz-security-token`).
pub fn sign_aws_sigv4(
    method: &str,
    url: &url::Url,
    body: &[u8],
    cfg: &AuthConfig,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<(String, String)>> {
    let access_key = cfg
        .aws_access_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("AWS Access Key ID is required for AWS SigV4")?;
    let secret_key = cfg
        .aws_secret_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("AWS Secret Access Key is required for AWS SigV4")?;
    let region = cfg
        .aws_region
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("us-east-1");
    let service = cfg
        .aws_service
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("execute-api");

    let host = match url.port() {
        Some(p) => format!("{}:{p}", url.host_str().unwrap_or("localhost")),
        None => url.host_str().unwrap_or("localhost").to_string(),
    };

    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let payload_hash = sha256_hex(body);

    let mut canonical_headers_map: BTreeMap<String, String> = BTreeMap::new();
    canonical_headers_map.insert("host".into(), host);
    canonical_headers_map.insert("x-amz-content-sha256".into(), payload_hash.clone());
    canonical_headers_map.insert("x-amz-date".into(), amz_date.clone());
    if let Some(tok) = cfg.aws_session_token.as_deref().filter(|s| !s.trim().is_empty()) {
        canonical_headers_map.insert("x-amz-security-token".into(), tok.trim().to_string());
    }

    let canonical_headers: String = canonical_headers_map
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect();
    let signed_headers: String = canonical_headers_map
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(";");

    let canonical_uri = if url.path().is_empty() { "/" } else { url.path() };

    let mut query_pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (aws_uri_encode(&k), aws_uri_encode(&v)))
        .collect();
    query_pairs.sort();
    let canonical_querystring: String = query_pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method.to_ascii_uppercase(),
        canonical_uri,
        canonical_querystring,
        canonical_headers,
        signed_headers,
        payload_hash
    );

    let credential_scope = format!("{date_stamp}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );

    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let auth_header = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let mut out = vec![
        ("Authorization".into(), auth_header),
        ("x-amz-date".into(), amz_date),
        ("x-amz-content-sha256".into(), payload_hash),
    ];
    if let Some(tok) = cfg.aws_session_token.as_deref().filter(|s| !s.trim().is_empty()) {
        out.push(("x-amz-security-token".into(), tok.trim().to_string()));
    }
    Ok(out)
}

fn aws_uri_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Generates a signed HS256 JSON Web Token (`header.payload.signature`).
pub fn generate_hs256_jwt(secret: &str, payload_json: Option<&str>) -> Result<String> {
    if secret.trim().is_empty() {
        bail!("JWT secret key cannot be empty");
    }
    let header_b64 = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);

    let raw_payload = payload_json.filter(|s| !s.trim().is_empty()).unwrap_or("{}");
    let mut claims: serde_json::Value =
        serde_json::from_str(raw_payload).context("JWT payload must be valid JSON")?;
    if let Some(obj) = claims.as_object_mut() {
        obj.entry("iat")
            .or_insert_with(|| serde_json::json!(chrono::Utc::now().timestamp()));
    }
    let payload_bytes = serde_json::to_vec(&claims)?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload_bytes);

    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig = hmac_sha256(secret.as_bytes(), signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);

    Ok(format!("{signing_input}.{sig_b64}"))
}

/// Exchanges OAuth 2.0 `client_credentials` or `password` credentials at `oauth_token_url`
/// for an `access_token`.
pub async fn fetch_oauth2_access_token(
    client: &reqwest::Client,
    cfg: &AuthConfig,
) -> Result<String> {
    let token_url = cfg
        .oauth_token_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("OAuth 2.0 Token URL is required")?;
    let grant = cfg
        .oauth_grant_type
        .as_deref()
        .unwrap_or("client_credentials");

    let mut form: Vec<(&str, String)> = vec![("grant_type", grant.to_string())];
    if let Some(cid) = cfg.oauth_client_id.as_deref().filter(|s| !s.is_empty()) {
        form.push(("client_id", cid.to_string()));
    }
    if let Some(csec) = cfg.oauth_client_secret.as_deref().filter(|s| !s.is_empty()) {
        form.push(("client_secret", csec.to_string()));
    }
    if let Some(scope) = cfg.oauth_scope.as_deref().filter(|s| !s.is_empty()) {
        form.push(("scope", scope.to_string()));
    }
    if grant == "password" {
        if let Some(u) = cfg.username.as_deref().filter(|s| !s.is_empty()) {
            form.push(("username", u.to_string()));
        }
        if let Some(p) = cfg.password.as_deref().filter(|s| !s.is_empty()) {
            form.push(("password", p.to_string()));
        }
    }

    let body_encoded: String = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form.iter().map(|(k, v)| (*k, v.as_str())))
        .finish();

    let resp = client
        .post(token_url)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("accept", "application/json")
        .body(body_encoded)
        .send()
        .await
        .context("sending OAuth 2.0 token request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("OAuth 2.0 token endpoint returned HTTP {status}: {text}");
    }

    let json: serde_json::Value = resp.json().await.context("parsing OAuth 2.0 token JSON")?;
    json.get("access_token")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .context("OAuth 2.0 response did not contain `access_token`")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn signs_aws_sigv4_headers_deterministically() {
        let cfg = AuthConfig {
            aws_access_key: Some("AKIAIOSFODNN7EXAMPLE".into()),
            aws_secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into()),
            aws_region: Some("us-east-1".into()),
            aws_service: Some("s3".into()),
            ..Default::default()
        };
        let url = url::Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap();
        let fixed_time = chrono::Utc.with_ymd_and_hms(2026, 11, 26, 12, 0, 0).unwrap();
        let headers = sign_aws_sigv4("GET", &url, b"", &cfg, fixed_time).unwrap();
        let auth = &headers.iter().find(|(k, _)| k == "Authorization").unwrap().1;
        assert!(auth.starts_with(
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20261126/us-east-1/s3/aws4_request"
        ));
        assert!(auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
    }

    #[test]
    fn generates_valid_three_part_hs256_jwt() {
        let jwt = generate_hs256_jwt("super-secret", Some(r#"{"sub":"user_42","iat":1700000000}"#))
            .unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let decoded_claims = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
        let val: serde_json::Value = serde_json::from_slice(&decoded_claims).unwrap();
        assert_eq!(val["sub"], "user_42");
    }
}
