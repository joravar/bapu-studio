//! Universal Enterprise ODBC Driver.
//! - On Windows: uses `odbc-api` backed by the built-in OS `odbc32.dll`.
//! - On macOS & Linux: dynamically loads `libodbc` / `libiodbc` at runtime via `libloading`
//!   (`dlopen`) so Bapu Studio has zero compile-time `-lodbc` link dependency, launches on every
//!   Mac/Linux machine without requiring Homebrew `unixodbc` at startup, and passes Apple
//!   Hardened Runtime & Gatekeeper Notarization cleanly.

use super::DbDriver;
use crate::types::{
    BatchMutationResult, ColumnDefinition, DatabaseConnection, DbKind, MutationOp, QueryResult,
    RowMutation, TableSchema,
};
use anyhow::{Context, Result};
use async_trait::async_trait;

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

pub(crate) fn parse_text_scalar(bytes: &[u8]) -> serde_json::Value {
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

fn odbc_sql_lit(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "NULL".into(),
        serde_json::Value::Bool(b) => {
            if *b {
                "1".into()
            } else {
                "0".into()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn quote_std(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

pub(crate) fn build_odbc_literal_mutation(table: &str, m: &RowMutation) -> Result<String> {
    let qt = quote_std(table);
    match m.op {
        MutationOp::Insert => {
            let mut cols: Vec<(&String, &serde_json::Value)> = m.values.iter().collect();
            cols.sort_by_key(|(k, _)| *k);
            let cl = cols
                .iter()
                .map(|(k, _)| quote_std(k))
                .collect::<Vec<_>>()
                .join(", ");
            let vl = cols
                .iter()
                .map(|(_, v)| odbc_sql_lit(v))
                .collect::<Vec<_>>()
                .join(", ");
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

#[cfg(windows)]
mod imp {
    use super::{build_odbc_connection_string, build_odbc_literal_mutation, parse_text_scalar};
    use crate::types::{
        BatchMutationResult, DatabaseConnection, MutationItemResult, QueryResult, RowMutation,
    };
    use anyhow::{Context, Result};
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

    pub fn test_conn_sync(cfg: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let env = odbc_env()?;
        let cs = build_odbc_connection_string(cfg);
        let conn = env
            .connect_with_connection_string(&cs, ConnectionOptions::default())
            .context("ODBC connect failed")?;
        let dbms = conn
            .database_management_system_name()
            .unwrap_or_else(|_| cfg.kind.display_name().to_string());
        Ok(format!(
            "Connected to {dbms} via ODBC ({}ms)",
            start.elapsed().as_millis()
        ))
    }

    pub fn query_sync(cfg: &DatabaseConnection, sql_owned: &str) -> Result<QueryResult> {
        let start = Instant::now();
        let env = odbc_env()?;
        let cs = build_odbc_connection_string(cfg);
        let conn = env
            .connect_with_connection_string(&cs, ConnectionOptions::default())
            .context("ODBC connect failed")?;
        let command = sql_owned
            .split_whitespace()
            .next()
            .map(|s| s.to_ascii_uppercase());

        let out = match conn
            .execute(sql_owned, (), None)
            .context("ODBC execute failed")?
        {
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
    }

    pub fn mutate_sync(
        cfg: &DatabaseConnection,
        tbl: &str,
        muts: &[RowMutation],
    ) -> Result<BatchMutationResult> {
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
        let cs = build_odbc_connection_string(cfg);
        let conn = env
            .connect_with_connection_string(&cs, ConnectionOptions::default())
            .context("ODBC connect failed")?;
        let _ = conn.set_autocommit(false);

        let mut results = Vec::with_capacity(muts.len());
        for m in muts {
            let sql = build_odbc_literal_mutation(tbl, m)?;
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
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{build_odbc_connection_string, build_odbc_literal_mutation, parse_text_scalar};
    use crate::types::{
        BatchMutationResult, DatabaseConnection, MutationItemResult, QueryResult, RowMutation,
    };
    use anyhow::{bail, Result};
    use libloading::{Library, Symbol};
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::sync::OnceLock;
    use std::time::Instant;

    type SqlHandle = *mut c_void;
    type SqlReturn = i16;

    const SQL_HANDLE_ENV: i16 = 1;
    const SQL_HANDLE_DBC: i16 = 2;
    const SQL_HANDLE_STMT: i16 = 3;
    const SQL_ATTR_ODBC_VERSION: i32 = 200;
    const SQL_OV_ODBC3: usize = 3;
    const SQL_ATTR_AUTOCOMMIT: i32 = 102;
    const SQL_AUTOCOMMIT_OFF: usize = 0;
    const SQL_DRIVER_NOPROMPT: u16 = 0;
    const SQL_C_CHAR: i16 = 1;
    const SQL_NO_DATA: SqlReturn = 100;
    const SQL_NULL_DATA: isize = -1;
    const SQL_COMMIT: i16 = 0;
    const SQL_ROLLBACK: i16 = 1;

    type FnAllocHandle = unsafe extern "C" fn(i16, SqlHandle, *mut SqlHandle) -> SqlReturn;
    type FnFreeHandle = unsafe extern "C" fn(i16, SqlHandle) -> SqlReturn;
    type FnSetEnvAttr = unsafe extern "C" fn(SqlHandle, i32, *mut c_void, i32) -> SqlReturn;
    type FnSetConnectAttr = unsafe extern "C" fn(SqlHandle, i32, *mut c_void, i32) -> SqlReturn;
    type FnDriverConnect = unsafe extern "C" fn(
        SqlHandle,
        *mut c_void,
        *const u8,
        i16,
        *mut u8,
        i16,
        *mut i16,
        u16,
    ) -> SqlReturn;
    type FnDisconnect = unsafe extern "C" fn(SqlHandle) -> SqlReturn;
    type FnExecDirect = unsafe extern "C" fn(SqlHandle, *const u8, i32) -> SqlReturn;
    type FnNumResultCols = unsafe extern "C" fn(SqlHandle, *mut i16) -> SqlReturn;
    type FnDescribeCol = unsafe extern "C" fn(
        SqlHandle,
        u16,
        *mut u8,
        i16,
        *mut i16,
        *mut i16,
        *mut usize,
        *mut i16,
        *mut i16,
    ) -> SqlReturn;
    type FnFetch = unsafe extern "C" fn(SqlHandle) -> SqlReturn;
    type FnGetData =
        unsafe extern "C" fn(SqlHandle, u16, i16, *mut c_void, isize, *mut isize) -> SqlReturn;
    type FnEndTran = unsafe extern "C" fn(i16, SqlHandle, i16) -> SqlReturn;
    type FnGetDiagRec = unsafe extern "C" fn(
        i16,
        SqlHandle,
        i16,
        *mut u8,
        *mut i32,
        *mut u8,
        i16,
        *mut i16,
    ) -> SqlReturn;

    struct DynOdbc {
        _lib: Library,
        alloc_handle: FnAllocHandle,
        free_handle: FnFreeHandle,
        set_env_attr: FnSetEnvAttr,
        set_connect_attr: FnSetConnectAttr,
        driver_connect: FnDriverConnect,
        disconnect: FnDisconnect,
        exec_direct: FnExecDirect,
        num_result_cols: FnNumResultCols,
        describe_col: FnDescribeCol,
        fetch: FnFetch,
        get_data: FnGetData,
        end_tran: FnEndTran,
        get_diag_rec: FnGetDiagRec,
    }

    // SAFETY: `DynOdbc` holds an immutable `Library` handle and plain C function pointers.
    unsafe impl Send for DynOdbc {}
    unsafe impl Sync for DynOdbc {}

    static DYN_ODBC: OnceLock<Result<DynOdbc, String>> = OnceLock::new();

    fn load_odbc() -> Result<&'static DynOdbc> {
        let res = DYN_ODBC.get_or_init(|| {
            let candidates = [
                "libodbc.2.dylib",
                "libodbc.dylib",
                "/opt/homebrew/lib/libodbc.dylib",
                "/usr/local/lib/libodbc.dylib",
                "libiodbc.2.dylib",
                "libiodbc.dylib",
                "libodbc.so.2",
                "libodbc.so",
            ];
            let lib = candidates
                .iter()
                .find_map(|name| unsafe { Library::new(name).ok() })
                .ok_or_else(|| {
                    "No ODBC Driver Manager found on this system. Install unixODBC (`brew install unixodbc` on macOS or `sudo apt install unixodbc` on Linux) to use ODBC drivers.".to_string()
                })?;

            unsafe {
                let alloc_handle = *lib
                    .get::<FnAllocHandle>(b"SQLAllocHandle\0")
                    .map_err(|e| e.to_string())?;
                let free_handle = *lib
                    .get::<FnFreeHandle>(b"SQLFreeHandle\0")
                    .map_err(|e| e.to_string())?;
                let set_env_attr = *lib
                    .get::<FnSetEnvAttr>(b"SQLSetEnvAttr\0")
                    .map_err(|e| e.to_string())?;
                let set_connect_attr = *lib
                    .get::<FnSetConnectAttr>(b"SQLSetConnectAttr\0")
                    .map_err(|e| e.to_string())?;
                let driver_connect = *lib
                    .get::<FnDriverConnect>(b"SQLDriverConnect\0")
                    .map_err(|e| e.to_string())?;
                let disconnect = *lib
                    .get::<FnDisconnect>(b"SQLDisconnect\0")
                    .map_err(|e| e.to_string())?;
                let exec_direct = *lib
                    .get::<FnExecDirect>(b"SQLExecDirect\0")
                    .map_err(|e| e.to_string())?;
                let num_result_cols = *lib
                    .get::<FnNumResultCols>(b"SQLNumResultCols\0")
                    .map_err(|e| e.to_string())?;
                let describe_col = *lib
                    .get::<FnDescribeCol>(b"SQLDescribeCol\0")
                    .map_err(|e| e.to_string())?;
                let fetch = *lib
                    .get::<FnFetch>(b"SQLFetch\0")
                    .map_err(|e| e.to_string())?;
                let get_data = *lib
                    .get::<FnGetData>(b"SQLGetData\0")
                    .map_err(|e| e.to_string())?;
                let end_tran = *lib
                    .get::<FnEndTran>(b"SQLEndTran\0")
                    .map_err(|e| e.to_string())?;
                let get_diag_rec: Symbol<FnGetDiagRec> = lib
                    .get(b"SQLGetDiagRec\0")
                    .map_err(|e| e.to_string())?;
                let get_diag_rec = *get_diag_rec;

                Ok(DynOdbc {
                    _lib: lib,
                    alloc_handle,
                    free_handle,
                    set_env_attr,
                    set_connect_attr,
                    driver_connect,
                    disconnect,
                    exec_direct,
                    num_result_cols,
                    describe_col,
                    fetch,
                    get_data,
                    end_tran,
                    get_diag_rec,
                })
            }
        });
        match res {
            Ok(api) => Ok(api),
            Err(msg) => bail!("{msg}"),
        }
    }

    fn sql_ok(rc: SqlReturn) -> bool {
        rc == 0 || rc == 1
    }

    unsafe fn diag_message(api: &DynOdbc, handle_type: i16, handle: SqlHandle) -> String {
        let mut state = [0u8; 6];
        let mut native_err = 0i32;
        let mut msg = vec![0u8; 1024];
        let mut msg_len = 0i16;
        let rc = (api.get_diag_rec)(
            handle_type,
            handle,
            1,
            state.as_mut_ptr(),
            &mut native_err,
            msg.as_mut_ptr(),
            msg.len() as i16,
            &mut msg_len,
        );
        if sql_ok(rc) && msg_len > 0 {
            let n = (msg_len as usize).min(msg.len());
            format!(
                "[{}] {}",
                String::from_utf8_lossy(&state[..5]),
                String::from_utf8_lossy(&msg[..n])
            )
        } else {
            format!("ODBC error (code {rc})")
        }
    }

    struct ConnGuard<'a> {
        api: &'a DynOdbc,
        env: SqlHandle,
        dbc: SqlHandle,
        connected: bool,
    }

    impl<'a> ConnGuard<'a> {
        fn open(api: &'a DynOdbc, cs: &str) -> Result<Self> {
            unsafe {
                let mut env: SqlHandle = std::ptr::null_mut();
                if !sql_ok((api.alloc_handle)(
                    SQL_HANDLE_ENV,
                    std::ptr::null_mut(),
                    &mut env,
                )) {
                    bail!("SQLAllocHandle(ENV) failed");
                }
                let _ = (api.set_env_attr)(
                    env,
                    SQL_ATTR_ODBC_VERSION,
                    SQL_OV_ODBC3 as *mut c_void,
                    0,
                );
                let mut dbc: SqlHandle = std::ptr::null_mut();
                if !sql_ok((api.alloc_handle)(SQL_HANDLE_DBC, env, &mut dbc)) {
                    let _ = (api.free_handle)(SQL_HANDLE_ENV, env);
                    bail!("SQLAllocHandle(DBC) failed");
                }
                let mut out_buf = [0u8; 1024];
                let mut out_len = 0i16;
                let rc = (api.driver_connect)(
                    dbc,
                    std::ptr::null_mut(),
                    cs.as_ptr(),
                    cs.len() as i16,
                    out_buf.as_mut_ptr(),
                    out_buf.len() as i16,
                    &mut out_len,
                    SQL_DRIVER_NOPROMPT,
                );
                if !sql_ok(rc) {
                    let err = diag_message(api, SQL_HANDLE_DBC, dbc);
                    let _ = (api.free_handle)(SQL_HANDLE_DBC, dbc);
                    let _ = (api.free_handle)(SQL_HANDLE_ENV, env);
                    bail!("ODBC connect failed: {err}");
                }
                Ok(Self {
                    api,
                    env,
                    dbc,
                    connected: true,
                })
            }
        }
    }

    impl Drop for ConnGuard<'_> {
        fn drop(&mut self) {
            unsafe {
                if self.connected {
                    let _ = (self.api.disconnect)(self.dbc);
                }
                let _ = (self.api.free_handle)(SQL_HANDLE_DBC, self.dbc);
                let _ = (self.api.free_handle)(SQL_HANDLE_ENV, self.env);
            }
        }
    }

    pub fn test_conn_sync(cfg: &DatabaseConnection) -> Result<String> {
        let start = Instant::now();
        let api = load_odbc()?;
        let cs = build_odbc_connection_string(cfg);
        let _guard = ConnGuard::open(api, &cs)?;
        Ok(format!(
            "Connected to {} via ODBC ({}ms)",
            cfg.kind.display_name(),
            start.elapsed().as_millis()
        ))
    }

    pub fn query_sync(cfg: &DatabaseConnection, sql_owned: &str) -> Result<QueryResult> {
        let start = Instant::now();
        let api = load_odbc()?;
        let cs = build_odbc_connection_string(cfg);
        let guard = ConnGuard::open(api, &cs)?;
        let command = sql_owned
            .split_whitespace()
            .next()
            .map(|s| s.to_ascii_uppercase());

        unsafe {
            let mut stmt: SqlHandle = std::ptr::null_mut();
            if !sql_ok((api.alloc_handle)(SQL_HANDLE_STMT, guard.dbc, &mut stmt)) {
                bail!("SQLAllocHandle(STMT) failed");
            }
            let rc = (api.exec_direct)(stmt, sql_owned.as_ptr(), sql_owned.len() as i32);
            if !sql_ok(rc) && rc != SQL_NO_DATA {
                let err = diag_message(api, SQL_HANDLE_STMT, stmt);
                let _ = (api.free_handle)(SQL_HANDLE_STMT, stmt);
                bail!("ODBC execute failed: {err}");
            }

            let mut num_cols = 0i16;
            let _ = (api.num_result_cols)(stmt, &mut num_cols);
            if num_cols <= 0 {
                let _ = (api.free_handle)(SQL_HANDLE_STMT, stmt);
                let mut status = HashMap::new();
                status.insert("status".into(), serde_json::json!("OK"));
                status.insert("message".into(), serde_json::json!("ODBC statement executed."));
                return Ok(QueryResult {
                    columns: vec!["status".into(), "message".into()],
                    rows: vec![status],
                    row_count: 0,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    notices: vec![],
                    query_plan: None,
                    command,
                });
            }

            let mut columns = Vec::with_capacity(num_cols as usize);
            for col_i in 1..=(num_cols as u16) {
                let mut name_buf = [0u8; 256];
                let mut name_len = 0i16;
                let mut data_type = 0i16;
                let mut col_size = 0usize;
                let mut dec_digits = 0i16;
                let mut nullable = 0i16;
                let _ = (api.describe_col)(
                    stmt,
                    col_i,
                    name_buf.as_mut_ptr(),
                    name_buf.len() as i16,
                    &mut name_len,
                    &mut data_type,
                    &mut col_size,
                    &mut dec_digits,
                    &mut nullable,
                );
                let n = (name_len.max(0) as usize).min(name_buf.len());
                columns.push(String::from_utf8_lossy(&name_buf[..n]).into_owned());
            }

            let mut rows = Vec::new();
            let mut val_buf = vec![0u8; 16384];
            loop {
                let frc = (api.fetch)(stmt);
                if frc == SQL_NO_DATA || !sql_ok(frc) {
                    break;
                }
                let mut row_map = HashMap::new();
                for (idx, col_name) in columns.iter().enumerate() {
                    let mut ind: isize = 0;
                    let grc = (api.get_data)(
                        stmt,
                        (idx + 1) as u16,
                        SQL_C_CHAR,
                        val_buf.as_mut_ptr() as *mut c_void,
                        val_buf.len() as isize,
                        &mut ind,
                    );
                    let jv = if !sql_ok(grc) || ind == SQL_NULL_DATA {
                        serde_json::Value::Null
                    } else {
                        let byte_len = (ind.max(0) as usize).min(val_buf.len().saturating_sub(1));
                        parse_text_scalar(&val_buf[..byte_len])
                    };
                    row_map.insert(col_name.clone(), jv);
                }
                rows.push(row_map);
            }
            let _ = (api.free_handle)(SQL_HANDLE_STMT, stmt);

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
    }

    pub fn mutate_sync(
        cfg: &DatabaseConnection,
        tbl: &str,
        muts: &[RowMutation],
    ) -> Result<BatchMutationResult> {
        let start = Instant::now();
        if muts.is_empty() {
            return Ok(BatchMutationResult {
                atomic: true,
                results: vec![],
                execution_time_ms: 0,
                message: None,
            });
        }
        let api = load_odbc()?;
        let cs = build_odbc_connection_string(cfg);
        let guard = ConnGuard::open(api, &cs)?;

        unsafe {
            let _ = (api.set_connect_attr)(
                guard.dbc,
                SQL_ATTR_AUTOCOMMIT,
                SQL_AUTOCOMMIT_OFF as *mut c_void,
                0,
            );
            let mut results = Vec::with_capacity(muts.len());
            for m in muts {
                let sql = build_odbc_literal_mutation(tbl, m)?;
                let mut stmt: SqlHandle = std::ptr::null_mut();
                if !sql_ok((api.alloc_handle)(SQL_HANDLE_STMT, guard.dbc, &mut stmt)) {
                    let _ = (api.end_tran)(SQL_HANDLE_DBC, guard.dbc, SQL_ROLLBACK);
                    bail!("SQLAllocHandle(STMT) failed");
                }
                let rc = (api.exec_direct)(stmt, sql.as_ptr(), sql.len() as i32);
                if !sql_ok(rc) && rc != SQL_NO_DATA {
                    let err = diag_message(api, SQL_HANDLE_STMT, stmt);
                    let _ = (api.free_handle)(SQL_HANDLE_STMT, stmt);
                    let _ = (api.end_tran)(SQL_HANDLE_DBC, guard.dbc, SQL_ROLLBACK);
                    bail!("ODBC transaction rolled back: {err}");
                }
                let _ = (api.free_handle)(SQL_HANDLE_STMT, stmt);
                results.push(MutationItemResult {
                    op: m.op,
                    rows_affected: 1,
                });
            }
            if !sql_ok((api.end_tran)(SQL_HANDLE_DBC, guard.dbc, SQL_COMMIT)) {
                bail!("ODBC commit failed");
            }
            Ok(BatchMutationResult {
                atomic: true,
                results,
                execution_time_ms: start.elapsed().as_millis() as u64,
                message: None,
            })
        }
    }
}

#[async_trait]
impl DbDriver for OdbcDriver {
    async fn test_connection(&self, config: &DatabaseConnection) -> Result<String> {
        let cfg = config.clone();
        tokio::task::spawn_blocking(move || imp::test_conn_sync(&cfg))
            .await
            .context("joining ODBC task")?
    }

    async fn query(&self, config: &DatabaseConnection, sql: &str) -> Result<QueryResult> {
        let cfg = config.clone();
        let sql_owned = sql.to_string();
        tokio::task::spawn_blocking(move || imp::query_sync(&cfg, &sql_owned))
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
        tokio::task::spawn_blocking(move || imp::mutate_sync(&cfg, &tbl, &muts))
            .await
            .context("joining ODBC mutate task")?
    }
}
