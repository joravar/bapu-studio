//! Native PostgreSQL driver via `sqlx` with full PK, FK, Index, View, DDL, and transactional batch support.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, ForeignKeyDef, IndexDef,
    MutationItemResult, MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{Column, Row};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
pub struct PostgresDriver {
    // One pool per connection id, built lazily on first use -- mirrors the Electron app's
    // `pgPools` Map in electron-main.cjs so repeated queries reuse the same connection pool.
    pools: Mutex<HashMap<String, sqlx::PgPool>>,
}

impl PostgresDriver {
    async fn connect_options(
        config: &DatabaseConnection,
    ) -> Result<(PgConnectOptions, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if let Some(conn_str) = config.connection_string.as_deref().filter(|s| !s.trim().is_empty()) {
            let mut opts = PgConnectOptions::from_str(conn_str)
                .context("invalid Postgres connection string")?;
            if config.ssl && config.ssl_reject_unauthorized == Some(false) {
                opts = opts.ssl_mode(PgSslMode::Require);
            }
            return Ok((opts, None));
        }

        let (host, port, tunnel) = super::ssh_tunnel::effective_target(config, 5432).await?;
        let mut opts = PgConnectOptions::new()
            .host(&host)
            .port(port)
            .database(&config.database)
            .username(config.username.as_deref().unwrap_or("postgres"));
        if let Some(pw) = &config.password {
            opts = opts.password(pw);
        }

        let ssl_mode = if !config.ssl {
            PgSslMode::Prefer
        } else if config.ssl_reject_unauthorized == Some(false) {
            PgSslMode::Require
        } else {
            PgSslMode::VerifyFull
        };
        opts = opts.ssl_mode(ssl_mode);

        if let Some(ca) = config.ssl_ca_cert.as_deref().filter(|s| !s.trim().is_empty()) {
            opts = opts.ssl_root_cert_from_pem(ca.as_bytes().to_vec());
        }
        if let Some(cert) = config.ssl_client_cert.as_deref().filter(|s| !s.trim().is_empty()) {
            opts = opts.ssl_client_cert_from_pem(cert);
        }
        if let Some(key) = config.ssl_client_key.as_deref().filter(|s| !s.trim().is_empty()) {
            opts = opts.ssl_client_key_from_pem(key);
        }

        Ok((opts, tunnel))
    }

    async fn pool_for(
        &self,
        config: &DatabaseConnection,
    ) -> Result<(sqlx::PgPool, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if !config.ssh_enabled {
            if let Some(pool) = self
                .pools
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&config.id)
            {
                return Ok((pool.clone(), None));
            }
        }
        let (opts, tunnel) = Self::connect_options(config).await?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await
            .context("connecting to Postgres")?;
        if tunnel.is_none() {
            self.pools
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(config.id.clone(), pool.clone());
        }
        Ok((pool, tunnel))
    }
}

fn is_pg_read_query(sql: &str) -> bool {
    let upper = sql.trim().to_ascii_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("WITH")
        || upper.starts_with("SHOW")
        || upper.starts_with("EXPLAIN")
        || upper.starts_with("VALUES")
        || upper.starts_with("TABLE ")
        || upper.contains(" RETURNING ")
        || upper.ends_with(" RETURNING *")
}

