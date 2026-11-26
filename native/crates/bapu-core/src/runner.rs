//! Collection Runner & Data-Driven API Workflow Engine.
//! Supports multi-iteration runs, CSV & JSON data files, shared CookieJar state across steps,
//! script-driven workflow branching (`setNextRequest`), and latency percentile telemetry.

use crate::cookies::CookieJar;
use crate::env_vars::{build_scoped_vars, interpolate_request_with_vars};
use crate::http_client::send_request_with_cookies;
use crate::script_engine::{execute_script_with_context, ScriptExtraContext};
use crate::types::{
    ApiResponse, Collection, Environment, HttpMethod, KeyValuePair, TestAssertionResult,
};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RunnerOptions {
    pub iterations: usize,
    pub delay_ms: u64,
    pub data_rows: Vec<HashMap<String, String>>,
}

impl Default for RunnerOptions {
    fn default() -> Self {
        Self {
            iterations: 1,
            delay_ms: 0,
            data_rows: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepRunRecord {
    pub iteration: usize,
    pub request_name: String,
    pub method: HttpMethod,
    pub url: String,
    pub status: Option<u16>,
    pub time_ms: u64,
    pub assertions: Vec<TestAssertionResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CollectionRunReport {
    pub collection_name: String,
    pub iterations_ran: usize,
    pub total_requests: usize,
    pub passed_assertions: usize,
    pub failed_assertions: usize,
    pub failed_requests: usize,
    pub min_ms: u64,
    pub avg_ms: u64,
    pub max_ms: u64,
    pub p95_ms: u64,
    pub steps: Vec<StepRunRecord>,
    pub updated_env: Environment,
    pub updated_globals: Vec<KeyValuePair>,
    pub updated_collection_vars: Vec<KeyValuePair>,
}

/// Parses either a JSON array of objects (`[{"user":"a"}, ...]`) or a CSV table with headers
/// into per-iteration key-value maps.
pub fn parse_iteration_data(raw: &str) -> Result<Vec<HashMap<String, String>>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }

    if trimmed.starts_with('[') {
        let arr: Vec<serde_json::Value> =
            serde_json::from_str(trimmed).context("invalid JSON iteration array")?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            let Some(obj) = item.as_object() else {
                bail!("Each item in JSON iteration data must be an object");
            };
            let mut map = HashMap::new();
            for (k, v) in obj {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => String::new(),
                    other => other.to_string(),
                };
                map.insert(k.clone(), s);
            }
            out.push(map);
        }
        return Ok(out);
    }

    // Parse as CSV
    let mut lines = trimmed.lines().filter(|l| !l.trim().is_empty());
    let Some(header_line) = lines.next() else {
        return Ok(vec![]);
    };
    let headers = parse_csv_row(header_line);
    if headers.is_empty() {
        return Ok(vec![]);
    }

    let mut rows = Vec::new();
    for line in lines {
        let vals = parse_csv_row(line);
        let mut map = HashMap::new();
        for (i, h) in headers.iter().enumerate() {
            if !h.is_empty() {
                map.insert(h.clone(), vals.get(i).cloned().unwrap_or_default());
            }
        }
        rows.push(map);
    }
    Ok(rows)
}

