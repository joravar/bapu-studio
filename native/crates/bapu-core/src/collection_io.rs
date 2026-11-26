//! Clean-room interoperability importers & exporters:
//! - OpenAPI 3.0 / 3.1 JSON (Import & Export)
//! - W3C HAR 1.2 HTTP Archive (Import)
//! - Postman Collection v2.1 JSON & Environment JSON (Compatible Import & Export)
//! - Markdown API Documentation Generator (Export)

use crate::types::{
    ApiExample, ApiRequest, AuthType, BodyType, Collection, Environment, HttpMethod, KeyValuePair,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn export_to_postman(collection: &Collection) -> Result<String> {
    let items: Vec<Value> = collection
        .requests
        .iter()
        .map(|req| {
            let headers: Vec<Value> = req
                .headers
                .iter()
                .filter(|h| h.enabled)
                .map(|h| json!({ "key": h.key, "value": h.value, "type": "text" }))
                .collect();

            let query: Vec<Value> = req
                .params
                .iter()
                .filter(|p| p.enabled)
                .map(|p| json!({ "key": p.key, "value": p.value }))
                .collect();

            let body = match req.body_type {
                BodyType::Json => Some(json!({
                    "mode": "raw",
                    "raw": if req.body_content.is_empty() { "{}" } else { &req.body_content },
                    "options": { "raw": { "language": "json" } }
                })),
                BodyType::Form if !req.form_fields.is_empty() => Some(json!({
                    "mode": "urlencoded",
                    "urlencoded": req.form_fields.iter().map(|f| json!({
                        "key": f.key,
                        "value": f.value,
                        "disabled": !f.enabled
                    })).collect::<Vec<_>>()
                })),
                BodyType::Raw | BodyType::Form | BodyType::Xml => Some(json!({
                    "mode": "raw",
                    "raw": req.body_content
                })),
                BodyType::Multipart => Some(json!({
                    "mode": "formdata",
                    "formdata": req.multipart_fields.iter().map(|f| json!({
                        "key": f.key,
                        "value": f.value,
                        "type": if f.is_file { "file" } else { "text" },
                        "disabled": !f.enabled
                    })).collect::<Vec<_>>()
                })),
                BodyType::Graphql => Some(json!({
                    "mode": "graphql",
                    "graphql": {
                        "query": req.graphql_query.as_deref().unwrap_or(""),
                        "variables": req.graphql_variables.as_deref().unwrap_or("{}")
                    }
                })),
                BodyType::None => None,
            };

            let mut events = Vec::new();
            if let Some(pre) = req.pre_request_script.as_deref().filter(|s| !s.trim().is_empty()) {
                events.push(json!({
                    "listen": "prerequest",
                    "script": {
                        "type": "text/javascript",
                        "exec": pre.lines().collect::<Vec<_>>()
                    }
                }));
            }
            let test_src = req.test_script.as_deref().or(req.tests.as_deref());
            if let Some(ts) = test_src.filter(|s| !s.trim().is_empty()) {
                events.push(json!({
                    "listen": "test",
                    "script": {
                        "type": "text/javascript",
                        "exec": ts.lines().collect::<Vec<_>>()
                    }
                }));
            }

            let auth = match req.auth_type {
                AuthType::Bearer => req.auth_config.token.as_ref().map(|t| {
                    json!({
                        "type": "bearer",
                        "bearer": [{ "key": "token", "value": t, "type": "string" }]
                    })
                }),
                AuthType::Basic => Some(json!({
                    "type": "basic",
                    "basic": [
                        { "key": "username", "value": req.auth_config.username.as_deref().unwrap_or(""), "type": "string" },
                        { "key": "password", "value": req.auth_config.password.as_deref().unwrap_or(""), "type": "string" }
                    ]
                })),
                AuthType::Apikey => Some(json!({
                    "type": "apikey",
                    "apikey": [
                        { "key": "key", "value": req.auth_config.api_key_name.as_deref().unwrap_or(""), "type": "string" },
                        { "key": "value", "value": req.auth_config.api_key_value.as_deref().unwrap_or(""), "type": "string" }
                    ]
                })),
                _ => None,
            };

            let mut url_obj = json!({ "raw": req.url });
            if !query.is_empty() {
                url_obj["query"] = Value::Array(query);
            }

            let mut req_obj = json!({
                "method": req.method.as_str(),
                "header": headers,
                "url": url_obj
            });
            if let Some(b) = body {
                req_obj["body"] = b;
            }
            if let Some(a) = auth {
                req_obj["auth"] = a;
            }

            let responses: Vec<Value> = req
                .examples
                .iter()
                .map(|ex| {
                    json!({
                        "name": ex.name,
                        "code": ex.status,
                        "body": ex.body
                    })
                })
                .collect();

            let mut item = json!({
                "name": req.name,
                "request": req_obj,
                "response": responses
            });
            if !events.is_empty() {
                item["event"] = Value::Array(events);
            }
            item
        })
        .collect();

    let vars: Vec<Value> = collection
        .variables
        .iter()
        .map(|v| json!({ "key": v.key, "value": v.value, "disabled": !v.enabled }))
        .collect();

    let doc = json!({
        "info": {
            "_postman_id": format!("bapu-{}", uuid::Uuid::new_v4()),
            "name": collection.name,
            "description": collection.description.as_deref().unwrap_or("Exported from Bapu Studio"),
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "variable": vars,
        "item": items
    });

    Ok(serde_json::to_string_pretty(&doc)?)
}

pub fn import_from_postman(raw_json: &str) -> Result<Collection> {
    let doc: Value = serde_json::from_str(raw_json).context("invalid Postman JSON")?;
    let name = doc
        .pointer("/info/name")
        .and_then(Value::as_str)
        .unwrap_or("Imported Collection")
        .to_string();
    let description = doc
        .pointer("/info/description")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let variables: Vec<KeyValuePair> = doc
        .get("variable")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(KeyValuePair {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: v.get("key")?.as_str()?.to_string(),
                        value: v.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
                        enabled: !v.get("disabled").and_then(Value::as_bool).unwrap_or(false),
                        is_secret: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut requests = Vec::new();
    if let Some(items) = doc.get("item").and_then(Value::as_array) {
        collect_postman_items(items, &mut requests);
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description,
        requests,
        variables,
        folder_path: None,
    })
}

fn collect_postman_items(items: &[Value], out: &mut Vec<ApiRequest>) {
    for item in items {
        if let Some(sub) = item.get("item").and_then(Value::as_array) {
            collect_postman_items(sub, out);
            continue;
        }
        let Some(req) = item.get("request") else {
            continue;
        };

        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Untitled Request")
            .to_string();

        let method = HttpMethod::from_str_loose(
            req.get("method").and_then(Value::as_str).unwrap_or("GET"),
        );

        let url = match req.get("url") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Object(o)) => o.get("raw").and_then(Value::as_str).unwrap_or("").to_string(),
            _ => String::new(),
        };

        let headers: Vec<KeyValuePair> = req
            .get("header")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|h| {
                        Some(KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: h.get("key")?.as_str()?.to_string(),
                            value: h.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
                            enabled: !h.get("disabled").and_then(Value::as_bool).unwrap_or(false),
                            is_secret: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let params: Vec<KeyValuePair> = req
            .pointer("/url/query")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|q| {
                        Some(KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: q.get("key")?.as_str()?.to_string(),
                            value: q.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
                            enabled: !q.get("disabled").and_then(Value::as_bool).unwrap_or(false),
                            is_secret: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut form_fields = Vec::new();
        let (body_type, body_content) = if let Some(b) = req.get("body") {
            let mode = b.get("mode").and_then(Value::as_str).unwrap_or("");
            match mode {
                "raw" => {
                    let raw = b.get("raw").and_then(Value::as_str).unwrap_or("").to_string();
                    let is_json =
                        b.pointer("/options/raw/language").and_then(Value::as_str) == Some("json")
                            || serde_json::from_str::<Value>(&raw).is_ok();
                    (if is_json { BodyType::Json } else { BodyType::Raw }, raw)
                }
                "urlencoded" => {
                    if let Some(arr) = b.get("urlencoded").and_then(Value::as_array) {
                        for f in arr {
                            if let Some(k) = f.get("key").and_then(Value::as_str) {
                                form_fields.push(KeyValuePair {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    key: k.to_string(),
                                    value: f.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
                                    enabled: !f.get("disabled").and_then(Value::as_bool).unwrap_or(false),
                                    is_secret: None,
                                });
                            }
                        }
                    }
                    (BodyType::Form, String::new())
                }
                _ => (BodyType::None, String::new()),
            }
        } else {
            (BodyType::None, String::new())
        };

        let mut pre_request_script = None;
        let mut test_script = None;
        if let Some(events) = item.get("event").and_then(Value::as_array) {
            for ev in events {
                let listen = ev.get("listen").and_then(Value::as_str).unwrap_or("");
                let lines = ev
                    .pointer("/script/exec")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join("\n")
                    });
                if listen == "prerequest" {
                    pre_request_script = lines;
                } else if listen == "test" {
                    test_script = lines;
                }
            }
        }

        let examples: Vec<ApiExample> = item
            .get("response")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .map(|ex| ApiExample {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: ex.get("name").and_then(Value::as_str).unwrap_or("Example").to_string(),
                        status: ex.get("code").and_then(Value::as_u64).unwrap_or(200) as u16,
                        headers: HashMap::new(),
                        body: ex.get("body").and_then(Value::as_str).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        out.push(ApiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            method,
            url,
            params,
            headers,
            body_type,
            body_content,
            form_fields,
            pre_request_script,
            test_script,
            examples,
            ..Default::default()
        });
    }
}

/// Imports an OpenAPI 3.0 / 3.1 JSON specification into a Bapu `Collection`.
pub fn import_from_openapi(raw_json: &str) -> Result<Collection> {
    let doc: Value = serde_json::from_str(raw_json).context("invalid OpenAPI JSON")?;
    if doc.get("openapi").is_none() && doc.get("swagger").is_none() {
        bail!("JSON document does not contain an `openapi` or `swagger` root field");
    }

    let title = doc
        .pointer("/info/title")
        .and_then(Value::as_str)
        .unwrap_or("OpenAPI Service")
        .to_string();
    let description = doc
        .pointer("/info/description")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let base_url = doc
        .pointer("/servers/0/url")
        .and_then(Value::as_str)
        .unwrap_or("https://api.example.com")
        .trim_end_matches('/');

    let mut requests = Vec::new();
    if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
        for (route, methods_val) in paths {
            let Some(methods_obj) = methods_val.as_object() else {
                continue;
            };
            for (m_key, op_val) in methods_obj {
                let upper = m_key.to_ascii_uppercase();
                if !matches!(
                    upper.as_str(),
                    "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
                ) {
                    continue;
                }
                let method = HttpMethod::from_str_loose(&upper);
                let op_name = op_val
                    .get("summary")
                    .or_else(|| op_val.get("operationId"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("{upper} {route}"));

                // Convert OpenAPI `{id}` path params to `:id` and populate `path_params`
                let mut normalized_route = route.clone();
                let mut params = Vec::new();
                let mut path_params = Vec::new();
                let mut headers = Vec::new();

                if let Some(p_arr) = op_val.get("parameters").and_then(Value::as_array) {
                    for p in p_arr {
                        let pname = p.get("name").and_then(Value::as_str).unwrap_or("");
                        let loc = p.get("in").and_then(Value::as_str).unwrap_or("query");
                        if pname.is_empty() {
                            continue;
                        }
                        let sample = p
                            .get("example")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let kv = KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: pname.to_string(),
                            value: sample,
                            enabled: true,
                            is_secret: None,
                        };
                        match loc {
                            "path" => {
                                normalized_route =
                                    normalized_route.replace(&format!("{{{pname}}}"), &format!(":{pname}"));
                                path_params.push(kv);
                            }
                            "header" => headers.push(kv),
                            _ => params.push(kv),
                        }
                    }
                }

                let (body_type, body_content) = if let Some(rb) =
                    op_val.pointer("/requestBody/content/application~1json")
                {
                    let sample = rb
                        .get("example")
                        .cloned()
                        .or_else(|| rb.get("schema").map(sample_from_schema))
                        .unwrap_or_else(|| json!({}));
                    (
                        BodyType::Json,
                        serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".into()),
                    )
                } else {
                    (BodyType::None, String::new())
                };

                requests.push(ApiRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: op_name,
                    method,
                    url: format!("{{{{BASE_URL}}}}{normalized_route}"),
                    params,
                    path_params,
                    headers,
                    body_type,
                    body_content,
                    ..Default::default()
                });
            }
        }
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: title,
        description,
        requests,
        variables: vec![KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: "BASE_URL".into(),
            value: base_url.to_string(),
            enabled: true,
            is_secret: None,
        }],
        folder_path: None,
    })
}

