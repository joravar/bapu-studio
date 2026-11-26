//! SQL Power Editor tools:
//! - Lexer-grade multi-statement SQL script splitter (`split_sql_statements`)
//! - SQL Formatter / Beautifier (`format_sql`)
//! - Named parameter extractor & safe literal binder (`:param` and `${param}`)
//! - Sequential multi-statement script runner (`execute_sql_script`)
//! - Instant result-set Column Statistics (`compute_column_stats`)

use crate::db::DbDriver;
use crate::types::{DatabaseConnection, DbKind, QueryResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementRunItem {
    pub index: usize,
    pub sql_preview: String,
    pub row_count: u64,
    pub elapsed_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionReport {
    pub total_statements: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_rows_affected: u64,
    pub total_elapsed_ms: u64,
    pub items: Vec<StatementRunItem>,
    pub last_result: Option<QueryResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub column: String,
    pub total_rows: usize,
    pub non_null: usize,
    pub null_count: usize,
    pub distinct_count: usize,
    pub min: Option<String>,
    pub max: Option<String>,
    pub sum: Option<f64>,
    pub avg: Option<f64>,
}

/// Splits a multi-statement SQL script on `;`, respecting single quotes, double quotes,
/// backticks, `--` line comments, `/* ... */` block comments, and Postgres `$$...$$` dollar blocks.
pub fn split_sql_statements(script: &str) -> Vec<String> {
    let mut stmts = Vec::new();
    let mut cur = String::new();
    let bytes = script.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut dollar_tag: Option<String> = None;

    while let Some(ch) = script[i..].chars().next() {
        let ch_len = ch.len_utf8();
        let next = if i + ch_len < n {
            script[i + ch_len..].chars().next()
        } else {
            None
        };

        if in_line_comment {
            cur.push(ch);
            if ch == '\n' {
                in_line_comment = false;
            }
            i += ch_len;
            continue;
        }

        if in_block_comment {
            cur.push(ch);
            if ch == '*' && next == Some('/') {
                cur.push('/');
                i += 2;
                in_block_comment = false;
                continue;
            }
            i += ch_len;
            continue;
        }

        if let Some(ref tag) = dollar_tag {
            if script[i..].starts_with(tag.as_str()) {
                cur.push_str(tag);
                i += tag.len();
                dollar_tag = None;
                continue;
            }
            cur.push(ch);
            i += ch_len;
            continue;
        }

        if in_single {
            cur.push(ch);
            if ch == '\'' {
                if next == Some('\'') {
                    cur.push('\'');
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += ch_len;
            continue;
        }

        if in_double {
            cur.push(ch);
            if ch == '"' {
                if next == Some('"') {
                    cur.push('"');
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += ch_len;
            continue;
        }

        if in_backtick {
            cur.push(ch);
            if ch == '`' {
                in_backtick = false;
            }
            i += ch_len;
            continue;
        }

        // Outside all quotes & comments:
        if ch == '-' && next == Some('-') {
            cur.push_str("--");
            i += 2;
            in_line_comment = true;
            continue;
        }
        if ch == '/' && next == Some('*') {
            cur.push_str("/*");
            i += 2;
            in_block_comment = true;
            continue;
        }
        if ch == '$' {
            if let Some(tag) = match_dollar_quote(&script[i..]) {
                cur.push_str(&tag);
                i += tag.len();
                dollar_tag = Some(tag);
                continue;
            }
        }
        if ch == '\'' {
            in_single = true;
            cur.push(ch);
            i += 1;
            continue;
        }
        if ch == '"' {
            in_double = true;
            cur.push(ch);
            i += 1;
            continue;
        }
        if ch == '`' {
            in_backtick = true;
            cur.push(ch);
            i += 1;
            continue;
        }

        if ch == ';' {
            let cleaned = strip_leading_empty_and_comments(&cur);
            if !cleaned.is_empty() {
                stmts.push(cur.trim().to_string());
            }
            cur.clear();
            i += 1;
            continue;
        }

        cur.push(ch);
        i += ch_len;
    }

    if !strip_leading_empty_and_comments(&cur).is_empty() {
        stmts.push(cur.trim().to_string());
    }
    stmts
}

fn match_dollar_quote(s: &str) -> Option<String> {
    let rest = s.strip_prefix('$')?;
    let end = rest.find('$')?;
    let tag_inner = &rest[..end];
    if tag_inner
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Some(format!("${tag_inner}$"))
    } else {
        None
    }
}

fn strip_leading_empty_and_comments(s: &str) -> String {
    let mut lines = Vec::new();
    for line in s.lines() {
        let t = line.trim();
        if !t.is_empty() && !t.starts_with("--") {
            lines.push(t);
        }
    }
    lines.join(" ")
}

/// Formats/beautifies SQL with uppercased keywords and clean clause line breaks,
/// preserving string literals, quoted identifiers, and comments.
pub fn format_sql(sql: &str) -> String {
    const UPPER_KEYWORDS: &[&str] = &[
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "AS", "ON", "JOIN",
        "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS", "GROUP", "BY", "HAVING", "ORDER",
        "ASC", "DESC", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
        "CREATE", "TABLE", "VIEW", "INDEX", "UNIQUE", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
        "CASCADE", "DEFAULT", "ALTER", "ADD", "DROP", "COLUMN", "DISTINCT", "COUNT", "SUM", "AVG",
        "MIN", "MAX", "CASE", "WHEN", "THEN", "ELSE", "END", "WITH", "UNION", "ALL", "EXISTS",
        "BETWEEN", "LIKE", "ILIKE", "RETURNING", "CONFLICT", "DO", "NOTHING", "EXPLAIN", "ANALYZE",
    ];
    const NEWLINE_CLAUSES: &[&str] = &[
        "SELECT", "FROM", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN", "FULL JOIN", "JOIN", "WHERE",
        "GROUP BY", "HAVING", "ORDER BY", "LIMIT", "OFFSET", "VALUES", "SET", "RETURNING", "UNION",
    ];
    const INDENT_CLAUSES: &[&str] = &["AND", "OR"];

    let tokens = tokenize_sql_preserving_literals(sql);
    if tokens.is_empty() {
        return String::new();
    }

    // Uppercase bare keywords first
    let mut norm: Vec<String> = Vec::with_capacity(tokens.len());
    for tok in tokens {
        let up = tok.to_ascii_uppercase();
        if !tok.starts_with('\'')
            && !tok.starts_with('"')
            && !tok.starts_with('`')
            && !tok.starts_with("--")
            && !tok.starts_with("/*")
            && UPPER_KEYWORDS.contains(&up.as_str())
        {
            norm.push(up);
        } else {
            norm.push(tok);
        }
    }

    // Merge compound clauses (`LEFT JOIN`, `GROUP BY`, `ORDER BY`, `INNER JOIN`, `INSERT INTO`, `DELETE FROM`)
    let mut merged: Vec<String> = Vec::with_capacity(norm.len());
    let mut idx = 0usize;
    while idx < norm.len() {
        if idx + 1 < norm.len() {
            let pair = format!("{} {}", norm[idx], norm[idx + 1]);
            if matches!(
                pair.as_str(),
                "LEFT JOIN"
                    | "RIGHT JOIN"
                    | "INNER JOIN"
                    | "FULL JOIN"
                    | "GROUP BY"
                    | "ORDER BY"
                    | "INSERT INTO"
                    | "DELETE FROM"
                    | "CREATE TABLE"
                    | "ALTER TABLE"
                    | "UNION ALL"
            ) {
                merged.push(pair);
                idx += 2;
                continue;
            }
        }
        merged.push(norm[idx].clone());
        idx += 1;
    }

    let mut out = String::new();
    let mut paren_depth = 0i32;

    for (i, tok) in merged.iter().enumerate() {
        if tok == "(" {
            paren_depth += 1;
            if !out.ends_with(' ') && !out.ends_with('(') && !out.is_empty() {
                out.push(' ');
            }
            out.push('(');
            continue;
        }
        if tok == ")" {
            paren_depth = (paren_depth - 1).max(0);
            out.push(')');
            continue;
        }
        if tok == "," {
            out.push_str(", ");
            continue;
        }
        if tok == ";" {
            out.push_str(";\n\n");
            continue;
        }

        if paren_depth == 0 && i > 0 && NEWLINE_CLAUSES.contains(&tok.as_str()) {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(tok);
            continue;
        }

        if paren_depth == 0 && i > 0 && INDENT_CLAUSES.contains(&tok.as_str()) {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("  ");
            out.push_str(tok);
            continue;
        }

        if !out.is_empty() && !out.ends_with('\n') && !out.ends_with('(') && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(tok);
    }

    out.trim().to_string()
}

fn tokenize_sql_preserving_literals(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = sql.chars().collect();
    let n = chars.len();
    let mut i = 0usize;

    let flush = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
        }
    };

    while i < n {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if c.is_whitespace() {
            flush(&mut cur, &mut out);
            i += 1;
            continue;
        }
        if c == '-' && next == Some('-') {
            flush(&mut cur, &mut out);
            let mut cm = String::new();
            while i < n && chars[i] != '\n' {
                cm.push(chars[i]);
                i += 1;
            }
            out.push(cm);
            continue;
        }
        if c == '\'' || c == '"' || c == '`' {
            flush(&mut cur, &mut out);
            let q = c;
            let mut lit = String::from(q);
            i += 1;
            while i < n {
                lit.push(chars[i]);
                if chars[i] == q {
                    if chars.get(i + 1) == Some(&q) {
                        lit.push(q);
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(lit);
            continue;
        }
        if matches!(c, '(' | ')' | ',' | ';') {
            flush(&mut cur, &mut out);
            out.push(c.to_string());
            i += 1;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    flush(&mut cur, &mut out);
    out
}

/// Extracts unique named SQL parameters (`:param_name` and `${param_name}`), ignoring
/// PostgreSQL `::type` casts, string literals, and comments.
pub fn extract_sql_params(sql: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    let chars: Vec<char> = sql.chars().collect();
    let n = chars.len();
    let mut i = 0usize;

    while i < n {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        // Skip quotes & comments
        if c == '\'' || c == '"' || c == '`' {
            let q = c;
            i += 1;
            while i < n {
                if chars[i] == q {
                    if chars.get(i + 1) == Some(&q) {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if c == '-' && next == Some('-') {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // `${param}` syntax
        if c == '$' && next == Some('{') {
            i += 2;
            let start = i;
            while i < n && chars[i] != '}' {
                i += 1;
            }
            if i < n {
                let name: String = chars[start..i].iter().collect();
                let trimmed = name.trim();
                if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
                    found.push(trimmed.to_string());
                }
                i += 1;
                continue;
            }
        }

        // `:param` syntax (ignore `::` Postgres casts)
        if c == ':' {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if next == Some(':') || prev == Some(':') {
                i += 1;
                continue;
            }
            if let Some(nc) = next {
                if nc.is_ascii_alphabetic() || nc == '_' {
                    i += 1;
                    let start = i;
                    while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    if seen.insert(name.clone()) {
                        found.push(name);
                    }
                    continue;
                }
            }
        }

        i += 1;
    }

    found
}

fn format_bound_literal(raw: &str) -> String {
    let t = raw.trim();
    if t.eq_ignore_ascii_case("null") {
        return "NULL".into();
    }
    if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false") {
        return t.to_ascii_uppercase();
    }
    if t.parse::<i64>().is_ok() || t.parse::<f64>().is_ok() {
        return t.to_string();
    }
    format!("'{}'", raw.replace('\'', "''"))
}

/// Safely binds `:param` and `${param}` values outside string literals and comments.
pub fn bind_sql_params(sql: &str, params: &HashMap<String, String>) -> String {
    if params.is_empty() {
        return sql.to_string();
    }
    let chars: Vec<char> = sql.chars().collect();
    let n = chars.len();
    let mut i = 0usize;
    let mut out = String::with_capacity(sql.len() + 32);

    while i < n {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if c == '\'' || c == '"' || c == '`' {
            let q = c;
            out.push(c);
            i += 1;
            while i < n {
                out.push(chars[i]);
                if chars[i] == q {
                    if chars.get(i + 1) == Some(&q) {
                        out.push(q);
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if c == '$' && next == Some('{') {
            let mut j = i + 2;
            while j < n && chars[j] != '}' {
                j += 1;
            }
            if j < n {
                let name: String = chars[i + 2..j].iter().collect();
                if let Some(val) = params.get(name.trim()) {
                    out.push_str(&format_bound_literal(val));
                    i = j + 1;
                    continue;
                }
            }
        }

        if c == ':' {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if next != Some(':') && prev != Some(':') {
                if let Some(nc) = next {
                    if nc.is_ascii_alphabetic() || nc == '_' {
                        let mut j = i + 1;
                        while j < n && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                            j += 1;
                        }
                        let name: String = chars[i + 1..j].iter().collect();
                        if let Some(val) = params.get(&name) {
                            out.push_str(&format_bound_literal(val));
                            i = j;
                            continue;
                        }
                    }
                }
            }
        }

        out.push(c);
        i += 1;
    }

    out
}

/// Executes a multi-statement SQL script sequentially via `DbDriver::query`.
pub async fn execute_sql_script(
    driver: &dyn DbDriver,
    config: &DatabaseConnection,
    script: &str,
    stop_on_error: bool,
) -> Result<ScriptExecutionReport> {
    let stmts = if config.kind == DbKind::Redis {
        vec![script.trim().to_string()]
    } else {
        split_sql_statements(script)
    };
    let start = Instant::now();
    let mut items = Vec::with_capacity(stmts.len());
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut total_rows = 0u64;
    let mut last_result = None;

    for (idx, stmt) in stmts.iter().enumerate() {
        let st_start = Instant::now();
        let preview: String = stmt
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("--"))
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(90)
            .collect();

        match driver.query(config, stmt).await {
            Ok(res) => {
                succeeded += 1;
                total_rows += res.row_count;
                items.push(StatementRunItem {
                    index: idx + 1,
                    sql_preview: preview,
                    row_count: res.row_count,
                    elapsed_ms: st_start.elapsed().as_millis() as u64,
                    error: None,
                });
                last_result = Some(res);
            }
            Err(e) => {
                failed += 1;
                items.push(StatementRunItem {
                    index: idx + 1,
                    sql_preview: preview,
                    row_count: 0,
                    elapsed_ms: st_start.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
                if stop_on_error {
                    break;
                }
            }
        }
    }

    Ok(ScriptExecutionReport {
        total_statements: stmts.len(),
        succeeded,
        failed,
        total_rows_affected: total_rows,
        total_elapsed_ms: start.elapsed().as_millis() as u64,
        items,
        last_result,
    })
}

/// Computes DBeaver-style summary statistics (`COUNT`, `NULLs`, `DISTINCT`, `MIN`, `MAX`, `SUM`, `AVG`) per column.
pub fn compute_column_stats(result: &QueryResult) -> Vec<ColumnStats> {
    let total_rows = result.rows.len();
    result
        .columns
        .iter()
        .map(|col| {
            let mut non_null = 0usize;
            let mut distinct = BTreeSet::new();
            let mut nums = Vec::new();
            let mut min_str: Option<String> = None;
            let mut max_str: Option<String> = None;

            for row in &result.rows {
                match row.get(col) {
                    None | Some(serde_json::Value::Null) => {}
                    Some(v) => {
                        non_null += 1;
                        let s = match v {
                            serde_json::Value::String(st) => st.clone(),
                            other => other.to_string(),
                        };
                        distinct.insert(s.clone());
                        if let Some(n) = v.as_f64().or_else(|| s.parse::<f64>().ok()) {
                            nums.push(n);
                        }
                        if min_str.as_ref().is_none_or(|m| s < *m) {
                            min_str = Some(s.clone());
                        }
                        if max_str.as_ref().is_none_or(|m| s > *m) {
                            max_str = Some(s);
                        }
                    }
                }
            }

            let (min, max, sum, avg) = if !nums.is_empty() && nums.len() == non_null {
                let min_n = nums.iter().cloned().fold(f64::INFINITY, f64::min);
                let max_n = nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let sum_n: f64 = nums.iter().sum();
                let avg_n = sum_n / (nums.len() as f64);
                (
                    Some(format!("{min_n}")),
                    Some(format!("{max_n}")),
                    Some(sum_n),
                    Some(avg_n),
                )
            } else {
                (min_str, max_str, None, None)
            };

            ColumnStats {
                column: col.clone(),
                total_rows,
                non_null,
                null_count: total_rows.saturating_sub(non_null),
                distinct_count: distinct.len(),
                min,
                max,
                sum,
                avg,
            }
        })
        .collect()
}

/// Built-in SQL / Query templates for rapid query authoring across all 33 engines.
pub fn builtin_sql_templates(dialect: DbKind) -> Vec<(&'static str, &'static str)> {
    match dialect {
        DbKind::Mongodb => vec![
            ("Find Filter", "users.find({\"plan\": \"Enterprise\"})"),
            ("Aggregate Group", "orders.aggregate([{\"$group\": {\"_id\": \"$status\", \"total\": {\"$sum\": \"$amount\"}}}])"),
            ("Insert Document", "users.insertOne({\"name\": \"Ada\", \"role\": \"Admin\"})"),
            ("Count Documents", "users.countDocuments({})"),
        ],
        DbKind::Redis => vec![
            ("Server Info", "INFO server"),
            ("Scan Keys", "SCAN 0 COUNT 50"),
            ("Set + Get Pipeline", "SET session:demo \"active\"\nGET session:demo\nTTL session:demo"),
            ("Hash Inspect", "HSET user:1 name \"Ada\" plan \"Pro\"\nHGETALL user:1"),
        ],
        DbKind::Mssql | DbKind::AzureSql => vec![
            ("SELECT TOP 50 (T-SQL)", "SELECT TOP 50 *\nFROM [users]\nWHERE [plan] = :plan\nORDER BY [id] DESC;"),
            ("Server & DB Version", "SELECT @@VERSION AS server_version, DB_NAME() AS current_db;"),
            ("Active Sessions (DMV)", "SELECT session_id, login_name, status, host_name FROM sys.dm_exec_sessions WHERE is_user_process = 1;"),
        ],
        DbKind::Oracle => vec![
            ("SELECT FETCH FIRST (Oracle)", "SELECT *\nFROM \"USERS\"\nFETCH FIRST 50 ROWS ONLY;"),
            ("Oracle Version Banner", "SELECT banner FROM v$version;"),
            ("Tablespace Usage", "SELECT tablespace_name, status FROM user_tablespaces;"),
        ],
        DbKind::Cassandra | DbKind::Scylladb => vec![
            ("Cluster Version (CQL)", "SELECT cluster_name, release_version FROM system.local;"),
            ("List Keyspaces", "SELECT keyspace_name, replication FROM system_schema.keyspaces;"),
        ],
        DbKind::Neo4j => vec![
            ("Match Nodes (Cypher)", "MATCH (n)\nRETURN labels(n) AS labels, properties(n) AS props\nLIMIT 25"),
            ("Create Graph Relationship", "CREATE (a:Service {name: 'Gateway'})-[:CALLS]->(b:Service {name: 'Auth'}) RETURN a, b"),
        ],
        DbKind::Elasticsearch => vec![
            ("Elasticsearch SQL", "SELECT * FROM \"logs\" LIMIT 25"),
            ("Elasticsearch JSON DSL", "{\n  \"size\": 25,\n  \"query\": { \"match_all\": {} }\n}"),
        ],
        DbKind::Influxdb => vec![
            ("Show Measurements", "SHOW MEASUREMENTS"),
            ("Recent Time-Series Points", "SELECT * FROM \"cpu\" WHERE time > now() - 1h LIMIT 50"),
        ],
        DbKind::Couchdb => vec![
            ("Mango Selector Find", "{\n  \"selector\": {},\n  \"limit\": 25\n}"),
        ],
        _ => vec![
            ("SELECT + Filter", "SELECT *\nFROM users\nWHERE plan = :plan\nORDER BY id DESC\nLIMIT 50;"),
            ("JOIN Aggregation", "SELECT u.id, u.name, COUNT(o.id) AS order_count, COALESCE(SUM(o.amount_cents), 0) AS total_cents\nFROM users u\nLEFT JOIN orders o ON o.user_id = u.id\nGROUP BY u.id, u.name\nORDER BY total_cents DESC;"),
            ("CTE + Window Rank", "WITH ranked AS (\n  SELECT *, ROW_NUMBER() OVER (PARTITION BY status ORDER BY amount_cents DESC) AS rn\n  FROM orders\n)\nSELECT * FROM ranked WHERE rn <= 3;"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multi_statements_preserving_quotes_comments_and_dollar_blocks() {
        let script = r#"
            -- Initial setup; comment semicolon ignored
            CREATE TABLE t (id INT PRIMARY KEY, note TEXT);
            INSERT INTO t VALUES (1, 'hello; world');
            CREATE FUNCTION f() RETURNS void AS $$
            BEGIN
                PERFORM 1;
            END;
            $$ LANGUAGE plpgsql;
        "#;
        let stmts = split_sql_statements(script);
        assert_eq!(stmts.len(), 3);
        assert!(stmts[1].contains("'hello; world'"));
        assert!(stmts[2].contains("PERFORM 1;"));
    }

    #[test]
    fn extracts_and_binds_named_params_ignoring_postgres_casts() {
        let sql = "SELECT id::text FROM users WHERE plan = :plan AND age >= ${min_age} AND tag = ':literal'";
        let params = extract_sql_params(sql);
        assert_eq!(params, vec!["plan".to_string(), "min_age".to_string()]);

        let mut map = HashMap::new();
        map.insert("plan".into(), "O'Reilly Pro".into());
        map.insert("min_age".into(), "21".into());
        let bound = bind_sql_params(sql, &map);
        assert!(bound.contains("id::text"));
        assert!(bound.contains("plan = 'O''Reilly Pro'"));
        assert!(bound.contains("age >= 21"));
        assert!(bound.contains("tag = ':literal'"));
    }

    #[test]
    fn formats_sql_and_computes_numeric_stats() {
        let formatted = format_sql("select id, name from users left join orders on orders.user_id = users.id where active = true and score > 10 order by id desc;");
        assert!(formatted.contains("SELECT id, name\nFROM users\nLEFT JOIN orders"));
        assert!(formatted.contains("\nWHERE active = true\n  AND score > 10\nORDER BY id DESC;"));

        let mut r1 = HashMap::new();
        r1.insert("amount".into(), serde_json::json!(10));
        let mut r2 = HashMap::new();
        r2.insert("amount".into(), serde_json::json!(30));
        let mut r3 = HashMap::new();
        r3.insert("amount".into(), serde_json::Value::Null);
        let qr = QueryResult {
            columns: vec!["amount".into()],
            rows: vec![r1, r2, r3],
            row_count: 3,
            execution_time_ms: 1,
            error: None,
            notices: vec![],
            query_plan: None,
            command: None,
        };
        let stats = compute_column_stats(&qr);
        assert_eq!(stats[0].non_null, 2);
        assert_eq!(stats[0].null_count, 1);
        assert_eq!(stats[0].sum, Some(40.0));
        assert_eq!(stats[0].avg, Some(20.0));
    }
}
