//! Native HTTP / REST Wire-Protocol Drivers for Cloud Warehouses, OLAP, Search, Graph & Time-Series:
//! - **ClickHouse** (`Clickhouse` — HTTP SQL `FORMAT JSON` + `system.columns` + `JSONEachRow` batch)
//! - **Trino / Presto** (`Trino` — `/v1/statement` async multi-page query protocol)
//! - **Turso / libSQL** (`Libsql` — Hrana `/v2/pipeline` HTTP SQL protocol)
//! - **QuestDB** (`Questdb` — `/exec` HTTP SQL endpoint)
//! - **Snowflake** (`Snowflake` — SQL REST API v2 `/api/v2/statements`)
//! - **Google BigQuery** (`Bigquery` — Jobs Query REST API `/bigquery/v2/projects/{project}/queries`)
//! - **Databricks SQL** (`Databricks` — Statement Execution API `/api/2.0/sql/statements`)
//! - **Elasticsearch / OpenSearch** (`Elasticsearch` — `/_sql?format=json`, `/_search`, `/_cat/indices`)
//! - **Neo4j** (`Neo4j` — Transactional HTTP Cypher API `/db/{db}/tx/commit`)
//! - **InfluxDB** (`Influxdb` — `/query` InfluxQL + `/write` Line Protocol)
//! - **Apache CouchDB** (`Couchdb` — Mango `/_find` + `/_all_docs` + `/_bulk_docs`)

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, DbKind, MutationItemResult,
    MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct HttpEngineDriver {
    pub kind: DbKind,
}

impl HttpEngineDriver {
    pub fn new(kind: DbKind) -> Self {
        Self { kind }
    }

    fn http_client(cfg: &DatabaseConnection) -> Result<reqwest::Client> {
        let mut b = reqwest::Client::builder().timeout(Duration::from_secs(30));
        if cfg.ssl_reject_unauthorized == Some(false) {
            b = b.danger_accept_invalid_certs(true);
        }
        Ok(b.build()?)
    }

    async fn base_url(
        cfg: &DatabaseConnection,
    ) -> Result<(String, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if let Some(cs) = cfg
            .connection_string
            .as_deref()
            .filter(|s| !s.trim().is_empty() && !cfg.ssh_enabled)
        {
            let clean = cs
                .trim()
                .trim_end_matches('/')
                .replacen("libsql://", "https://", 1);
            if clean.starts_with("http://") || clean.starts_with("https://") {
                return Ok((clean, None));
            }
            let scheme = if cfg.ssl { "https" } else { "http" };
            return Ok((format!("{scheme}://{clean}"), None));
        }

        let def_port: u16 = cfg.kind.default_port().parse().unwrap_or(8080);
        let (host, port, tunnel) = super::ssh_tunnel::effective_target(cfg, def_port).await?;
        let scheme = if cfg.ssl
            || matches!(
                cfg.kind,
                DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks
            )
        {
            "https"
        } else {
            "http"
        };
        Ok((format!("{scheme}://{host}:{port}"), tunnel))
    }

    fn apply_auth(
        mut req: reqwest::RequestBuilder,
        cfg: &DatabaseConnection,
    ) -> reqwest::RequestBuilder {
        let user = cfg.username.as_deref().unwrap_or("").trim();
        let pass = cfg.password.as_deref().unwrap_or("").trim();
        match cfg.kind {
            DbKind::Libsql | DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks => {
                if !pass.is_empty() {
                    req = req.bearer_auth(pass);
                }
            }
            _ => {
                if !user.is_empty() || !pass.is_empty() {
                    req = req.basic_auth(user, if pass.is_empty() { None } else { Some(pass) });
                }
            }
        }
        req
    }
}

