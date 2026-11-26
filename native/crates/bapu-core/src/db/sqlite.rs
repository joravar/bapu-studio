//! Native SQLite driver via `sqlx`.
//! Replaces the Electron app's `sql.js` WASM engine with real native SQLite file access
//! (including persistent file paths across restarts and `:memory:` for unit tests).

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, ForeignKeyDef, IndexDef,
    MutationItemResult, MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Column, Row};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
pub struct SqliteDriver {
    pools: Mutex<HashMap<String, sqlx::SqlitePool>>,
}

impl SqliteDriver {
    fn connect_options(config: &DatabaseConnection) -> Result<SqliteConnectOptions> {
        let target = config
            .connection_string
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(config.database.as_str())
            .trim();

        if target.is_empty() {
            anyhow::bail!("SQLite database path is empty");
        }

        let opts = if target.starts_with("sqlite:") {
            SqliteConnectOptions::from_str(target).context("invalid SQLite connection string")?
        } else if target == ":memory:" {
            SqliteConnectOptions::from_str("sqlite::memory:")?
        } else {
            SqliteConnectOptions::new().filename(target).create_if_missing(false)
        };
        Ok(opts.foreign_keys(true))
    }

    async fn pool_for(&self, config: &DatabaseConnection) -> Result<sqlx::SqlitePool> {
        if let Some(pool) = self
            .pools
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&config.id)
        {
            return Ok(pool.clone());
        }
        let opts = Self::connect_options(config)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .context("opening SQLite database")?;
        self.pools
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(config.id.clone(), pool.clone());
        Ok(pool)
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn is_read_query(sql: &str) -> bool {
    let trimmed = sql.trim();
    let upper = trimmed.to_ascii_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("PRAGMA")
        || upper.starts_with("EXPLAIN")
        || upper.starts_with("WITH")
}

#[async_trait]
impl DbDriver for SqliteDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let opts = Self::connect_options(config)?;
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        pool.close().await;
        Ok(format!("Connected to SQLite successfully ({}ms)", start.elapsed().as_millis()))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let pool = self.pool_for(config).await?;
        let start = Instant::now();
        let command = sql
            .split_whitespace()
            .next()
            .map(|s| s.to_ascii_uppercase());

        if !is_read_query(sql) {
            let done = sqlx::query(sql).execute(&pool).await.context("SQLite execution failed")?;
            let execution_time_ms = start.elapsed().as_millis() as u64;
            let affected = done.rows_affected();
            let mut row = HashMap::new();
            row.insert("status".into(), serde_json::Value::String("OK".into()));
            row.insert(
                "message".into(),
                serde_json::Value::String(format!(
                    "Command executed successfully. {affected} row(s) affected."
                )),
            );
            return Ok(QueryResult {
                columns: vec!["status".into(), "message".into()],
                rows: vec![row],
                row_count: affected,
                execution_time_ms,
                error: None,
                notices: vec![],
                query_plan: None,
                command,
            });
        }

        let rows = sqlx::query(sql).fetch_all(&pool).await.context("SQLite query failed")?;
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
                    map.insert(col.name().to_string(), sqlite_value_to_json(row, col));
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
        let pool = self.pool_for(config).await?;
        let table_rows = sqlx::query(
            "SELECT name, type, sql FROM sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY type, name",
        )
        .fetch_all(&pool)
        .await?;

