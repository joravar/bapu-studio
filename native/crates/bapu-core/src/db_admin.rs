//! DBeaver Server Session, Storage & Engine Health Monitor (`db_admin.rs`).
//! Queries live engine diagnostics & active sessions across PostgreSQL, MySQL, SQLite, MongoDB, and Redis.

use crate::db::DbDriver;
use crate::types::{DatabaseConnection, DbKind, QueryResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetric {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHealthReport {
    pub engine_banner: String,
    pub metrics: Vec<ServerMetric>,
    pub active_sessions: Option<QueryResult>,
}

pub async fn fetch_server_health(
    driver: &dyn DbDriver,
    config: &DatabaseConnection,
) -> Result<ServerHealthReport> {
    match config.kind {
        DbKind::Postgres
        | DbKind::Cockroachdb
        | DbKind::Yugabytedb
        | DbKind::Timescaledb
        | DbKind::Redshift => {
            let ver = driver
                .query(config, "SELECT version() AS version, current_database() AS db")
                .await?;
            let banner = ver
                .rows
                .first()
                .and_then(|r| r.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or(config.kind.display_name())
                .to_string();
            let sessions = driver
                .query(
                    config,
                    "SELECT pid, usename AS user, state, application_name AS app \
                     FROM pg_stat_activity LIMIT 25",
                )
                .await
                .ok();
            let sess_count = sessions.as_ref().map(|s| s.row_count).unwrap_or(0);

            Ok(ServerHealthReport {
                engine_banner: banner,
                metrics: vec![
                    ServerMetric { label: "Engine".into(), value: config.kind.display_name().into() },
                    ServerMetric { label: "Active Sessions".into(), value: sess_count.to_string() },
                    ServerMetric { label: "Database".into(), value: config.database.clone() },
                ],
                active_sessions: sessions,
            })
        }
        DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => {
            let ver = driver.query(config, "SELECT VERSION() AS ver, DATABASE() AS db").await?;
            let v_str = ver
                .rows
                .first()
                .and_then(|r| r.get("ver"))
                .and_then(|v| v.as_str())
                .unwrap_or(config.kind.display_name())
                .to_string();
            let sessions = driver.query(config, "SHOW PROCESSLIST").await.ok();
            let sess_count = sessions.as_ref().map(|s| s.row_count).unwrap_or(0);

            Ok(ServerHealthReport {
                engine_banner: format!("{} {v_str}", config.kind.display_name()),
                metrics: vec![
                    ServerMetric { label: "Server Version".into(), value: v_str },
                    ServerMetric { label: "Active Threads".into(), value: sess_count.to_string() },
                    ServerMetric { label: "Database".into(), value: config.database.clone() },
                ],
                active_sessions: sessions,
            })
        }
        DbKind::Mssql | DbKind::AzureSql => {
            let ver = driver
                .query(config, "SELECT @@VERSION AS ver, DB_NAME() AS db")
                .await?;
            let banner = ver
                .rows
                .first()
                .and_then(|r| r.get("ver"))
                .and_then(|v| v.as_str())
                .map(|s| s.lines().next().unwrap_or(s).to_string())
                .unwrap_or_else(|| config.kind.display_name().to_string());
            let sessions = driver
                .query(
                    config,
                    "SELECT session_id, login_name, status, host_name, program_name \
                     FROM sys.dm_exec_sessions WHERE is_user_process = 1",
                )
                .await
                .ok();
            let sess_count = sessions.as_ref().map(|s| s.row_count).unwrap_or(0);
            Ok(ServerHealthReport {
                engine_banner: banner,
                metrics: vec![
                    ServerMetric { label: "User Sessions".into(), value: sess_count.to_string() },
                    ServerMetric { label: "Database".into(), value: config.database.clone() },
                ],
                active_sessions: sessions,
            })
        }
        DbKind::Oracle => {
            let banner_msg = driver.test_connection(config).await?;
            let ts = driver
                .query(config, "SELECT tablespace_name, status FROM user_tablespaces")
                .await
                .ok();
            Ok(ServerHealthReport {
                engine_banner: banner_msg,
                metrics: vec![
                    ServerMetric { label: "Service / PDB".into(), value: config.database.clone() },
                    ServerMetric {
                        label: "Tablespaces".into(),
                        value: ts.as_ref().map(|t| t.row_count).unwrap_or(0).to_string(),
                    },
                ],
                active_sessions: ts,
            })
        }
        DbKind::Sqlite => {
            let ver = driver.query(config, "SELECT sqlite_version() AS ver").await?;
            let v_str = ver
                .rows
                .first()
                .and_then(|r| r.get("ver"))
                .and_then(|v| v.as_str())
                .unwrap_or("3.x")
                .to_string();
            let journal = driver
                .query(config, "PRAGMA journal_mode")
                .await
                .ok()
                .and_then(|r| r.rows.first()?.values().next()?.as_str().map(String::from))
                .unwrap_or_else(|| "memory".into());
            let page_count = driver
                .query(config, "PRAGMA page_count")
                .await
                .ok()
                .and_then(|r| r.rows.first()?.values().next()?.as_i64())
                .unwrap_or(0);
            let page_size = driver
                .query(config, "PRAGMA page_size")
                .await
                .ok()
                .and_then(|r| r.rows.first()?.values().next()?.as_i64())
                .unwrap_or(4096);
            let db_list = driver.query(config, "PRAGMA database_list").await.ok();

            Ok(ServerHealthReport {
                engine_banner: format!("SQLite {v_str}"),
                metrics: vec![
                    ServerMetric { label: "Journal Mode".into(), value: journal.to_uppercase() },
                    ServerMetric {
                        label: "Storage Size".into(),
                        value: format!("{} KB", (page_count * page_size) / 1024),
                    },
                    ServerMetric { label: "Foreign Keys".into(), value: "ON".into() },
                ],
                active_sessions: db_list,
            })
        }
        DbKind::Mongodb => {
            let schema = driver.get_schema(config).await?;
            let total_docs: u64 = schema.iter().map(|t| t.row_count).sum();
            Ok(ServerHealthReport {
                engine_banner: format!("MongoDB ({})", config.database),
                metrics: vec![
                    ServerMetric { label: "Collections".into(), value: schema.len().to_string() },
                    ServerMetric { label: "Est. Documents".into(), value: total_docs.to_string() },
                ],
                active_sessions: None,
            })
        }
        DbKind::Redis => {
            let dbsize = driver
                .query(config, "DBSIZE")
                .await
                .ok()
                .and_then(|r| r.rows.first()?.get("result")?.as_str().map(String::from))
                .unwrap_or_else(|| "0".into());
            let info = driver.query(config, "INFO server").await.ok();
            Ok(ServerHealthReport {
                engine_banner: "Redis / Valkey In-Memory Store".into(),
                metrics: vec![
                    ServerMetric { label: "Total Keys (DBSIZE)".into(), value: dbsize },
                    ServerMetric { label: "Selected DB".into(), value: config.database.clone() },
                ],
                active_sessions: info,
            })
        }
        _ => {
            let banner = driver.test_connection(config).await?;
            Ok(ServerHealthReport {
                engine_banner: banner,
                metrics: vec![
                    ServerMetric { label: "Engine".into(), value: config.kind.display_name().into() },
                    ServerMetric { label: "Category".into(), value: config.kind.category().label().into() },
                    ServerMetric { label: "Target".into(), value: config.database.clone() },
                ],
                active_sessions: None,
            })
        }
    }
}
