//! Pure-Rust Oracle Database (19c / 21c / 23ai) driver via `oracle-rs` (Oracle Net TNS protocol).
//! Zero OCI / ODPI-C / Oracle Instant Client C libraries required!
//! Supports EZConnect strings, TLS, SSH bastion tunneling, `USER_TABLES`/`USER_VIEWS`/
//! `USER_CONSTRAINTS`/`USER_INDEXES` introspection, and atomic `commit()` / `rollback()` batches.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, ForeignKeyDef, IndexDef,
    MutationItemResult, MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use oracle_rs::{Config, Connection, Value as OraValue};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

#[derive(Default)]
pub struct OracleDriver;

impl OracleDriver {
    async fn connect(
        cfg: &DatabaseConnection,
    ) -> Result<(Connection, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        let user = cfg.username.as_deref().unwrap_or("SYSTEM");
        let pass = cfg.password.as_deref().unwrap_or("");

        if let Some(cs) = cfg
            .connection_string
            .as_deref()
            .filter(|s| !s.trim().is_empty() && !cfg.ssh_enabled)
        {
            let clean_cs = cs
                .strip_prefix("oracle://")
                .unwrap_or(cs)
                .trim();
            let conn = Connection::connect(clean_cs, user, pass)
                .await
                .context("Oracle TNS connect failed")?;
            return Ok((conn, None));
        }

        let (host, port, tunnel) = super::ssh_tunnel::effective_target(cfg, 1521).await?;
        let service = if cfg.database.trim().is_empty() {
            "FREEPDB1"
        } else {
            cfg.database.trim()
        };

        let mut ora_cfg = Config::new(&host, port, service, user, pass);
        if cfg.ssl {
            ora_cfg = ora_cfg.with_tls().context("configuring Oracle TLS")?;
        }
        let conn = Connection::connect_with_config(ora_cfg)
            .await
            .with_context(|| format!("connecting to Oracle {host}:{port}/{service}"))?;
        Ok((conn, tunnel))
    }
}

fn quote_ora_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn is_ora_read(sql: &str) -> bool {
    let up = sql.trim().to_ascii_uppercase();
    up.starts_with("SELECT") || up.starts_with("WITH")
}

fn ora_value_to_json(v: &OraValue) -> serde_json::Value {
    match v {
        OraValue::Null => serde_json::Value::Null,
        OraValue::String(s) => serde_json::Value::String(s.clone()),
        OraValue::Integer(i) => serde_json::Value::Number((*i).into()),
        OraValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        OraValue::Boolean(b) => serde_json::Value::Bool(*b),
        OraValue::Json(j) => j.clone(),
        OraValue::Bytes(b) => {
            let hex_s: String = b.iter().map(|byte| format!("{byte:02x}")).collect();
            serde_json::Value::String(format!("0x{hex_s}"))
        }
        other => {
            if let Some(s) = other.as_str() {
                serde_json::Value::String(s.to_string())
            } else if let Some(i) = other.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = other.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::String(format!("{other:?}"))
            }
        }
    }
}

fn json_to_ora_value(v: &serde_json::Value) -> OraValue {
    match v {
        serde_json::Value::Null => OraValue::Null,
        serde_json::Value::Bool(b) => OraValue::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                OraValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                OraValue::Float(f)
            } else {
                OraValue::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => OraValue::String(s.clone()),
        other => OraValue::String(other.to_string()),
    }
}

