//! Native Redis driver via the `redis` crate (async multiplexed connection).

use super::DbDriver;
use crate::types::{ColumnDefinition, DatabaseConnection, QueryResult, TableSchema};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

#[derive(Default)]
pub struct RedisDriver;

impl RedisDriver {
    async fn build_url(
        config: &DatabaseConnection,
    ) -> Result<(String, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if let Some(uri) = config.connection_string.as_deref().filter(|s| !s.trim().is_empty()) {
            return Ok((uri.to_string(), None));
        }
        let (host, port, tunnel) = super::ssh_tunnel::effective_target(config, 6379).await?;
        let scheme = if config.ssl { "rediss" } else { "redis" };
        let auth = match (config.username.as_deref(), config.password.as_deref()) {
            (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => format!("{u}:{p}@"),
            (_, Some(p)) if !p.is_empty() => format!(":{p}@"),
            _ => String::new(),
        };
        let db = config.database.parse::<u32>().unwrap_or(0);
        Ok((format!("{scheme}://{auth}{host}:{port}/{db}"), tunnel))
    }

    async fn connect(
        config: &DatabaseConnection,
    ) -> Result<(redis::aio::MultiplexedConnection, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        let (url, tunnel) = Self::build_url(config).await?;
        let client = redis::Client::open(url).context("invalid Redis connection URL")?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .context("connecting to Redis")?;
        Ok((conn, tunnel))
    }
}

/// Splits a Redis command line into tokens, respecting single/double-quoted arguments
/// (e.g. `SET mykey "hello world"`).
pub fn parse_redis_command_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for ch in line.chars() {
        match quote {
            Some(q) if ch == q => {
                quote = None;
            }
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn redis_value_to_string(v: &redis::Value) -> String {
    match v {
        redis::Value::Nil => "(nil)".into(),
        redis::Value::Int(i) => i.to_string(),
        redis::Value::BulkString(b) => String::from_utf8_lossy(b).into_owned(),
        redis::Value::SimpleString(s) => s.clone(),
        redis::Value::Okay => "OK".into(),
        redis::Value::Double(f) => f.to_string(),
        redis::Value::Boolean(b) => b.to_string(),
        redis::Value::BigNumber(n) => format!("{n:?}"),
        redis::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(redis_value_to_string).collect();
            serde_json::to_string(&items).unwrap_or_default()
        }
        other => format!("{other:?}"),
    }
}

fn format_redis_result(value: redis::Value) -> (Vec<String>, Vec<HashMap<String, serde_json::Value>>) {
    match value {
        redis::Value::Array(items) => {
            let rows = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let mut map = HashMap::new();
                    map.insert("index".into(), serde_json::Value::Number(i.into()));
                    map.insert("value".into(), serde_json::Value::String(redis_value_to_string(item)));
                    map
                })
                .collect();
            (vec!["index".into(), "value".into()], rows)
        }
        redis::Value::Map(pairs) => {
            let rows = pairs
                .iter()
                .map(|(k, v)| {
                    let mut map = HashMap::new();
                    map.insert("field".into(), serde_json::Value::String(redis_value_to_string(k)));
                    map.insert("value".into(), serde_json::Value::String(redis_value_to_string(v)));
                    map
                })
                .collect();
            (vec!["field".into(), "value".into()], rows)
        }
        other => {
            let mut map = HashMap::new();
            map.insert("result".into(), serde_json::Value::String(redis_value_to_string(&other)));
            (vec!["result".into()], vec![map])
        }
    }
}

#[async_trait]
impl DbDriver for RedisDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (mut conn, tunnel) = Self::connect(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(format!(
            "Connected to Redis successfully ({}ms){via}",
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let lines: Vec<&str> = sql
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("--"))
            .collect();
        if lines.is_empty() {
            bail!("Enter a Redis command, e.g. GET mykey or KEYS *");
        }
        let (mut conn, _tunnel) = Self::connect(config).await?;
        let start = Instant::now();

        if lines.len() == 1 {
            let tokens = parse_redis_command_line(lines[0]);
            if tokens.is_empty() {
                bail!("Enter a Redis command, e.g. GET mykey or KEYS *");
            }
            let mut cmd = redis::cmd(&tokens[0]);
            for arg in &tokens[1..] {
                cmd.arg(arg);
            }
            let raw: redis::Value = cmd.query_async(&mut conn).await.context("Redis command failed")?;
            let execution_time_ms = start.elapsed().as_millis() as u64;
            let (columns, rows) = format_redis_result(raw);
            return Ok(QueryResult {
                row_count: rows.len() as u64,
                columns,
                rows,
                execution_time_ms,
                error: None,
                notices: vec![],
                query_plan: None,
                command: Some(tokens[0].to_ascii_uppercase()),
            });
        }

        // Multi-command Redis pipeline / batch execution
        let mut rows = Vec::with_capacity(lines.len());
        for (idx, line) in lines.iter().enumerate() {
            let tokens = parse_redis_command_line(line);
            if tokens.is_empty() {
                continue;
            }
            let mut cmd = redis::cmd(&tokens[0]);
            for arg in &tokens[1..] {
                cmd.arg(arg);
            }
            let raw: redis::Value = cmd
                .query_async(&mut conn)
                .await
                .with_context(|| format!("Redis step {} (`{line}`) failed", idx + 1))?;
            let mut map = HashMap::new();
            map.insert("step".into(), serde_json::Value::Number((idx + 1).into()));
            map.insert("command".into(), serde_json::Value::String((*line).to_string()));
            map.insert("result".into(), serde_json::Value::String(redis_value_to_string(&raw)));
            rows.push(map);
        }

        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns: vec!["step".into(), "command".into(), "result".into()],
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("PIPELINE".into()),
        })
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let (mut conn, _tunnel) = Self::connect(config).await?;
        let mut type_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut cursor: u64 = 0;
        let mut scanned: usize = 0;

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("COUNT")
                .arg(200)
                .query_async(&mut conn)
                .await?;
            cursor = next_cursor;

            if !keys.is_empty() {
                let mut pipe = redis::pipe();
                for k in &keys {
                    pipe.cmd("TYPE").arg(k);
                }
                let types: Vec<String> = pipe.query_async(&mut conn).await.unwrap_or_default();
                for (key, key_type) in keys.iter().zip(types) {
                    let group = type_groups.entry(key_type).or_default();
                    if group.len() < 50 {
                        group.push(key.clone());
                    }
                }
            }
            scanned += keys.len();
            if cursor == 0 || scanned >= 1000 {
                break;
            }
        }

        let tables = type_groups
            .into_iter()
            .map(|(redis_type, keys)| TableSchema {
                name: redis_type,
                object_type: "KEYSPACE".into(),
                row_count: keys.len() as u64,
                columns: keys
                    .into_iter()
                    .map(|k| ColumnDefinition {
                        name: k,
                        col_type: "KEY".into(),
                        is_primary_key: false,
                        is_nullable: true,
                        ..Default::default()
                    })
                    .collect(),
                foreign_keys: vec![],
                indexes: vec![],
                ddl: None,
            })
            .collect();

        Ok(tables)
    }
}
