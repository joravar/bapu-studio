//! Native MongoDB driver via the official `mongodb` v3 crate.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, IndexDef, MutationItemResult,
    MutationOp, QueryResult, RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::ClientOptions;
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct MongoDriver {
    clients: Mutex<HashMap<String, mongodb::Client>>,
}

impl MongoDriver {
    async fn build_uri(
        config: &DatabaseConnection,
    ) -> Result<(String, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if let Some(uri) = config.connection_string.as_deref().filter(|s| !s.trim().is_empty()) {
            return Ok((uri.to_string(), None));
        }
        let (host, port, tunnel) = super::ssh_tunnel::effective_target(config, 27017).await?;
        let auth = match (config.username.as_deref(), config.password.as_deref()) {
            (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => format!("{u}:{p}@"),
            _ => String::new(),
        };
        let db = if config.database.is_empty() { "test" } else { &config.database };
        Ok((format!("mongodb://{auth}{host}:{port}/{db}"), tunnel))
    }

    async fn client_for(
        &self,
        config: &DatabaseConnection,
    ) -> Result<(mongodb::Client, Option<super::ssh_tunnel::SshTunnelGuard>)> {
        if !config.ssh_enabled {
            if let Some(client) = self
                .clients
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&config.id)
            {
                return Ok((client.clone(), None));
            }
        }
        let (uri, tunnel) = Self::build_uri(config).await?;
        let mut opts = ClientOptions::parse(&uri).await.context("parsing MongoDB URI")?;
        opts.server_selection_timeout = Some(Duration::from_secs(7));
        let client = mongodb::Client::with_options(opts).context("creating MongoDB client")?;
        if tunnel.is_none() {
            self.clients
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(config.id.clone(), client.clone());
        }
        Ok((client, tunnel))
    }

    fn db_name(config: &DatabaseConnection) -> &str {
        if config.database.is_empty() {
            "test"
        } else {
            &config.database
        }
    }
}

/// Parses either a raw JSON filter `{ ... }` or `collectionName.find({ ... })`.
pub fn parse_mongo_query(input: &str, default_collection: &str) -> (String, Document) {
    let trimmed = input.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let filter = serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .and_then(|v| mongodb::bson::to_document(&v).ok())
            .unwrap_or_default();
        return (default_collection.to_string(), filter);
    }

    if let Some(dot_find) = trimmed.find(".find(") {
        let col = trimmed[..dot_find].trim();
        let rest = &trimmed[dot_find + 6..];
        let inner = rest.strip_suffix(')').unwrap_or(rest).trim();
        let filter = if inner.is_empty() {
            Document::new()
        } else {
            serde_json::from_str::<serde_json::Value>(inner)
                .ok()
                .and_then(|v| mongodb::bson::to_document(&v).ok())
                .unwrap_or_default()
        };
        if !col.is_empty() {
            return (col.to_string(), filter);
        }
    }

    (default_collection.to_string(), Document::new())
}