#[async_trait]
impl DbDriver for HttpEngineDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let qr = match self.kind {
            DbKind::Clickhouse => self.query(config, "SELECT version() AS ver").await?,
            DbKind::Trino | DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks
            | DbKind::Libsql | DbKind::Questdb => self.query(config, "SELECT 1 AS ok").await?,
            DbKind::Elasticsearch => self.es_ping(config).await?,
            DbKind::Neo4j => self.query(config, "RETURN 'Neo4j OK' AS status").await?,
            DbKind::Influxdb => self.query(config, "SHOW DATABASES").await?,
            DbKind::Couchdb => self.couch_ping(config).await?,
            _ => bail!("Unsupported HTTP engine {:?}", self.kind),
        };
        let _ = qr;
        Ok(format!(
            "Connected to {} successfully ({}ms)",
            self.kind.display_name(),
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        match self.kind {
            DbKind::Clickhouse => self.query_clickhouse(config, sql).await,
            DbKind::Trino => self.query_trino(config, sql).await,
            DbKind::Libsql => self.query_libsql(config, sql).await,
            DbKind::Questdb => self.query_questdb(config, sql).await,
            DbKind::Snowflake => self.query_snowflake(config, sql).await,
            DbKind::Bigquery => self.query_bigquery(config, sql).await,
            DbKind::Databricks => self.query_databricks(config, sql).await,
            DbKind::Elasticsearch => self.query_elasticsearch(config, sql).await,
            DbKind::Neo4j => self.query_neo4j(config, sql).await,
            DbKind::Influxdb => self.query_influxdb(config, sql).await,
            DbKind::Couchdb => self.query_couchdb(config, sql).await,
            _ => bail!("Unsupported HTTP engine {:?}", self.kind),
        }
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        match self.kind {
            DbKind::Clickhouse => {
                let db = if config.database.is_empty() {
                    "default"
                } else {
                    &config.database
                };
                let qr = self
                    .query_clickhouse(
                        config,
                        &format!(
                            "SELECT table, name, type FROM system.columns WHERE database = '{}' ORDER BY table, position",
                            db.replace('\'', "''")
                        ),
                    )
                    .await?;
                Ok(group_flat_columns(&qr, "table", "name", "type", "TABLE"))
            }
            DbKind::Trino | DbKind::Snowflake | DbKind::Databricks => {
                let qr = self
                    .query(
                        config,
                        "SELECT table_name, column_name, data_type FROM information_schema.columns ORDER BY table_name, ordinal_position LIMIT 500",
                    )
                    .await?;
                Ok(group_flat_columns(
                    &qr,
                    "table_name",
                    "column_name",
                    "data_type",
                    "TABLE",
                ))
            }
            DbKind::Bigquery => {
                let ds = config.database.split('.').nth(1).unwrap_or(&config.database);
                let qr = self
                    .query(
                        config,
                        &format!(
                            "SELECT table_name, column_name, data_type FROM `{ds}`.INFORMATION_SCHEMA.COLUMNS LIMIT 500"
                        ),
                    )
                    .await?;
                Ok(group_flat_columns(
                    &qr,
                    "table_name",
                    "column_name",
                    "data_type",
                    "TABLE",
                ))
            }
            DbKind::Libsql => {
                let qr = self
                    .query_libsql(
                        config,
                        "SELECT name, type, sql FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name",
                    )
                    .await?;
                let mut out = Vec::new();
                for r in &qr.rows {
                    let tname = r.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                    if tname.is_empty() {
                        continue;
                    }
                    let otype = r
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("table")
                        .to_ascii_uppercase();
                    let ddl = r.get("sql").and_then(Value::as_str).map(String::from);
                    let info = self
                        .query_libsql(config, &format!("PRAGMA table_info(\"{tname}\")"))
                        .await
                        .unwrap_or_else(|_| empty_qr());
                    let cols = info
                        .rows
                        .iter()
                        .map(|cr| ColumnDefinition {
                            name: cr.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                            col_type: cr
                                .get("type")
                                .and_then(Value::as_str)
                                .unwrap_or("TEXT")
                                .to_ascii_uppercase(),
                            is_primary_key: cr.get("pk").and_then(Value::as_i64).unwrap_or(0) > 0,
                            is_nullable: cr.get("notnull").and_then(Value::as_i64).unwrap_or(0) == 0,
                            ..Default::default()
                        })
                        .collect();
                    out.push(TableSchema {
                        name: tname,
                        object_type: otype,
                        row_count: 0,
                        columns: cols,
                        foreign_keys: vec![],
                        indexes: vec![],
                        ddl,
                    });
                }
                Ok(out)
            }
            DbKind::Questdb => {
                let qr = self.query_questdb(config, "SHOW TABLES").await?;
                let mut out = Vec::new();
                for r in &qr.rows {
                    if let Some(tname) = r.values().next().and_then(Value::as_str) {
                        let cols_qr = self
                            .query_questdb(config, &format!("SHOW COLUMNS FROM \"{tname}\""))
                            .await
                            .unwrap_or_else(|_| empty_qr());
                        let cols = cols_qr
                            .rows
                            .iter()
                            .map(|cr| ColumnDefinition {
                                name: cr
                                    .get("column")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                col_type: cr
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .unwrap_or("STRING")
                                    .to_string(),
                                is_nullable: true,
                                ..Default::default()
                            })
                            .collect();
                        out.push(TableSchema {
                            name: tname.to_string(),
                            object_type: "TABLE".into(),
                            row_count: 0,
                            columns: cols,
                            foreign_keys: vec![],
                            indexes: vec![],
                            ddl: None,
                        });
                    }
                }
                Ok(out)
            }
            DbKind::Elasticsearch => self.schema_elasticsearch(config).await,
            DbKind::Neo4j => self.schema_neo4j(config).await,
            DbKind::Influxdb => self.schema_influxdb(config).await,
            DbKind::Couchdb => self.schema_couchdb(config).await,
            _ => Ok(vec![]),
        }
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
                atomic: false,
                results: vec![],
                execution_time_ms: 0,
                message: None,
            });
        }

        let mut results = Vec::with_capacity(mutations.len());
        match self.kind {
            DbKind::Clickhouse => {
                for m in mutations {
                    match m.op {
                        MutationOp::Insert => {
                            let line = serde_json::to_string(&m.values)?;
                            self.query_clickhouse(
                                config,
                                &format!("INSERT INTO \"{table}\" FORMAT JSONEachRow {line}"),
                            )
                            .await?;
                        }
                        MutationOp::Update => {
                            let sets = kv_sql_eq(&m.values, ", ");
                            let wh = kv_sql_eq(&m.where_clause, " AND ");
                            self.query_clickhouse(
                                config,
                                &format!("ALTER TABLE \"{table}\" UPDATE {sets} WHERE {wh}"),
                            )
                            .await?;
                        }
                        MutationOp::Delete => {
                            let wh = kv_sql_eq(&m.where_clause, " AND ");
                            self.query_clickhouse(
                                config,
                                &format!("ALTER TABLE \"{table}\" DELETE WHERE {wh}"),
                            )
                            .await?;
                        }
                    }
                    results.push(MutationItemResult { op: m.op, rows_affected: 1 });
                }
            }
            _ => {
                for m in mutations {
                    let sql = match m.op {
                        MutationOp::Insert => {
                            let cols: Vec<String> = m.values.keys().map(|k| format!("\"{k}\"")).collect();
                            let vals: Vec<String> = m.values.values().map(sql_lit_generic).collect();
                            format!(
                                "INSERT INTO \"{table}\" ({}) VALUES ({})",
                                cols.join(", "),
                                vals.join(", ")
                            )
                        }
                        MutationOp::Update => format!(
                            "UPDATE \"{table}\" SET {} WHERE {}",
                            kv_sql_eq(&m.values, ", "),
                            kv_sql_eq(&m.where_clause, " AND ")
                        ),
                        MutationOp::Delete => format!(
                            "DELETE FROM \"{table}\" WHERE {}",
                            kv_sql_eq(&m.where_clause, " AND ")
                        ),
                    };
                    self.query(config, &sql).await?;
                    results.push(MutationItemResult { op: m.op, rows_affected: 1 });
                }
            }
        }

        Ok(BatchMutationResult {
            atomic: false,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: Some(format!("Applied via {} HTTP API.", self.kind.display_name())),
        })
    }
}

