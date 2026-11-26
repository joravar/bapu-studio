//! DBeaver-grade Data Transfer Engine:
//! - 8 Result/Table Export Formats (`CSV`, `TSV`, `JSON`, `NDJSON`, `SQL INSERT`, `Markdown`, `HTML`, `XML`)
//! - RFC-4180 CSV & JSON Array Importer with Column Mapping -> Transactional `RowMutation` batches
//! - Instant Data Grid Quick-Search, Column Filter (`=`, `!=`, `contains`, `>`, `<`, `IS NULL`), and Numeric-Aware Sorting

use crate::db_ddl::quote_dialect_ident;
use crate::types::{DbKind, MutationOp, QueryResult, RowMutation};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    Ndjson,
    SqlInsert,
    Markdown,
    Html,
    Xml,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 8] = [
        ExportFormat::Csv,
        ExportFormat::Tsv,
        ExportFormat::Json,
        ExportFormat::Ndjson,
        ExportFormat::SqlInsert,
        ExportFormat::Markdown,
        ExportFormat::Html,
        ExportFormat::Xml,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "CSV (RFC-4180)",
            ExportFormat::Tsv => "TSV (Spreadsheet)",
            ExportFormat::Json => "JSON (Pretty)",
            ExportFormat::Ndjson => "NDJSON (JSON Lines)",
            ExportFormat::SqlInsert => "SQL INSERT Script",
            ExportFormat::Markdown => "Markdown Table",
            ExportFormat::Html => "HTML Table",
            ExportFormat::Xml => "XML Dataset",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Tsv => "tsv",
            ExportFormat::Json => "json",
            ExportFormat::Ndjson => "ndjson",
            ExportFormat::SqlInsert => "sql",
            ExportFormat::Markdown => "md",
            ExportFormat::Html => "html",
            ExportFormat::Xml => "xml",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridFilterOp {
    Contains,
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    IsNull,
    IsNotNull,
}

impl GridFilterOp {
    pub const ALL: [GridFilterOp; 7] = [
        GridFilterOp::Contains,
        GridFilterOp::Equals,
        GridFilterOp::NotEquals,
        GridFilterOp::GreaterThan,
        GridFilterOp::LessThan,
        GridFilterOp::IsNull,
        GridFilterOp::IsNotNull,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            GridFilterOp::Contains => "contains",
            GridFilterOp::Equals => "=",
            GridFilterOp::NotEquals => "!=",
            GridFilterOp::GreaterThan => ">",
            GridFilterOp::LessThan => "<",
            GridFilterOp::IsNull => "IS NULL",
            GridFilterOp::IsNotNull => "IS NOT NULL",
        }
    }
}

