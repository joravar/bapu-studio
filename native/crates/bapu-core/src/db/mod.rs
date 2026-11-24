//! Database driver abstraction. `postgres.rs` is a real, working first slice (connect, run a
//! query, introspect schema). mysql/sqlite/mongodb/redis are stubs that return a clear
//! "not yet ported" error rather than silently doing nothing -- porting each of these from the
//! Electron app's `electron-main.cjs` IPC handlers is tracked work, not forgotten scope.

mod mysql;
mod postgres;
mod mongodb_driver;
mod redis_driver;
mod sqlite;

pub use postgres::PostgresDriver;

use crate::types::{DatabaseConnection, DbKind, QueryResult, TableSchema};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait DbDriver: Send + Sync {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String>;
    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult>;
    async fn get_schema(&self, config: &DatabaseConnection) -> Result<Vec<TableSchema>>;
}

pub fn driver_for(kind: DbKind) -> Box<dyn DbDriver> {
    match kind {
        DbKind::Postgres => Box::new(postgres::PostgresDriver::default()),
        DbKind::Mysql => Box::new(mysql::MysqlDriver),
        DbKind::Sqlite => Box::new(sqlite::SqliteDriver),
        DbKind::Mongodb => Box::new(mongodb_driver::MongoDriver),
        DbKind::Redis => Box::new(redis_driver::RedisDriver),
    }
}
