//! Pure-Rust Apache Cassandra & ScyllaDB CQL v4/v5 binary wire-protocol driver via `cdrs-tokio`.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, IndexDef, MutationItemResult,
    MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use cdrs_tokio::authenticators::StaticPasswordAuthenticatorProvider;
use cdrs_tokio::cluster::session::{Session, SessionBuilder, TcpSessionBuilder};
use cdrs_tokio::cluster::NodeTcpConfigBuilder;
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::transport::TransportTcp;
use cdrs_tokio::types::{ByIndex, ByName};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

#[derive(Default)]
pub struct CassandraDriver;

type CqlSession =
    Session<TransportTcp, cdrs_tokio::cluster::TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, cdrs_tokio::cluster::TcpConnectionManager>>;

impl CassandraDriver {
    async fn connect(
        cfg: &DatabaseConnection,
    ) -> Result<(CqlSession, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        let (contact, tunnel) = if let Some(cs) = cfg
            .connection_string
            .as_deref()
            .filter(|s| !s.trim().is_empty() && !cfg.ssh_enabled)
        {
            let clean = cs
                .trim()
                .strip_prefix("cassandra://")
                .or_else(|| cs.trim().strip_prefix("scylla://"))
                .unwrap_or(cs.trim());
            let addr = if clean.contains(':') {
                clean.to_string()
            } else {
                format!("{clean}:9042")
            };
            (addr, None)
        } else {
            let (host, port, tun) = super::ssh_tunnel::effective_target(cfg, 9042).await?;
            (format!("{host}:{port}"), tun)
        };

        let mut node_builder = NodeTcpConfigBuilder::new().with_contact_point(contact.into());
        if let Some(user) = cfg.username.as_deref().filter(|s| !s.is_empty()) {
            let pass = cfg.password.as_deref().unwrap_or("");
            node_builder = node_builder.with_authenticator_provider(Arc::new(
                StaticPasswordAuthenticatorProvider::new(user, pass),
            ));
        }

        let cluster_cfg = node_builder
            .build()
            .await
            .context("building Cassandra cluster config")?;
        let session =
            TcpSessionBuilder::new(RoundRobinLoadBalancingStrategy::new(), cluster_cfg)
                .build()
                .await
                .context("connecting to Cassandra / ScyllaDB CQL port")?;
        Ok((session, tunnel))
    }
}