fn sample_from_schema(schema: &Value) -> Value {
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                for (k, prop_schema) in props {
                    map.insert(k.clone(), sample_from_schema(prop_schema));
                }
            }
            Value::Object(map)
        }
        Some("array") => {
            let item = schema.get("items").map(sample_from_schema).unwrap_or(Value::String("item".into()));
            Value::Array(vec![item])
        }
        Some("integer") | Some("number") => json!(0),
        Some("boolean") => json!(true),
        _ => json!("string"),
    }
}

/// Imports W3C HAR 1.2 (`log.entries`) browser-captured HTTP traffic into a `Collection`.
pub fn import_from_har(raw_json: &str) -> Result<Collection> {
    let doc: Value = serde_json::from_str(raw_json).context("invalid HAR JSON")?;
    let entries = doc
        .pointer("/log/entries")
        .and_then(Value::as_array)
        .context("HAR JSON missing `log.entries` array")?;

    let mut requests = Vec::new();
    for entry in entries {
        let Some(req) = entry.get("request") else { continue };
        let method = HttpMethod::from_str_loose(
            req.get("method").and_then(Value::as_str).unwrap_or("GET"),
        );
        let raw_url = req.get("url").and_then(Value::as_str).unwrap_or("");
        if raw_url.is_empty() {
            continue;
        }
        let short_name = url::Url::parse(raw_url)
            .map(|u| u.path().to_string())
            .unwrap_or_else(|_| raw_url.to_string());

        let headers: Vec<KeyValuePair> = req
            .get("headers")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|h| {
                        let k = h.get("name")?.as_str()?;
                        if k.starts_with(':') {
                            return None; // skip HTTP/2 pseudo-headers
                        }
                        Some(KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: k.to_string(),
                            value: h.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
                            enabled: true,
                            is_secret: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let (body_type, body_content) = if let Some(pd) = req.get("postData") {
            let txt = pd.get("text").and_then(Value::as_str).unwrap_or("").to_string();
            let mime = pd.get("mimeType").and_then(Value::as_str).unwrap_or("");
            let bt = if mime.contains("json") || serde_json::from_str::<Value>(&txt).is_ok() {
                BodyType::Json
            } else if mime.contains("urlencoded") {
                BodyType::Form
            } else {
                BodyType::Raw
            };
            (bt, txt)
        } else {
            (BodyType::None, String::new())
        };

        requests.push(ApiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{} {short_name}", method.as_str()),
            method,
            url: raw_url.split('?').next().unwrap_or(raw_url).to_string(),
            headers,
            body_type,
            body_content,
            ..Default::default()
        });
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Imported HAR Session".into(),
        description: Some("Captured HTTP requests imported from HAR 1.2".into()),
        requests,
        variables: vec![],
        folder_path: None,
    })
}