        let mut tables = Vec::new();
        for row in table_rows {
            let table_name: String = row.try_get("name")?;
            let obj_type_raw: String = row.try_get("type").unwrap_or_else(|_| "table".into());
            let ddl: Option<String> = row.try_get("sql").ok().flatten();
            let object_type = if obj_type_raw.eq_ignore_ascii_case("view") {
                "VIEW".to_string()
            } else {
                "TABLE".to_string()
            };
            let quoted = quote_ident(&table_name);
            let ddl_upper = ddl.as_deref().unwrap_or("").to_ascii_uppercase();

            // Collect indexes & unique single-column indexes first
            let mut indexes: Vec<IndexDef> = Vec::new();
            let mut unique_cols = std::collections::HashSet::new();
            if object_type == "TABLE" {
                let idx_rows = sqlx::query(&format!("PRAGMA index_list({quoted})"))
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default();
                for ir in idx_rows {
                    let idx_name: String = ir.try_get("name").unwrap_or_default();
                    let unique: i64 = ir.try_get("unique").unwrap_or(0);
                    let origin: String = ir.try_get("origin").unwrap_or_default();
                    if idx_name.is_empty() {
                        continue;
                    }
                    let q_idx = quote_ident(&idx_name);
                    let info_rows = sqlx::query(&format!("PRAGMA index_info({q_idx})"))
                        .fetch_all(&pool)
                        .await
                        .unwrap_or_default();
                    let cols: Vec<String> = info_rows
                        .iter()
                        .filter_map(|r| r.try_get::<String, _>("name").ok())
                        .collect();
                    if unique > 0 && cols.len() == 1 {
                        unique_cols.insert(cols[0].clone());
                    }
                    indexes.push(IndexDef {
                        name: idx_name,
                        columns: cols,
                        is_unique: unique > 0,
                        is_primary: origin == "pk",
                    });
                }
            }

            let col_rows = sqlx::query(&format!("PRAGMA table_info({quoted})"))
                .fetch_all(&pool)
                .await
                .unwrap_or_default();

            let columns = col_rows
                .iter()
                .map(|c| {
                    let name: String = c.try_get("name").unwrap_or_default();
                    let col_type: String = c.try_get("type").unwrap_or_else(|_| "TEXT".into());
                    let notnull: i64 = c.try_get("notnull").unwrap_or(0);
                    let pk: i64 = c.try_get("pk").unwrap_or(0);
                    let default_value: Option<String> = c.try_get("dflt_value").ok().flatten();
                    let upper_type = if col_type.is_empty() {
                        "TEXT".into()
                    } else {
                        col_type.to_uppercase()
                    };
                    let auto_inc = pk > 0
                        && (upper_type == "INTEGER" || ddl_upper.contains("AUTOINCREMENT"));
                    ColumnDefinition {
                        is_unique: unique_cols.contains(&name),
                        name,
                        col_type: upper_type,
                        is_primary_key: pk > 0,
                        is_nullable: notnull == 0 && pk == 0,
                        default_value,
                        auto_increment: auto_inc,
                        comment: None,
                    }
                })
                .collect();

            let mut foreign_keys: Vec<ForeignKeyDef> = Vec::new();
            if object_type == "TABLE" {
                let fk_rows = sqlx::query(&format!("PRAGMA foreign_key_list({quoted})"))
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default();
                for fk in fk_rows {
                    let from_col: String = fk.try_get("from").unwrap_or_default();
                    let ref_table: String = fk.try_get("table").unwrap_or_default();
                    let to_col: String = fk.try_get("to").unwrap_or_default();
                    let on_delete: Option<String> = fk.try_get("on_delete").ok();
                    let on_update: Option<String> = fk.try_get("on_update").ok();
                    if !from_col.is_empty() && !ref_table.is_empty() {
                        foreign_keys.push(ForeignKeyDef {
                            constraint_name: None,
                            column: from_col,
                            referenced_table: ref_table,
                            referenced_column: if to_col.is_empty() { "id".into() } else { to_col },
                            on_delete,
                            on_update,
                        });
                    }
                }
            }

            let row_count: i64 = if object_type == "TABLE" {
                sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {quoted}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(0)
            } else {
                0
            };

            tables.push(TableSchema {
                name: table_name,
                object_type,
                row_count: row_count.max(0) as u64,
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

        let pool = self.pool_for(config).await?;
        let mut tx = pool.begin().await.context("starting SQLite transaction")?;
        let mut results = Vec::with_capacity(mutations.len());

        for m in mutations {
            let (sql, params) = build_sqlite_mutation_sql(table, m)?;
            let mut q = sqlx::query(&sql);
            for val in params {
                q = bind_sqlite_value(q, val);
            }
            let done = q.execute(&mut *tx).await.context("executing SQLite mutation")?;
            results.push(MutationItemResult {
                op: m.op,
                rows_affected: done.rows_affected(),
            });
        }

        tx.commit().await.context("committing SQLite transaction")?;
        Ok(BatchMutationResult {
            atomic: true,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}

fn build_sqlite_mutation_sql<'a>(
    table: &str,
    m: &'a RowMutation,
) -> Result<(String, Vec<&'a serde_json::Value>)> {
    let qt = quote_ident(table);
    match m.op {
        MutationOp::Insert => {
            if m.values.is_empty() {
                anyhow::bail!("INSERT mutation requires at least one column value");
            }
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let col_list = cols.iter().map(|(k, _)| quote_ident(k)).collect::<Vec<_>>().join(", ");
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
                .map(|(k, _)| format!("{} = ?", quote_ident(k)))
                .collect::<Vec<_>>()
                .join(", ");
            let mut params: Vec<&serde_json::Value> = set_cols.into_iter().map(|(_, v)| v).collect();
            let where_clause = where_cols
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_ident(k))
                    } else {
                        params.push(v);
                        format!("{} = ?", quote_ident(k))
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
                        format!("{} IS NULL", quote_ident(k))
                    } else {
                        params.push(v);
                        format!("{} = ?", quote_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("DELETE FROM {qt} WHERE {where_clause}"), params))
        }
    }
}

fn bind_sqlite_value<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    val: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
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

fn sqlite_value_to_json(
    row: &sqlx::sqlite::SqliteRow,
    col: &sqlx::sqlite::SqliteColumn,
) -> serde_json::Value {
    let idx = col.ordinal();
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(|n| serde_json::Value::Number(n.into())).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null);
    }
    if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        return serde_json::Value::String(format!("0x{hex}"));
    }
    serde_json::Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DbKind;

    fn memory_config() -> DatabaseConnection {
        DatabaseConnection {
            id: format!("sqlite-test-{}", uuid::Uuid::new_v4()),
            name: "Memory".into(),
            kind: DbKind::Sqlite,
            connection_string: None,
            host: None,
            port: None,
            database: ":memory:".into(),
            username: None,
            password: None,
            ssl: false,
            ssl_ca_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            ssl_reject_unauthorized: None,
            ssh_enabled: false,
            ssh_host: None,
            ssh_port: None,
            ssh_username: None,
            ssh_password: None,
            ssh_private_key: None,
            ssh_passphrase: None,
            is_connected: false,
            is_demo_db: false,
            tables: vec![],
        }
    }

    #[tokio::test]
    async fn sqlite_full_lifecycle_query_and_schema() {
        let driver = SqliteDriver::default();
        let cfg = memory_config();

        driver
            .query(&cfg, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, score REAL)")
            .await
            .unwrap();
        let ins = driver
            .query(&cfg, "INSERT INTO users (name, score) VALUES ('Ada', 99.5), ('Grace', 100.0)")
            .await
            .unwrap();
        assert_eq!(ins.row_count, 2);

        let sel = driver
            .query(&cfg, "SELECT id, name, score FROM users ORDER BY id")
            .await
            .unwrap();
        assert_eq!(sel.row_count, 2);
        assert_eq!(sel.columns, vec!["id", "name", "score"]);
        assert_eq!(sel.rows[0]["name"], serde_json::json!("Ada"));

        // Regression check: empty SELECT returns 0 rows, not a fabricated OK message
        let empty = driver
            .query(&cfg, "SELECT * FROM users WHERE name = 'Nobody'")
            .await
            .unwrap();
        assert_eq!(empty.row_count, 0);
        assert!(empty.rows.is_empty());

        let schema = driver.get_schema(&cfg).await.unwrap();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "users");
        assert_eq!(schema[0].row_count, 2);
        assert!(schema[0].columns[0].is_primary_key);
    }

    #[tokio::test]
    async fn sqlite_mutate_batch_commits_and_rolls_back_atomically() {
        let driver = SqliteDriver::default();
        let cfg = memory_config();

        driver
            .query(&cfg, "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE)")
            .await
            .unwrap();
        driver
            .query(&cfg, "INSERT INTO items (id, name) VALUES (1, 'alpha'), (2, 'beta')")
            .await
            .unwrap();

        // Atomic commit: update + insert + delete in one transaction
        let mut upd_vals = HashMap::new();
        upd_vals.insert("name".into(), serde_json::json!("alpha_v2"));
        let mut upd_where = HashMap::new();
        upd_where.insert("id".into(), serde_json::json!(1));

        let mut ins_vals = HashMap::new();
        ins_vals.insert("id".into(), serde_json::json!(3));
        ins_vals.insert("name".into(), serde_json::json!("gamma"));

        let mut del_where = HashMap::new();
        del_where.insert("id".into(), serde_json::json!(2));

        let res = driver
            .mutate_batch(
                &cfg,
                "items",
                &[
                    RowMutation { op: MutationOp::Update, values: upd_vals, where_clause: upd_where },
                    RowMutation { op: MutationOp::Insert, values: ins_vals, where_clause: HashMap::new() },
                    RowMutation { op: MutationOp::Delete, values: HashMap::new(), where_clause: del_where },
                ],
            )
            .await
            .unwrap();
        assert!(res.atomic);
        assert_eq!(res.results.len(), 3);

        let after_commit = driver.query(&cfg, "SELECT id, name FROM items ORDER BY id").await.unwrap();
        assert_eq!(after_commit.row_count, 2);
        assert_eq!(after_commit.rows[0]["name"], serde_json::json!("alpha_v2"));
        assert_eq!(after_commit.rows[1]["name"], serde_json::json!("gamma"));

        // Atomic rollback: first mutation succeeds, second violates UNIQUE constraint -> entire batch rolls back!
        let mut ok_ins = HashMap::new();
        ok_ins.insert("id".into(), serde_json::json!(4));
        ok_ins.insert("name".into(), serde_json::json!("delta"));

        let mut bad_ins = HashMap::new();
        bad_ins.insert("id".into(), serde_json::json!(5));
        bad_ins.insert("name".into(), serde_json::json!("gamma")); // duplicate UNIQUE name

        let err = driver
            .mutate_batch(
                &cfg,
                "items",
                &[
                    RowMutation { op: MutationOp::Insert, values: ok_ins, where_clause: HashMap::new() },
                    RowMutation { op: MutationOp::Insert, values: bad_ins, where_clause: HashMap::new() },
                ],
            )
            .await;
        assert!(err.is_err());

        let after_rollback = driver.query(&cfg, "SELECT id, name FROM items ORDER BY id").await.unwrap();
        assert_eq!(after_rollback.row_count, 2);
    }
}