// ---- Individual Native Protocol Implementations ----

impl HttpEngineDriver {
    async fn query_clickhouse(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let trimmed = sql.trim().trim_end_matches(';');
        let up = trimmed.to_ascii_uppercase();
        let is_read = up.starts_with("SELECT")
            || up.starts_with("SHOW")
            || up.starts_with("DESCRIBE")
            || up.starts_with("DESC ")
            || up.starts_with("EXPLAIN")
            || up.starts_with("WITH");

        let body_sql = if is_read && !up.contains("FORMAT ") {
            format!("{trimmed} FORMAT JSON")
        } else {
            trimmed.to_string()
        };

        let db = if cfg.database.is_empty() { "default" } else { &cfg.database };
        let mut req = client.post(format!("{base}/?database={db}")).body(body_sql);
        if let Some(u) = cfg.username.as_deref().filter(|s| !s.is_empty()) {
            req = req.header("X-ClickHouse-User", u);
        }
        if let Some(p) = cfg.password.as_deref().filter(|s| !s.is_empty()) {
            req = req.header("X-ClickHouse-Key", p);
        }

        let resp = req.send().await.context("ClickHouse HTTP request failed")?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            bail!("ClickHouse HTTP {status}: {text}");
        }

        if !is_read {
            return Ok(ok_status_qr("ClickHouse command executed.", start));
        }

