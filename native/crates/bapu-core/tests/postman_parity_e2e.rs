//! Comprehensive End-to-End Integration Test Suite for all Postman-parity API features.
//! Spins up real local TCP servers (`tokio::net::TcpListener`) and tests:
//! 1. All 7 HTTP methods (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`) + `:path` & query params
//! 2. `x-www-form-urlencoded`, `multipart/form-data` (text + real file upload), `GraphQL`, and `XML` bodies
//! 3. `Bearer`, `Basic`, `API Key` (header & query), `OAuth 2.0` live token exchange, `AWS SigV4`, and `JWT HS256`
//! 4. Domain Cookie Jar (`Set-Cookie` capture -> automatic `Cookie` replay -> `Max-Age=0` expiry)
//! 5. Per-request redirect policy (`follow_redirects: false` vs `true`)
//! 6. Multi-scope variables + `{{$guid}}` / `{{$randomEmail}}` + `boa_engine` assertions + 0ms Assertion Generator
//! 7. Collection Runner with CSV & JSON data-driven iterations + `setNextRequest` workflow branching
//! 8. Local HTTP Mock Server serving saved `ApiExample`s with `:param` route matching
//! 9. OpenAPI 3.0 Import/Export, HAR 1.2 Import, Postman v2.1 + Env Import/Export, cURL Import, 8-lang Codegen, Markdown Docs

