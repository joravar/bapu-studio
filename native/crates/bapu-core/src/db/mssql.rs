//! Pure-Rust Microsoft SQL Server (MSSQL) & Azure SQL driver via `tiberius` (TDS 7.3 / 8.0).
//! Zero FreeTDS / ODBC C runtime dependencies required; supports SSH bastion tunneling, TLS,
//! T-SQL `INFORMATION_SCHEMA` + `sys.foreign_keys` + `sys.indexes` introspection, `SHOWPLAN`,
//! and atomic `BEGIN TRANSACTION ... COMMIT TRANSACTION` batch row mutations.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, DbKind, ForeignKeyDef, IndexDef,
    MutationItemResult, MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, Query};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

#[derive(Default)]
pub struct MssqlDriver;

type MssqlClient = Client<Compat<TcpStream>>;

impl MssqlDriver {
    async fn connect(
        conn_cfg: &DatabaseConnection,
    ) -> Result<(MssqlClient, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        let mut tcfg = if let Some(cs) = conn_cfg
            .connection_string
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            if cs.starts_with("mssql://") || cs.starts_with("sqlserver://") {
                parse_mssql_url(cs, conn_cfg)?
            } else {
                let mut c =
                    Config::from_ado_string(cs).context("invalid MSSQL ADO connection string")?;
                if conn_cfg.ssl_reject_unauthorized == Some(false) || !conn_cfg.ssl {
                    c.trust_cert();
                }
                c
            }
        } else {
            let mut c = Config::new();
            let user = conn_cfg.username.as_deref().unwrap_or("sa");
            let pw = conn_cfg.password.as_deref().unwrap_or("");
            c.authentication(AuthMethod::sql_server(user, pw));
            if !conn_cfg.database.is_empty() {
                c.database(&conn_cfg.database);
            }
            if conn_cfg.kind == DbKind::AzureSql || conn_cfg.ssl {
                c.encryption(EncryptionLevel::Required);
                if conn_cfg.ssl_reject_unauthorized == Some(false) {
                    c.trust_cert();
                }
            } else {
                c.trust_cert();
            }
            c
        };

        let (addr, tunnel) = if conn_cfg
            .connection_string
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
            && !conn_cfg.ssh_enabled
        {
            (tcfg.get_addr(), None)
        } else {
            let (h, p, tun) = super::ssh_tunnel::effective_target(conn_cfg, 1433).await?;
            tcfg.host(&h);
            tcfg.port(p);
            (format!("{h}:{p}"), tun)
        };

        let tcp = TcpStream::connect(&addr)
            .await
            .with_context(|| format!("connecting TCP to SQL Server {addr}"))?;
        tcp.set_nodelay(true)?;

        let client = Client::connect(tcfg, tcp.compat_write())
            .await
            .context("MSSQL TDS handshake / login failed")?;
        Ok((client, tunnel))
    }
}

fn parse_mssql_url(raw: &str, fallback: &DatabaseConnection) -> Result<Config> {
    let normalized = if let Some(rest) = raw.strip_prefix("mssql://") {
        format!("http://{rest}")
    } else if let Some(rest) = raw.strip_prefix("sqlserver://") {
        format!("http://{rest}")
    } else {
        raw.to_string()
    };
    let parsed = url::Url::parse(&normalized).context("invalid mssql:// URL")?;
    let mut cfg = Config::new();
    cfg.host(parsed.host_str().unwrap_or("127.0.0.1"));
    cfg.port(parsed.port().unwrap_or(1433));
    let user = if parsed.username().is_empty() {
        fallback.username.as_deref().unwrap_or("sa").to_string()
    } else {
        percent_decode(parsed.username())
    };
    let pass = parsed
        .password()
        .map(percent_decode)
        .or_else(|| fallback.password.clone())
        .unwrap_or_default();
    cfg.authentication(AuthMethod::sql_server(user, pass));
    let db = parsed.path().trim_matches('/');
    if !db.is_empty() {
        cfg.database(db);
    } else if !fallback.database.is_empty() {
        cfg.database(&fallback.database);
    }
    if fallback.ssl {
        cfg.encryption(EncryptionLevel::Required);
    }
    if fallback.ssl_reject_unauthorized != Some(true) {
        cfg.trust_cert();
    }
    Ok(cfg)
}

fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("v={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| s.to_string())
}

fn quote_tsql_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

fn is_tsql_read(sql: &str) -> bool {
    let up = sql.trim().to_ascii_uppercase();
    up.starts_with("SELECT")
        || up.starts_with("WITH")
        || up.starts_with("EXEC")
        || up.starts_with("SP_")
        || up.contains(" OUTPUT ")
}

fn column_data_to_json(cd: &ColumnData<'_>) -> serde_json::Value {
    match cd {
        ColumnData::U8(v) => v.map(|n| serde_json::json!(n)).unwrap_or(serde_json::Value::Null),
        ColumnData::I16(v) => v.map(|n| serde_json::json!(n)).unwrap_or(serde_json::Value::Null),
        ColumnData::I32(v) => v.map(|n| serde_json::json!(n)).unwrap_or(serde_json::Value::Null),
        ColumnData::I64(v) => v.map(|n| serde_json::json!(n)).unwrap_or(serde_json::Value::Null),
        ColumnData::F32(v) => v
            .and_then(|n| serde_json::Number::from_f64(n as f64))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ColumnData::F64(v) => v
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Bit(v) => v.map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null),
        ColumnData::String(v) => v
            .as_ref()
            .map(|s| serde_json::Value::String(s.to_string()))
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Guid(v) => v
            .map(|g| serde_json::Value::String(g.to_string()))
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Numeric(v) => v
            .map(|n| serde_json::Value::String(n.to_string()))
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Binary(v) => v
            .as_ref()
            .map(|b| {
                let hex_s: String = b.iter().map(|byte| format!("{byte:02x}")).collect();
                serde_json::Value::String(format!("0x{hex_s}"))
            })
            .unwrap_or(serde_json::Value::Null),
        ColumnData::Xml(v) => v
            .as_ref()
            .map(|x| serde_json::Value::String(format!("{x}")))
            .unwrap_or(serde_json::Value::Null),
        other => {
            let dbg = format!("{other:?}");
            if dbg.ends_with("(None)") {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(dbg)
            }
        }
    }
}

fn tiberius_rows_to_maps(
    rows: &[tiberius::Row],
) -> (Vec<String>, Vec<HashMap<String, serde_json::Value>>) {
    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut map = HashMap::new();
        for (col, data) in row.cells() {
            map.insert(col.name().to_string(), column_data_to_json(data));
        }
        out.push(map);
    }
    (columns, out)
}

