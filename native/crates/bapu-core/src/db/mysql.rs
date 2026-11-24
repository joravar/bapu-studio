//! STUB -- MySQL driver not yet ported from the Electron app's electron-main.cjs IPC handler.
//! Returns a clear error instead of silently pretending to work. Porting this is tracked as
//! follow-up scope, not forgotten.

use super::DbDriver;
use crate::types::{DatabaseConnection, QueryResult, TableSchema};
use anyhow::{bail, Result};
use async_trait::async_trait;

pub struct MysqlDriver;

#[async_trait]
impl DbDriver for MysqlDriver {
    async fn test_connection(&self, _config: &DatabaseConnection) -> Result<String> {
        bail!("MySQL support is not yet implemented in the native Rust rewrite (tracked follow-up work)")
    }

    async fn query(&self, _config: &DatabaseConnection, _sql: &str) -> Result<QueryResult> {
        bail!("MySQL support is not yet implemented in the native Rust rewrite (tracked follow-up work)")
    }

    async fn get_schema(&self, _config: &DatabaseConnection) -> Result<Vec<TableSchema>> {
        bail!("MySQL support is not yet implemented in the native Rust rewrite (tracked follow-up work)")
    }
}