#[async_trait]
impl DbDriver for OracleDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (conn, tunnel) = Self::connect(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let banner = match conn
            .query("SELECT banner FROM v$version WHERE ROWNUM = 1", &[])
            .await
        {
            Ok(qr) => qr
                .first()
                .and_then(|r| r.get_string(0))
                .unwrap_or("Oracle Database")
                .to_string(),
            Err(_) => {
                conn.query("SELECT 'Oracle Database' FROM DUAL", &[]).await?;
                "Oracle Database".to_string()
            }
        };
        let _ = conn.close().await;
        Ok(format!(
            "Connected to {banner} ({}ms){via}",
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let (conn, _tunnel) = Self::connect(config).await?;
        let start = Instant::now();
        let clean_sql = sql.trim().trim_end_matches(';');
        let command = clean_sql
            .split_whitespace()
            .next()
            .map(|s| s.to_ascii_uppercase());

        if !is_ora_read(clean_sql) {
            let res = conn
                .execute(clean_sql, &[])
                .await
                .context("Oracle execution failed")?;
            let _ = conn.commit().await;
            let _ = conn.close().await;
            let mut row = HashMap::new();
            row.insert("status".into(), serde_json::json!("OK"));
            row.insert(
                "message".into(),
                serde_json::json!(format!("{} row(s) affected.", res.rows_affected)),
            );
            return Ok(QueryResult {
                columns: vec!["status".into(), "message".into()],
                rows: vec![row],
                row_count: res.rows_affected,
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: None,
                notices: vec![],
                query_plan: None,
                command,
            });
        }

        let ora_res = conn
            .query(clean_sql, &[])
            .await
            .context("Oracle SELECT failed")?;
        let columns: Vec<String> = ora_res.columns.iter().map(|c| c.name.clone()).collect();
        let mut rows = Vec::with_capacity(ora_res.rows.len());
        for r in &ora_res.rows {
            let mut map = HashMap::new();
            for (idx, col_name) in columns.iter().enumerate() {
                let jv = r
                    .get(idx)
                    .map(ora_value_to_json)
                    .unwrap_or(serde_json::Value::Null);
                map.insert(col_name.clone(), jv);
            }
            rows.push(map);
        }
        let _ = conn.close().await;

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

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let (conn, _tunnel) = Self::connect(config).await?;
        let obj_res = conn
            .query(
                "SELECT object_name, object_type FROM user_objects \
                 WHERE object_type IN ('TABLE', 'VIEW') AND object_name NOT LIKE 'BIN$%' \
                 ORDER BY object_type, object_name",
                &[],
            )
            .await?;

        let mut tables = Vec::new();
        for row in &obj_res.rows {
            let table_name = row.get_string(0).unwrap_or("").to_string();
            let object_type = row.get_string(1).unwrap_or("TABLE").to_string();
            if table_name.is_empty() {
                continue;
            }

            let pk_res = conn
                .query(
                    "SELECT cols.column_name \
                     FROM user_constraints cons \
                     JOIN user_cons_columns cols ON cons.constraint_name = cols.constraint_name \
                     WHERE cons.table_name = :1 AND cons.constraint_type = 'P'",
                    &[OraValue::String(table_name.clone())],
                )
                .await
                .unwrap_or_else(|_| oracle_rs::QueryResult::empty());
            let pk_set: HashSet<String> = pk_res
                .rows
                .iter()
                .filter_map(|r| r.get_string(0).map(String::from))
                .collect();

            let col_res = conn
                .query(
                    "SELECT column_name, data_type, nullable FROM user_tab_columns \
                     WHERE table_name = :1 ORDER BY column_id",
                    &[OraValue::String(table_name.clone())],
                )
                .await
                .unwrap_or_else(|_| oracle_rs::QueryResult::empty());
            let columns: Vec<ColumnDefinition> = col_res
                .rows
                .iter()
                .map(|r| {
                    let cname = r.get_string(0).unwrap_or("").to_string();
                    let dtype = r.get_string(1).unwrap_or("VARCHAR2").to_string();
                    let nullable = r.get_string(2).unwrap_or("Y") == "Y";
                    ColumnDefinition {
                        is_primary_key: pk_set.contains(&cname),
                        is_nullable: nullable,
                        name: cname,
                        col_type: dtype,
                        ..Default::default()
                    }
                })
                .collect();

            let fk_res = conn
                .query(
                    "SELECT a.constraint_name, a.column_name, c_pk.table_name, b.column_name, c.delete_rule \
                     FROM user_cons_columns a \
                     JOIN user_constraints c ON a.constraint_name = c.constraint_name \
                     JOIN user_constraints c_pk ON c.r_constraint_name = c_pk.constraint_name \
                     JOIN user_cons_columns b ON c_pk.constraint_name = b.constraint_name AND a.position = b.position \
                     WHERE c.constraint_type = 'R' AND a.table_name = :1",
                    &[OraValue::String(table_name.clone())],
                )
                .await
                .unwrap_or_else(|_| oracle_rs::QueryResult::empty());
            let foreign_keys: Vec<ForeignKeyDef> = fk_res
                .rows
                .iter()
                .map(|r| ForeignKeyDef {
                    constraint_name: r.get_string(0).map(String::from),
                    column: r.get_string(1).unwrap_or("").to_string(),
                    referenced_table: r.get_string(2).unwrap_or("").to_string(),
                    referenced_column: r.get_string(3).unwrap_or("").to_string(),
                    on_delete: r.get_string(4).map(String::from),
                    on_update: None,
                })
                .collect();

            let idx_res = conn
                .query(
                    "SELECT i.index_name, ic.column_name, i.uniqueness \
                     FROM user_indexes i \
                     JOIN user_ind_columns ic ON i.index_name = ic.index_name \
                     WHERE i.table_name = :1 ORDER BY i.index_name, ic.column_position",
                    &[OraValue::String(table_name.clone())],
                )
                .await
                .unwrap_or_else(|_| oracle_rs::QueryResult::empty());
            let mut idx_map: BTreeMap<String, (bool, Vec<String>)> = BTreeMap::new();
            for r in &idx_res.rows {
                let iname = r.get_string(0).unwrap_or("").to_string();
                let cname = r.get_string(1).unwrap_or("").to_string();
                let uniq = r.get_string(2).unwrap_or("") == "UNIQUE";
                if !iname.is_empty() {
                    idx_map.entry(iname).or_insert((uniq, Vec::new())).1.push(cname);
                }
            }
            let indexes: Vec<IndexDef> = idx_map
                .into_iter()
                .map(|(name, (is_unique, cols))| IndexDef {
                    name,
                    columns: cols,
                    is_unique,
                    is_primary: false,
                })
                .collect();

            tables.push(TableSchema {
                name: table_name,
                object_type,
                row_count: 0,
                columns,
                foreign_keys,
                indexes,
                ddl: None,
            });
        }

        let _ = conn.close().await;
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

        let (conn, _tunnel) = Self::connect(config).await?;
        let mut results = Vec::with_capacity(mutations.len());

        for m in mutations {
            let (sql, binds) = build_ora_mutation(table, m)?;
            match conn.execute(&sql, &binds).await {
                Ok(done) => results.push(MutationItemResult {
                    op: m.op,
                    rows_affected: done.rows_affected,
                }),
                Err(e) => {
                    let _ = conn.rollback().await;
                    let _ = conn.close().await;
                    return Err(anyhow::anyhow!("Oracle transaction rolled back: {e}"));
                }
            }
        }

        conn.commit().await.context("committing Oracle transaction")?;
        let _ = conn.close().await;
        Ok(BatchMutationResult {
            atomic: true,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}

fn build_ora_mutation(table: &str, m: &RowMutation) -> Result<(String, Vec<OraValue>)> {
    let qt = quote_ora_ident(table);
    match m.op {
        MutationOp::Insert => {
            if m.values.is_empty() {
                anyhow::bail!("INSERT requires at least one column");
            }
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let col_list = cols
                .iter()
                .map(|(k, _)| quote_ora_ident(k))
                .collect::<Vec<_>>()
                .join(", ");
            let phs = (1..=cols.len())
                .map(|i| format!(":{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            let binds = cols.into_iter().map(|(_, v)| json_to_ora_value(v)).collect();
            Ok((format!("INSERT INTO {qt} ({col_list}) VALUES ({phs})"), binds))
        }
        MutationOp::Update => {
            if m.values.is_empty() || m.where_clause.is_empty() {
                anyhow::bail!("UPDATE requires SET and WHERE");
            }
            let mut sets: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            sets.sort_by_key(|(k, _)| *k);
            let mut wheres: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            wheres.sort_by_key(|(k, _)| *k);
            let set_sql = sets
                .iter()
                .enumerate()
                .map(|(i, (k, _))| format!("{} = :{}", quote_ora_ident(k), i + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let mut idx = sets.len();
            let mut binds: Vec<OraValue> = sets.into_iter().map(|(_, v)| json_to_ora_value(v)).collect();
            let where_sql = wheres
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_ora_ident(k))
                    } else {
                        idx += 1;
                        binds.push(json_to_ora_value(v));
                        format!("{} = :{idx}", quote_ora_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("UPDATE {qt} SET {set_sql} WHERE {where_sql}"), binds))
        }
        MutationOp::Delete => {
            if m.where_clause.is_empty() {
                anyhow::bail!("DELETE requires WHERE");
            }
            let mut wheres: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            wheres.sort_by_key(|(k, _)| *k);
            let mut idx = 0usize;
            let mut binds = Vec::new();
            let where_sql = wheres
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_ora_ident(k))
                    } else {
                        idx += 1;
                        binds.push(json_to_ora_value(v));
                        format!("{} = :{idx}", quote_ora_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("DELETE FROM {qt} WHERE {where_sql}"), binds))
        }
    }
}