fn extract_index_cols(index_def: &str) -> Vec<String> {
    if let Some(open) = index_def.rfind('(') {
        if let Some(close_rel) = index_def[open + 1..].find(')') {
            return index_def[open + 1..open + 1 + close_rel]
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    vec![]
}

#[async_trait]
impl DbDriver for PostgresDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (opts, tunnel) = Self::connect_options(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let pool = PgPoolOptions::new().max_connections(1).connect_with(opts).await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        pool.close().await;
        Ok(format!(
            "Connected to PostgreSQL successfully ({}ms){via}",
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let (pool, _tunnel) = self.pool_for(config).await?;
        let start = Instant::now();
        let command = sql.split_whitespace().next().map(|s| s.to_ascii_uppercase());

        if !is_pg_read_query(sql) {
            let done = sqlx::query(sql)
                .execute(&pool)
                .await
                .context("PostgreSQL execution failed")?;
            let execution_time_ms = start.elapsed().as_millis() as u64;
            let affected = done.rows_affected();
            let mut status_row = HashMap::new();
            status_row.insert("status".into(), serde_json::Value::String("OK".into()));
            status_row.insert(
                "message".into(),
                serde_json::Value::String(format!(
                    "Statement executed successfully. {affected} row(s) affected."
                )),
            );
            return Ok(QueryResult {
                columns: vec!["status".into(), "message".into()],
                rows: vec![status_row],
                row_count: affected,
                execution_time_ms,
                error: None,
                notices: vec![format!("{affected} row(s) affected")],
                query_plan: None,
                command,
            });
        }

        let rows = sqlx::query(sql).fetch_all(&pool).await.context("query execution failed")?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        let columns: Vec<String> = rows
            .first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();

        let out_rows: Vec<HashMap<String, serde_json::Value>> = rows
            .iter()
            .map(|row| {
                let mut map = HashMap::new();
                for col in row.columns() {
                    let value = pg_value_to_json(row, col);
                    map.insert(col.name().to_string(), value);
                }
                map
            })
            .collect();

        let query_plan = if sql.trim().to_ascii_uppercase().starts_with("EXPLAIN") {
            Some(serde_json::to_string_pretty(&out_rows).unwrap_or_default())
        } else {
            None
        };

        Ok(QueryResult {
            row_count: out_rows.len() as u64,
            columns,
            rows: out_rows,
            execution_time_ms,
            error: None,
            notices: vec![],
            query_plan,
            command,
        })
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let (pool, _tunnel) = self.pool_for(config).await?;
        let table_rows = sqlx::query(
            "SELECT table_name::text AS table_name, table_type::text AS table_type \
             FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_type IN ('BASE TABLE', 'VIEW') \
             ORDER BY table_type, table_name LIMIT 100",
        )
        .fetch_all(&pool)
        .await?;

        let mut tables = Vec::new();
        for row in table_rows {
            let table_name: String = row.try_get("table_name")?;
            let table_type_raw: String = row.try_get("table_type").unwrap_or_default();
            let object_type = if table_type_raw.contains("VIEW") {
                "VIEW".to_string()
            } else {
                "TABLE".to_string()
            };

            let pk_rows = sqlx::query(
                "SELECT kcu.column_name::text AS col \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
                 WHERE tc.table_schema = 'public' AND tc.table_name = $1 AND tc.constraint_type = 'PRIMARY KEY'",
            )
            .bind(&table_name)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();
            let pk_set: HashSet<String> = pk_rows
                .iter()
                .filter_map(|r| r.try_get::<String, _>("col").ok())
                .collect();

            let uniq_rows = sqlx::query(
                "SELECT kcu.column_name::text AS col \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
                 WHERE tc.table_schema = 'public' AND tc.table_name = $1 AND tc.constraint_type = 'UNIQUE'",
            )
            .bind(&table_name)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();
            let uniq_set: HashSet<String> = uniq_rows
                .iter()
                .filter_map(|r| r.try_get::<String, _>("col").ok())
                .collect();

            let col_rows = sqlx::query(
                "SELECT column_name::text AS column_name, data_type::text AS data_type, \
                        is_nullable::text AS is_nullable, column_default::text AS column_default, \
                        is_identity::text AS is_identity \
                 FROM information_schema.columns \
                 WHERE table_name = $1 AND table_schema = 'public' \
                 ORDER BY ordinal_position",
            )
            .bind(&table_name)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let columns = col_rows
                .iter()
                .map(|c| {
                    let name: String = c.try_get("column_name").unwrap_or_default();
                    let data_type: String = c.try_get("data_type").unwrap_or_default();
                    let is_nullable: String = c.try_get("is_nullable").unwrap_or_default();
                    let default_val: Option<String> = c.try_get("column_default").ok().flatten();
                    let is_identity: String = c.try_get("is_identity").unwrap_or_default();
                    let auto_inc = is_identity == "YES"
                        || default_val
                            .as_deref()
                            .is_some_and(|d| d.starts_with("nextval("));
                    ColumnDefinition {
                        is_primary_key: pk_set.contains(&name),
                        is_nullable: is_nullable == "YES",
                        is_unique: uniq_set.contains(&name),
                        auto_increment: auto_inc,
                        default_value: default_val,
                        comment: None,
                        name,
                        col_type: data_type.to_uppercase(),
                    }
                })
                .collect();

            let fk_rows = sqlx::query(
                "SELECT tc.constraint_name::text AS constraint_name, \
                        kcu.column_name::text AS column_name, \
                        ccu.table_name::text AS ref_table, \
                        ccu.column_name::text AS ref_col, \
                        rc.delete_rule::text AS delete_rule, \
                        rc.update_rule::text AS update_rule \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu \
                   ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema \
                 JOIN information_schema.constraint_column_usage ccu \
                   ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema \
                 LEFT JOIN information_schema.referential_constraints rc \
                   ON rc.constraint_name = tc.constraint_name AND rc.constraint_schema = tc.table_schema \
                 WHERE tc.constraint_type = 'FOREIGN KEY' \
                   AND tc.table_schema = 'public' AND tc.table_name = $1",
            )
            .bind(&table_name)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let foreign_keys: Vec<ForeignKeyDef> = fk_rows
                .iter()
                .map(|r| ForeignKeyDef {
                    constraint_name: r.try_get("constraint_name").ok(),
                    column: r.try_get("column_name").unwrap_or_default(),
                    referenced_table: r.try_get("ref_table").unwrap_or_default(),
                    referenced_column: r.try_get("ref_col").unwrap_or_default(),
                    on_delete: r.try_get("delete_rule").ok(),
                    on_update: r.try_get("update_rule").ok(),
                })
                .collect();

            let idx_rows = sqlx::query(
                "SELECT indexname::text AS index_name, indexdef::text AS index_def \
                 FROM pg_indexes WHERE schemaname = 'public' AND tablename = $1",
            )
            .bind(&table_name)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let indexes: Vec<IndexDef> = idx_rows
                .iter()
                .map(|r| {
                    let idx_name: String = r.try_get("index_name").unwrap_or_default();
                    let idx_def: String = r.try_get("index_def").unwrap_or_default();
                    let is_primary = idx_name.ends_with("_pkey");
                    let is_unique =
                        is_primary || idx_def.to_ascii_uppercase().contains("UNIQUE INDEX");
                    IndexDef {
                        name: idx_name,
                        columns: extract_index_cols(&idx_def),
                        is_unique,
                        is_primary,
                    }
                })
                .collect();

            let row_count = if object_type == "TABLE" {
                let qt = quote_pg_ident(&table_name);
                sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*)::bigint FROM {qt}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(0)
                    .max(0) as u64
            } else {
                0
            };

            tables.push(TableSchema {
                name: table_name,
                object_type,
                row_count,
                columns,
                foreign_keys,
                indexes,
                ddl: None,
            });
        }

        Ok(tables)
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

        let (pool, _tunnel) = self.pool_for(config).await?;
        let mut tx = pool.begin().await.context("starting Postgres transaction")?;
        let mut results = Vec::with_capacity(mutations.len());

        for m in mutations {
            let (sql, params) = build_pg_mutation_sql(table, m)?;
            let mut q = sqlx::query(&sql);
            for val in params {
                q = bind_pg_value(q, val);
            }
            let done = q.execute(&mut *tx).await.context("executing Postgres mutation")?;
            results.push(MutationItemResult {
                op: m.op,
                rows_affected: done.rows_affected(),
            });
        }

        tx.commit().await.context("committing Postgres transaction")?;
        Ok(BatchMutationResult {
            atomic: true,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}

fn quote_pg_ident(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

fn build_pg_mutation_sql<'a>(
    table: &str,
    m: &'a RowMutation,
) -> Result<(String, Vec<&'a serde_json::Value>)> {
    let qt = quote_pg_ident(table);
    match m.op {
        MutationOp::Insert => {
            if m.values.is_empty() {
                anyhow::bail!("INSERT mutation requires at least one column value");
            }
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let col_list = cols.iter().map(|(k, _)| quote_pg_ident(k)).collect::<Vec<_>>().join(", ");
            let placeholders = (1..=cols.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ");
            let params = cols.into_iter().map(|(_, v)| v).collect();
            Ok((format!("INSERT INTO {qt} ({col_list}) VALUES ({placeholders})"), params))
        }
        MutationOp::Update => {
            if m.values.is_empty() || m.where_clause.is_empty() {
                anyhow::bail!("UPDATE mutation requires both SET values and WHERE key(s)");
            }
            let mut set_cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            set_cols.sort_by_key(|(k, _)| *k);
            let mut where_cols: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            where_cols.sort_by_key(|(k, _)| *k);

            let set_clause = set_cols
                .iter()
                .enumerate()
                .map(|(i, (k, _))| format!("{} = ${}", quote_pg_ident(k), i + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let mut param_idx = set_cols.len();
            let mut params: Vec<&serde_json::Value> = set_cols.into_iter().map(|(_, v)| v).collect();
            let where_clause = where_cols
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_pg_ident(k))
                    } else {
                        param_idx += 1;
                        params.push(v);
                        format!("{} = ${param_idx}", quote_pg_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("UPDATE {qt} SET {set_clause} WHERE {where_clause}"), params))
        }
        MutationOp::Delete => {
            if m.where_clause.is_empty() {
                anyhow::bail!("DELETE mutation requires WHERE key(s)");
            }
            let mut where_cols: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            where_cols.sort_by_key(|(k, _)| *k);
            let mut param_idx = 0usize;
            let mut params = Vec::new();
            let where_clause = where_cols
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_pg_ident(k))
                    } else {
                        param_idx += 1;
                        params.push(v);
                        format!("{} = ${param_idx}", quote_pg_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("DELETE FROM {qt} WHERE {where_clause}"), params))
        }
    }
}

fn bind_pg_value<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    val: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match val {
        serde_json::Value::Null => q.bind(None::<String>),
        serde_json::Value::Bool(b) => q.bind(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        serde_json::Value::String(s) => q.bind(s.as_str()),
        other => q.bind(other.to_string()),
    }
}

/// Best-effort conversion of a Postgres value to JSON.
fn pg_value_to_json(row: &sqlx::postgres::PgRow, col: &sqlx::postgres::PgColumn) -> serde_json::Value {
    let idx = col.ordinal();
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i16>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f32>, _>(idx) {
        return v
            .and_then(|n| serde_json::Number::from_f64(n as f64))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
        return v.unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx) {
        return v
            .map(|dt| serde_json::Value::String(dt.to_rfc3339()))
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
        return v
            .map(|dt| serde_json::Value::String(dt.to_string()))
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
        return v
            .map(|d| serde_json::Value::String(d.to_string()))
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        return serde_json::Value::String(format!("0x{hex}"));
    }
    serde_json::Value::Null
}
