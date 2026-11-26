//! Parses a `curl ...` command string into an `ApiRequest`.
//! Supports `-X`/`--request`, `-H`/`--header`, `-d`/`--data`/`--data-raw`/`--data-binary`,
//! `--data-urlencode`, `-F`/`--form` (multipart text & `@file`), `-u`/`--user` (Basic Auth),
//! `-b`/`--cookie`, `-I`/`--head`, and `-k`/`--insecure`.

use crate::types::{
    ApiRequest, AuthConfig, AuthType, BodyType, HttpMethod, KeyValuePair, MultipartField,
};

pub fn parse_curl_command(raw_curl: &str) -> ApiRequest {
    let cleaned = raw_curl
        .trim()
        .replace("\\\r\n", " ")
        .replace("\\\n", " ");

    let tokens = tokenize_shell(&cleaned);

    let mut method: Option<HttpMethod> = None;
    let mut url = String::new();
    let mut headers: Vec<KeyValuePair> = Vec::new();
    let mut params: Vec<KeyValuePair> = Vec::new();
    let mut form_fields: Vec<KeyValuePair> = Vec::new();
    let mut multipart_fields: Vec<MultipartField> = Vec::new();
    let mut body_content = String::new();
    let mut body_type = BodyType::None;
    let mut auth_type = AuthType::None;
    let mut auth_config = AuthConfig::default();
    let mut verify_tls = true;

    let mut i = 0;
    while i < tokens.len() {
        let tok = tokens[i].as_str();
        match tok {
            "curl" => {}
            "-I" | "--head" => {
                method = Some(HttpMethod::HEAD);
            }
            "-k" | "--insecure" => {
                verify_tls = false;
            }
            "-X" | "--request" => {
                if let Some(m) = tokens.get(i + 1) {
                    method = Some(HttpMethod::from_str_loose(m));
                    i += 1;
                }
            }
            "-u" | "--user" => {
                if let Some(creds) = tokens.get(i + 1) {
                    let (u, p) = creds.split_once(':').unwrap_or((creds.as_str(), ""));
                    auth_type = AuthType::Basic;
                    auth_config.username = Some(u.to_string());
                    auth_config.password = Some(p.to_string());
                    i += 1;
                }
            }
            "-b" | "--cookie" => {
                if let Some(cookie_val) = tokens.get(i + 1) {
                    headers.push(KeyValuePair {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: "Cookie".into(),
                        value: cookie_val.clone(),
                        enabled: true,
                        is_secret: None,
                    });
                    i += 1;
                }
            }
            "-H" | "--header" => {
                if let Some(h) = tokens.get(i + 1) {
                    if let Some((k, v)) = h.split_once(':') {
                        headers.push(KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: k.trim().to_string(),
                            value: v.trim().to_string(),
                            enabled: true,
                            is_secret: None,
                        });
                    }
                    i += 1;
                }
            }
            "--data-urlencode" => {
                if let Some(d) = tokens.get(i + 1) {
                    let (k, v) = d.split_once('=').unwrap_or(("", d.as_str()));
                    form_fields.push(KeyValuePair {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: true,
                        is_secret: None,
                    });
                    body_type = BodyType::Form;
                    if method.is_none() {
                        method = Some(HttpMethod::POST);
                    }
                    i += 1;
                }
            }
            "-F" | "--form" => {
                if let Some(f) = tokens.get(i + 1) {
                    if let Some((k, v)) = f.split_once('=') {
                        let is_file = v.starts_with('@');
                        let val = if is_file { &v[1..] } else { v };
                        multipart_fields.push(MultipartField {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: k.to_string(),
                            value: val.to_string(),
                            is_file,
                            enabled: true,
                        });
                        body_type = BodyType::Multipart;
                        if method.is_none() {
                            method = Some(HttpMethod::POST);
                        }
                    }
                    i += 1;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                if let Some(d) = tokens.get(i + 1) {
                    body_content = d.clone();
                    body_type = if serde_json::from_str::<serde_json::Value>(d).is_ok() {
                        BodyType::Json
                    } else {
                        BodyType::Raw
                    };
                    if method.is_none() {
                        method = Some(HttpMethod::POST);
                    }
                    i += 1;
                }
            }
            other
                if (other.starts_with("http://") || other.starts_with("https://"))
                    && url.is_empty() =>
            {
                if let Ok(parsed) = reqwest::Url::parse(other) {
                    let base = format!(
                        "{}://{}{}{}",
                        parsed.scheme(),
                        parsed.host_str().unwrap_or(""),
                        parsed.port().map(|p| format!(":{p}")).unwrap_or_default(),
                        parsed.path()
                    );
                    url = base;
                    for (k, v) in parsed.query_pairs() {
                        params.push(KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: k.into_owned(),
                            value: v.into_owned(),
                            enabled: true,
                            is_secret: None,
                        });
                    }
                } else {
                    url = other.to_string();
                }
            }
            _ => {}
        }
        i += 1;
    }

    let final_method = method.unwrap_or(HttpMethod::GET);
    ApiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("Imported from cURL ({})", final_method.as_str()),
        method: final_method,
        url: if url.is_empty() {
            "https://api.example.com/v1/resource".into()
        } else {
            url
        },
        params,
        headers,
        body_type,
        body_content,
        form_fields,
        multipart_fields,
        auth_type,
        auth_config,
        verify_tls,
        ..Default::default()
    }
}

fn tokenize_shell(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some('"') if ch == '\\' => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            Some(_) => cur.push(ch),
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            None => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_post_json_basic_auth_and_multipart_curl() {
        let req = parse_curl_command(
            r#"curl -X POST 'https://api.example.com/users?role=admin' -u 'alice:s3cr3t' -H 'X-Trace: 1' -d '{"name":"Bapu"}' -k"#,
        );
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.params[0].key, "role");
        assert_eq!(req.auth_type, AuthType::Basic);
        assert_eq!(req.auth_config.username.as_deref(), Some("alice"));
        assert!(!req.verify_tls);
        assert_eq!(req.body_type, BodyType::Json);

        let mp = parse_curl_command(
            r#"curl https://api.example.com/upload -F "title=avatar" -F "file=@photo.png""#,
        );
        assert_eq!(mp.body_type, BodyType::Multipart);
        assert_eq!(mp.multipart_fields.len(), 2);
        assert!(mp.multipart_fields[1].is_file);
        assert_eq!(mp.multipart_fields[1].value, "photo.png");
    }
}
