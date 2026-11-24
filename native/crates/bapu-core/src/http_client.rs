//! Executes an `ApiRequest` with `reqwest` and produces an `ApiResponse`, for the API Studio tab.
//! This is a first working slice: method/url/query params/headers/body/bearer+basic+apikey auth
//! and timing/size are implemented. Variable interpolation (`{{VAR}}`) against the active
//! environment, and running pre-request/test scripts around the call, are wired in by the
//! caller (see `bapu-app`'s API Studio UI) rather than baked in here, so this function stays a
//! plain, testable "send this request" primitive.

use crate::types::{ApiRequest, ApiResponse, AuthType, BodyType};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::Instant;

pub async fn send_request(client: &reqwest::Client, req: &ApiRequest) -> Result<ApiResponse> {
    let method = reqwest::Method::from_bytes(req.method.as_str().as_bytes())
        .context("invalid HTTP method")?;

    let mut url = reqwest::Url::parse(&req.url).with_context(|| format!("invalid URL: {}", req.url))?;
    {
        let mut pairs = url.query_pairs_mut();
        for p in req.params.iter().filter(|p| p.enabled) {
            pairs.append_pair(&p.key, &p.value);
        }
    }

    let mut builder = client.request(method, url);

    for h in req.headers.iter().filter(|h| h.enabled) {
        builder = builder.header(&h.key, &h.value);
    }

    builder = match req.auth_type {
        AuthType::Bearer => {
            if let Some(token) = &req.auth_config.token {
                builder.bearer_auth(token)
            } else {
                builder
            }
        }
        AuthType::Basic => {
            if let Some(user) = &req.auth_config.username {
                builder.basic_auth(user, req.auth_config.password.clone())
            } else {
                builder
            }
        }
        AuthType::Apikey => {
            if let (Some(name), Some(value)) =
                (&req.auth_config.api_key_name, &req.auth_config.api_key_value)
            {
                if req.auth_config.add_to.as_deref() == Some("query") {
                    builder.query(&[(name, value)])
                } else {
                    builder.header(name, value)
                }
            } else {
                builder
            }
        }
        AuthType::None => builder,
    };

    builder = match req.body_type {
        BodyType::Json => builder
            .header("content-type", "application/json")
            .body(req.body_content.clone()),
        BodyType::Graphql => {
            let payload = serde_json::json!({
                "query": req.graphql_query.clone().unwrap_or_default(),
                "variables": serde_json::from_str::<serde_json::Value>(
                    req.graphql_variables.as_deref().unwrap_or("{}")
                ).unwrap_or(serde_json::json!({})),
            });
            builder.header("content-type", "application/json").json(&payload)
        }
        BodyType::Raw | BodyType::Form => builder.body(req.body_content.clone()),
        BodyType::None => builder,
    };

    let start = Instant::now();
    let response = builder.send().await.context("request failed")?;
    let status = response.status();
    let mut headers = HashMap::new();
    for (name, value) in response.headers() {
        headers.insert(name.to_string(), value.to_str().unwrap_or("").to_string());
    }

    let bytes = response.bytes().await.context("reading response body")?;
    let time_ms = start.elapsed().as_millis() as u64;
    let size_bytes = bytes.len() as u64;

    let data: serde_json::Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string()));

    Ok(ApiResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        time_ms,
        size_bytes,
        headers,
        data,
        timestamp: chrono::Utc::now().to_rfc3339(),
        test_results: vec![],
        console_logs: vec![],
    })
}
