//! Multi-scope `{{VARIABLE}}` interpolation + built-in `{{$...}}` dynamic mock variables.
//!
//! Precedence (highest to lowest):
//! 1. Iteration data variables (CSV/JSON row in Collection Runner)
//! 2. Active `Environment` variables
//! 3. `Collection` variables
//! 4. Global variables
//! 5. Built-in dynamic variables (`{{$guid}}`, `{{$timestamp}}`, `{{$randomEmail}}`, ...)

use crate::types::{ApiRequest, AuthConfig, Environment, KeyValuePair, MultipartField};
use rand::Rng;
use std::collections::HashMap;

pub fn env_map(env: &Environment) -> HashMap<String, String> {
    kv_slice_to_map(&env.variables)
}

pub fn kv_slice_to_map(kvs: &[KeyValuePair]) -> HashMap<String, String> {
    kvs.iter()
        .filter(|kv| kv.enabled && !kv.key.trim().is_empty())
        .map(|kv| (kv.key.clone(), kv.value.clone()))
        .collect()
}

/// Merge variable scopes according to precedence: globals < collection < environment < iteration_data.
pub fn build_scoped_vars(
    globals: &[KeyValuePair],
    collection_vars: &[KeyValuePair],
    env: &Environment,
    iteration_data: Option<&HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut merged = kv_slice_to_map(globals);
    for (k, v) in kv_slice_to_map(collection_vars) {
        merged.insert(k, v);
    }
    for (k, v) in env_map(env) {
        merged.insert(k, v);
    }
    if let Some(iter_vars) = iteration_data {
        for (k, v) in iter_vars {
            merged.insert(k.clone(), v.clone());
        }
    }
    merged
}

/// Resolve a built-in `{{$...}}` dynamic variable token (without braces).
pub fn resolve_dynamic_var(key: &str) -> Option<String> {
    if !key.starts_with('$') {
        return None;
    }
    let mut rng = rand::thread_rng();
    const FIRST_NAMES: &[&str] = &[
        "Ada", "Grace", "Linus", "Margaret", "Alan", "Katherine", "Ken", "Barbara", "Donald", "Radia",
    ];
    const LAST_NAMES: &[&str] = &[
        "Lovelace", "Hopper", "Torvalds", "Hamilton", "Turing", "Johnson", "Thompson", "Liskov", "Knuth", "Perlman",
    ];
    const COMPANIES: &[&str] = &[
        "Acme Labs", "Nebula Systems", "Bapu Cloud", "Quantum Forge", "Apex Robotics", "Hyperion IO",
    ];
    const CITIES: &[&str] = &[
        "San Francisco", "Toronto", "Bengaluru", "London", "Tokyo", "Berlin", "Austin", "Zurich",
    ];

    match key {
        "$guid" | "$uuid" | "$randomUUID" => Some(uuid::Uuid::new_v4().to_string()),
        "$timestamp" => Some(chrono::Utc::now().timestamp().to_string()),
        "$isoTimestamp" => Some(chrono::Utc::now().to_rfc3339()),
        "$randomInt" => Some(rng.gen_range(0..=1000).to_string()),
        "$randomBoolean" => Some(rng.gen_bool(0.5).to_string()),
        "$randomFirstName" => Some(FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())].to_string()),
        "$randomLastName" => Some(LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())].to_string()),
        "$randomFullName" => {
            let f = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())];
            let l = LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())];
            Some(format!("{f} {l}"))
        }
        "$randomUserName" => {
            let f = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())].to_ascii_lowercase();
            let n = rng.gen_range(10..999);
            Some(format!("{f}_{n}"))
        }
        "$randomEmail" => {
            let f = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())].to_ascii_lowercase();
            let l = LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())].to_ascii_lowercase();
            let n = rng.gen_range(10..999);
            Some(format!("{f}.{l}{n}@example.com"))
        }
        "$randomHexColor" => Some(format!("#{:06x}", rng.gen_range(0u32..=0xFF_FFFF))),
        "$randomIP" => Some(format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(1..255)
        )),
        "$randomAlphaNumeric" => {
            const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
            let s: String = (0..12)
                .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
                .collect();
            Some(s)
        }
        "$randomCompanyName" => Some(COMPANIES[rng.gen_range(0..COMPANIES.len())].to_string()),
        "$randomCity" => Some(CITIES[rng.gen_range(0..CITIES.len())].to_string()),
        _ => None,
    }
}

/// Replace all `{{KEY}}` and `{{$dynamic}}` placeholders in `input`.
pub fn interpolate_str(input: &str, vars: &HashMap<String, String>) -> String {
    if !input.contains("{{") {
        return input.to_string();
    }

    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let key = after_open[..end].trim();
            if let Some(val) = vars.get(key) {
                out.push_str(val);
            } else if let Some(dyn_val) = resolve_dynamic_var(key) {
                out.push_str(&dyn_val);
            } else {
                out.push_str("{{");
                out.push_str(&after_open[..end]);
                out.push_str("}}");
            }
            rest = &after_open[end + 2..];
        } else {
            out.push_str(&rest[start..]);
            return out;
        }
    }
    out.push_str(rest);
    out
}

pub fn interpolate_request(req: &ApiRequest, env: &Environment) -> ApiRequest {
    let vars = env_map(env);
    interpolate_request_with_vars(req, &vars)
}

