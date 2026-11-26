//! Built-in Local HTTP Mock Server backed by any `Collection` and its saved `ApiExample`s.
//! Binds a real `127.0.0.1:<port>` TCP listener on tokio, matches incoming `method` + `path`
//! (including `:param` / `{param}` path templates), interpolates dynamic/environment variables
//! into the response body, and logs every incoming mock hit back to the caller.

use crate::env_vars::{env_map, interpolate_str};
use crate::types::{ApiExample, ApiRequest, Collection, Environment};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct MockHit {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub matched_request: Option<String>,
    pub timestamp: String,
}

pub struct MockServerHandle {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl MockServerHandle {
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for MockServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub async fn start_mock_server(
    port: u16,
    collection: Collection,
    env: Environment,
    hits_tx: Option<mpsc::Sender<MockHit>>,
) -> Result<MockServerHandle> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("binding local mock server on 127.0.0.1:{port}"))?;
    let actual_port = listener.local_addr()?.port();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let vars = env_map(&env);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accept_res = listener.accept() => {
                    let Ok((mut socket, _)) = accept_res else { continue; };
                    let col = collection.clone();
                    let env_vars = vars.clone();
                    let tx = hits_tx.clone();

                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 16 * 1024];
                        let Ok(n) = socket.read(&mut buf).await else { return; };
                        if n == 0 {
                            return;
                        }
                        let raw = String::from_utf8_lossy(&buf[..n]);
                        let mut lines = raw.lines();
                        let first_line = lines.next().unwrap_or("GET / HTTP/1.1");
                        let mut parts = first_line.split_whitespace();
                        let method = parts.next().unwrap_or("GET").to_ascii_uppercase();
                        let full_target = parts.next().unwrap_or("/");
                        let (path, query) = full_target
                            .split_once('?')
                            .unwrap_or((full_target, ""));

                        let mut req_headers = HashMap::new();
                        for line in lines {
                            if line.is_empty() {
                                break;
                            }
                            if let Some((k, v)) = line.split_once(':') {
                                req_headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
                            }
                        }

                        let wanted_example = query_param(query, "__example")
                            .or_else(|| req_headers.get("x-mock-response-name").cloned());

                        let (status, resp_headers, body, matched_name) =
                            resolve_mock_response(&col, &method, path, wanted_example.as_deref(), &env_vars);

                        if let Some(tx) = tx {
                            let _ = tx.send(MockHit {
                                method,
                                path: full_target.to_string(),
                                status,
                                matched_request: matched_name,
                                timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
                            });
                        }

                        let reason = match status {
                            200 => "OK",
                            201 => "Created",
                            204 => "No Content",
                            400 => "Bad Request",
                            401 => "Unauthorized",
                            403 => "Forbidden",
                            404 => "Not Found",
                            _ => "Mock Response",
                        };

                        let mut http_resp = format!(
                            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n",
                            body.len()
                        );
                        let mut has_ct = false;
                        for (k, v) in resp_headers {
                            if k.eq_ignore_ascii_case("content-type") {
                                has_ct = true;
                            }
                            http_resp.push_str(&format!("{k}: {v}\r\n"));
                        }
                        if !has_ct {
                            http_resp.push_str("Content-Type: application/json\r\n");
                        }
                        http_resp.push_str("\r\n");
                        let _ = socket.write_all(http_resp.as_bytes()).await;
                        let _ = socket.write_all(body.as_bytes()).await;
                    });
                }
            }
        }
    });

    Ok(MockServerHandle {
        port: actual_port,
        shutdown_tx: Some(shutdown_tx),
    })
}

fn query_param(query: &str, name: &str) -> Option<String> {
    form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
}

fn extract_route_path(raw_url: &str) -> String {
    let cleaned = raw_url.trim();
    if let Ok(u) = url::Url::parse(&cleaned.replace("{{", "x").replace("}}", "x")) {
        return u.path().to_string();
    }
    if let Some(idx) = cleaned.find("}}/") {
        return cleaned[idx + 2..].split('?').next().unwrap_or("/").to_string();
    }
    if cleaned.starts_with('/') {
        return cleaned.split('?').next().unwrap_or("/").to_string();
    }
    "/".into()
}

fn route_matches(pattern_path: &str, incoming_path: &str) -> bool {
    let p_segs: Vec<&str> = pattern_path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    let i_segs: Vec<&str> = incoming_path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    if p_segs.len() != i_segs.len() {
        return false;
    }
    p_segs.iter().zip(i_segs.iter()).all(|(p, i)| {
        p.starts_with(':') || (p.starts_with('{') && p.ends_with('}')) || p == i
    })
}

fn resolve_mock_response(
    col: &Collection,
    method: &str,
    path: &str,
    wanted_example: Option<&str>,
    vars: &HashMap<String, String>,
) -> (u16, HashMap<String, String>, String, Option<String>) {
    let matched: Option<&ApiRequest> = col.requests.iter().find(|r| {
        r.method.as_str().eq_ignore_ascii_case(method)
            && route_matches(&extract_route_path(&r.url), path)
    });

    let Some(req) = matched else {
        let routes: Vec<String> = col
            .requests
            .iter()
            .map(|r| format!("{} {}", r.method.as_str(), extract_route_path(&r.url)))
            .collect();
        let body = serde_json::json!({
            "error": "No matching mock route in collection",
            "method": method,
            "path": path,
            "availableRoutes": routes
        })
        .to_string();
        return (404, HashMap::new(), body, None);
    };

    let chosen_ex: Option<&ApiExample> = match wanted_example {
        Some(name) => req
            .examples
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .or_else(|| req.examples.first()),
        None => req.examples.first(),
    };

    if let Some(ex) = chosen_ex {
        let rendered = interpolate_str(&ex.body, vars);
        (ex.status, ex.headers.clone(), rendered, Some(req.name.clone()))
    } else {
        let default_body = serde_json::json!({
            "mock": true,
            "request": req.name,
            "method": method,
            "path": path
        })
        .to_string();
        (200, HashMap::new(), default_body, Some(req.name.clone()))
    }
}
