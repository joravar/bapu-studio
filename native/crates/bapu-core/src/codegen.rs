//! Multi-language client code snippet generator for any `ApiRequest`.
//! Translates URL, path params, query params, headers, auth, and body into runnable
//! code across 8 popular languages & HTTP libraries.

use crate::env_vars::interpolate_request;
use crate::http_client::apply_path_params;
use crate::types::{ApiRequest, AuthType, BodyType, Environment, HttpMethod};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeLang {
    ShellCurl,
    RustReqwest,
    PythonRequests,
    PythonHttpx,
    JsFetch,
    NodeAxios,
    GoHttp,
    CSharpHttpClient,
}

impl CodeLang {
    pub const ALL: [CodeLang; 8] = [
        CodeLang::ShellCurl,
        CodeLang::RustReqwest,
        CodeLang::PythonRequests,
        CodeLang::PythonHttpx,
        CodeLang::JsFetch,
        CodeLang::NodeAxios,
        CodeLang::GoHttp,
        CodeLang::CSharpHttpClient,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            CodeLang::ShellCurl => "Shell (cURL)",
            CodeLang::RustReqwest => "Rust (reqwest)",
            CodeLang::PythonRequests => "Python (requests)",
            CodeLang::PythonHttpx => "Python (httpx)",
            CodeLang::JsFetch => "JavaScript (fetch)",
            CodeLang::NodeAxios => "Node.js (axios)",
            CodeLang::GoHttp => "Go (net/http)",
            CodeLang::CSharpHttpClient => "C# (HttpClient)",
        }
    }
}

