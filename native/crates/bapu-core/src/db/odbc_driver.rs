//! Universal Enterprise ODBC Driver via `odbc-api`.
//! Powers IBM Db2, SAP HANA, Teradata Vantage, DuckDB ODBC, Firebird ODBC, and Generic ODBC
//! (Sybase ASE, Informix, Vertica, Exasol, Netezza, Microsoft Access, etc.).

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, DbKind, MutationItemResult,
    MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use odbc_api::buffers::TextRowSet;
use odbc_api::{ConnectionOptions, Cursor, Environment, ResultSetMetadata};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

static ODBC_ENV: OnceLock<Result<Environment, String>> = OnceLock::new();

fn odbc_env() -> Result<&'static Environment> {
    match ODBC_ENV.get_or_init(|| Environment::new().map_err(|e| e.to_string())) {
        Ok(env) => Ok(env),
        Err(msg) => anyhow::bail!("ODBC Environment initialization failed: {msg}"),
    }
}

#[derive(Default)]
pub struct OdbcDriver;

pub fn build_odbc_connection_string(cfg: &DatabaseConnection) -> String {
    if let Some(cs) = cfg
        .connection_string
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        return cs.trim().to_string();
    }

    let host = cfg.host.as_deref().unwrap_or("localhost");
    let port = cfg.port.as_deref().unwrap_or_else(|| cfg.kind.default_port());
    let db = &cfg.database;
    let uid = cfg.username.as_deref().unwrap_or("");
    let pwd = cfg.password.as_deref().unwrap_or("");

    match cfg.kind {
        DbKind::Db2 => format!(
            "Driver={{IBM DB2 ODBC DRIVER}};Hostname={host};Port={port};Database={db};Protocol=TCPIP;Uid={uid};Pwd={pwd};"
        ),
        DbKind::SapHana => {
            format!("Driver={{HDBODBC}};ServerNode={host}:{port};DATABASENAME={db};UID={uid};PWD={pwd};")
        }
        DbKind::Teradata => {
            format!("Driver={{Teradata Database ODBC Driver}};DBCName={host};Database={db};UID={uid};PWD={pwd};")
        }
        DbKind::Duckdb => {
            format!("Driver={{DuckDB Driver}};Database={db};")
        }
        DbKind::Firebird => {
            format!("Driver={{Firebird/InterBase(r) driver}};Dbname={host}/{port}:{db};UID={uid};PWD={pwd};")
        }
        _ => {
            if db.contains('=') {
                db.clone()
            } else {
                format!("DSN={db};UID={uid};PWD={pwd};")
            }
        }
    }
}

fn parse_text_scalar(bytes: &[u8]) -> serde_json::Value {
    let s = String::from_utf8_lossy(bytes).into_owned();
    if let Ok(i) = s.parse::<i64>() {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(n);
        }
    }
    serde_json::Value::String(s)
}

