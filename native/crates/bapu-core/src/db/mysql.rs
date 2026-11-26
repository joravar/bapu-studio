//! Native MySQL / MariaDB driver via `sqlx`.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, ForeignKeyDef, IndexDef,
    MutationItemResult, MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
use sqlx::{Column, Row};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
pub struct MysqlDriver {
    pools: Mutex<HashMap<String, sqlx::MySqlPool>>,
}

impl MysqlDriver {
    async fn connect_options(
        config: &DatabaseConnection,
    ) -> Result<(MySqlConnectOptions, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if let Some(conn_str) = config.connection_string.as_deref().filter(|s| !s.trim().is_empty()) {
            let mut opts =
                MySqlConnectOptions::from_str(conn_str).context("invalid MySQL connection string")?;
            if config.ssl && config.ssl_reject_unauthorized == Some(false) {
                opts = opts.ssl_mode(MySqlSslMode::Required);
            }
            return Ok((opts, None));
        }

        let (host, port, tunnel) = super::ssh_tunnel::effective_target(config, 3306).await?;
        let mut opts = MySqlConnectOptions::new()
            .host(&host)
            .port(port)
            .username(config.username.as_deref().unwrap_or("root"));

        if !config.database.is_empty() {
            opts = opts.database(&config.database);
        }
        if let Some(pw) = &config.password {
            opts = opts.password(pw);
        }

        let ssl_mode = if !config.ssl {
            MySqlSslMode::Preferred
        } else if config.ssl_reject_unauthorized == Some(false) {
            MySqlSslMode::Required
        } else {
            MySqlSslMode::VerifyIdentity
        };
        opts = opts.ssl_mode(ssl_mode);

        if let Some(ca) = config.ssl_ca_cert.as_deref().filter(|s| !s.trim().is_empty()) {
            opts = opts.ssl_ca_from_pem(ca.as_bytes().to_vec());
        }
        if let Some(cert) = config.ssl_client_cert.as_deref().filter(|s| !s.trim().is_empty()) {
            opts = opts.ssl_client_cert_from_pem(cert.as_bytes());
        }
        if let Some(key) = config.ssl_client_key.as_deref().filter(|s| !s.trim().is_empty()) {
            opts = opts.ssl_client_key_from_pem(key.as_bytes());
        }

        Ok((opts, tunnel))
    }

    async fn pool_for(
        &self,
        config: &DatabaseConnection,
    ) -> Result<(sqlx::MySqlPool, Option<super::ssh_tunnel::SshTunnelGuard>)> {
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
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await
            .context("connecting to MySQL")?;
        if tunnel.is_none() {
            self.pools
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(config.id.clone(), pool.clone());
        }
        Ok((pool, tunnel))
    }
}

fn is_read_query(sql: &str) -> bool {
    let upper = sql.trim().to_ascii_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("SHOW")
        || upper.starts_with("DESCRIBE")
        || upper.starts_with("DESC ")
        || upper.starts_with("EXPLAIN")
        || upper.starts_with("WITH")
}