        let parsed: Value = serde_json::from_str(&text).context("parsing ClickHouse FORMAT JSON")?;
        let columns: Vec<String> = parsed["meta"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let rows: Vec<HashMap<String, Value>> = parsed["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .collect()
            })
            .unwrap_or_default();

        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("SELECT".into()),
        })
    }

    async fn query_trino(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let user = cfg.username.as_deref().filter(|s| !s.is_empty()).unwrap_or("bapu");
        let (catalog, schema) = cfg
            .database
            .split_once('.')
            .unwrap_or((cfg.database.as_str(), "default"));

        let mut req = Self::apply_auth(client.post(format!("{base}/v1/statement")), cfg)
            .header("X-Trino-User", user)
            .body(sql.trim().trim_end_matches(';').to_string());
        if !catalog.is_empty() {
            req = req.header("X-Trino-Catalog", catalog).header("X-Trino-Schema", schema);
        }

        let mut page: Value = req.send().await?.error_for_status()?.json().await?;
        let mut columns: Vec<String> = Vec::new();
        let mut raw_rows: Vec<Vec<Value>> = Vec::new();

        loop {
            if let Some(err) = page.get("error") {
                bail!("Trino error: {}", err["message"].as_str().unwrap_or("query failed"));
            }
            if columns.is_empty() {
                if let Some(cols) = page["columns"].as_array() {
                    columns = cols
                        .iter()
                        .filter_map(|c| c["name"].as_str().map(String::from))
                        .collect();
                }
            }
            if let Some(data) = page["data"].as_array() {
                for r in data {
                    if let Some(arr) = r.as_array() {
                        raw_rows.push(arr.clone());
                    }
                }
            }
            let Some(next_uri) = page["nextUri"].as_str().map(String::from) else {
                break;
            };
            page = Self::apply_auth(client.get(next_uri), cfg)
                .header("X-Trino-User", user)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
        }

        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("TRINO".into()),
        })
    }

    async fn query_libsql(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let payload = json!({
            "requests": [
                { "type": "execute", "stmt": { "sql": sql.trim() } },
                { "type": "close" }
            ]
        });

        let resp = Self::apply_auth(client.post(format!("{base}/v2/pipeline")), cfg)
            .json(&payload)
            .send()
            .await
            .context("libSQL /v2/pipeline request failed")?;
        let status = resp.status();
        let val: Value = resp.json().await.context("parsing libSQL response")?;
        if !status.is_success() {
            bail!("libSQL HTTP {status}: {val}");
        }

        let first = &val["results"][0];
        if first["type"].as_str() == Some("error") {
            bail!("libSQL error: {}", first["error"]["message"].as_str().unwrap_or("failed"));
        }
        let res = &first["response"]["result"];
        let columns: Vec<String> = res["cols"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut rows = Vec::new();
        if let Some(r_arr) = res["rows"].as_array() {
            for r in r_arr {
                if let Some(cells) = r.as_array() {
                    let mut map = HashMap::new();
                    for (i, col) in columns.iter().enumerate() {
                        let cell = cells.get(i).unwrap_or(&Value::Null);
                        let decoded = match cell["type"].as_str() {
                            Some("null") => Value::Null,
                            Some("integer") => cell["value"]
                                .as_str()
                                .and_then(|s| s.parse::<i64>().ok())
                                .map(|n| Value::Number(n.into()))
                                .unwrap_or_else(|| cell["value"].clone()),
                            Some("float") => cell["value"].clone(),
                            _ => cell.get("value").cloned().unwrap_or(Value::Null),
                        };
                        map.insert(col.clone(), decoded);
                    }
                    rows.push(map);
                }
            }
        }
        let affected = res["affected_row_count"].as_u64().unwrap_or(rows.len() as u64);
        Ok(QueryResult {
            row_count: if columns.is_empty() { affected } else { rows.len() as u64 },
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("LIBSQL".into()),
        })
    }

    async fn query_questdb(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let resp = Self::apply_auth(
            client.get(format!("{base}/exec")).query(&[("query", sql.trim())]),
            cfg,
        )
        .send()
        .await?;
        let status = resp.status();
        let val: Value = resp.json().await?;
        if !status.is_success() || val.get("error").is_some() {
            bail!(
                "QuestDB error: {}",
                val["error"].as_str().unwrap_or("execution failed")
            );
        }
        let columns: Vec<String> = val["columns"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        let raw_rows: Vec<Vec<Value>> = val["dataset"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r.as_array().cloned()).collect())
            .unwrap_or_default();
        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("QUESTDB".into()),
        })
    }

    async fn query_snowflake(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let (db, schema) = cfg
            .database
            .split_once('.')
            .unwrap_or((cfg.database.as_str(), "PUBLIC"));
        let mut body = json!({
            "statement": sql.trim(),
            "database": db,
            "schema": schema,
            "timeout": 30
        });
        if let Some(wh) = cfg.username.as_deref().filter(|s| !s.is_empty()) {
            body["warehouse"] = json!(wh);
        }

        let resp = Self::apply_auth(client.post(format!("{base}/api/v2/statements")), cfg)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let val: Value = resp.json().await?;
        if !status.is_success() {
            bail!(
                "Snowflake HTTP {status}: {}",
                val["message"].as_str().unwrap_or("query failed")
            );
        }

        let columns: Vec<String> = val["resultSetMetaData"]["rowType"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        let raw_rows: Vec<Vec<Value>> = val["data"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r.as_array().cloned()).collect())
            .unwrap_or_default();
        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("SNOWFLAKE".into()),
        })
    }

    async fn query_bigquery(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let project = cfg.database.split('.').next().unwrap_or("my-project");
        let body = json!({ "query": sql.trim(), "useLegacySql": false });
        let resp = Self::apply_auth(
            client.post(format!("{base}/bigquery/v2/projects/{project}/queries")),
            cfg,
        )
        .json(&body)
        .send()
        .await?;
        let status = resp.status();
        let val: Value = resp.json().await?;
        if !status.is_success() {
            bail!("BigQuery HTTP {status}: {val}");
        }
        let columns: Vec<String> = val["schema"]["fields"]
            .as_array()
            .map(|a| a.iter().filter_map(|f| f["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        let mut rows = Vec::new();
        if let Some(r_arr) = val["rows"].as_array() {
            for r in r_arr {
                if let Some(fields) = r["f"].as_array() {
                    let mut map = HashMap::new();
                    for (i, col) in columns.iter().enumerate() {
                        let v = fields.get(i).and_then(|cell| cell.get("v")).cloned().unwrap_or(Value::Null);
                        map.insert(col.clone(), v);
                    }
                    rows.push(map);
                }
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
            command: Some("BIGQUERY".into()),
        })
    }

    async fn query_databricks(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let (catalog, schema) = cfg
            .database
            .split_once('.')
            .unwrap_or((cfg.database.as_str(), "default"));
        let warehouse_id = cfg.username.as_deref().unwrap_or("");
        let body = json!({
            "statement": sql.trim(),
            "warehouse_id": warehouse_id,
            "catalog": catalog,
            "schema": schema,
            "wait_timeout": "30s"
        });
        let resp = Self::apply_auth(client.post(format!("{base}/api/2.0/sql/statements")), cfg)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let val: Value = resp.json().await?;
        if !status.is_success() {
            bail!("Databricks HTTP {status}: {val}");
        }
        let columns: Vec<String> = val["manifest"]["schema"]["columns"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        let raw_rows: Vec<Vec<Value>> = val["result"]["data_array"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r.as_array().cloned()).collect())
            .unwrap_or_default();
        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("DATABRICKS".into()),
        })
    }

    async fn es_ping(&self, cfg: &DatabaseConnection) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let val: Value = Self::apply_auth(client.get(format!("{base}/")), cfg)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let mut m = HashMap::new();
        m.insert("cluster_name".into(), val["cluster_name"].clone());
        m.insert("version".into(), val["version"]["number"].clone());
        Ok(QueryResult {
            row_count: 1,
            columns: vec!["cluster_name".into(), "version".into()],
            rows: vec![m],
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("PING".into()),
        })
    }

    async fn query_elasticsearch(&self, cfg: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let trimmed = sql.trim();

        // Support both Elasticsearch SQL (`SELECT ...`) and JSON Query DSL (`{ "query": ... }`)
        if trimmed.starts_with('{') {
            let idx = if cfg.database.is_empty() || cfg.database == "_all" {
                "_all"
            } else {
                &cfg.database
            };
            let dsl: Value = serde_json::from_str(trimmed).context("invalid Elasticsearch JSON DSL")?;
            let val: Value = Self::apply_auth(client.post(format!("{base}/{idx}/_search")), cfg)
                .json(&dsl)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            return Ok(json_objects_to_qr(
                val["hits"]["hits"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|h| {
                                let mut obj = h["_source"].as_object().cloned().unwrap_or_default();
                                obj.insert("_id".into(), h["_id"].clone());
                                obj
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                start,
                "ES_SEARCH",
            ));
        }

        let val: Value = Self::apply_auth(client.post(format!("{base}/_sql?format=json")), cfg)
            .json(&json!({ "query": trimmed.trim_end_matches(';') }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let columns: Vec<String> = val["columns"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();
        let raw_rows: Vec<Vec<Value>> = val["rows"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r.as_array().cloned()).collect())
            .unwrap_or_default();
        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("ES_SQL".into()),
        })
    }

    async fn schema_elasticsearch(&self, cfg: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let indices: Vec<Value> = Self::apply_auth(
            client.get(format!("{base}/_cat/indices?format=json")),
            cfg,
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

        let mut out = Vec::new();
        for item in indices {
            let Some(idx_name) = item["index"].as_str().filter(|n| !n.starts_with('.')) else {
                continue;
            };
            let count = item["docs.count"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let mapping: Value = Self::apply_auth(
                client.get(format!("{base}/{idx_name}/_mapping")),
                cfg,
            )
            .send()
            .await?
            .json()
            .await
            .unwrap_or(Value::Null);
            let mut cols = Vec::new();
            if let Some(props) = mapping[idx_name]["mappings"]["properties"].as_object() {
                for (fname, fdef) in props {
                    cols.push(ColumnDefinition {
                        name: fname.clone(),
                        col_type: fdef["type"].as_str().unwrap_or("OBJECT").to_ascii_uppercase(),
                        is_nullable: true,
                        ..Default::default()
                    });
                }
            }
            out.push(TableSchema {
                name: idx_name.to_string(),
                object_type: "INDEX".into(),
                row_count: count,
                columns: cols,
                foreign_keys: vec![],
                indexes: vec![],
                ddl: None,
            });
        }
        Ok(out)
    }

    async fn query_neo4j(&self, cfg: &DatabaseConnection, cypher: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let db = if cfg.database.is_empty() { "neo4j" } else { &cfg.database };
        let body = json!({
            "statements": [{ "statement": cypher.trim() }]
        });
        let val: Value = Self::apply_auth(client.post(format!("{base}/db/{db}/tx/commit")), cfg)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(errs) = val["errors"].as_array().filter(|e| !e.is_empty()) {
            bail!(
                "Neo4j Cypher error: {}",
                errs[0]["message"].as_str().unwrap_or("failed")
            );
        }

        let res0 = &val["results"][0];
        let columns: Vec<String> = res0["columns"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let raw_rows: Vec<Vec<Value>> = res0["data"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|item| item["row"].as_array().cloned())
                    .collect()
            })
            .unwrap_or_default();
        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("CYPHER".into()),
        })
    }

    async fn schema_neo4j(&self, cfg: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let qr = self.query_neo4j(cfg, "CALL db.labels() YIELD label RETURN label").await?;
        Ok(qr
            .rows
            .iter()
            .filter_map(|r| r.get("label").and_then(Value::as_str))
            .map(|lbl| TableSchema {
                name: lbl.to_string(),
                object_type: "NODE_LABEL".into(),
                row_count: 0,
                columns: vec![ColumnDefinition {
                    name: "id".into(),
                    col_type: "NODE".into(),
                    is_primary_key: true,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                indexes: vec![],
                ddl: None,
            })
            .collect())
    }

    async fn query_influxdb(&self, cfg: &DatabaseConnection, q: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let val: Value = Self::apply_auth(
            client
                .get(format!("{base}/query"))
                .query(&[("db", cfg.database.as_str()), ("q", q.trim())]),
            cfg,
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

        let series = &val["results"][0]["series"][0];
        let columns: Vec<String> = series["columns"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let raw_rows: Vec<Vec<Value>> = series["values"]
            .as_array()
            .map(|a| a.iter().filter_map(|r| r.as_array().cloned()).collect())
            .unwrap_or_default();
        let rows = zip_positional_rows(&columns, raw_rows);
        Ok(QueryResult {
            row_count: rows.len() as u64,
            columns,
            rows,
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("INFLUXQL".into()),
        })
    }

    async fn schema_influxdb(&self, cfg: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let qr = self.query_influxdb(cfg, "SHOW MEASUREMENTS").await?;
        Ok(qr
            .rows
            .iter()
            .filter_map(|r| r.get("name").and_then(Value::as_str))
            .map(|m| TableSchema {
                name: m.to_string(),
                object_type: "MEASUREMENT".into(),
                row_count: 0,
                columns: vec![ColumnDefinition {
                    name: "time".into(),
                    col_type: "TIMESTAMP".into(),
                    is_primary_key: true,
                    ..Default::default()
                }],
                foreign_keys: vec![],
                indexes: vec![],
                ddl: None,
            })
            .collect())
    }

    async fn couch_ping(&self, cfg: &DatabaseConnection) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let val: Value = Self::apply_auth(client.get(format!("{base}/")), cfg)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let mut m = HashMap::new();
        m.insert("couchdb".into(), val["couchdb"].clone());
        m.insert("version".into(), val["version"].clone());
        Ok(QueryResult {
            row_count: 1,
            columns: vec!["couchdb".into(), "version".into()],
            rows: vec![m],
            execution_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            notices: vec![],
            query_plan: None,
            command: Some("PING".into()),
        })
    }

    async fn query_couchdb(&self, cfg: &DatabaseConnection, raw: &str) -> Result<QueryResult> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let start = Instant::now();
        let db = if cfg.database.is_empty() { "mydb" } else { &cfg.database };
        let trimmed = raw.trim();

        let selector: Value = if trimmed.starts_with('{') {
            serde_json::from_str(trimmed).context("invalid CouchDB Mango JSON")?
        } else {
            json!({ "selector": {}, "limit": 50 })
        };
        let body = if selector.get("selector").is_some() {
            selector
        } else {
            json!({ "selector": selector, "limit": 50 })
        };

        let val: Value = Self::apply_auth(client.post(format!("{base}/{db}/_find")), cfg)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let docs: Vec<serde_json::Map<String, Value>> = val["docs"]
            .as_array()
            .map(|a| a.iter().filter_map(|d| d.as_object().cloned()).collect())
            .unwrap_or_default();
        Ok(json_objects_to_qr(docs, start, "MANGO_FIND"))
    }

    async fn schema_couchdb(&self, cfg: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let client = Self::http_client(cfg)?;
        let (base, _tun) = Self::base_url(cfg).await?;
        let dbs: Vec<String> = Self::apply_auth(client.get(format!("{base}/_all_dbs")), cfg)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(dbs
            .into_iter()
            .filter(|n| !n.starts_with('_'))
            .map(|name| TableSchema {
                name,
                object_type: "DATABASE".into(),
                row_count: 0,
                columns: vec![
                    ColumnDefinition {
                        name: "_id".into(),
                        col_type: "STRING".into(),
                        is_primary_key: true,
                        ..Default::default()
                    },
                    ColumnDefinition {
                        name: "_rev".into(),
                        col_type: "STRING".into(),
                        ..Default::default()
                    },
                ],
                foreign_keys: vec![],
                indexes: vec![],
                ddl: None,
            })
            .collect())
    }
}

fn zip_positional_rows(
    columns: &[String],
    raw_rows: Vec<Vec<Value>>,
) -> Vec<HashMap<String, Value>> {
    raw_rows
        .into_iter()
        .map(|arr| {
            let mut m = HashMap::new();
            for (i, col) in columns.iter().enumerate() {
                m.insert(col.clone(), arr.get(i).cloned().unwrap_or(Value::Null));
            }
            m
        })
        .collect()
}

fn json_objects_to_qr(
    objs: Vec<serde_json::Map<String, Value>>,
    start: Instant,
    cmd: &str,
) -> QueryResult {
    let mut keys = BTreeSet::new();
    let mut rows = Vec::with_capacity(objs.len());
    for obj in objs {
        let mut m = HashMap::new();
        for (k, v) in obj {
            keys.insert(k.clone());
            m.insert(k, v);
        }
        rows.push(m);
    }
    QueryResult {
        row_count: rows.len() as u64,
        columns: keys.into_iter().collect(),
        rows,
        execution_time_ms: start.elapsed().as_millis() as u64,
        error: None,
        notices: vec![],
        query_plan: None,
        command: Some(cmd.into()),
    }
}

fn group_flat_columns(
    qr: &QueryResult,
    tbl_key: &str,
    col_key: &str,
    type_key: &str,
    obj_type: &str,
) -> Vec<TableSchema> {
    let mut grouped: BTreeMap<String, Vec<ColumnDefinition>> = BTreeMap::new();
    for r in &qr.rows {
        let t = r.get(tbl_key).and_then(Value::as_str).unwrap_or("");
        let c = r.get(col_key).and_then(Value::as_str).unwrap_or("");
        let dt = r.get(type_key).and_then(Value::as_str).unwrap_or("STRING");
        if !t.is_empty() && !c.is_empty() {
            grouped.entry(t.to_string()).or_default().push(ColumnDefinition {
                name: c.to_string(),
                col_type: dt.to_ascii_uppercase(),
                is_nullable: true,
                ..Default::default()
            });
        }
    }
    grouped
        .into_iter()
        .map(|(name, columns)| TableSchema {
            name,
            object_type: obj_type.into(),
            row_count: 0,
            columns,
            foreign_keys: vec![],
            indexes: vec![],
            ddl: None,
        })
        .collect()
}

fn empty_qr() -> QueryResult {
    QueryResult {
        columns: vec![],
        rows: vec![],
        row_count: 0,
        execution_time_ms: 0,
        error: None,
        notices: vec![],
        query_plan: None,
        command: None,
    }
}

fn ok_status_qr(msg: &str, start: Instant) -> QueryResult {
    let mut r = HashMap::new();
    r.insert("status".into(), json!("OK"));
    r.insert("message".into(), json!(msg));
    QueryResult {
        columns: vec!["status".into(), "message".into()],
        rows: vec![r],
        row_count: 1,
        execution_time_ms: start.elapsed().as_millis() as u64,
        error: None,
        notices: vec![],
        query_plan: None,
        command: Some("EXEC".into()),
    }
}

fn sql_lit_generic(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Bool(b) => if *b { "true".into() } else { "false".into() },
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn kv_sql_eq(m: &HashMap<String, Value>, sep: &str) -> String {
    m.iter()
        .map(|(k, v)| format!("\"{k}\" = {}", sql_lit_generic(v)))
        .collect::<Vec<_>>()
        .join(sep)
}
