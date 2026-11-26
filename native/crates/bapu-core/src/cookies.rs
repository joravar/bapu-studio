//! Domain- and path-scoped HTTP Cookie Jar (RFC 6265 subset).
//! Automatically captures `Set-Cookie` headers from responses, persists non-deleted cookies,
//! and synthesizes `Cookie: name=value; ...` headers for matching outgoing requests.

use crate::types::CookieEntry;

#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    pub entries: Vec<CookieEntry>,
}

impl CookieJar {
    pub fn from_entries(entries: Vec<CookieEntry>) -> Self {
        Self { entries }
    }

    /// Parse a single `Set-Cookie` header value against the request URL's host & path.
    pub fn parse_set_cookie(raw: &str, request_url: &url::Url) -> Option<CookieEntry> {
        let mut parts = raw.split(';');
        let first = parts.next()?.trim();
        let (name, value) = first.split_once('=')?;
        let name = name.trim().to_string();
        if name.is_empty() {
            return None;
        }
        let value = value.trim().to_string();

        let default_domain = request_url.host_str()?.to_ascii_lowercase();
        let default_path = default_cookie_path(request_url.path());

        let mut domain = default_domain;
        let mut path = default_path;
        let mut secure = false;
        let mut http_only = false;
        let mut expires = None;
        let mut max_age_zero = false;

        for attr in parts {
            let attr = attr.trim();
            if attr.eq_ignore_ascii_case("secure") {
                secure = true;
            } else if attr.eq_ignore_ascii_case("httponly") {
                http_only = true;
            } else if let Some((k, v)) = attr.split_once('=') {
                let k = k.trim();
                let v = v.trim();
                if k.eq_ignore_ascii_case("domain") {
                    let clean = v.trim_start_matches('.').to_ascii_lowercase();
                    if !clean.is_empty() {
                        domain = clean;
                    }
                } else if k.eq_ignore_ascii_case("path") && v.starts_with('/') {
                    path = v.to_string();
                } else if k.eq_ignore_ascii_case("expires") {
                    expires = Some(v.to_string());
                } else if k.eq_ignore_ascii_case("max-age") {
                    if let Ok(sec) = v.parse::<i64>() {
                        if sec <= 0 {
                            max_age_zero = true;
                        }
                    }
                }
            }
        }

        if max_age_zero {
            return Some(CookieEntry {
                domain,
                path,
                name,
                value: String::new(),
                secure,
                http_only,
                expires: Some("expired".into()),
            });
        }

        Some(CookieEntry {
            domain,
            path,
            name,
            value,
            secure,
            http_only,
            expires,
        })
    }

    /// Upsert a parsed cookie (or delete it if expired / Max-Age=0).
    pub fn store_cookie(&mut self, cookie: CookieEntry) {
        self.entries.retain(|c| {
            !(c.domain == cookie.domain && c.path == cookie.path && c.name == cookie.name)
        });
        if cookie.expires.as_deref() != Some("expired") {
            self.entries.push(cookie);
        }
    }

    /// Build the `Cookie` header string for an outgoing `url`, respecting domain, path, and `Secure`.
    pub fn cookie_header_for(&self, url: &url::Url) -> Option<String> {
        let host = url.host_str()?.to_ascii_lowercase();
        let req_path = if url.path().is_empty() { "/" } else { url.path() };
        let is_https = url.scheme().eq_ignore_ascii_case("https");

        let pairs: Vec<String> = self
            .entries
            .iter()
            .filter(|c| {
                if c.secure && !is_https {
                    return false;
                }
                domain_matches(&host, &c.domain) && path_matches(req_path, &c.path)
            })
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();

        if pairs.is_empty() {
            None
        } else {
            Some(pairs.join("; "))
        }
    }
}

fn default_cookie_path(uri_path: &str) -> String {
    if !uri_path.starts_with('/') || uri_path == "/" {
        return "/".into();
    }
    match uri_path.rfind('/') {
        Some(0) | None => "/".into(),
        Some(idx) => uri_path[..idx].to_string(),
    }
}

fn domain_matches(host: &str, cookie_domain: &str) -> bool {
    if host == cookie_domain {
        return true;
    }
    host.ends_with(&format!(".{cookie_domain}"))
}

fn path_matches(req_path: &str, cookie_path: &str) -> bool {
    if req_path == cookie_path {
        return true;
    }
    if req_path.starts_with(cookie_path) {
        return cookie_path.ends_with('/')
            || req_path.as_bytes().get(cookie_path.len()) == Some(&b'/');
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stores_and_matches_cookies_across_paths_and_security() {
        let mut jar = CookieJar::default();
        let login_url = url::Url::parse("https://api.example.com/v1/auth/login").unwrap();

        let c1 = CookieJar::parse_set_cookie(
            "sid=abc123; Path=/v1; Secure; HttpOnly",
            &login_url,
        )
        .unwrap();
        let c2 = CookieJar::parse_set_cookie("theme=dark; Domain=.example.com; Path=/", &login_url)
            .unwrap();
        jar.store_cookie(c1);
        jar.store_cookie(c2);

        let sub_https = url::Url::parse("https://sub.example.com/v1/profile").unwrap();
        // `sid` was host-only/api.example.com, `theme` is .example.com
        assert_eq!(jar.cookie_header_for(&sub_https).as_deref(), Some("theme=dark"));

        let api_https = url::Url::parse("https://api.example.com/v1/orders").unwrap();
        assert_eq!(
            jar.cookie_header_for(&api_https).as_deref(),
            Some("sid=abc123; theme=dark")
        );

        let api_http = url::Url::parse("http://api.example.com/v1/orders").unwrap();
        // `sid` is Secure so omitted over plain HTTP
        assert_eq!(jar.cookie_header_for(&api_http).as_deref(), Some("theme=dark"));

        // Max-Age=0 expires & removes cookie
        let del = CookieJar::parse_set_cookie("theme=; Domain=.example.com; Path=/; Max-Age=0", &login_url).unwrap();
        jar.store_cookie(del);
        assert_eq!(jar.cookie_header_for(&api_http), None);
    }
}