#[async_trait]
impl DbDriver for MysqlDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (opts, tunnel) = Self::connect_options(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let pool = MySqlPoolOptions::new().max_connections(1).connect_with(opts).await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        pool.close().await;
        Ok(format!(
            "Connected to MySQL successfully ({}ms){via}",
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let (pool, _tunnel) = self.pool_for(config).await?;
        let start = Instant::now();
        let command = sql.split_whitespace().next().map(|s| s.to_ascii_uppercase());

        if !is_read_query(sql) {
            let done = sqlx::query(sql).execute(&pool).await.context("MySQL execution failed")?;
            let execution_time_ms = start.elapsed().as_millis() as u64;
            let affected = done.rows_affected();
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                row_count: affected,
                execution_time_ms,
                error: None,
                notices: vec![format!("{affected} row(s) affected")],
                query_plan: None,
                command,
            });
        }

        let rows = sqlx::query(sql).fetch_all(&pool).await.context("MySQL query failed")?;
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
                    map.insert(col.name().to_string(), mysql_value_to_json(row, col));
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
            "SELECT CAST(TABLE_NAME AS CHAR) AS tname, CAST(TABLE_TYPE AS CHAR) AS ttype \
             FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = ? AND TABLE_TYPE IN ('BASE TABLE', 'VIEW') \
             ORDER BY TABLE_TYPE, TABLE_NAME LIMIT 100",
        )
        .bind(&config.database)
        .fetch_all(&pool)
        .await?;

        let mut tables = Vec::new();
        for row in table_rows {
            let table_name = mysql_row_str(&row, "tname");
            if table_name.is_empty() {
                continue;
            }
            let ttype = mysql_row_str(&row, "ttype");
            let object_type = if ttype.contains("VIEW") {
                "VIEW".to_string()
            } else {
                "TABLE".to_string()
            };

            let safe_name = table_name.replace('`', "``");
            let col_rows = sqlx::query(&format!("DESCRIBE `{safe_name}`"))
                .fetch_all(&pool)
                .await
                .unwrap_or_default();

            let columns = col_rows
                .iter()
                .map(|c| {
                    let name = mysql_row_str(c, "Field");
                    let col_type = {
                        let t = mysql_row_str(c, "Type");
                        if t.is_empty() { "VARCHAR".into() } else { t }
                    };
                    let key = mysql_row_str(c, "Key");
                    let null_flag = mysql_row_str(c, "Null");
                    let default_val = mysql_row_opt_str(c, "Default");
                    let extra = mysql_row_str(c, "Extra");
                    ColumnDefinition {
                        name,
                        col_type: col_type.to_uppercase(),
                        is_primary_key: key == "PRI",
                        is_nullable: null_flag == "YES",
                        is_unique: key == "UNI",
                        auto_increment: extra.to_ascii_lowercase().contains("auto_increment"),
                        default_value: default_val,
                        comment: None,
                    }
                })
                .collect();

            let fk_rows = sqlx::query(
                "SELECT CAST(k.CONSTRAINT_NAME AS CHAR) AS cname, \
                        CAST(k.COLUMN_NAME AS CHAR) AS col_name, \
                        CAST(k.REFERENCED_TABLE_NAME AS CHAR) AS ref_table, \
                        CAST(k.REFERENCED_COLUMN_NAME AS CHAR) AS ref_col, \
                        CAST(r.DELETE_RULE AS CHAR) AS del_rule, \
                        CAST(r.UPDATE_RULE AS CHAR) AS upd_rule \
                 FROM information_schema.KEY_COLUMN_USAGE k \
                 LEFT JOIN information_schema.REFERENTIAL_CONSTRAINTS r \
                   ON k.CONSTRAINT_SCHEMA = r.CONSTRAINT_SCHEMA AND k.CONSTRAINT_NAME = r.CONSTRAINT_NAME \
                 WHERE k.TABLE_SCHEMA = ? AND k.TABLE_NAME = ? AND k.REFERENCED_TABLE_NAME IS NOT NULL",
            )
            .bind(&config.database)
            .bind(&table_name)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let foreign_keys: Vec<ForeignKeyDef> = fk_rows
                .iter()
                .map(|r| ForeignKeyDef {
                    constraint_name: mysql_row_opt_str(r, "cname"),
                    column: mysql_row_str(r, "col_name"),
                    referenced_table: mysql_row_str(r, "ref_table"),
                    referenced_column: mysql_row_str(r, "ref_col"),
                    on_delete: mysql_row_opt_str(r, "del_rule"),
                    on_update: mysql_row_opt_str(r, "upd_rule"),
                })
                .collect();

            let mut indexes: Vec<IndexDef> = Vec::new();
            if object_type == "TABLE" {
                let idx_rows = sqlx::query(&format!("SHOW INDEX FROM `{safe_name}`"))
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default();
                let mut idx_map: std::collections::BTreeMap<String, (bool, Vec<String>)> =
                    std::collections::BTreeMap::new();
                for r in &idx_rows {
                    let kname = mysql_row_str(r, "Key_name");
                    let cname = mysql_row_str(r, "Column_name");
                    let non_unique: i64 = r.try_get("Non_unique").unwrap_or(1);
                    if !kname.is_empty() && !cname.is_empty() {
                        let entry = idx_map.entry(kname).or_insert((non_unique == 0, Vec::new()));
                        entry.1.push(cname);
                    }
                }
                for (name, (is_unique, cols)) in idx_map {
                    let is_primary = name == "PRIMARY";
                    indexes.push(IndexDef {
                        name,
                        columns: cols,
                        is_unique,
                        is_primary,
                    });
                }
            }

            let ddl = sqlx::query(&format!(
                "SHOW CREATE {} `{safe_name}`",
                if object_type == "VIEW" { "VIEW" } else { "TABLE" }
            ))
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| {
                r.try_get::<String, _>(1)
                    .ok()
                    .or_else(|| r.try_get::<Vec<u8>, _>(1).ok().map(|b| String::from_utf8_lossy(&b).into_owned()))
            });

            let row_count = if object_type == "TABLE" {
                sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM `{safe_name}`"))
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
                ddl,
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
        let mut tx = pool.begin().await.context("starting MySQL transaction")?;
        let mut results = Vec::with_capacity(mutations.len());

        for m in mutations {
            let (sql, params) = build_mysql_mutation_sql(table, m)?;
            let mut q = sqlx::query(&sql);
            for val in params {
                q = bind_mysql_value(q, val);
            }
            let done = q.execute(&mut *tx).await.context("executing MySQL mutation")?;
            results.push(MutationItemResult {
                op: m.op,
                rows_affected: done.rows_affected(),
            });
        }

        tx.commit().await.context("committing MySQL transaction")?;
        Ok(BatchMutationResult {
            atomic: true,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}

fn quote_mysql_ident(id: &str) -> String {
    format!("`{}`", id.replace('`', "``"))
}

fn build_mysql_mutation_sql<'a>(
    table: &str,
    m: &'a RowMutation,
) -> Result<(String, Vec<&'a serde_json::Value>)> {
    let qt = quote_mysql_ident(table);
    match m.op {
        MutationOp::Insert => {
            if m.values.is_empty() {
                anyhow::bail!("INSERT mutation requires at least one column value");
            }
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let col_list = cols.iter().map(|(k, _)| quote_mysql_ident(k)).collect::<Vec<_>>().join(", ");
            let placeholders = vec!["?"; cols.len()].join(", ");
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
                .map(|(k, _)| format!("{} = ?", quote_mysql_ident(k)))
                .collect::<Vec<_>>()
                .join(", ");
            let mut params: Vec<&serde_json::Value> = set_cols.into_iter().map(|(_, v)| v).collect();
            let where_clause = where_cols
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_mysql_ident(k))
                    } else {
                        params.push(v);
                        format!("{} = ?", quote_mysql_ident(k))
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
            let mut params = Vec::new();
            let where_clause = where_cols
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_mysql_ident(k))
                    } else {
                        params.push(v);
                        format!("{} = ?", quote_mysql_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("DELETE FROM {qt} WHERE {where_clause}"), params))
        }
    }
}

fn bind_mysql_value<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    val: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
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

fn mysql_row_opt_str(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<String> {
    if let Ok(opt) = row.try_get::<Option<String>, _>(col) {
        return opt;
    }
    if let Ok(opt) = row.try_get::<Option<Vec<u8>>, _>(col) {
        return opt.map(|b| String::from_utf8_lossy(&b).into_owned());
    }
    None
}

fn mysql_row_str(row: &sqlx::mysql::MySqlRow, col: &str) -> String {
    mysql_row_opt_str(row, col).unwrap_or_default()
}

fn mysql_value_to_json(
    row: &sqlx::mysql::MySqlRow,
    col: &sqlx::mysql::MySqlColumn,
) -> serde_json::Value {
    let idx = col.ordinal();
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<u64>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<u32>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i16>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i8>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
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
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
        return v
            .map(|dt| serde_json::Value::String(dt.to_string()))
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx) {
        return v
            .map(|dt| serde_json::Value::String(dt.to_rfc3339()))
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
        return v
            .map(|d| serde_json::Value::String(d.to_string()))
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
        return v.unwrap_or(serde_json::Value::Null);
    }
    if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned());
    }
    serde_json::Value::Null
}