pub fn interpolate_request_with_vars(
    req: &ApiRequest,
    vars: &HashMap<String, String>,
) -> ApiRequest {
    let resolve_opt = |o: &Option<String>| o.as_ref().map(|s| interpolate_str(s, vars));
    let resolve_kvs = |kvs: &[KeyValuePair]| {
        kvs.iter()
            .map(|kv| KeyValuePair {
                id: kv.id.clone(),
                key: interpolate_str(&kv.key, vars),
                value: interpolate_str(&kv.value, vars),
                enabled: kv.enabled,
                is_secret: kv.is_secret,
            })
            .collect()
    };

    ApiRequest {
        id: req.id.clone(),
        name: req.name.clone(),
        method: req.method,
        url: interpolate_str(&req.url, vars),
        description: req.description.clone(),
        params: resolve_kvs(&req.params),
        path_params: resolve_kvs(&req.path_params),
        headers: resolve_kvs(&req.headers),
        body_type: req.body_type,
        body_content: interpolate_str(&req.body_content, vars),
        form_fields: resolve_kvs(&req.form_fields),
        multipart_fields: req
            .multipart_fields
            .iter()
            .map(|m| MultipartField {
                id: m.id.clone(),
                key: interpolate_str(&m.key, vars),
                value: interpolate_str(&m.value, vars),
                is_file: m.is_file,
                enabled: m.enabled,
            })
            .collect(),
        graphql_query: resolve_opt(&req.graphql_query),
        graphql_variables: resolve_opt(&req.graphql_variables),
        auth_type: req.auth_type,
        auth_config: AuthConfig {
            token: resolve_opt(&req.auth_config.token),
            username: resolve_opt(&req.auth_config.username),
            password: resolve_opt(&req.auth_config.password),
            api_key_name: resolve_opt(&req.auth_config.api_key_name),
            api_key_value: resolve_opt(&req.auth_config.api_key_value),
            add_to: req.auth_config.add_to.clone(),
            oauth_grant_type: req.auth_config.oauth_grant_type.clone(),
            oauth_token_url: resolve_opt(&req.auth_config.oauth_token_url),
            oauth_client_id: resolve_opt(&req.auth_config.oauth_client_id),
            oauth_client_secret: resolve_opt(&req.auth_config.oauth_client_secret),
            oauth_scope: resolve_opt(&req.auth_config.oauth_scope),
            oauth_header_prefix: resolve_opt(&req.auth_config.oauth_header_prefix),
            aws_access_key: resolve_opt(&req.auth_config.aws_access_key),
            aws_secret_key: resolve_opt(&req.auth_config.aws_secret_key),
            aws_session_token: resolve_opt(&req.auth_config.aws_session_token),
            aws_region: resolve_opt(&req.auth_config.aws_region),
            aws_service: resolve_opt(&req.auth_config.aws_service),
            jwt_secret: resolve_opt(&req.auth_config.jwt_secret),
            jwt_payload: resolve_opt(&req.auth_config.jwt_payload),
        },
        pre_request_script: req.pre_request_script.clone(),
        test_script: req.test_script.clone(),
        tests: req.tests.clone(),
        collection_id: req.collection_id.clone(),
        examples: req.examples.clone(),
        follow_redirects: req.follow_redirects,
        verify_tls: req.verify_tls,
        timeout_ms: req.timeout_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_known_and_preserves_unknown_vars() {
        let mut vars = HashMap::new();
        vars.insert("BASE_URL".into(), "https://api.bapustudio.dev".into());
        vars.insert("TOKEN".into(), "secret-123".into());

        assert_eq!(
            interpolate_str("{{BASE_URL}}/v1/users?auth={{ TOKEN }}&x={{UNKNOWN}}", &vars),
            "https://api.bapustudio.dev/v1/users?auth=secret-123&x={{UNKNOWN}}"
        );
    }

    #[test]
    fn resolves_dynamic_variables_and_scope_hierarchy() {
        let vars = HashMap::new();
        let guid = interpolate_str("{{$guid}}", &vars);
        assert_eq!(guid.len(), 36);
        let email = interpolate_str("{{$randomEmail}}", &vars);
        assert!(email.contains('@') && email.ends_with("@example.com"));
        let ts: i64 = interpolate_str("{{$timestamp}}", &vars).parse().unwrap();
        assert!(ts > 1_700_000_000);

        let globals = vec![KeyValuePair {
            id: "1".into(),
            key: "HOST".into(),
            value: "global".into(),
            enabled: true,
            is_secret: None,
        }];
        let col = vec![KeyValuePair {
            id: "2".into(),
            key: "HOST".into(),
            value: "collection".into(),
            enabled: true,
            is_secret: None,
        }];
        let env = Environment {
            id: "e".into(),
            name: "E".into(),
            variables: vec![KeyValuePair {
                id: "3".into(),
                key: "HOST".into(),
                value: "env".into(),
                enabled: true,
                is_secret: None,
            }],
        };
        let mut iter_map = HashMap::new();
        iter_map.insert("HOST".into(), "iter".into());

        let s1 = build_scoped_vars(&globals, &col, &env, None);
        assert_eq!(s1["HOST"], "env");
        let s2 = build_scoped_vars(&globals, &col, &env, Some(&iter_map));
        assert_eq!(s2["HOST"], "iter");
    }

    #[test]
    fn handles_unclosed_braces_safely() {
        let mut vars = HashMap::new();
        vars.insert("A".into(), "1".into());
        assert_eq!(interpolate_str("hello {{A", &vars), "hello {{A");
    }
}
