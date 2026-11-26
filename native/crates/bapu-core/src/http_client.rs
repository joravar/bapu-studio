//! Executes an `ApiRequest` with `reqwest` and produces an `ApiResponse`.
//! Supports all HTTP methods (`GET`..`OPTIONS`), `:path` variable substitution,
//! `x-www-form-urlencoded` & `multipart/form-data` (fields + files), `GraphQL`, `XML`,
//! `Bearer` / `Basic` / `API Key` / `OAuth 2.0` / `AWS SigV4` / `JWT HS256` auth,
//! automatic `CookieJar` attachment & `Set-Cookie` capture, and per-request redirect/TLS/timeout settings.

use crate::auth;
use crate::cookies::CookieJar;
use crate::types::{ApiRequest, ApiResponse, AuthType, BodyType, CookieEntry};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

pub async fn send_request(client: &reqwest::Client, req: &ApiRequest) -> Result<ApiResponse> {
    send_request_with_cookies(client, req, None).await
}

pub async fn send_request_with_cookies(
    default_client: &reqwest::Client,
    req: &ApiRequest,
    mut cookie_jar: Option<&mut CookieJar>,
) -> Result<ApiResponse> {
    let custom_client;
    let client = if !req.follow_redirects || !req.verify_tls || req.timeout_ms.is_some() {
        let mut cb = reqwest::Client::builder()
            .timeout(Duration::from_millis(req.timeout_ms.unwrap_or(30_000)))
            .danger_accept_invalid_certs(!req.verify_tls);
        if !req.follow_redirects {
            cb = cb.redirect(reqwest::redirect::Policy::none());
        }
        custom_client = cb.build().context("building custom HTTP client")?;
        &custom_client
    } else {
        default_client
    };

    let method = reqwest::Method::from_bytes(req.method.as_str().as_bytes())
        .context("invalid HTTP method")?;

    let resolved_url_str = apply_path_params(&req.url, &req.path_params);
    let mut url = reqwest::Url::parse(&resolved_url_str)
        .with_context(|| format!("invalid URL: {resolved_url_str}"))?;

    {
        let mut pairs = url.query_pairs_mut();
        for p in req.params.iter().filter(|p| p.enabled && !p.key.is_empty()) {
            pairs.append_pair(&p.key, &p.value);
        }
        if req.auth_type == AuthType::Apikey
            && req.auth_config.add_to.as_deref() == Some("query")
        {
            if let (Some(k), Some(v)) =
                (&req.auth_config.api_key_name, &req.auth_config.api_key_value)
            {
                pairs.append_pair(k, v);
            }
        }
    }
    if url.query() == Some("") {
        url.set_query(None);
    }

    // Prepare body bytes + content-type up front so AWS SigV4 can hash the exact payload.
    let (body_bytes, auto_content_type): (Option<Vec<u8>>, Option<String>) = match req.body_type {
        BodyType::None => (None, None),
        BodyType::Json => (
            Some(req.body_content.as_bytes().to_vec()),
            Some("application/json".into()),
        ),
        BodyType::Xml => (
            Some(req.body_content.as_bytes().to_vec()),
            Some("application/xml".into()),
        ),
        BodyType::Raw => (
            Some(req.body_content.as_bytes().to_vec()),
            Some("text/plain; charset=utf-8".into()),
        ),
        BodyType::Form => {
            let encoded = if !req.form_fields.is_empty() {
                let mut ser = form_urlencoded::Serializer::new(String::new());
                for f in req.form_fields.iter().filter(|f| f.enabled && !f.key.is_empty()) {
                    ser.append_pair(&f.key, &f.value);
                }
                ser.finish()
            } else {
                req.body_content.clone()
            };
            (
                Some(encoded.into_bytes()),
                Some("application/x-www-form-urlencoded".into()),
            )
        }
        BodyType::Multipart => {
            let boundary = format!("----BapuStudioBoundary{}", uuid::Uuid::new_v4().simple());
            let bytes = build_multipart_body(&boundary, &req.multipart_fields)?;
            (
                Some(bytes),
                Some(format!("multipart/form-data; boundary={boundary}")),
            )
        }
        BodyType::Graphql => {
            let payload = serde_json::json!({
                "query": req.graphql_query.clone().unwrap_or_default(),
                "variables": serde_json::from_str::<serde_json::Value>(
                    req.graphql_variables.as_deref().unwrap_or("{}")
                ).unwrap_or_else(|_| serde_json::json!({})),
            });
            (
                Some(serde_json::to_vec(&payload)?),
                Some("application/json".into()),
            )
        }
    };

    let mut builder = client.request(method, url.clone());

    let mut has_explicit_content_type = false;
    let mut has_explicit_cookie = false;
    for h in req.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        if h.key.eq_ignore_ascii_case("content-type") {
            has_explicit_content_type = true;
        }
        if h.key.eq_ignore_ascii_case("cookie") {
            has_explicit_cookie = true;
        }
        builder = builder.header(&h.key, &h.value);
    }

    if !has_explicit_content_type {
        if let Some(ct) = auto_content_type {
            builder = builder.header("content-type", ct);
        }
    }

    if !has_explicit_cookie {
        if let Some(ref jar) = cookie_jar {
            if let Some(cookie_hdr) = jar.cookie_header_for(&url) {
                builder = builder.header("cookie", cookie_hdr);
            }
        }
    }

    // Apply Authorization scheme
    match req.auth_type {
        AuthType::None => {}
        AuthType::Bearer => {
            if let Some(token) = req.auth_config.token.as_deref().filter(|s| !s.is_empty()) {
                builder = builder.bearer_auth(token);
            }
        }
        AuthType::Basic => {
            if let Some(user) = req.auth_config.username.as_deref() {
                builder = builder.basic_auth(user, req.auth_config.password.clone());
            }
        }
        AuthType::Apikey => {
            if req.auth_config.add_to.as_deref() != Some("query") {
                if let (Some(name), Some(value)) =
                    (&req.auth_config.api_key_name, &req.auth_config.api_key_value)
                {
                    if !name.trim().is_empty() {
                        builder = builder.header(name, value);
                    }
                }
            }
        }
        AuthType::Oauth2 => {
            let token = match req.auth_config.token.as_deref().filter(|s| !s.trim().is_empty()) {
                Some(existing) => existing.to_string(),
                None => auth::fetch_oauth2_access_token(client, &req.auth_config).await?,
            };
            let prefix = req
                .auth_config
                .oauth_header_prefix
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Bearer");
            builder = builder.header("Authorization", format!("{prefix} {token}"));
        }
        AuthType::Awssigv4 => {
            let payload_slice = body_bytes.as_deref().unwrap_or(&[]);
            let sig_headers = auth::sign_aws_sigv4(
                req.method.as_str(),
                &url,
                payload_slice,
                &req.auth_config,
                chrono::Utc::now(),
            )?;
            for (k, v) in sig_headers {
                builder = builder.header(k, v);
            }
        }
        AuthType::Jwt => {
            if let Some(secret) = req.auth_config.jwt_secret.as_deref() {
                let jwt = auth::generate_hs256_jwt(secret, req.auth_config.jwt_payload.as_deref())?;
                builder = builder.bearer_auth(jwt);
            }
        }
    }

    if let Some(bytes) = body_bytes {
        builder = builder.body(bytes);
    }

    let start = Instant::now();
    let response = builder.send().await.context("request failed")?;
    let status = response.status();
    let http_version = Some(format!("{:?}", response.version()));
    let remote_addr = response.remote_addr().map(|a| a.to_string());
    let final_url = response.url().clone();

    let mut response_cookies: Vec<CookieEntry> = Vec::new();
    for val in response.headers().get_all("set-cookie") {
        if let Ok(raw_cookie) = val.to_str() {
            if let Some(parsed) = CookieJar::parse_set_cookie(raw_cookie, &final_url) {
                if let Some(ref mut jar) = cookie_jar {
                    jar.store_cookie(parsed.clone());
                }
                response_cookies.push(parsed);
            }
        }
    }

    let mut headers = HashMap::new();
    for (name, value) in response.headers() {
        let val_str = value.to_str().unwrap_or("").to_string();
        headers
            .entry(name.to_string())
            .and_modify(|existing: &mut String| {
                existing.push_str("; ");
                existing.push_str(&val_str);
            })
            .or_insert(val_str);
    }

    let bytes = response.bytes().await.context("reading response body")?;
    let time_ms = start.elapsed().as_millis() as u64;
    let size_bytes = bytes.len() as u64;

    let data: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::String(String::new())
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };

    Ok(ApiResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        time_ms,
        size_bytes,
        headers,
        cookies: response_cookies,
        data,
        timestamp: chrono::Utc::now().to_rfc3339(),
        http_version,
        remote_addr,
        test_results: vec![],
        console_logs: vec![],
    })
}