fn decode_cql_cell(row: &cdrs_tokio::types::rows::Row, idx: usize) -> serde_json::Value {
    if row.is_empty(idx) {
        return serde_json::Value::Null;
    }
    if let Ok(Some(s)) = row.by_index::<String>(idx) {
        return serde_json::Value::String(s);
    }
    if let Ok(Some(i)) = row.by_index::<i64>(idx) {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(Some(i)) = row.by_index::<i32>(idx) {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(Some(b)) = row.by_index::<bool>(idx) {
        return serde_json::Value::Bool(b);
    }
    if let Ok(Some(f)) = row.by_index::<f64>(idx) {
        return serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(Some(u)) = row.by_index::<uuid::Uuid>(idx) {
        return serde_json::Value::String(u.to_string());
    }
    if let Ok(Some(blob)) = row.by_index::<cdrs_tokio::types::blob::Blob>(idx) {
        let bytes = blob.into_vec();
        if let Ok(utf8) = String::from_utf8(bytes.clone()) {
            return serde_json::Value::String(utf8);
        }
        let hex_s: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        return serde_json::Value::String(format!("0x{hex_s}"));
    }
    serde_json::Value::String("<cql_value>".into())
}

#[async_trait]
impl DbDriver for CassandraDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (session, tunnel) = Self::connect(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let env = session
            .query("SELECT release_version FROM system.local")
            .await
            .context("CQL ping failed")?;
        let body = env.response_body()?;
        let ver = body
            .into_rows()
            .and_then(|rows| rows.into_iter().next())
            .and_then(|r| r.by_index::<String>(0).ok().flatten())
            .unwrap_or_else(|| "CQL".into());
        Ok(format!(
            "Connected to {} {ver} ({}ms){via}",
            config.kind.display_name(),
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, cql: &str) -> Result<QueryResult> {
        let (session, _tunnel) = Self::connect(config).await?;
        let start = Instant::now();
        if !config.database.is_empty() && config.database != "system_schema" {
            let _ = session
                .query(format!("USE \"{}\"", config.database.replace('"', "\"\"")))
                .await;
        }
        let command = cql.split_whitespace().next().map(|s| s.to_ascii_uppercase());
        let env = session.query(cql.trim()).await.context("CQL query failed")?;
        let body = env.response_body()?;

        let columns: Vec<String> = body
            .as_rows_metadata()
            .map(|m| m.col_specs.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default();

        match body.into_rows() {
            Some(cql_rows) => {
                let mut rows = Vec::with_capacity(cql_rows.len());
                for r in &cql_rows {
                    let mut map = HashMap::new();
                    for (idx, col) in columns.iter().enumerate() {
                        map.insert(col.clone(), decode_cql_cell(r, idx));
                    }
                    rows.push(map);
                }
                Ok(QueryResult {
                    row_count: rows.len() as u64,
                    columns,
                    rows,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command,
                })
            }
            None => {
                let mut row = HashMap::new();
                row.insert("status".into(), serde_json::json!("OK"));
                row.insert("message".into(), serde_json::json!("CQL statement executed."));
                Ok(QueryResult {
                    columns: vec!["status".into(), "message".into()],
                    rows: vec![row],
                    row_count: 0,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command,
                })
            }
        }
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let (session, _tunnel) = Self::connect(config).await?;
        let ks = if config.database.trim().is_empty() {
            "system_schema"
        } else {
            config.database.trim()
        };

        let col_env = session
            .query(format!(
                "SELECT table_name, column_name, type, kind FROM system_schema.columns WHERE keyspace_name = '{}'",
                ks.replace('\'', "''")
            ))
            .await?;
        let col_rows = col_env.response_body()?.into_rows().unwrap_or_default();

        let mut grouped: BTreeMap<String, Vec<ColumnDefinition>> = BTreeMap::new();
        for r in &col_rows {
            let tname: String = r.by_name("table_name").ok().flatten().unwrap_or_default();
            let cname: String = r.by_name("column_name").ok().flatten().unwrap_or_default();
            let ctype: String = r.by_name("type").ok().flatten().unwrap_or_else(|| "text".into());
            let kind_str: String = r.by_name("kind").ok().flatten().unwrap_or_default();
            if !tname.is_empty() && !cname.is_empty() {
                grouped.entry(tname).or_default().push(ColumnDefinition {
                    name: cname,
                    col_type: ctype.to_ascii_uppercase(),
                    is_primary_key: kind_str == "partition_key" || kind_str == "clustering",
                    is_nullable: kind_str != "partition_key",
                    ..Default::default()
                });
            }
        }

        let idx_env = session
            .query(format!(
                "SELECT table_name, index_name FROM system_schema.indexes WHERE keyspace_name = '{}'",
                ks.replace('\'', "''")
            ))
            .await
            .ok();
        let mut idx_by_tbl: HashMap<String, Vec<IndexDef>> = HashMap::new();
        if let Some(env) = idx_env {
            if let Ok(body) = env.response_body() {
                for r in body.into_rows().unwrap_or_default() {
                    let tname: String = r.by_name("table_name").ok().flatten().unwrap_or_default();
                    let iname: String = r.by_name("index_name").ok().flatten().unwrap_or_default();
                    if !tname.is_empty() && !iname.is_empty() {
                        idx_by_tbl.entry(tname).or_default().push(IndexDef {
                            name: iname,
                            columns: vec![],
                            is_unique: false,
                            is_primary: false,
                        });
                    }
                }
            }
        }

        Ok(grouped
            .into_iter()
            .map(|(name, columns)| TableSchema {
                indexes: idx_by_tbl.remove(&name).unwrap_or_default(),
                name,
                object_type: "TABLE".into(),
                row_count: 0,
                columns,
                foreign_keys: vec![],
                ddl: None,
            })
            .collect())
    }

    async fn mutate_batch(
        &self,
        config: &DatabaseConnection,
        table: &str,
        mutations: &[RowMutation],
    ) -> Result<BatchMutationResult> {
        let start = Instant::now();
        if mutations.is_empty() {
            return Ok(BatchMutationResult {
                atomic: true,
                results: vec![],
                execution_time_ms: 0,
                message: None,
            });
        }

        let (session, _tunnel) = Self::connect(config).await?;
        if !config.database.is_empty() {
            let _ = session
                .query(format!("USE \"{}\"", config.database.replace('"', "\"\"")))
                .await;
        }

        let mut stmts = Vec::with_capacity(mutations.len());
        let mut results = Vec::with_capacity(mutations.len());
        for m in mutations {
            stmts.push(build_cql_mutation(table, m));
            results.push(MutationItemResult { op: m.op, rows_affected: 1 });
        }

        let batch_cql = format!("BEGIN BATCH\n{}\nAPPLY BATCH;", stmts.join("\n"));
        session
            .query(batch_cql)
            .await
            .context("CQL logged BATCH failed")?;

        Ok(BatchMutationResult {
            atomic: true,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: Some("Applied via CQL logged BATCH.".into()),
        })
    }
}

fn cql_lit(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn build_cql_mutation(table: &str, m: &RowMutation) -> String {
    let qt = format!("\"{}\"", table.replace('"', "\"\""));
    match m.op {
        MutationOp::Insert => {
            let cols: Vec<String> = m.values.keys().map(|k| format!("\"{k}\"")).collect();
            let vals: Vec<String> = m.values.values().map(cql_lit).collect();
            format!("INSERT INTO {qt} ({}) VALUES ({});", cols.join(", "), vals.join(", "))
        }
        MutationOp::Update => {
            let sets: Vec<String> = m
                .values
                .iter()
                .map(|(k, v)| format!("\"{k}\" = {}", cql_lit(v)))
                .collect();
            let whs: Vec<String> = m
                .where_clause
                .iter()
                .map(|(k, v)| format!("\"{k}\" = {}", cql_lit(v)))
                .collect();
            format!("UPDATE {qt} SET {} WHERE {};", sets.join(", "), whs.join(" AND "))
        }
        MutationOp::Delete => {
            let whs: Vec<String> = m
                .where_clause
                .iter()
                .map(|(k, v)| format!("\"{k}\" = {}", cql_lit(v)))
                .collect();
            format!("DELETE FROM {qt} WHERE {};", whs.join(" AND "))
        }
    }
}