pub fn generate_code_snippet(req: &ApiRequest, env: &Environment, lang: CodeLang) -> String {
    let resolved = interpolate_request(req, env);
    let mut full_url = apply_path_params(&resolved.url, &resolved.path_params);

    let active_params: Vec<_> = resolved
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .collect();
    if !active_params.is_empty() {
        let qs = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(active_params.iter().map(|p| (p.key.as_str(), p.value.as_str())))
            .finish();
        full_url.push(if full_url.contains('?') { '&' } else { '?' });
        full_url.push_str(&qs);
    }

    let mut headers: Vec<(String, String)> = resolved
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();

    match resolved.auth_type {
        AuthType::Bearer | AuthType::Oauth2 => {
            if let Some(tok) = resolved.auth_config.token.as_deref().filter(|s| !s.is_empty()) {
                headers.push(("Authorization".into(), format!("Bearer {tok}")));
            }
        }
        AuthType::Apikey => {
            if let (Some(k), Some(v)) = (
                resolved.auth_config.api_key_name.as_deref(),
                resolved.auth_config.api_key_value.as_deref(),
            ) {
                if resolved.auth_config.add_to.as_deref() == Some("query") {
                    let sep = if full_url.contains('?') { '&' } else { '?' };
                    full_url.push_str(&format!("{sep}{k}={v}"));
                } else {
                    headers.push((k.into(), v.into()));
                }
            }
        }
        _ => {}
    }

    let effective_body: Option<String> = match resolved.body_type {
        BodyType::None => None,
        BodyType::Graphql => {
            let vars = serde_json::from_str::<serde_json::Value>(
                resolved.graphql_variables.as_deref().unwrap_or("{}"),
            )
            .unwrap_or_else(|_| serde_json::json!({}));
            let payload = serde_json::json!({
                "query": resolved.graphql_query.unwrap_or_default(),
                "variables": vars
            });
            if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                headers.push(("Content-Type".into(), "application/json".into()));
            }
            Some(serde_json::to_string_pretty(&payload).unwrap_or_default())
        }
        BodyType::Form if !resolved.form_fields.is_empty() => {
            if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                headers.push(("Content-Type".into(), "application/x-www-form-urlencoded".into()));
            }
            Some(
                form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(
                        resolved
                            .form_fields
                            .iter()
                            .filter(|f| f.enabled)
                            .map(|f| (f.key.as_str(), f.value.as_str())),
                    )
                    .finish(),
            )
        }
        _ if !resolved.body_content.is_empty() => Some(resolved.body_content.clone()),
        _ => None,
    };

    let has_body = effective_body.is_some()
        && !matches!(resolved.method, HttpMethod::GET | HttpMethod::HEAD);
    let body_str = effective_body.unwrap_or_default();
    let m = resolved.method.as_str();

    match lang {
        CodeLang::ShellCurl => {
            let mut cmd = format!("curl -X {m} \"{full_url}\"");
            for (k, v) in &headers {
                cmd.push_str(&format!(" \\\n  -H \"{k}: {v}\""));
            }
            if has_body {
                cmd.push_str(&format!(" \\\n  -d '{}'", body_str.replace('\'', "'\\''")));
            }
            cmd
        }
        CodeLang::RustReqwest => {
            let hdr_lines: String = headers
                .iter()
                .map(|(k, v)| format!("        .header(\"{k}\", \"{v}\")\n"))
                .collect();
            let body_line = if has_body {
                format!("        .body(r#\"{body_str}\"#)\n")
            } else {
                String::new()
            };
            format!(
                "#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    let client = reqwest::Client::new();\n    let resp = client\n        .request(reqwest::Method::from_bytes(b\"{m}\")?, \"{full_url}\")\n{hdr_lines}{body_line}        .send()\n        .await?;\n    println!(\"Status: {{}}\", resp.status());\n    println!(\"{{}}\", resp.text().await?);\n    Ok(())\n}}"
            )
        }
        CodeLang::PythonRequests => {
            let hdr_map: serde_json::Map<String, serde_json::Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            let hdr_json = serde_json::to_string_pretty(&hdr_map).unwrap_or_else(|_| "{}".into());
            let data_arg = if has_body {
                format!(",\n    data={:?}", body_str)
            } else {
                String::new()
            };
            format!(
                "import requests\n\nurl = \"{full_url}\"\nheaders = {hdr_json}\n\nresponse = requests.request(\n    \"{m}\",\n    url,\n    headers=headers{data_arg}\n)\nprint(response.status_code)\nprint(response.text)"
            )
        }
        CodeLang::PythonHttpx => {
            let hdr_map: serde_json::Map<String, serde_json::Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            let hdr_json = serde_json::to_string_pretty(&hdr_map).unwrap_or_else(|_| "{}".into());
            let content_arg = if has_body {
                format!(", content={:?}", body_str)
            } else {
                String::new()
            };
            format!(
                "import asyncio\nimport httpx\n\nasync def main():\n    async with httpx.AsyncClient() as client:\n        resp = await client.request(\"{m}\", \"{full_url}\", headers={hdr_json}{content_arg})\n        print(resp.status_code, resp.text)\n\nasyncio.run(main())"
            )
        }
        CodeLang::JsFetch => {
            let hdr_map: serde_json::Map<String, serde_json::Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            let hdr_json = serde_json::to_string_pretty(&hdr_map).unwrap_or_else(|_| "{}".into());
            let body_prop = if has_body {
                format!(",\n  body: {:?}", body_str)
            } else {
                String::new()
            };
            format!(
                "const response = await fetch(\"{full_url}\", {{\n  method: \"{m}\",\n  headers: {hdr_json}{body_prop}\n}});\nconsole.log(response.status, await response.text());"
            )
        }
        CodeLang::NodeAxios => {
            let hdr_map: serde_json::Map<String, serde_json::Value> = headers
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            let hdr_json = serde_json::to_string_pretty(&hdr_map).unwrap_or_else(|_| "{}".into());
            let data_prop = if has_body {
                format!(",\n  data: {:?}", body_str)
            } else {
                String::new()
            };
            format!(
                "import axios from 'axios';\n\nconst res = await axios({{\n  method: '{}',\n  url: '{full_url}',\n  headers: {hdr_json}{data_prop}\n}});\nconsole.log(res.status, res.data);",
                m.to_ascii_lowercase()
            )
        }
        CodeLang::GoHttp => {
            let hdr_lines: String = headers
                .iter()
                .map(|(k, v)| format!("\treq.Header.Set(\"{k}\", \"{v}\")\n"))
                .collect();
            let reader = if has_body {
                format!("strings.NewReader({:?})", body_str)
            } else {
                "nil".into()
            };
            let imports = if has_body {
                "\"fmt\"\n\t\"io\"\n\t\"net/http\"\n\t\"strings\""
            } else {
                "\"fmt\"\n\t\"io\"\n\t\"net/http\""
            };
            format!(
                "package main\n\nimport (\n\t{imports}\n)\n\nfunc main() {{\n\treq, _ := http.NewRequest(\"{m}\", \"{full_url}\", {reader})\n{hdr_lines}\tres, err := http.DefaultClient.Do(req)\n\tif err != nil {{\n\t\tpanic(err)\n\t}}\n\tdefer res.Body.Close()\n\tbody, _ := io.ReadAll(res.Body)\n\tfmt.Println(res.StatusCode, string(body))\n}}"
            )
        }
        CodeLang::CSharpHttpClient => {
            let hdr_lines: String = headers
                .iter()
                .filter(|(k, _)| !k.eq_ignore_ascii_case("content-type"))
                .map(|(k, v)| format!("request.Headers.TryAddWithoutValidation(\"{k}\", \"{v}\");\n"))
                .collect();
            let body_line = if has_body {
                format!(
                    "request.Content = new StringContent({:?}, System.Text.Encoding.UTF8, \"application/json\");\n",
                    body_str
                )
            } else {
                String::new()
            };
            format!(
                "using var client = new HttpClient();\nusing var request = new HttpRequestMessage(new HttpMethod(\"{m}\"), \"{full_url}\");\n{hdr_lines}{body_line}using var response = await client.SendAsync(request);\nConsole.WriteLine(await response.Content.ReadAsStringAsync());"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KeyValuePair;

    #[test]
    fn generates_snippets_across_all_languages() {
        let req = ApiRequest {
            method: HttpMethod::POST,
            url: "{{BASE}}/users/:id".into(),
            path_params: vec![KeyValuePair {
                id: "p".into(),
                key: "id".into(),
                value: "42".into(),
                enabled: true,
                is_secret: None,
            }],
            body_type: BodyType::Json,
            body_content: r#"{"active":true}"#.into(),
            ..Default::default()
        };
        let env = Environment {
            id: "e".into(),
            name: "Dev".into(),
            variables: vec![KeyValuePair {
                id: "v".into(),
                key: "BASE".into(),
                value: "https://api.example.com".into(),
                enabled: true,
                is_secret: None,
            }],
        };

        for lang in CodeLang::ALL {
            let code = generate_code_snippet(&req, &env, lang);
            assert!(code.contains("https://api.example.com/users/42"));
            assert!(code.contains("POST") || code.contains("post"));
        }
    }
}