/// Replaces `:param` or `{param}` segments in a URL path with enabled `path_params`.
pub fn apply_path_params(raw_url: &str, path_params: &[crate::types::KeyValuePair]) -> String {
    let mut out = raw_url.to_string();
    for p in path_params.iter().filter(|p| p.enabled && !p.key.trim().is_empty()) {
        let encoded: String = form_urlencoded::byte_serialize(p.value.as_bytes()).collect();
        out = out.replace(&format!(":{}", p.key), &encoded);
        out = out.replace(&format!("{{{}}}", p.key), &encoded);
    }
    out
}

/// Extracts `:param` path variable names from a URL template (ignoring `:port` numbers).
pub fn extract_path_param_names(raw_url: &str) -> Vec<String> {
    let after_scheme = raw_url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(raw_url);
    let path_part = after_scheme
        .split_once('/')
        .map(|(_, p)| p.split('?').next().unwrap_or(p))
        .unwrap_or("");

    let mut names = Vec::new();
    for seg in path_part.split('/') {
        if let Some(stripped) = seg.strip_prefix(':') {
            if !stripped.is_empty()
                && stripped.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !names.iter().any(|n| n == stripped)
            {
                names.push(stripped.to_string());
            }
        }
    }
    names
}

fn build_multipart_body(
    boundary: &str,
    fields: &[crate::types::MultipartField],
) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
        buf.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        if f.is_file {
            let path = Path::new(f.value.trim());
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("upload.bin");
            let file_bytes = std::fs::read(path)
                .with_context(|| format!("reading multipart file {}", path.display()))?;
            buf.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
                    f.key, file_name
                )
                .as_bytes(),
            );
            buf.extend_from_slice(&file_bytes);
            buf.extend_from_slice(b"\r\n");
        } else {
            buf.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
                    f.key, f.value
                )
                .as_bytes(),
            );
        }
    }
    buf.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok(buf)
}