fn val_to_plain_str(v: Option<&serde_json::Value>) -> String {
    match v {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn escape_csv_cell(cell: &str, delim: char) -> String {
    if cell.contains(delim) || cell.contains('"') || cell.contains('\n') || cell.contains('\r') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

fn val_to_sql_literal(v: Option<&serde_json::Value>, dialect: DbKind) -> String {
    match v {
        None | Some(serde_json::Value::Null) => "NULL".into(),
        Some(serde_json::Value::Bool(b)) => match dialect {
            DbKind::Sqlite | DbKind::Mysql => if *b { "1".into() } else { "0".into() },
            _ => if *b { "TRUE".into() } else { "FALSE".into() },
        },
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => format!("'{}'", s.replace('\'', "''")),
        Some(other) => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Exports a `QueryResult` into any of the 8 DBeaver-parity formats.
pub fn export_query_result(
    result: &QueryResult,
    table_name: &str,
    dialect: DbKind,
    format: ExportFormat,
) -> String {
    let tbl = if table_name.trim().is_empty() {
        "export_table"
    } else {
        table_name.trim()
    };

    match format {
        ExportFormat::Csv | ExportFormat::Tsv => {
            let delim = if format == ExportFormat::Csv { ',' } else { '\t' };
            let mut out = result
                .columns
                .iter()
                .map(|c| escape_csv_cell(c, delim))
                .collect::<Vec<_>>()
                .join(&delim.to_string());
            out.push('\n');
            for row in &result.rows {
                let line = result
                    .columns
                    .iter()
                    .map(|c| escape_csv_cell(&val_to_plain_str(row.get(c)), delim))
                    .collect::<Vec<_>>()
                    .join(&delim.to_string());
                out.push_str(&line);
                out.push('\n');
            }
            out
        }
        ExportFormat::Json => {
            let ordered_rows: Vec<serde_json::Map<String, serde_json::Value>> = result
                .rows
                .iter()
                .map(|r| {
                    let mut m = serde_json::Map::new();
                    for c in &result.columns {
                        m.insert(
                            c.clone(),
                            r.get(c).cloned().unwrap_or(serde_json::Value::Null),
                        );
                    }
                    m
                })
                .collect();
            serde_json::to_string_pretty(&ordered_rows).unwrap_or_else(|_| "[]".into())
        }
        ExportFormat::Ndjson => result
            .rows
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n"),
        ExportFormat::SqlInsert => {
            if result.columns.is_empty() || result.rows.is_empty() {
                return format!("-- No rows to export for {tbl}\n");
            }
            let qt = quote_dialect_ident(tbl, dialect);
            let cols = result
                .columns
                .iter()
                .map(|c| quote_dialect_ident(c, dialect))
                .collect::<Vec<_>>()
                .join(", ");
            let mut stmts = Vec::with_capacity(result.rows.len());
            for row in &result.rows {
                let vals = result
                    .columns
                    .iter()
                    .map(|c| val_to_sql_literal(row.get(c), dialect))
                    .collect::<Vec<_>>()
                    .join(", ");
                stmts.push(format!("INSERT INTO {qt} ({cols}) VALUES ({vals});"));
            }
            stmts.join("\n")
        }
        ExportFormat::Markdown => {
            if result.columns.is_empty() {
                return String::new();
            }
            let header = format!("| {} |", result.columns.join(" | "));
            let sep = format!(
                "| {} |",
                vec!["---"; result.columns.len()].join(" | ")
            );
            let mut lines = vec![header, sep];
            for row in &result.rows {
                let cells = result
                    .columns
                    .iter()
                    .map(|c| match row.get(c) {
                        None | Some(serde_json::Value::Null) => "NULL".into(),
                        Some(v) => val_to_plain_str(Some(v)).replace('|', "\\|").replace('\n', " "),
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                lines.push(format!("| {cells} |"));
            }
            lines.join("\n")
        }
        ExportFormat::Html => {
            let mut html = String::from("<table>\n  <thead>\n    <tr>");
            for c in &result.columns {
                html.push_str(&format!("<th>{}</th>", escape_html(c)));
            }
            html.push_str("</tr>\n  </thead>\n  <tbody>\n");
            for row in &result.rows {
                html.push_str("    <tr>");
                for c in &result.columns {
                    html.push_str(&format!(
                        "<td>{}</td>",
                        escape_html(&val_to_plain_str(row.get(c)))
                    ));
                }
                html.push_str("</tr>\n");
            }
            html.push_str("  </tbody>\n</table>");
            html
        }
        ExportFormat::Xml => {
            let mut xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<dataset table=\"{}\">\n", escape_html(tbl));
            for row in &result.rows {
                xml.push_str("  <row>\n");
                for c in &result.columns {
                    let tag: String = c
                        .chars()
                        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' { ch } else { '_' })
                        .collect();
                    let tag = if tag.is_empty() { "col" } else { tag.as_str() };
                    match row.get(c) {
                        None | Some(serde_json::Value::Null) => {
                            xml.push_str(&format!("    <{tag} xsi:nil=\"true\"/>\n"));
                        }
                        Some(v) => {
                            xml.push_str(&format!(
                                "    <{tag}>{}</{tag}>\n",
                                escape_html(&val_to_plain_str(Some(v)))
                            ));
                        }
                    }
                }
                xml.push_str("  </row>\n");
            }
            xml.push_str("</dataset>");
            xml
        }
    }
}

/// Filters and sorts row indices (`0..result.rows.len()`) in memory while preserving original row indices
/// so staged cell edits/deletes continue to map cleanly to `result.rows[orig_idx]`.
pub fn filter_and_sort_row_indices(
    result: &QueryResult,
    quick_search: &str,
    filter_col: Option<&str>,
    filter_op: GridFilterOp,
    filter_val: &str,
    sort_col: Option<&str>,
    sort_asc: bool,
) -> Vec<usize> {
    let q_lower = quick_search.trim().to_ascii_lowercase();
    let f_lower = filter_val.trim().to_ascii_lowercase();

    let mut indices: Vec<usize> = (0..result.rows.len())
        .filter(|&idx| {
            let row = &result.rows[idx];
            if !q_lower.is_empty() {
                let any_match = result.columns.iter().any(|c| {
                    val_to_plain_str(row.get(c))
                        .to_ascii_lowercase()
                        .contains(&q_lower)
                });
                if !any_match {
                    return false;
                }
            }

            if let Some(col) = filter_col.filter(|c| !c.is_empty()) {
                let cell = row.get(col);
                let is_null = matches!(cell, None | Some(serde_json::Value::Null));
                match filter_op {
                    GridFilterOp::IsNull => return is_null,
                    GridFilterOp::IsNotNull => return !is_null,
                    _ if f_lower.is_empty() => {}
                    GridFilterOp::Contains => {
                        if !val_to_plain_str(cell).to_ascii_lowercase().contains(&f_lower) {
                            return false;
                        }
                    }
                    GridFilterOp::Equals => {
                        if val_to_plain_str(cell).to_ascii_lowercase() != f_lower {
                            return false;
                        }
                    }
                    GridFilterOp::NotEquals => {
                        if val_to_plain_str(cell).to_ascii_lowercase() == f_lower {
                            return false;
                        }
                    }
                    GridFilterOp::GreaterThan => {
                        let s = val_to_plain_str(cell);
                        if let (Ok(a), Ok(b)) = (s.parse::<f64>(), filter_val.trim().parse::<f64>()) {
                            if a <= b {
                                return false;
                            }
                        } else if s.to_ascii_lowercase() <= f_lower {
                            return false;
                        }
                    }
                    GridFilterOp::LessThan => {
                        let s = val_to_plain_str(cell);
                        if let (Ok(a), Ok(b)) = (s.parse::<f64>(), filter_val.trim().parse::<f64>()) {
                            if a >= b {
                                return false;
                            }
                        } else if s.to_ascii_lowercase() >= f_lower {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .collect();

    if let Some(scol) = sort_col.filter(|c| !c.is_empty()) {
        indices.sort_by(|&a, &b| {
            let va = result.rows[a].get(scol);
            let vb = result.rows[b].get(scol);
            let ord = compare_json_cells(va, vb);
            if sort_asc { ord } else { ord.reverse() }
        });
    }

    indices
}

fn compare_json_cells(
    a: Option<&serde_json::Value>,
    b: Option<&serde_json::Value>,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None | Some(serde_json::Value::Null), None | Some(serde_json::Value::Null)) => Ordering::Equal,
        (None | Some(serde_json::Value::Null), _) => Ordering::Less,
        (_, None | Some(serde_json::Value::Null)) => Ordering::Greater,
        (Some(va), Some(vb)) => {
            if let (Some(na), Some(nb)) = (va.as_f64(), vb.as_f64()) {
                return na.partial_cmp(&nb).unwrap_or(Ordering::Equal);
            }
            val_to_plain_str(Some(va)).cmp(&val_to_plain_str(Some(vb)))
        }
    }
}

/// Parses RFC-4180 CSV text into `(headers, rows)`.
pub fn parse_csv_records(raw: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut cur_row: Vec<String> = Vec::new();
    let mut cur_cell = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = raw.trim_start_matches('\u{feff}').chars().collect();
    let n = chars.len();
    let mut i = 0usize;

    while i < n {
        let c = chars[i];
        if in_quotes {
            if c == '"' {
                if chars.get(i + 1) == Some(&'"') {
                    cur_cell.push('"');
                    i += 2;
                    continue;
                }
                in_quotes = false;
            } else {
                cur_cell.push(c);
            }
            i += 1;
            continue;
        }

        match c {
            '"' => in_quotes = true,
            ',' => {
                cur_row.push(std::mem::take(&mut cur_cell));
            }
            '\r' => {
                if chars.get(i + 1) == Some(&'\n') {
                    i += 1;
                }
                cur_row.push(std::mem::take(&mut cur_cell));
                if cur_row.iter().any(|s| !s.is_empty()) {
                    rows.push(std::mem::take(&mut cur_row));
                } else {
                    cur_row.clear();
                }
            }
            '\n' => {
                cur_row.push(std::mem::take(&mut cur_cell));
                if cur_row.iter().any(|s| !s.is_empty()) {
                    rows.push(std::mem::take(&mut cur_row));
                } else {
                    cur_row.clear();
                }
            }
            other => cur_cell.push(other),
        }
        i += 1;
    }

    if !cur_cell.is_empty() || !cur_row.is_empty() {
        cur_row.push(cur_cell);
        if cur_row.iter().any(|s| !s.is_empty()) {
            rows.push(cur_row);
        }
    }

    if rows.is_empty() {
        bail!("CSV payload is empty.");
    }
    let headers: Vec<String> = rows.remove(0).into_iter().map(|h| h.trim().to_string()).collect();
    Ok((headers, rows))
}

fn infer_json_scalar(s: &str) -> serde_json::Value {
    let t = s.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("null") {
        return serde_json::Value::Null;
    }
    if t.eq_ignore_ascii_case("true") {
        return serde_json::Value::Bool(true);
    }
    if t.eq_ignore_ascii_case("false") {
        return serde_json::Value::Bool(false);
    }
    if let Ok(i) = t.parse::<i64>() {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(f) = t.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(num);
        }
    }
    serde_json::Value::String(s.to_string())
}

pub type ParsedImportRows = Vec<HashMap<String, serde_json::Value>>;

/// Auto-detects JSON array of objects OR CSV text and returns `(source_columns, typed_rows)`.
pub fn parse_import_payload(raw: &str) -> Result<(Vec<String>, ParsedImportRows)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("Import payload is empty.");
    }

    if trimmed.starts_with('[') {
        let arr: Vec<serde_json::Map<String, serde_json::Value>> =
            serde_json::from_str(trimmed).context("parsing JSON array for import")?;
        let mut cols = BTreeSet::new();
        let mut out_rows = Vec::with_capacity(arr.len());
        for obj in arr {
            let mut map = HashMap::new();
            for (k, v) in obj {
                cols.insert(k.clone());
                map.insert(k, v);
            }
            out_rows.push(map);
        }
        return Ok((cols.into_iter().collect(), out_rows));
    }

    let (headers, raw_rows) = parse_csv_records(trimmed)?;
    let mut out_rows = Vec::with_capacity(raw_rows.len());
    for r in raw_rows {
        let mut map = HashMap::new();
        for (idx, col) in headers.iter().enumerate() {
            if !col.is_empty() {
                let cell = r.get(idx).map(String::as_str).unwrap_or("");
                map.insert(col.clone(), infer_json_scalar(cell));
            }
        }
        out_rows.push(map);
    }
    Ok((headers, out_rows))
}

/// Converts parsed import rows + `(source_col -> target_col)` mapping into `RowMutation::Insert` items.
/// Mapping to `""` or `"(skip)"` omits that source column.
pub fn build_import_mutations(
    rows: &[HashMap<String, serde_json::Value>],
    column_mapping: &HashMap<String, String>,
) -> Vec<RowMutation> {
    let mut mutations = Vec::with_capacity(rows.len());
    for row in rows {
        let mut values = HashMap::new();
        for (src_col, val) in row {
            let target = column_mapping
                .get(src_col)
                .map(String::as_str)
                .unwrap_or(src_col.as_str())
                .trim();
            if target.is_empty() || target == "(skip)" {
                continue;
            }
            if !val.is_null() {
                values.insert(target.to_string(), val.clone());
            }
        }
        if !values.is_empty() {
            mutations.push(RowMutation {
                op: MutationOp::Insert,
                values,
                where_clause: HashMap::new(),
            });
        }
    }
    mutations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_all_eight_formats_and_round_trips_rfc4180_csv_and_sql_inserts() {
        let mut r1 = HashMap::new();
        r1.insert("id".into(), serde_json::json!(1));
        r1.insert("name".into(), serde_json::json!("Ada, \"Countess\""));
        let mut r2 = HashMap::new();
        r2.insert("id".into(), serde_json::json!(2));
        r2.insert("name".into(), serde_json::json!("O'Connor"));

        let qr = QueryResult {
            columns: vec!["id".into(), "name".into()],
            rows: vec![r1, r2],
            row_count: 2,
            execution_time_ms: 1,
            error: None,
            notices: vec![],
            query_plan: None,
            command: None,
        };

        for fmt in ExportFormat::ALL {
            let exported = export_query_result(&qr, "users", DbKind::Postgres, fmt);
            assert!(!exported.is_empty());
        }

        let sql_ins = export_query_result(&qr, "users", DbKind::Postgres, ExportFormat::SqlInsert);
        assert!(sql_ins.contains("INSERT INTO \"users\" (\"id\", \"name\") VALUES (2, 'O''Connor');"));

        let csv = export_query_result(&qr, "users", DbKind::Postgres, ExportFormat::Csv);
        let (cols, imported) = parse_import_payload(&csv).unwrap();
        assert_eq!(cols, vec!["id", "name"]);
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0]["name"], serde_json::json!("Ada, \"Countess\""));

        let mut mapping = HashMap::new();
        mapping.insert("id".into(), "(skip)".into());
        mapping.insert("name".into(), "full_name".into());
        let muts = build_import_mutations(&imported, &mapping);
        assert_eq!(muts.len(), 2);
        assert!(!muts[0].values.contains_key("id"));
        assert_eq!(muts[0].values["full_name"], serde_json::json!("Ada, \"Countess\""));
    }

    #[test]
    fn filters_and_sorts_grid_rows_numerically() {
        let rows = vec![
            HashMap::from([("id".into(), serde_json::json!(10)), ("tag".into(), serde_json::json!("pro"))]),
            HashMap::from([("id".into(), serde_json::json!(2)), ("tag".into(), serde_json::json!("enterprise"))]),
            HashMap::from([("id".into(), serde_json::json!(25)), ("tag".into(), serde_json::json!("pro"))]),
        ];
        let qr = QueryResult {
            columns: vec!["id".into(), "tag".into()],
            rows,
            row_count: 3,
            execution_time_ms: 1,
            error: None,
            notices: vec![],
            query_plan: None,
            command: None,
        };

        let idxs = filter_and_sort_row_indices(
            &qr,
            "",
            Some("id"),
            GridFilterOp::GreaterThan,
            "5",
            Some("id"),
            false,
        );
        assert_eq!(idxs, vec![2, 0]); // 25 then 10!
    }
}