#[async_trait]
impl DbDriver for MongoDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let (uri, tunnel) = Self::build_uri(config).await?;
        let via = if tunnel.is_some() { " via SSH tunnel" } else { "" };
        let mut opts = ClientOptions::parse(&uri).await.context("parsing MongoDB URI")?;
        opts.server_selection_timeout = Some(Duration::from_secs(7));
        let client = mongodb::Client::with_options(opts)?;
        let db = client.database(Self::db_name(config));
        db.run_command(doc! { "ping": 1 }).await.context("MongoDB ping failed")?;
        Ok(format!(
            "Connected to MongoDB successfully ({}ms){via}",
            start.elapsed().as_millis()
        ))
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let (client, _tunnel) = self.client_for(config).await?;
        let start = Instant::now();
        let db = client.database(Self::db_name(config));

        let default_col = config
            .tables
            .first()
            .map(|t| t.name.as_str())
            .unwrap_or("documents");

        let trimmed = sql.trim().trim_end_matches(';').trim();

        // Support shell-style collection.op(...) operations: insertOne, insertMany, countDocuments, aggregate, deleteMany
        if let Some((col_part, method_call)) = trimmed.split_once('.') {
            let col_name = col_part.trim().strip_prefix("db.").unwrap_or(col_part.trim());
            let collection = db.collection::<Document>(col_name);

            if let Some(arg) = method_call.strip_prefix("insertOne(").and_then(|s| s.strip_suffix(')')) {
                let val: serde_json::Value = serde_json::from_str(arg.trim())
                    .context("invalid JSON in insertOne(...)")?;
                let doc = mongodb::bson::to_document(&val)?;
                let res = collection.insert_one(doc).await.context("MongoDB insertOne failed")?;
                let mut row = HashMap::new();
                row.insert("acknowledged".into(), serde_json::Value::Bool(true));
                row.insert("insertedId".into(), serde_json::Value::String(format!("{:?}", res.inserted_id)));
                return Ok(QueryResult {
                    columns: vec!["acknowledged".into(), "insertedId".into()],
                    rows: vec![row],
                    row_count: 1,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command: Some("INSERT_ONE".into()),
                });
            }

            if let Some(arg) = method_call.strip_prefix("insertMany(").and_then(|s| s.strip_suffix(')')) {
                let vals: Vec<serde_json::Value> = serde_json::from_str(arg.trim())
                    .context("invalid JSON array in insertMany([...])")?;
                let mut docs = Vec::with_capacity(vals.len());
                for v in vals {
                    docs.push(mongodb::bson::to_document(&v)?);
                }
                let res = collection.insert_many(docs).await.context("MongoDB insertMany failed")?;
                let count = res.inserted_ids.len() as u64;
                let mut row = HashMap::new();
                row.insert("insertedCount".into(), serde_json::Value::Number(count.into()));
                return Ok(QueryResult {
                    columns: vec!["insertedCount".into()],
                    rows: vec![row],
                    row_count: count,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command: Some("INSERT_MANY".into()),
                });
            }

            if let Some(arg) = method_call.strip_prefix("countDocuments(").and_then(|s| s.strip_suffix(')')) {
                let filter = if arg.trim().is_empty() {
                    Document::new()
                } else {
                    let val: serde_json::Value = serde_json::from_str(arg.trim())?;
                    mongodb::bson::to_document(&val)?
                };
                let count = collection.count_documents(filter).await?;
                let mut row = HashMap::new();
                row.insert("count".into(), serde_json::Value::Number(count.into()));
                return Ok(QueryResult {
                    columns: vec!["count".into()],
                    rows: vec![row],
                    row_count: 1,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command: Some("COUNT".into()),
                });
            }

            if let Some(arg) = method_call.strip_prefix("deleteMany(").and_then(|s| s.strip_suffix(')')) {
                let filter = if arg.trim().is_empty() {
                    Document::new()
                } else {
                    let val: serde_json::Value = serde_json::from_str(arg.trim())?;
                    mongodb::bson::to_document(&val)?
                };
                let res = collection.delete_many(filter).await?;
                let mut row = HashMap::new();
                row.insert("deletedCount".into(), serde_json::Value::Number(res.deleted_count.into()));
                return Ok(QueryResult {
                    columns: vec!["deletedCount".into()],
                    rows: vec![row],
                    row_count: res.deleted_count,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command: Some("DELETE_MANY".into()),
                });
            }

            if let Some(arg) = method_call.strip_prefix("aggregate(").and_then(|s| s.strip_suffix(')')) {
                let stages_json: Vec<serde_json::Value> = serde_json::from_str(arg.trim())
                    .context("invalid JSON pipeline array in aggregate([...])")?;
                let mut pipeline = Vec::with_capacity(stages_json.len());
                for s in stages_json {
                    pipeline.push(mongodb::bson::to_document(&s)?);
                }
                let mut cursor = collection.aggregate(pipeline).await.context("MongoDB aggregate failed")?;
                let mut docs = Vec::new();
                while let Some(doc) = cursor.try_next().await? {
                    docs.push(doc);
                }
                return Ok(docs_to_query_result(docs, start.elapsed().as_millis() as u64, "AGGREGATE"));
            }
        }

        let (collection_name, filter) = parse_mongo_query(trimmed, default_col);
        let col_clean = collection_name.strip_prefix("db.").unwrap_or(&collection_name);

        let collection = db.collection::<Document>(col_clean);
        let mut cursor = collection
            .find(filter)
            .limit(100)
            .await
            .context("MongoDB find failed")?;

        let mut docs = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            docs.push(doc);
        }
        Ok(docs_to_query_result(docs, start.elapsed().as_millis() as u64, "FIND"))
    }

    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        let (client, _tunnel) = self.client_for(config).await?;
        let db = client.database(Self::db_name(config));
        let names = db.list_collection_names().await?;

        let mut tables = Vec::new();
        for name in names {
            let col = db.collection::<Document>(&name);
            let count = col.estimated_document_count().await.unwrap_or(0);
            let sample = col.find_one(doc! {}).await.ok().flatten();
            let columns = match sample {
                Some(doc) => doc
                    .iter()
                    .map(|(k, v)| ColumnDefinition {
                        name: k.clone(),
                        col_type: format!("{:?}", v.element_type()).to_uppercase(),
                        is_primary_key: k == "_id",
                        is_nullable: k != "_id",
                        ..Default::default()
                    })
                    .collect(),
                None => vec![ColumnDefinition {
                    name: "_id".into(),
                    col_type: "OBJECTID".into(),
                    is_primary_key: true,
                    is_nullable: false,
                    ..Default::default()
                }],
            };
            let idx_names = col.list_index_names().await.unwrap_or_default();
            let indexes: Vec<IndexDef> = idx_names
                .into_iter()
                .map(|iname| IndexDef {
                    is_primary: iname == "_id_",
                    is_unique: iname == "_id_",
                    columns: if iname == "_id_" { vec!["_id".into()] } else { vec![] },
                    name: iname,
                })
                .collect();

            tables.push(TableSchema {
                name,
                object_type: "COLLECTION".into(),
                row_count: count,
                columns,
                foreign_keys: vec![],
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

        let (client, _tunnel) = self.client_for(config).await?;
        let db = client.database(Self::db_name(config));
        let col = db.collection::<Document>(table);

        // Apply sequentially and report atomic: false honestly when not in a replica-set transaction.
        let mut results = Vec::with_capacity(mutations.len());
        for (idx, m) in mutations.iter().enumerate() {
            let apply_res = match m.op {
                MutationOp::Insert => {
                    let mut doc = mongodb::bson::to_document(&m.values)
                        .context("serializing insert document")?;
                    doc.remove("_id");
                    col.insert_one(doc).await.map(|_| 1u64)
                }
                MutationOp::Update => {
                    let id_val = m
                        .where_clause
                        .get("_id")
                        .context("Missing _id to target MongoDB document")?;
                    let filter = doc! { "_id": to_mongo_id(id_val) };
                    let mut set_doc = mongodb::bson::to_document(&m.values)
                        .context("serializing update document")?;
                    set_doc.remove("_id");
                    col.update_one(filter, doc! { "$set": set_doc })
                        .await
                        .map(|r| r.modified_count)
                }
                MutationOp::Delete => {
                    let id_val = m
                        .where_clause
                        .get("_id")
                        .context("Missing _id to target MongoDB document")?;
                    let filter = doc! { "_id": to_mongo_id(id_val) };
                    col.delete_one(filter).await.map(|r| r.deleted_count)
                }
            };

            match apply_res {
                Ok(affected) => results.push(MutationItemResult {
                    op: m.op,
                    rows_affected: affected,
                }),
                Err(e) => {
                    anyhow::bail!(
                        "Change {} of {} failed ({e}). Applied sequentially (non-atomic), so {} prior change(s) were not rolled back.",
                        idx + 1,
                        mutations.len(),
                        idx
                    );
                }
            }
        }

        Ok(BatchMutationResult {
            atomic: false,
            results,
            execution_time_ms: start.elapsed().as_millis() as u64,
            message: Some(
                "Applied sequentially (non-atomic) on MongoDB."
                    .into(),
            ),
        })
    }
}

fn docs_to_query_result(docs: Vec<Document>, execution_time_ms: u64, cmd: &str) -> QueryResult {
    let mut key_set = BTreeSet::new();
    let mut rows = Vec::with_capacity(docs.len());
    for d in docs {
        let mut map = HashMap::new();
        for (k, v) in d {
            key_set.insert(k.clone());
            let json_val = match v {
                mongodb::bson::Bson::ObjectId(oid) => serde_json::Value::String(oid.to_hex()),
                other => serde_json::to_value(other).unwrap_or(serde_json::Value::Null),
            };
            map.insert(k, json_val);
        }
        rows.push(map);
    }
    let columns: Vec<String> = key_set.into_iter().collect();
    QueryResult {
        row_count: rows.len() as u64,
        columns,
        rows,
        execution_time_ms,
        error: None,
        notices: vec![],
        query_plan: None,
        command: Some(cmd.into()),
    }
}

fn to_mongo_id(val: &serde_json::Value) -> mongodb::bson::Bson {
    if let Some(s) = val.as_str() {
        if let Ok(oid) = mongodb::bson::oid::ObjectId::parse_str(s) {
            return mongodb::bson::Bson::ObjectId(oid);
        }
    }
    mongodb::bson::to_bson(val).unwrap_or(mongodb::bson::Bson::Null)
}