fn parse_csv_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            cells.push(std::mem::take(&mut cur).trim().to_string());
        } else {
            cur.push(c);
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

pub async fn run_collection(
    client: &reqwest::Client,
    collection: &Collection,
    env: &Environment,
    globals: &[KeyValuePair],
    cookie_jar: &mut CookieJar,
    opts: &RunnerOptions,
) -> Result<CollectionRunReport> {
    if collection.requests.is_empty() {
        bail!("Collection '{}' has no requests to run.", collection.name);
    }

    let total_iters = if !opts.data_rows.is_empty() {
        opts.data_rows.len().max(opts.iterations)
    } else {
        opts.iterations.max(1)
    };

    let mut current_env = env.clone();
    let mut current_globals = globals.to_vec();
    let mut current_col_vars = collection.variables.clone();

    let mut steps: Vec<StepRunRecord> = Vec::new();
    let mut latencies: Vec<u64> = Vec::new();
    let mut passed_assertions = 0usize;
    let mut failed_assertions = 0usize;
    let mut failed_requests = 0usize;

    const MAX_STEPS_PER_ITER: usize = 100;

    for iter_idx in 0..total_iters {
        let iter_data = if opts.data_rows.is_empty() {
            None
        } else {
            opts.data_rows.get(iter_idx % opts.data_rows.len())
        };

        let mut req_idx = 0usize;
        let mut steps_in_iter = 0usize;

        while req_idx < collection.requests.len() && steps_in_iter < MAX_STEPS_PER_ITER {
            steps_in_iter += 1;
            let req = &collection.requests[req_idx];

            // Inject iteration vars into temporary env copy for scripts
            let mut script_env = current_env.clone();
            if let Some(row) = iter_data {
                for (k, v) in row {
                    upsert_kv(&mut script_env.variables, k, v);
                }
            }

            let extra = ScriptExtraContext {
                globals: current_globals.clone(),
                collection_vars: current_col_vars.clone(),
                iteration: iter_idx,
            };

            let mut explicit_next: Option<Option<String>> = None;
            if let Some(pre) = req.pre_request_script.as_deref().filter(|s| !s.trim().is_empty()) {
                let pre_res = execute_script_with_context(pre, req, &script_env, None, &extra);
                for (k, v) in pre_res.updated_env_vars {
                    upsert_kv(&mut current_env.variables, &k, &v);
                    upsert_kv(&mut script_env.variables, &k, &v);
                }
                for (k, v) in pre_res.updated_globals {
                    upsert_kv(&mut current_globals, &k, &v);
                }
                for (k, v) in pre_res.updated_collection_vars {
                    upsert_kv(&mut current_col_vars, &k, &v);
                }
                if pre_res.next_request.is_some() {
                    explicit_next = pre_res.next_request;
                }
            }

            let merged_vars = build_scoped_vars(
                &current_globals,
                &current_col_vars,
                &current_env,
                iter_data,
            );
            let resolved = interpolate_request_with_vars(req, &merged_vars);

            let send_res: Result<ApiResponse> =
                send_request_with_cookies(client, &resolved, Some(cookie_jar)).await;

            match send_res {
                Ok(resp) => {
                    latencies.push(resp.time_ms);
                    let mut assertions = Vec::new();
                    let test_src = req
                        .test_script
                        .as_deref()
                        .or(req.tests.as_deref())
                        .unwrap_or("");
                    if !test_src.trim().is_empty() {
                        let extra_post = ScriptExtraContext {
                            globals: current_globals.clone(),
                            collection_vars: current_col_vars.clone(),
                            iteration: iter_idx,
                        };
                        let post_res = execute_script_with_context(
                            test_src,
                            &resolved,
                            &script_env,
                            Some(&resp),
                            &extra_post,
                        );
                        for (k, v) in post_res.updated_env_vars {
                            upsert_kv(&mut current_env.variables, &k, &v);
                        }
                        for (k, v) in post_res.updated_globals {
                            upsert_kv(&mut current_globals, &k, &v);
                        }
                        for (k, v) in post_res.updated_collection_vars {
                            upsert_kv(&mut current_col_vars, &k, &v);
                        }
                        if post_res.next_request.is_some() {
                            explicit_next = post_res.next_request;
                        }
                        for a in &post_res.test_results {
                            if a.passed {
                                passed_assertions += 1;
                            } else {
                                failed_assertions += 1;
                            }
                        }
                        assertions = post_res.test_results;
                    }

                    steps.push(StepRunRecord {
                        iteration: iter_idx + 1,
                        request_name: req.name.clone(),
                        method: req.method,
                        url: resolved.url,
                        status: Some(resp.status),
                        time_ms: resp.time_ms,
                        assertions,
                        error: None,
                    });
                }
                Err(e) => {
                    failed_requests += 1;
                    steps.push(StepRunRecord {
                        iteration: iter_idx + 1,
                        request_name: req.name.clone(),
                        method: req.method,
                        url: resolved.url,
                        status: None,
                        time_ms: 0,
                        assertions: vec![],
                        error: Some(e.to_string()),
                    });
                }
            }

            if opts.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(opts.delay_ms)).await;
            }

            match explicit_next {
                None => req_idx += 1,
                Some(None) => break, // setNextRequest(null) stops this iteration
                Some(Some(target_name)) => {
                    if let Some(found_idx) = collection
                        .requests
                        .iter()
                        .position(|r| r.name == target_name)
                    {
                        req_idx = found_idx;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    latencies.sort_unstable();
    let (min_ms, avg_ms, max_ms, p95_ms) = if latencies.is_empty() {
        (0, 0, 0, 0)
    } else {
        let min = *latencies.first().unwrap_or(&0);
        let max = *latencies.last().unwrap_or(&0);
        let sum: u64 = latencies.iter().sum();
        let avg = sum / (latencies.len() as u64);
        let p95_idx = ((latencies.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(latencies.len() - 1);
        (min, avg, max, latencies[p95_idx])
    };

    Ok(CollectionRunReport {
        collection_name: collection.name.clone(),
        iterations_ran: total_iters,
        total_requests: steps.len(),
        passed_assertions,
        failed_assertions,
        failed_requests,
        min_ms,
        avg_ms,
        max_ms,
        p95_ms,
        steps,
        updated_env: current_env,
        updated_globals: current_globals,
        updated_collection_vars: current_col_vars,
    })
}

fn upsert_kv(kvs: &mut Vec<KeyValuePair>, key: &str, value: &str) {
    if let Some(existing) = kvs.iter_mut().find(|v| v.key == key) {
        existing.value = value.to_string();
        existing.enabled = true;
    } else {
        kvs.push(KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: key.to_string(),
            value: value.to_string(),
            enabled: true,
            is_secret: Some(false),
        });
    }
}