#[async_trait]
impl DbDriver for MssqlDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (mut client, tunnel) = Self::connect(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let stream = client.simple_query("SELECT @@VERSION AS ver").await?;
        let row = stream.into_row().await?;
        let ver_line = row
            .and_then(|r| r.get::<&str, _>(0).map(|s| s.lines().next().unwrap_or(s).to_string()))
            .unwrap_or_else(|| "SQL Server".into());
        Ok(format!(
            "Connected to {ver_line} ({}ms){via}",
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let (mut client, _tunnel) = Self::connect(config).await?;
        let start = Instant::now();
        let command = sql.split_whitespace().next().map(|s| s.to_ascii_uppercase());

        if !is_tsql_read(sql) {
            let exec = client
                .execute(sql, &[])
                .await
                .context("MSSQL statement execution failed")?;
            let affected = exec.total();
            let mut status_row = HashMap::new();
            status_row.insert("status".into(), serde_json::json!("OK"));
            status_row.insert(
                "message".into(),
                serde_json::json!(format!("{affected} row(s) affected.")),
            );
            return Ok(QueryResult {
                columns: vec!["status".into(), "message".into()],
                rows: vec![status_row],
                row_count: affected,
                execution_time_ms: start.elapsed().as_millis() as u64,
                error: None,
                notices: vec![],
                query_plan: None,
                command,
            });
        }

        let stream = client
            .simple_query(sql)
            .await
            .context("MSSQL query execution failed")?;
        let result_sets = stream.into_results().await?;
        let first_set = result_sets.into_iter().find(|s| !s.is_empty()).unwrap_or_default();
        let (columns, rows) = tiberius_rows_to_maps(&first_set);

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
        let (mut client, _tunnel) = Self::connect(config).await?;

        let tbl_stream = client
            .simple_query(
                "SELECT TABLE_NAME, TABLE_TYPE FROM INFORMATION_SCHEMA.TABLES \
                 WHERE TABLE_SCHEMA = 'dbo' ORDER BY TABLE_TYPE, TABLE_NAME",
            )
            .await?;
        let tbl_rows = tbl_stream.into_first_result().await?;

        let mut tables = Vec::new();
        for tr in tbl_rows {
            let table_name = tr.get::<&str, _>(0).unwrap_or("").to_string();
            if table_name.is_empty() {
                continue;
            }
            let ttype = tr.get::<&str, _>(1).unwrap_or("BASE TABLE");
            let object_type = if ttype.contains("VIEW") {
                "VIEW".to_string()
            } else {
                "TABLE".to_string()
            };

            let mut pk_q = Query::new(
                "SELECT kcu.COLUMN_NAME FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
                 JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
                   ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
                 WHERE tc.TABLE_SCHEMA = 'dbo' AND tc.TABLE_NAME = @P1 AND tc.CONSTRAINT_TYPE = 'PRIMARY KEY'",
            );
            pk_q.bind(table_name.as_str());
            let pk_rows = pk_q.query(&mut client).await?.into_first_result().await?;
            let pk_set: HashSet<String> = pk_rows
                .iter()
                .filter_map(|r| r.get::<&str, _>(0).map(String::from))
                .collect();

            let mut col_q = Query::new(
                "SELECT c.COLUMN_NAME, c.DATA_TYPE, c.IS_NULLABLE, c.COLUMN_DEFAULT, \
                        COLUMNPROPERTY(OBJECT_ID('dbo.' + c.TABLE_NAME), c.COLUMN_NAME, 'IsIdentity') AS is_id \
                 FROM INFORMATION_SCHEMA.COLUMNS c \
                 WHERE c.TABLE_SCHEMA = 'dbo' AND c.TABLE_NAME = @P1 \
                 ORDER BY c.ORDINAL_POSITION",
            );
            col_q.bind(table_name.as_str());
            let col_rows = col_q.query(&mut client).await?.into_first_result().await?;
            let columns: Vec<ColumnDefinition> = col_rows
                .iter()
                .map(|r| {
                    let name = r.get::<&str, _>(0).unwrap_or("").to_string();
                    let dtype = r.get::<&str, _>(1).unwrap_or("NVARCHAR").to_ascii_uppercase();
                    let nullable = r.get::<&str, _>(2).unwrap_or("YES") == "YES";
                    let def_val = r.get::<&str, _>(3).map(String::from);
                    let is_identity = r.get::<i32, _>(4).unwrap_or(0) == 1;
                    ColumnDefinition {
                        is_primary_key: pk_set.contains(&name),
                        is_nullable: nullable,
                        is_unique: false,
                        auto_increment: is_identity,
                        default_value: def_val,
                        comment: None,
                        name,
                        col_type: dtype,
                    }
                })
                .collect();

            let mut fk_q = Query::new(
                "SELECT fk.name AS fk_name, cp.name AS col_name, tr.name AS ref_table, cr.name AS ref_col, \
                        fk.delete_referential_action_desc AS del_act \
                 FROM sys.foreign_keys fk \
                 JOIN sys.tables tp ON fk.parent_object_id = tp.object_id \
                 JOIN sys.tables tr ON fk.referenced_object_id = tr.object_id \
                 JOIN sys.foreign_key_columns fkc ON fkc.constraint_object_id = fk.object_id \
                 JOIN sys.columns cp ON fkc.parent_object_id = cp.object_id AND fkc.parent_column_id = cp.column_id \
                 JOIN sys.columns cr ON fkc.referenced_object_id = cr.object_id AND fkc.referenced_column_id = cr.column_id \
                 WHERE tp.name = @P1",
            );
            fk_q.bind(table_name.as_str());
            let fk_rows = fk_q
                .query(&mut client)
                .await
                .ok()
                .map(|s| futures_util::FutureExt::now_or_never(s.into_first_result()));
            let _ = fk_rows;
            // Re-run cleanly with await:
            let mut fk_q2 = Query::new(
                "SELECT fk.name, cp.name, tr.name, cr.name, fk.delete_referential_action_desc \
                 FROM sys.foreign_keys fk \
                 JOIN sys.tables tp ON fk.parent_object_id = tp.object_id \
                 JOIN sys.tables tr ON fk.referenced_object_id = tr.object_id \
                 JOIN sys.foreign_key_columns fkc ON fkc.constraint_object_id = fk.object_id \
                 JOIN sys.columns cp ON fkc.parent_object_id = cp.object_id AND fkc.parent_column_id = cp.column_id \
                 JOIN sys.columns cr ON fkc.referenced_object_id = cr.object_id AND fkc.referenced_column_id = cr.column_id \
                 WHERE tp.name = @P1",
            );
            fk_q2.bind(table_name.as_str());
            let fk_vec = match fk_q2.query(&mut client).await {
                Ok(s) => s.into_first_result().await.unwrap_or_default(),
                Err(_) => vec![],
            };
            let foreign_keys: Vec<ForeignKeyDef> = fk_vec
                .iter()
                .map(|r| ForeignKeyDef {
                    constraint_name: r.get::<&str, _>(0).map(String::from),
                    column: r.get::<&str, _>(1).unwrap_or("").to_string(),
                    referenced_table: r.get::<&str, _>(2).unwrap_or("").to_string(),
                    referenced_column: r.get::<&str, _>(3).unwrap_or("").to_string(),
                    on_delete: r.get::<&str, _>(4).map(String::from),
                    on_update: None,
                })
                .collect();

            let mut idx_q = Query::new(
                "SELECT i.name, c.name, i.is_unique, i.is_primary_key \
                 FROM sys.indexes i \
                 JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
                 JOIN sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
                 JOIN sys.tables t ON i.object_id = t.object_id \
                 WHERE t.name = @P1 AND i.name IS NOT NULL ORDER BY i.name, ic.key_ordinal",
            );
            idx_q.bind(table_name.as_str());
            let idx_vec = match idx_q.query(&mut client).await {
                Ok(s) => s.into_first_result().await.unwrap_or_default(),
                Err(_) => vec![],
            };
            let mut idx_map: BTreeMap<String, (bool, bool, Vec<String>)> = BTreeMap::new();
            for r in &idx_vec {
                let iname = r.get::<&str, _>(0).unwrap_or("").to_string();
                let cname = r.get::<&str, _>(1).unwrap_or("").to_string();
                let is_u = r.get::<bool, _>(2).unwrap_or(false);
                let is_p = r.get::<bool, _>(3).unwrap_or(false);
                if !iname.is_empty() {
                    idx_map
                        .entry(iname)
                        .or_insert((is_u, is_p, Vec::new()))
                        .2
                        .push(cname);
                }
            }
            let indexes: Vec<IndexDef> = idx_map
                .into_iter()
                .map(|(name, (is_unique, is_primary, cols))| IndexDef {
                    name,
                    columns: cols,
                    is_unique,
                    is_primary,
                })
                .collect();

            let row_count = if object_type == "TABLE" {
                let qt = quote_tsql_ident(&table_name);
                match client
                    .simple_query(format!("SELECT COUNT_BIG(*) FROM dbo.{qt}"))
                    .await
                {
                    Ok(s) => s
                        .into_row()
                        .await
                        .ok()
                        .flatten()
                        .and_then(|r| r.get::<i64, _>(0))
                        .unwrap_or(0)
                        .max(0) as u64,
                    Err(_) => 0,
                }
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

        let (mut client, _tunnel) = Self::connect(config).await?;
        client
            .simple_query("BEGIN TRANSACTION")
            .await?
            .into_results()
            .await?;

        let mut results = Vec::with_capacity(mutations.len());
        for m in mutations {
            let (sql, params) = build_tsql_mutation(table, m)?;
            let mut q = Query::new(sql);
            for p in &params {
                bind_tsql_json(&mut q, p);
            }
            match q.execute(&mut client).await {
                Ok(done) => results.push(MutationItemResult {
                    op: m.op,
                    rows_affected: done.total(),
                }),
                Err(e) => {
                    let _ = client.simple_query("ROLLBACK TRANSACTION").await;
                    return Err(anyhow::anyhow!("MSSQL mutation rolled back: {e}"));
                }
            }
        }

        client
            .simple_query("COMMIT TRANSACTION")
            .await?
            .into_results()
            .await?;
        Ok(BatchMutationResult {
            atomic: true,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}

fn build_tsql_mutation<'a>(
    table: &str,
    m: &'a RowMutation,
) -> Result<(String, Vec<&'a serde_json::Value>)> {
    let qt = quote_tsql_ident(table);
    match m.op {
        MutationOp::Insert => {
            if m.values.is_empty() {
                anyhow::bail!("INSERT requires at least one column");
            }
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let col_list = cols
                .iter()
                .map(|(k, _)| quote_tsql_ident(k))
                .collect::<Vec<_>>()
                .join(", ");
            let phs = (1..=cols.len())
                .map(|i| format!("@P{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            let params = cols.into_iter().map(|(_, v)| v).collect();
            Ok((format!("INSERT INTO {qt} ({col_list}) VALUES ({phs})"), params))
        }
        MutationOp::Update => {
            if m.values.is_empty() || m.where_clause.is_empty() {
                anyhow::bail!("UPDATE requires SET values and WHERE keys");
            }
            let mut sets: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            sets.sort_by_key(|(k, _)| *k);
            let mut wheres: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            wheres.sort_by_key(|(k, _)| *k);

            let set_sql = sets
                .iter()
                .enumerate()
                .map(|(i, (k, _))| format!("{} = @P{}", quote_tsql_ident(k), i + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let mut idx = sets.len();
            let mut params: Vec<&serde_json::Value> = sets.into_iter().map(|(_, v)| v).collect();
            let where_sql = wheres
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_tsql_ident(k))
                    } else {
                        idx += 1;
                        params.push(v);
                        format!("{} = @P{idx}", quote_tsql_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("UPDATE {qt} SET {set_sql} WHERE {where_sql}"), params))
        }
        MutationOp::Delete => {
            if m.where_clause.is_empty() {
                anyhow::bail!("DELETE requires WHERE keys");
            }
            let mut wheres: Vec<(&String, &serde_json::Value)> = m.where_clause.iter().collect();
            wheres.sort_by_key(|(k, _)| *k);
            let mut idx = 0usize;
            let mut params = Vec::new();
            let where_sql = wheres
                .into_iter()
                .map(|(k, v)| {
                    if v.is_null() {
                        format!("{} IS NULL", quote_tsql_ident(k))
                    } else {
                        idx += 1;
                        params.push(v);
                        format!("{} = @P{idx}", quote_tsql_ident(k))
                    }
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            Ok((format!("DELETE FROM {qt} WHERE {where_sql}"), params))
        }
    }
}

fn bind_tsql_json<'a>(q: &mut Query<'a>, val: &'a serde_json::Value) {
    match val {
        serde_json::Value::Null => q.bind(None::<&str>),
        serde_json::Value::Bool(b) => q.bind(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i);
            } else if let Some(f) = n.as_f64() {
                q.bind(f);
            } else {
                q.bind(n.to_string());
            }
        }
        serde_json::Value::String(s) => q.bind(s.as_str()),
        other => q.bind(other.to_string()),
    }
}