#[async_trait]
impl DbDriver for OdbcDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let cfg = config.clone();
        tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let env = odbc_env()?;
            let cs = build_odbc_connection_string(&cfg);
            let conn = env
                .connect_with_connection_string(&cs, ConnectionOptions::default())
                .context("ODBC connect failed")?;
            let dbms = conn.database_management_system_name().unwrap_or_else(|_| {
                cfg.kind.display_name().to_string()
            });
            Ok(format!(
                "Connected to {dbms} via ODBC ({}ms)",
                start.elapsed().as_millis()
            ))
        })
        .await
        .context("joining ODBC task")?
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let cfg = config.clone();
        let sql_owned = sql.to_string();
        tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let env = odbc_env()?;
            let cs = build_odbc_connection_string(&cfg);
            let conn = env
                .connect_with_connection_string(&cs, ConnectionOptions::default())
                .context("ODBC connect failed")?;
            let command = sql_owned
                .split_whitespace()
                .next()
                .map(|s| s.to_ascii_uppercase());

            let out = match conn.execute(&sql_owned, (), None).context("ODBC execute failed")? {
                Some(mut cursor) => {
                    let columns: Vec<String> = cursor
                        .column_names()?
                        .collect::<std::result::Result<_, _>>()?;
                    let mut buffers = TextRowSet::for_cursor(250, &mut cursor, Some(8192))?;
                    let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;
                    let mut rows = Vec::new();

                    while let Some(batch) = row_set_cursor.fetch()? {
                        for r_idx in 0..batch.num_rows() {
                            let mut map = HashMap::new();
                            for (c_idx, col_name) in columns.iter().enumerate() {
                                let jv = match batch.at(c_idx, r_idx) {
                                    None => serde_json::Value::Null,
                                    Some(bytes) => parse_text_scalar(bytes),
                                };
                                map.insert(col_name.clone(), jv);
                            }
                            rows.push(map);
                        }
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
                    let mut status = HashMap::new();
                    status.insert("status".into(), serde_json::json!("OK"));
                    status.insert("message".into(), serde_json::json!("ODBC statement executed."));
                    Ok(QueryResult {
                        columns: vec!["status".into(), "message".into()],
                        rows: vec![status],
                        row_count: 0,
                        execution_time_ms: start.elapsed().as_millis() as u64,
                        error: None,
                        notices: vec![],
                        query_plan: None,
                        command,
                    })
                }
            };
            out
        })
        .await
        .context("joining ODBC query task")?
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let catalog_sql = match config.kind {
            DbKind::Db2 => {
                "SELECT TABNAME AS table_name, COLNAME AS column_name, TYPENAME AS data_type, NULLS AS is_nullable \
                 FROM SYSCAT.COLUMNS WHERE TABSCHEMA NOT LIKE 'SYS%' ORDER BY TABNAME, COLNO FETCH FIRST 500 ROWS ONLY"
            }
            DbKind::SapHana => {
                "SELECT TABLE_NAME AS table_name, COLUMN_NAME AS column_name, DATA_TYPE_NAME AS data_type, IS_NULLABLE AS is_nullable \
                 FROM SYS.TABLE_COLUMNS WHERE SCHEMA_NAME = CURRENT_SCHEMA ORDER BY TABLE_NAME, POSITION LIMIT 500"
            }
            DbKind::Teradata => {
                "SELECT TOP 500 TableName AS table_name, ColumnName AS column_name, ColumnType AS data_type, Nullable AS is_nullable \
                 FROM DBC.ColumnsV WHERE DatabaseName = DATABASE ORDER BY TableName, ColumnId"
            }
            DbKind::Firebird => {
                "SELECT TRIM(rf.RDB$RELATION_NAME) AS table_name, TRIM(rf.RDB$FIELD_NAME) AS column_name, \
                 'VARCHAR' AS data_type, 'YES' AS is_nullable \
                 FROM RDB$RELATION_FIELDS rf JOIN RDB$RELATIONS r ON rf.RDB$RELATION_NAME = r.RDB$RELATION_NAME \
                 WHERE COALESCE(r.RDB$SYSTEM_FLAG, 0) = 0 ORDER BY rf.RDB$RELATION_NAME, rf.RDB$FIELD_POSITION"
            }
            _ => {
                "SELECT table_name, column_name, data_type, is_nullable \
                 FROM information_schema.columns ORDER BY table_name, ordinal_position"
            }
        };

        let qr = self.query(config, catalog_sql).await?;
        let mut grouped: std::collections::BTreeMap<String, Vec<ColumnDefinition>> =
            std::collections::BTreeMap::new();
        for row in &qr.rows {
            let get_ci = |target: &str| -> String {
                row.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(target))
                    .and_then(|(_, v)| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            let tname = get_ci("table_name");
            let cname = get_ci("column_name");
            let dtype = get_ci("data_type");
            let null_raw = get_ci("is_nullable");
            if !tname.is_empty() && !cname.is_empty() {
                grouped.entry(tname).or_default().push(ColumnDefinition {
                    name: cname,
                    col_type: if dtype.is_empty() {
                        "VARCHAR".into()
                    } else {
                        dtype.to_ascii_uppercase()
                    },
                    is_nullable: null_raw.eq_ignore_ascii_case("yes")
                        || null_raw.eq_ignore_ascii_case("y")
                        || null_raw.eq_ignore_ascii_case("true"),
                    ..Default::default()
                });
            }
        }

        Ok(grouped
            .into_iter()
            .map(|(name, columns)| TableSchema {
                name,
                object_type: "TABLE".into(),
                row_count: 0,
                columns,
                foreign_keys: vec![],
                indexes: vec![],
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
        let cfg = config.clone();
        let tbl = table.to_string();
        let muts = mutations.to_vec();
        tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            if muts.is_empty() {
                return Ok(BatchMutationResult {
                    atomic: true,
                    results: vec![],
                    execution_time_ms: 0,
                    message: None,
                });
            }
            let env = odbc_env()?;
            let cs = build_odbc_connection_string(&cfg);
            let conn = env
                .connect_with_connection_string(&cs, ConnectionOptions::default())
                .context("ODBC connect failed")?;
            let _ = conn.set_autocommit(false);

            let mut results = Vec::with_capacity(muts.len());
            for m in &muts {
                let sql = build_odbc_literal_mutation(&tbl, m)?;
                if let Err(e) = conn.execute(&sql, (), None) {
                    let _ = conn.rollback();
                    anyhow::bail!("ODBC transaction rolled back: {e}");
                }
                results.push(MutationItemResult {
                    op: m.op,
                    rows_affected: 1,
                });
            }
            conn.commit().context("ODBC commit failed")?;
            Ok(BatchMutationResult {
                atomic: true,
                results,
                execution_time_ms: start.elapsed().as_millis() as u64,
                message: None,
            })
        })
        .await
        .context("joining ODBC mutate task")?
    }
}

fn odbc_sql_lit(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "NULL".into(),
        serde_json::Value::Bool(b) => if *b { "1".into() } else { "0".into() },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn quote_std(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

fn build_odbc_literal_mutation(table: &str, m: &RowMutation) -> Result<String> {
    let qt = quote_std(table);
    match m.op {
        MutationOp::Insert => {
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let cl = cols.iter().map(|(k, _)| quote_std(k)).collect::<Vec<_>>().join(", ");
            let vl = cols.iter().map(|(_, v)| odbc_sql_lit(v)).collect::<Vec<_>>().join(", ");
            Ok(format!("INSERT INTO {qt} ({cl}) VALUES ({vl})"))
        }
        MutationOp::Update => {
            let mut sets: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            sets.sort_by_key(|(k, _)| *k);
            let mut whs: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            whs.sort_by_key(|(k, _)| *k);
            let sc = sets
                .iter()
                .map(|(k, v)| format!("{} = {}", quote_std(k), odbc_sql_lit(v)))
                .collect::<Vec<_>>()
                .join(", ");
            let wc = whs
                .iter()
                .map(|(k, v)| format!("{} = {}", quote_std(k), odbc_sql_lit(v)))
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok(format!("UPDATE {qt} SET {sc} WHERE {wc}"))
        }
        MutationOp::Delete => {
            let mut whs: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            whs.sort_by_key(|(k, _)| *k);
            let wc = whs
                .iter()
                .map(|(k, v)| format!("{} = {}", quote_std(k), odbc_sql_lit(v)))
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok(format!("DELETE FROM {qt} WHERE {wc}"))
        }
    }
}
