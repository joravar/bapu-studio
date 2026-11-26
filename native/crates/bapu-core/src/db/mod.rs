//! Universal Database Driver Router supporting all 33 DBeaver-class relational,
//! analytical/OLAP, cloud data warehouse, NoSQL, wide-column, graph, search, time-series,
//! and enterprise ODBC database engines.

pub mod cassandra_driver;
pub mod http_engines;
mod mongodb_driver;
pub mod mssql;
mod mysql;
pub mod odbc_driver;
pub mod oracle_driver;
mod postgres;
mod redis_driver;
mod sqlite;
pub mod ssh_tunnel;

pub use postgres::PostgresDriver;

use crate::types::{
    BatchMutationResult, DatabaseConnection, DbKind, QueryResult, RowMutation, TableSchema,
};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait DbDriver: Send + Sync {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String>;
    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult>;
    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>>;
    async fn mutate_batch(
        &self,
        _config: &DatabaseConnection,
        _table: &str,
        _mutations: &[RowMutation],
    ) -> Result<BatchMutationResult> {
        anyhow::bail!("Batch row editing is not supported for this driver.")
    }
}

pub fn driver_for(kind: DbKind) -> Box<dyn DbDriver> {
    match kind {
        // PostgreSQL wire-compatible engines
        DbKind::Postgres
        | DbKind::Cockroachdb
        | DbKind::Yugabytedb
        | DbKind::Timescaledb
        | DbKind::Redshift => Box::new(postgres::PostgresDriver::default()),

        // MySQL / MariaDB wire-compatible engines
        DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => Box::new(mysql::MysqlDriver::default()),

        // Embedded SQLite
        DbKind::Sqlite => Box::new(sqlite::SqliteDriver::default()),

        // Microsoft SQL Server & Azure SQL (TDS 7.3 / 8.0)
        DbKind::Mssql | DbKind::AzureSql => Box::new(mssql::MssqlDriver),

        // Oracle Database (Pure-Rust Oracle Net TNS)
        DbKind::Oracle => Box::new(oracle_driver::OracleDriver),

        // Apache Cassandra & ScyllaDB (Pure-Rust CQL v4/v5)
        DbKind::Cassandra | DbKind::Scylladb => Box::new(cassandra_driver::CassandraDriver),

        // MongoDB / DocumentDB (BSON wire protocol)
        DbKind::Mongodb => Box::new(mongodb_driver::MongoDriver::default()),

        // Redis / Valkey / Dragonfly (RESP2/RESP3 wire protocol)
        DbKind::Redis => Box::new(redis_driver::RedisDriver),

        // Native HTTP / Cloud Warehouse / OLAP / Graph / Search / Time-Series protocols
        DbKind::Clickhouse
        | DbKind::Trino
        | DbKind::Libsql
        | DbKind::Questdb
        | DbKind::Snowflake
        | DbKind::Bigquery
        | DbKind::Databricks
        | DbKind::Elasticsearch
        | DbKind::Neo4j
        | DbKind::Influxdb
        | DbKind::Couchdb => Box::new(http_engines::HttpEngineDriver::new(kind)),

        // Enterprise RDBMS & Universal ODBC (IBM Db2, SAP HANA, Teradata, DuckDB, Firebird, Generic ODBC)
        DbKind::Db2
        | DbKind::SapHana
        | DbKind::Teradata
        | DbKind::Duckdb
        | DbKind::Firebird
        | DbKind::Odbc => Box::new(odbc_driver::OdbcDriver),
    }
}
