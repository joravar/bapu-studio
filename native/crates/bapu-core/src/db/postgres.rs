//! Real Postgres driver via `sqlx`. First fully-working database backend in the rewrite --
//! the others in this directory are stubs until ported.

use super::DbDriver;
use crate::types::{ColumnDefinition, DatabaseConnection, QueryResult, TableSchema};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{Column, Row, TypeInfo};
use std::collections::HashMap;
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
    fn connect_options(config: &DatabaseConnection) -> Result<PgConnectOptions> {
        if let Some(conn_str) = &config.connection_string {
            let mut opts = PgConnectOptions::from_str(conn_str)
                .context("invalid Postgres connection string")?;
            if config.ssl_reject_unauthorized == Some(false) {
                opts = opts.ssl_mode(PgSslMode::Require); // encrypted, but no cert verification
            }
            return Ok(opts);
        }

        let mut opts = PgConnectOptions::new()
            .host(config.host.as_deref().unwrap_or("localhost"))
            .port(config.port.as_deref().unwrap_or("5432").parse().unwrap_or(5432))
            .database(&config.database)
            .username(config.username.as_deref().unwrap_or("postgres"));
        if let Some(pw) = &config.password {
            opts = opts.password(pw);
        }

        // TLS certificate validation is ON by default (VerifyFull when ssl is requested).
        // Skipping validation is an explicit, named opt-in via ssl_reject_unauthorized == false,
        // never a silent default -- this is the fix documented in the Electron app's SECURITY.md,
        // carried over rather than re-introducing the original `rejectUnauthorized: false` bug.
        let ssl_mode = if !config.ssl {
            PgSslMode::Prefer
        } else if config.ssl_reject_unauthorized == Some(false) {
            PgSslMode::Require
        } else {
            PgSslMode::VerifyFull
        };
        opts = opts.ssl_mode(ssl_mode);

        Ok(opts)
    }

    async fn pool_for(&self, config: &DatabaseConnection) -> Result<sqlx::PgPool> {
        if let Some(pool) = self.pools.lock().unwrap().get(&config.id) {
            return Ok(pool.clone());
        }
        let opts = Self::connect_options(config)?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await
            .context("connecting to Postgres")?;
        self.pools.lock().unwrap().insert(config.id.clone(), pool.clone());
        Ok(pool)
    }
}

#[async_trait]
impl DbDriver for PostgresDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let opts = Self::connect_options(config)?;
        let pool = PgPoolOptions::new().max_connections(1).connect_with(opts).await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        pool.close().await;
        Ok(format!("Connected to PostgreSQL successfully ({}ms)", start.elapsed().as_millis()))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let pool = self.pool_for(config).await?;
        let start = Instant::now();
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

        Ok(QueryResult {
            row_count: out_rows.len() as u64,
            columns,
            rows: out_rows,
            execution_time_ms,
            error: None,
            notices: vec![],
            query_plan: None,
            command: None,
        })
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let pool = self.pool_for(config).await?;
        let table_rows = sqlx::query(
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
             ORDER BY table_name LIMIT 50",
        )
        .fetch_all(&pool)
        .await?;

        let mut tables = Vec::new();
        for row in table_rows {
            let table_name: String = row.try_get("table_name")?;
            let col_rows = sqlx::query(
                "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
                 WHERE table_name = $1 AND table_schema = 'public'",
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
                    ColumnDefinition {
                        is_primary_key: name == "id",
                        is_nullable: is_nullable == "YES",
                        name,
                        col_type: data_type.to_uppercase(),
                    }
                })
                .collect();

            tables.push(TableSchema { name: table_name, row_count: 0, columns });
        }

        Ok(tables)
    }
}

/// Best-effort conversion of a Postgres value to JSON. sqlx requires the Rust type to decode
/// into be known ahead of time, so for a generic "run any query" client we try common types in
/// order and fall back to NULL -- the same trade-off the Electron driver's dynamic JS types get
/// for free. Widening this (numeric, arrays, jsonb, timestamps) is follow-up work.
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
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    serde_json::Value::Null
}