pub fn export_to_openapi(collection: &Collection) -> Result<String> {
    let mut paths = serde_json::Map::new();
    for req in &collection.requests {
        let path_str = reqwest::Url::parse(&req.url.replace("{{", "").replace("}}", ""))
            .map(|u| u.path().to_string())
            .unwrap_or_else(|_| "/".into());
        let method_key = req.method.as_str().to_ascii_lowercase();
        let entry = paths
            .entry(path_str)
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(
                method_key,
                json!({
                    "summary": req.name,
                    "description": req.description,
                    "responses": {
                        "200": { "description": "Successful response" }
                    }
                }),
            );
        }
    }

    let doc = json!({
        "openapi": "3.0.3",
        "info": {
            "title": collection.name,
            "version": "1.0.0",
            "description": collection.description.as_deref().unwrap_or("Exported from Bapu Studio")
        },
        "paths": Value::Object(paths)
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// Imports a Postman Environment JSON file (`{ "name": "...", "values": [...] }`).
pub fn import_postman_environment(raw_json: &str) -> Result<Environment> {
    let doc: Value = serde_json::from_str(raw_json).context("invalid Environment JSON")?;
    let name = doc
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Imported Environment")
        .to_string();
    let values = doc
        .get("values")
        .and_then(Value::as_array)
        .context("Environment JSON missing `values` array")?;

    let variables = values
        .iter()
        .filter_map(|v| {
            let key = v.get("key")?.as_str()?.to_string();
            let value = v.get("value").and_then(Value::as_str).unwrap_or("").to_string();
            let enabled = v.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            let is_secret = v.get("type").and_then(Value::as_str) == Some("secret");
            Some(KeyValuePair {
                id: uuid::Uuid::new_v4().to_string(),
                key,
                value,
                enabled,
                is_secret: Some(is_secret),
            })
        })
        .collect();

    Ok(Environment {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        variables,
    })
}

pub fn export_postman_environment(env: &Environment) -> Result<String> {
    let values: Vec<Value> = env
        .variables
        .iter()
        .map(|v| {
            json!({
                "key": v.key,
                "value": if v.is_secret.unwrap_or(false) { "" } else { &v.value },
                "type": if v.is_secret.unwrap_or(false) { "secret" } else { "default" },
                "enabled": v.enabled
            })
        })
        .collect();

    let doc = json!({
        "id": env.id,
        "name": env.name,
        "values": values,
        "_postman_variable_scope": "environment"
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// Generates shareable GitHub-Flavored Markdown API documentation from a `Collection`.
pub fn export_to_markdown_docs(collection: &Collection) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {} — API Documentation\n\n", collection.name));
    if let Some(desc) = &collection.description {
        md.push_str(&format!("{desc}\n\n"));
    }

    md.push_str("## Endpoints\n\n");
    for (idx, req) in collection.requests.iter().enumerate() {
        md.push_str(&format!(
            "- {}. **`{}`** `{}` — {}\n",
            idx + 1,
            req.method.as_str(),
            req.url,
            req.name
        ));
    }
    md.push_str("\n---\n\n");

    for req in &collection.requests {
        md.push_str(&format!("### `{}` {}\n\n", req.method.as_str(), req.name));
        md.push_str(&format!("**URL:** `{}`\n\n", req.url));
        if let Some(desc) = &req.description {
            md.push_str(&format!("{desc}\n\n"));
        }

        if !req.params.is_empty() {
            md.push_str("#### Query Parameters\n\n| Key | Value |\n| :--- | :--- |\n");
            for p in req.params.iter().filter(|p| p.enabled) {
                md.push_str(&format!("| `{}` | `{}` |\n", p.key, p.value));
            }
            md.push('\n');
        }

        if !req.headers.is_empty() {
            md.push_str("#### Headers\n\n| Header | Value |\n| :--- | :--- |\n");
            for h in req.headers.iter().filter(|h| h.enabled) {
                md.push_str(&format!("| `{}` | `{}` |\n", h.key, h.value));
            }
            md.push('\n');
        }

        if req.body_type != BodyType::None && !req.body_content.is_empty() {
            md.push_str(&format!("#### Request Body (`{:?}`)\n\n```json\n{}\n```\n\n", req.body_type, req.body_content));
        }

        for ex in &req.examples {
            md.push_str(&format!(
                "#### Example Response: {} (`HTTP {}`)\n\n```json\n{}\n```\n\n",
                ex.name, ex.status, ex.body
            ));
        }
        md.push_str("---\n\n");
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postman_openapi_har_env_and_markdown_round_trip() {
        let col = Collection {
            id: "c1".into(),
            name: "Payments API".into(),
            description: Some("Core billing endpoints".into()),
            folder_path: None,
            variables: vec![],
            requests: vec![ApiRequest {
                id: "r1".into(),
                name: "Create Charge".into(),
                method: HttpMethod::POST,
                url: "https://api.example.com/v1/charges".into(),
                headers: vec![KeyValuePair {
                    id: "h1".into(),
                    key: "X-Api-Version".into(),
                    value: "2026".into(),
                    enabled: true,
                    is_secret: None,
                }],
                body_type: BodyType::Json,
                body_content: r#"{"amount":1000}"#.into(),
                pre_request_script: Some("console.log('pre');".into()),
                test_script: Some("pm.test('ok', () => expect(response.status).toBe(200));".into()),
                examples: vec![ApiExample {
                    id: "ex1".into(),
                    name: "200 Created".into(),
                    status: 200,
                    headers: HashMap::new(),
                    body: r#"{"id":"ch_1"}"#.into(),
                }],
                ..Default::default()
            }],
        };

        let exported = export_to_postman(&col).unwrap();
        let reimported = import_from_postman(&exported).unwrap();
        assert_eq!(reimported.name, "Payments API");
        assert_eq!(reimported.requests.len(), 1);
        assert_eq!(reimported.requests[0].examples.len(), 1);

        let openapi = export_to_openapi(&col).unwrap();
        let from_oa = import_from_openapi(&openapi).unwrap();
        assert_eq!(from_oa.requests.len(), 1);

        let har = r#"{"log":{"entries":[{"request":{"method":"DELETE","url":"https://api.example.com/v1/charges/ch_1","headers":[{"name":"Authorization","value":"Bearer xyz"}]}}]}}"#;
        let from_har = import_from_har(har).unwrap();
        assert_eq!(from_har.requests[0].method, HttpMethod::DELETE);

        let md = export_to_markdown_docs(&col);
        assert!(md.contains("# Payments API"));
        assert!(md.contains("Create Charge"));
    }
}