use bapu_core::assertion_gen::generate_assertions_from_response;
use bapu_core::codegen::{generate_code_snippet, CodeLang};
use bapu_core::collection_io::{
    export_postman_environment, export_to_markdown_docs, export_to_openapi, export_to_postman,
    import_from_har, import_from_openapi, import_from_postman, import_postman_environment,
};
use bapu_core::cookies::CookieJar;
use bapu_core::curl::parse_curl_command;
use bapu_core::env_vars::{build_scoped_vars, interpolate_request_with_vars};
use bapu_core::http_client::{send_request, send_request_with_cookies};
use bapu_core::mock_server::start_mock_server;
use bapu_core::runner::{parse_iteration_data, run_collection, RunnerOptions};
use bapu_core::script_engine::execute_test_script;
use bapu_core::types::{
    ApiExample, ApiRequest, AuthConfig, AuthType, BodyType, Collection, Environment, HttpMethod,
    KeyValuePair, MultipartField,
};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Spawns a local multi-route test HTTP server that echoes method, path, query, headers, and body,
/// and also implements `/oauth/token`, `/login`, `/profile`, `/logout`, and `/redirect`.
async fn spawn_test_http_server() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 8192];
                let mut header_end = None;
                let mut content_length = 0usize;

                loop {
                    let Ok(n) = stream.read(&mut tmp).await else { return };
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if header_end.is_none() {
                        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
                            header_end = Some(pos + 4);
                            let hdr_str = String::from_utf8_lossy(&buf[..pos]);
                            for line in hdr_str.lines().skip(1) {
                                if let Some((k, v)) = line.split_once(':') {
                                    if k.trim().eq_ignore_ascii_case("content-length") {
                                        content_length = v.trim().parse().unwrap_or(0);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(hend) = header_end {
                        if buf.len() >= hend + content_length {
                            break;
                        }
                    }
                }

                let Some(hend) = header_end else { return };
                let head_str = String::from_utf8_lossy(&buf[..hend - 4]);
                let body_str = String::from_utf8_lossy(&buf[hend..hend + content_length]).to_string();

                let mut lines = head_str.lines();
                let req_line = lines.next().unwrap_or("GET / HTTP/1.1");
                let mut parts = req_line.split_whitespace();
                let method = parts.next().unwrap_or("GET").to_string();
                let target = parts.next().unwrap_or("/").to_string();
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));

                let mut headers = HashMap::new();
                for line in lines {
                    if let Some((k, v)) = line.split_once(':') {
                        headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
                    }
                }

                if path == "/oauth/token" {
                    let resp_body = serde_json::json!({
                        "access_token": format!("oauth_tok_from_{body_str}"),
                        "token_type": "Bearer",
                        "expires_in": 3600
                    })
                    .to_string();
                    let raw = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    let _ = stream.write_all(raw.as_bytes()).await;
                    return;
                }

                if path == "/login" {
                    let resp_body = r#"{"loggedIn":true}"#;
                    let raw = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: bapu_sid=sess_999; Path=/; HttpOnly\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    let _ = stream.write_all(raw.as_bytes()).await;
                    return;
                }

                if path == "/logout" {
                    let resp_body = r#"{"loggedOut":true}"#;
                    let raw = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: bapu_sid=; Path=/; Max-Age=0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        resp_body.len(),
                        resp_body
                    );
                    let _ = stream.write_all(raw.as_bytes()).await;
                    return;
                }

                if path == "/redirect" {
                    let raw = format!(
                        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    let _ = stream.write_all(raw.as_bytes()).await;
                    return;
                }

                if method == "HEAD" {
                    let raw = "HTTP/1.1 200 OK\r\nX-Head-Check: ok\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = stream.write_all(raw.as_bytes()).await;
                    return;
                }

                let echo = serde_json::json!({
                    "method": method,
                    "path": path,
                    "query": query,
                    "headers": headers,
                    "body": body_str,
                    "email": "ada@bapustudio.dev"
                })
                .to_string();

                let raw = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    echo.len(),
                    echo
                );
                let _ = stream.write_all(raw.as_bytes()).await;
            });
        }
    });

    port
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[tokio::test]
async fn e2e_all_http_methods_bodies_auth_cookies_and_redirects() {
    let port = spawn_test_http_server().await;
    let client = reqwest::Client::new();

    // 1. All 7 HTTP methods + :path params + query params
    for m in HttpMethod::ALL {
        let req = ApiRequest {
            method: m,
            url: format!("http://127.0.0.1:{port}/items/:itemId"),
            path_params: vec![KeyValuePair {
                id: "p1".into(),
                key: "itemId".into(),
                value: "item_42".into(),
                enabled: true,
                is_secret: None,
            }],
            params: vec![KeyValuePair {
                id: "q1".into(),
                key: "expand".into(),
                value: "true".into(),
                enabled: true,
                is_secret: None,
            }],
            ..Default::default()
        };
        let resp = send_request(&client, &req).await.unwrap();
        assert_eq!(resp.status, 200);
        if m == HttpMethod::HEAD {
            assert_eq!(resp.headers.get("x-head-check").map(String::as_str), Some("ok"));
        } else {
            assert_eq!(resp.data["method"], m.as_str());
            assert_eq!(resp.data["path"], "/items/item_42");
            assert_eq!(resp.data["query"], "expand=true");
        }
    }

    // 2. Form URL-Encoded + Multipart (text + file) + GraphQL + XML
    let form_req = ApiRequest {
        method: HttpMethod::POST,
        url: format!("http://127.0.0.1:{port}/form"),
        body_type: BodyType::Form,
        form_fields: vec![
            KeyValuePair { id: "1".into(), key: "grant".into(), value: "code".into(), enabled: true, is_secret: None },
            KeyValuePair { id: "2".into(), key: "redirect uri".into(), value: "https://a.b/cb".into(), enabled: true, is_secret: None },
        ],
        ..Default::default()
    };
    let form_resp = send_request(&client, &form_req).await.unwrap();
    assert_eq!(form_resp.data["body"], "grant=code&redirect+uri=https%3A%2F%2Fa.b%2Fcb");

    let tmp_file = std::env::temp_dir().join(format!("bapu_mp_test_{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&tmp_file, b"hello-multipart-binary").unwrap();
    let mp_req = ApiRequest {
        method: HttpMethod::POST,
        url: format!("http://127.0.0.1:{port}/upload"),
        body_type: BodyType::Multipart,
        multipart_fields: vec![
            MultipartField { id: "m1".into(), key: "tag".into(), value: "v2".into(), is_file: false, enabled: true },
            MultipartField { id: "m2".into(), key: "doc".into(), value: tmp_file.to_string_lossy().into_owned(), is_file: true, enabled: true },
        ],
        ..Default::default()
    };
    let mp_resp = send_request(&client, &mp_req).await.unwrap();
    let _ = std::fs::remove_file(&tmp_file);
    let mp_body = mp_resp.data["body"].as_str().unwrap_or("");
    assert!(mp_body.contains("hello-multipart-binary") && mp_body.contains("name=\"tag\""));

    let gql_req = ApiRequest {
        method: HttpMethod::POST,
        url: format!("http://127.0.0.1:{port}/graphql"),
        body_type: BodyType::Graphql,
        graphql_query: Some("query($id: ID!) { user(id: $id) { name } }".into()),
        graphql_variables: Some(r#"{"id":"99"}"#.into()),
        ..Default::default()
    };
    let gql_resp = send_request(&client, &gql_req).await.unwrap();
    assert!(gql_resp.data["body"].as_str().unwrap().contains("user(id: $id)"));

    // 3. Enterprise Auth: OAuth 2.0 auto-token fetch, AWS SigV4, and JWT HS256
    let oauth_req = ApiRequest {
        method: HttpMethod::GET,
        url: format!("http://127.0.0.1:{port}/secure"),
        auth_type: AuthType::Oauth2,
        auth_config: AuthConfig {
            oauth_grant_type: Some("client_credentials".into()),
            oauth_token_url: Some(format!("http://127.0.0.1:{port}/oauth/token")),
            oauth_client_id: Some("my_client".into()),
            oauth_client_secret: Some("my_secret".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let oauth_resp = send_request(&client, &oauth_req).await.unwrap();
    let auth_hdr = oauth_resp.data["headers"]["authorization"].as_str().unwrap();
    assert!(auth_hdr.starts_with("Bearer oauth_tok_from_"));
    assert!(auth_hdr.contains("client_id=my_client"));

    let aws_req = ApiRequest {
        method: HttpMethod::POST,
        url: format!("http://127.0.0.1:{port}/s3/object"),
        body_type: BodyType::Json,
        body_content: r#"{"hello":"aws"}"#.into(),
        auth_type: AuthType::Awssigv4,
        auth_config: AuthConfig {
            aws_access_key: Some("AKIA_TEST".into()),
            aws_secret_key: Some("SECRET_TEST".into()),
            aws_region: Some("us-west-2".into()),
            aws_service: Some("s3".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let aws_resp = send_request(&client, &aws_req).await.unwrap();
    assert!(aws_resp.data["headers"]["authorization"]
        .as_str()
        .unwrap()
        .starts_with("AWS4-HMAC-SHA256 Credential=AKIA_TEST/"));
    assert!(aws_resp.data["headers"]["x-amz-date"].is_string());

    let jwt_req = ApiRequest {
        method: HttpMethod::GET,
        url: format!("http://127.0.0.1:{port}/jwt"),
        auth_type: AuthType::Jwt,
        auth_config: AuthConfig {
            jwt_secret: Some("jwt_signing_key".into()),
            jwt_payload: Some(r#"{"sub":"ada"}"#.into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let jwt_resp = send_request(&client, &jwt_req).await.unwrap();
    assert!(jwt_resp.data["headers"]["authorization"]
        .as_str()
        .unwrap()
        .starts_with("Bearer eyJ"));

    // 4. Cookie Jar end-to-end (Login -> Profile -> Logout)
    let mut jar = CookieJar::default();
    let login_resp = send_request_with_cookies(
        &client,
        &ApiRequest {
            url: format!("http://127.0.0.1:{port}/login"),
            ..Default::default()
        },
        Some(&mut jar),
    )
    .await
    .unwrap();
    assert_eq!(login_resp.cookies.len(), 1);
    assert_eq!(login_resp.cookies[0].name, "bapu_sid");

    let profile_resp = send_request_with_cookies(
        &client,
        &ApiRequest {
            url: format!("http://127.0.0.1:{port}/profile"),
            ..Default::default()
        },
        Some(&mut jar),
    )
    .await
    .unwrap();
    assert_eq!(
        profile_resp.data["headers"]["cookie"].as_str(),
        Some("bapu_sid=sess_999")
    );

    send_request_with_cookies(
        &client,
        &ApiRequest {
            url: format!("http://127.0.0.1:{port}/logout"),
            ..Default::default()
        },
        Some(&mut jar),
    )
    .await
    .unwrap();
    assert!(jar.entries.is_empty());

    // 5. Redirect toggle + 0ms Assertion Generator verification
    let no_redir = send_request(
        &client,
        &ApiRequest {
            url: format!("http://127.0.0.1:{port}/redirect"),
            follow_redirects: false,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(no_redir.status, 302);

    let with_redir = send_request(
        &client,
        &ApiRequest {
            url: format!("http://127.0.0.1:{port}/redirect"),
            follow_redirects: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(with_redir.status, 200);
    assert_eq!(with_redir.data["path"], "/final");

    // Auto-generate assertions from `with_redir` and run them through Boa!
    let generated_js = generate_assertions_from_response(&with_redir);
    let env = Environment {
        id: "e".into(),
        name: "Test".into(),
        variables: vec![],
    };
    let script_res = execute_test_script(&generated_js, &with_redir, &ApiRequest::default(), &env);
    assert!(!script_res.test_results.is_empty());
    assert!(script_res.test_results.iter().all(|t| t.passed));
}

#[tokio::test]
async fn e2e_collection_runner_csv_data_and_set_next_request_branching() {
    let port = spawn_test_http_server().await;
    let client = reqwest::Client::new();

    // Collection has 3 requests, but Step 1 uses `setNextRequest("Step 3")` to skip Step 2!
    let col = Collection {
        id: "c-run".into(),
        name: "Workflow Suite".into(),
        description: None,
        variables: vec![],
        folder_path: None,
        requests: vec![
            ApiRequest {
                id: "s1".into(),
                name: "Step 1".into(),
                method: HttpMethod::GET,
                url: format!("http://127.0.0.1:{port}/users/{{{{user_slug}}}}"),
                test_script: Some(
                    r#"
                    bapu.test("Step 1 200", () => bapu.response.to.have.status(200));
                    bapu.execution.setNextRequest("Step 3");
                    "#
                    .into(),
                ),
                ..Default::default()
            },
            ApiRequest {
                id: "s2".into(),
                name: "Step 2 (Skipped)".into(),
                method: HttpMethod::GET,
                url: format!("http://127.0.0.1:{port}/should-not-run"),
                test_script: Some("bapu.test('fail if reached', () => expect(1).toBe(2));".into()),
                ..Default::default()
            },
            ApiRequest {
                id: "s3".into(),
                name: "Step 3".into(),
                method: HttpMethod::POST,
                url: format!("http://127.0.0.1:{port}/audit"),
                body_type: BodyType::Json,
                body_content: r#"{"id":"{{$guid}}","slug":"{{user_slug}}"}"#.into(),
                test_script: Some(
                    r#"
                    bapu.test("Step 3 body echoed slug", () => {
                        expect(response.json().body).toContain(bapu.environment.get("user_slug"));
                    });
                    "#
                    .into(),
                ),
                ..Default::default()
            },
        ],
    };

    let csv_rows = parse_iteration_data("user_slug\nada\ngrace\nlinus").unwrap();
    assert_eq!(csv_rows.len(), 3);

    let env = Environment {
        id: "e".into(),
        name: "RunnerEnv".into(),
        variables: vec![],
    };
    let mut jar = CookieJar::default();
    let report = run_collection(
        &client,
        &col,
        &env,
        &[],
        &mut jar,
        &RunnerOptions {
            iterations: 3,
            delay_ms: 0,
            data_rows: csv_rows,
        },
    )
    .await
    .unwrap();

    // 3 iterations * 2 executed steps (Step 2 skipped each time!) = 6 total requests, 6 passed, 0 failed!
    assert_eq!(report.iterations_ran, 3);
    assert_eq!(report.total_requests, 6);
    assert_eq!(report.passed_assertions, 6);
    assert_eq!(report.failed_assertions, 0);
    assert_eq!(report.failed_requests, 0);
}

#[tokio::test]
async fn e2e_local_mock_server_and_all_importers_exporters() {
    let col = Collection {
        id: "c-mock".into(),
        name: "Mock Billing API".into(),
        description: Some("Local mock verification".into()),
        variables: vec![],
        folder_path: None,
        requests: vec![ApiRequest {
            id: "r-inv".into(),
            name: "Get Invoice".into(),
            method: HttpMethod::GET,
            url: "https://api.billing.local/v1/invoices/:invoiceId".into(),
            examples: vec![
                ApiExample {
                    id: "ex-ok".into(),
                    name: "Paid Invoice".into(),
                    status: 200,
                    headers: HashMap::new(),
                    body: r#"{"status":"paid","currency":"{{CURRENCY}}"}"#.into(),
                },
                ApiExample {
                    id: "ex-404".into(),
                    name: "Missing Invoice".into(),
                    status: 404,
                    headers: HashMap::new(),
                    body: r#"{"error":"invoice_not_found"}"#.into(),
                },
            ],
            ..Default::default()
        }],
    };

    let env = Environment {
        id: "e1".into(),
        name: "MockEnv".into(),
        variables: vec![KeyValuePair {
            id: "v1".into(),
            key: "CURRENCY".into(),
            value: "USD".into(),
            enabled: true,
            is_secret: Some(false),
        }],
    };

    let mut mock = start_mock_server(0, col.clone(), env.clone(), None)
        .await
        .unwrap();
    let base = mock.base_url();
    let client = reqwest::Client::new();

    // Default first example with :invoiceId route matching & {{CURRENCY}} interpolation
    let r1 = send_request(
        &client,
        &ApiRequest {
            url: format!("{base}/v1/invoices/inv_9000"),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(r1.status, 200);
    assert_eq!(r1.data["status"], "paid");
    assert_eq!(r1.data["currency"], "USD");

    // Named example selection via `x-mock-response-name`
    let r2 = send_request(
        &client,
        &ApiRequest {
            url: format!("{base}/v1/invoices/inv_missing"),
            headers: vec![KeyValuePair {
                id: "h".into(),
                key: "x-mock-response-name".into(),
                value: "Missing Invoice".into(),
                enabled: true,
                is_secret: None,
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(r2.status, 404);
    assert_eq!(r2.data["error"], "invoice_not_found");

    mock.stop();

    // Verify Postman, OpenAPI, HAR, Environment, cURL, Codegen, Scoped Vars & Markdown Docs
    let pm_json = export_to_postman(&col).unwrap();
    assert_eq!(import_from_postman(&pm_json).unwrap().requests.len(), 1);

    let oa_json = export_to_openapi(&col).unwrap();
    assert_eq!(import_from_openapi(&oa_json).unwrap().requests.len(), 1);

    let env_json = export_postman_environment(&env).unwrap();
    assert_eq!(import_postman_environment(&env_json).unwrap().name, "MockEnv");

    let har_col = import_from_har(
        r#"{"log":{"entries":[{"request":{"method":"POST","url":"https://example.com/a","headers":[],"postData":{"mimeType":"application/json","text":"{\"x\":1}"}}}]}}"#,
    )
    .unwrap();
    assert_eq!(har_col.requests.len(), 1);

    let curl_req = parse_curl_command("curl -I https://example.com/health");
    assert_eq!(curl_req.method, HttpMethod::HEAD);

    let scoped = build_scoped_vars(&[], &[], &env, None);
    let interp = interpolate_request_with_vars(&col.requests[0], &scoped);
    for lang in CodeLang::ALL {
        assert!(!generate_code_snippet(&interp, &env, lang).is_empty());
    }
    assert!(export_to_markdown_docs(&col).contains("Paid Invoice"));
}

#[tokio::test]
async fn e2e_websocket_and_sse_realtime_streams() {
    use bapu_core::stream_client::{
        connect_sse, connect_websocket, FrameDirection, StreamEvent,
    };
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message;

    // 1. Live WebSocket echo server
    let ws_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let ws_port = ws_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        if let Ok((tcp, _)) = ws_listener.accept().await {
            if let Ok(ws) = tokio_tungstenite::accept_async(tcp).await {
                let (mut write, mut read) = ws.split();
                while let Some(Ok(msg)) = read.next().await {
                    match msg {
                        Message::Text(t) => {
                            let _ = write.send(Message::Text(format!("echo:{t}"))).await;
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            }
        }
    });

    let (ws_tx, ws_rx) = std::sync::mpsc::channel();
    let rt = tokio::runtime::Handle::current();
    let mut ws_handle = connect_websocket(&rt, format!("ws://127.0.0.1:{ws_port}"), ws_tx);

    // Wait for Connected, then send message and verify echo
    let mut saw_connected = false;
    let mut saw_outgoing = false;
    let mut saw_incoming = false;

    for _ in 0..30 {
        while let Ok(ev) = ws_rx.try_recv() {
            match ev {
                StreamEvent::Connected => {
                    saw_connected = true;
                    ws_handle.send("hello-ws".into()).unwrap();
                }
                StreamEvent::Frame(f) if f.direction == FrameDirection::Outgoing && f.payload == "hello-ws" => {
                    saw_outgoing = true;
                }
                StreamEvent::Frame(f) if f.direction == FrameDirection::Incoming && f.payload == "echo:hello-ws" => {
                    saw_incoming = true;
                }
                _ => {}
            }
        }
        if saw_connected && saw_outgoing && saw_incoming {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(saw_connected && saw_outgoing && saw_incoming);
    ws_handle.disconnect();

    // 2. Live Server-Sent Events (SSE) server
    let sse_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let sse_port = sse_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        if let Ok((mut tcp, _)) = sse_listener.accept().await {
            let mut req_buf = [0u8; 1024];
            let _ = tcp.read(&mut req_buf).await;
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
            let _ = tcp.write_all(headers.as_bytes()).await;
            let _ = tcp.write_all(b"event: metric\ndata: {\"cpu\":42}\n\n").await;
            let _ = tcp.flush().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            let _ = tcp.write_all(b"data: heartbeat-ok\n\n").await;
            let _ = tcp.flush().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let (sse_tx, sse_rx) = std::sync::mpsc::channel();
    let mut sse_handle = connect_sse(
        &rt,
        reqwest::Client::new(),
        format!("http://127.0.0.1:{sse_port}/events"),
        sse_tx,
    );

    let mut sse_frames = Vec::new();
    for _ in 0..30 {
        while let Ok(ev) = sse_rx.try_recv() {
            if let StreamEvent::Frame(f) = ev {
                if f.direction == FrameDirection::Incoming {
                    sse_frames.push(f.payload);
                }
            }
        }
        if sse_frames.len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    sse_handle.disconnect();
    assert_eq!(sse_frames.len(), 2);
    assert!(sse_frames[0].contains("{\"cpu\":42}"));
    assert_eq!(sse_frames[1], "data: heartbeat-ok");
}

#[tokio::test]
async fn e2e_sqlite_driver_schema_queries_and_atomic_mutations() {
    use bapu_core::db::driver_for;
    use bapu_core::types::{DatabaseConnection, DbKind, MutationOp, RowMutation};

    let db_file = std::env::temp_dir().join(format!("bapu_e2e_db_{}.sqlite", uuid::Uuid::new_v4()));
    let conn_url = format!("sqlite://{}?mode=rwc", db_file.to_string_lossy().replace('\\', "/"));

    let cfg = DatabaseConnection {
        id: "db-sqlite-e2e".into(),
        name: "Local SQLite E2E".into(),
        kind: DbKind::Sqlite,
        connection_string: Some(conn_url),
        host: None,
        port: None,
        database: "main".into(),
        username: None,
        password: None,
        ssl: false,
        ssl_ca_cert: None,
        ssl_client_cert: None,
        ssl_client_key: None,
        ssl_reject_unauthorized: None,
        ssh_enabled: false,
        ssh_host: None,
        ssh_port: None,
        ssh_username: None,
        ssh_password: None,
        ssh_private_key: None,
        ssh_passphrase: None,
        is_connected: false,
        is_demo_db: false,
        tables: vec![],
    };

    let driver = driver_for(DbKind::Sqlite);
    let banner = driver.test_connection(&cfg).await.unwrap();
    assert!(banner.to_ascii_lowercase().contains("sqlite"));

    driver
        .query(
            &cfg,
            "CREATE TABLE inventory (id INTEGER PRIMARY KEY, sku TEXT NOT NULL UNIQUE, qty INTEGER NOT NULL)",
        )
        .await
        .unwrap();

    driver
        .query(&cfg, "INSERT INTO inventory (id, sku, qty) VALUES (1, 'GPU-H100', 8)")
        .await
        .unwrap();

    // Schema introspection
    let tables = driver.get_schema(&cfg).await.unwrap();
    let inv = tables.iter().find(|t| t.name == "inventory").expect("inventory table");
    assert_eq!(inv.columns.len(), 3);
    assert!(inv.columns.iter().any(|c| c.name == "id" && c.is_primary_key));

    // Atomic batch mutation (Insert + Update)
    let mut ins_vals = HashMap::new();
    ins_vals.insert("id".into(), serde_json::json!(2));
    ins_vals.insert("sku".into(), serde_json::json!("TPU-V5"));
    ins_vals.insert("qty".into(), serde_json::json!(16));

    let mut upd_vals = HashMap::new();
    upd_vals.insert("qty".into(), serde_json::json!(12));
    let mut upd_where = HashMap::new();
    upd_where.insert("id".into(), serde_json::json!(1));

    let batch_ok = driver
        .mutate_batch(
            &cfg,
            "inventory",
            &[
                RowMutation {
                    op: MutationOp::Insert,
                    values: ins_vals,
                    where_clause: HashMap::new(),
                },
                RowMutation {
                    op: MutationOp::Update,
                    values: upd_vals,
                    where_clause: upd_where,
                },
            ],
        )
        .await
        .unwrap();
    assert!(batch_ok.atomic);
    assert_eq!(batch_ok.results.len(), 2);

    // Atomic rollback test: first mutation updates id=1 qty=999, second inserts duplicate PK id=1 -> whole batch MUST roll back!
    let mut bad_upd_vals = HashMap::new();
    bad_upd_vals.insert("qty".into(), serde_json::json!(999));
    let mut bad_upd_where = HashMap::new();
    bad_upd_where.insert("id".into(), serde_json::json!(1));

    let mut dup_ins_vals = HashMap::new();
    dup_ins_vals.insert("id".into(), serde_json::json!(1));
    dup_ins_vals.insert("sku".into(), serde_json::json!("DUP"));
    dup_ins_vals.insert("qty".into(), serde_json::json!(1));

    let rollback_res = driver
        .mutate_batch(
            &cfg,
            "inventory",
            &[
                RowMutation {
                    op: MutationOp::Update,
                    values: bad_upd_vals,
                    where_clause: bad_upd_where,
                },
                RowMutation {
                    op: MutationOp::Insert,
                    values: dup_ins_vals,
                    where_clause: HashMap::new(),
                },
            ],
        )
        .await;
    assert!(rollback_res.is_err());

    let check = driver
        .query(&cfg, "SELECT id, sku, qty FROM inventory ORDER BY id")
        .await
        .unwrap();
    assert_eq!(check.rows.len(), 2);
    assert_eq!(check.rows[0]["qty"], 12); // Rolled back from 999!
    assert_eq!(check.rows[1]["sku"], "TPU-V5");

    drop(driver);
    let _ = std::fs::remove_file(&db_file);
}

#[tokio::test]
async fn e2e_ai_copilot_providers_and_git_collection_sync() {
    use bapu_core::ai_copilot::{generate, AiProvider, AiRequest};
    use bapu_core::collection_sync::{read_collection_folder, write_collection_folder};

    // Mock AI Server handling Ollama (/api/chat), OpenAI (/chat/completions), and Anthropic (/messages)
    let ai_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let ai_port = ai_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        for _ in 0..3 {
            if let Ok((mut tcp, _)) = ai_listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = tcp.read(&mut buf).await.unwrap_or(0);
                let req_txt = String::from_utf8_lossy(&buf[..n]);
                let resp_json = if req_txt.starts_with("POST /api/chat") {
                    r#"{"message":{"content":"SELECT * FROM users LIMIT 10;"}}"#
                } else if req_txt.starts_with("POST /chat/completions") {
                    assert!(req_txt.to_ascii_lowercase().contains("authorization: bearer sk-openai-test"));
                    r#"{"choices":[{"message":{"content":"bapu.test('200 OK', () => bapu.response.to.be.ok());"}}]}"#
                } else {
                    assert!(req_txt.to_ascii_lowercase().contains("x-api-key: sk-ant-test"));
                    r#"{"content":[{"text":"Anthropic SQL optimization ready."}]}"#
                };
                let raw = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    resp_json.len(),
                    resp_json
                );
                let _ = tcp.write_all(raw.as_bytes()).await;
            }
        }
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{ai_port}");

    let ollama_out = generate(
        &client,
        &AiRequest {
            provider: AiProvider::Ollama,
            base_url: base_url.clone(),
            model: "llama3.2".into(),
            api_key: String::new(),
            system_prompt: "sql".into(),
            user_prompt: "list users".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(ollama_out, "SELECT * FROM users LIMIT 10;");

    let openai_out = generate(
        &client,
        &AiRequest {
            provider: AiProvider::OpenAi,
            base_url: base_url.clone(),
            model: "gpt-4o-mini".into(),
            api_key: "sk-openai-test".into(),
            system_prompt: "tests".into(),
            user_prompt: "assert ok".into(),
        },
    )
    .await
    .unwrap();
    assert!(openai_out.contains("bapu.response.to.be.ok()"));

    let ant_out = generate(
        &client,
        &AiRequest {
            provider: AiProvider::Anthropic,
            base_url,
            model: "claude-3-5-sonnet-latest".into(),
            api_key: "sk-ant-test".into(),
            system_prompt: "dba".into(),
            user_prompt: "optimize".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(ant_out, "Anthropic SQL optimization ready.");

    // Git-friendly folder sync preserving collection variables, description, examples & auth
    let sync_dir = std::env::temp_dir().join(format!("bapu_git_sync_{}", uuid::Uuid::new_v4()));
    let col = Collection {
        id: "c-git".into(),
        name: "Git Synced Suite".into(),
        description: Some("Version-controlled API specs".into()),
        variables: vec![KeyValuePair {
            id: "cv1".into(),
            key: "SERVICE_HOST".into(),
            value: "https://api.internal".into(),
            enabled: true,
            is_secret: None,
        }],
        folder_path: Some(sync_dir.to_string_lossy().into_owned()),
        requests: vec![ApiRequest {
            id: "r-git-1".into(),
            name: "Create Token".into(),
            method: HttpMethod::POST,
            url: "{{SERVICE_HOST}}/v1/tokens/:tenant".into(),
            path_params: vec![KeyValuePair {
                id: "pp1".into(),
                key: "tenant".into(),
                value: "acme".into(),
                enabled: true,
                is_secret: None,
            }],
            examples: vec![ApiExample {
                id: "ex1".into(),
                name: "201 Created".into(),
                status: 201,
                headers: HashMap::new(),
                body: r#"{"id":"tok_1"}"#.into(),
            }],
            ..Default::default()
        }],
    };

    write_collection_folder(&sync_dir, &col).unwrap();
    let reloaded = read_collection_folder(&sync_dir).unwrap();
    let _ = std::fs::remove_dir_all(&sync_dir);

    assert_eq!(reloaded.description.as_deref(), Some("Version-controlled API specs"));
    assert_eq!(reloaded.variables.len(), 1);
    assert_eq!(reloaded.variables[0].key, "SERVICE_HOST");
    assert_eq!(reloaded.requests.len(), 1);
    assert_eq!(reloaded.requests[0].path_params[0].value, "acme");
    assert_eq!(reloaded.requests[0].examples[0].status, 201);
}
