//! Clean-room DDL Generator, Cross-Dialect Type Mapper, Forward-Engineering Blueprint,
//! Mermaid `erDiagram` Exporter, and Standalone Vector SVG ER Diagram Exporter.

use crate::types::{ColumnDefinition, DbKind, TableSchema};
use std::collections::{HashMap, HashSet};

pub fn quote_dialect_ident(name: &str, dialect: DbKind) -> String {
    match dialect {
        DbKind::Mssql | DbKind::AzureSql => format!("[{}]", name.replace(']', "]]")),
        DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb | DbKind::Bigquery | DbKind::Databricks => {
            format!("`{}`", name.replace('`', "``"))
        }
        DbKind::Mongodb | DbKind::Redis | DbKind::Neo4j | DbKind::Elasticsearch | DbKind::Couchdb => {
            name.to_string()
        }
        _ => format!("\"{}\"", name.replace('"', "\"\"")),
    }
}

pub fn map_column_type(raw_type: &str, col: &ColumnDefinition, dialect: DbKind) -> String {
    let up = raw_type.trim().to_ascii_uppercase();
    match dialect {
        DbKind::Postgres
        | DbKind::Cockroachdb
        | DbKind::Yugabytedb
        | DbKind::Timescaledb
        | DbKind::Redshift => {
            if col.auto_increment && col.is_primary_key {
                if up.contains("BIG") {
                    return "BIGSERIAL".into();
                }
                return "SERIAL".into();
            }
            if up == "INT" || up == "INTEGER" || up.starts_with("INT(") {
                "INTEGER".into()
            } else if up.starts_with("BIGINT") {
                "BIGINT".into()
            } else if up == "TINYINT(1)" || up == "BOOL" || up == "BOOLEAN" || up == "BIT" {
                "BOOLEAN".into()
            } else if up == "DATETIME" || up == "DATETIME2" || up.contains("TIMESTAMP") {
                "TIMESTAMPTZ".into()
            } else if up == "DOUBLE" || up == "REAL" || up == "FLOAT" {
                "DOUBLE PRECISION".into()
            } else if up == "JSON" {
                "JSONB".into()
            } else if up.is_empty() {
                "TEXT".into()
            } else {
                up
            }
        }
        DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => {
            if up == "SERIAL" {
                return "INT AUTO_INCREMENT".into();
            }
            if up == "BIGSERIAL" {
                return "BIGINT AUTO_INCREMENT".into();
            }
            let base = if up == "TEXT" && (col.is_primary_key || col.is_unique) {
                "VARCHAR(255)".to_string()
            } else if up == "INTEGER" {
                "INT".to_string()
            } else if up == "BOOLEAN" {
                "TINYINT(1)".to_string()
            } else if up == "TIMESTAMPTZ" || up.contains("TIMESTAMP WITH") {
                "DATETIME".to_string()
            } else if up == "JSONB" {
                "JSON".to_string()
            } else if up == "DOUBLE PRECISION" || up == "REAL" {
                "DOUBLE".to_string()
            } else if up.is_empty() {
                "VARCHAR(255)".to_string()
            } else {
                up
            };
            if col.auto_increment && !base.contains("AUTO_INCREMENT") {
                format!("{base} AUTO_INCREMENT")
            } else {
                base
            }
        }
        DbKind::Mssql | DbKind::AzureSql => {
            let base = if up == "SERIAL" || up == "INTEGER" || up == "INT" {
                "INT".to_string()
            } else if up == "BIGSERIAL" || up.starts_with("BIGINT") {
                "BIGINT".to_string()
            } else if up == "TEXT" {
                if col.is_primary_key || col.is_unique {
                    "NVARCHAR(255)".to_string()
                } else {
                    "NVARCHAR(MAX)".to_string()
                }
            } else if up == "BOOLEAN" || up == "BOOL" || up == "TINYINT(1)" {
                "BIT".to_string()
            } else if up.contains("TIMESTAMP") || up == "DATETIME" {
                "DATETIME2".to_string()
            } else if up == "DOUBLE PRECISION" || up == "REAL" {
                "FLOAT".to_string()
            } else if up == "JSON" || up == "JSONB" {
                "NVARCHAR(MAX)".to_string()
            } else if up.is_empty() {
                "NVARCHAR(255)".to_string()
            } else {
                up
            };
            if col.auto_increment && col.is_primary_key && !base.contains("IDENTITY") {
                format!("{base} IDENTITY(1,1)")
            } else {
                base
            }
        }
        DbKind::Oracle => {
            let base = if up.contains("INT") || up.contains("SERIAL") {
                "NUMBER(19)".to_string()
            } else if up == "TEXT" || up.starts_with("VARCHAR") {
                "VARCHAR2(255)".to_string()
            } else if up == "BOOLEAN" || up == "BOOL" {
                "NUMBER(1)".to_string()
            } else if up.contains("TIME") || up == "DATETIME" {
                "TIMESTAMP".to_string()
            } else if up.contains("DOUBLE") || up == "REAL" || up == "FLOAT" {
                "BINARY_DOUBLE".to_string()
            } else if up.is_empty() {
                "VARCHAR2(255)".to_string()
            } else {
                up
            };
            if col.auto_increment && col.is_primary_key {
                format!("{base} GENERATED BY DEFAULT AS IDENTITY")
            } else {
                base
            }
        }
        DbKind::Clickhouse => {
            if up.contains("BIGINT") || up == "BIGSERIAL" {
                "Int64".into()
            } else if up.contains("INT") || up == "SERIAL" {
                "Int32".into()
            } else if up == "BOOLEAN" || up == "BOOL" {
                "Bool".into()
            } else if up.contains("FLOA") || up.contains("DOUB") || up == "REAL" {
                "Float64".into()
            } else if up.contains("TIME") || up == "DATETIME" {
                "DateTime".into()
            } else {
                "String".into()
            }
        }
        DbKind::Cassandra | DbKind::Scylladb => {
            if up.contains("BIGINT") {
                "bigint".into()
            } else if up.contains("INT") {
                "int".into()
            } else if up == "BOOLEAN" || up == "BOOL" {
                "boolean".into()
            } else if up.contains("UUID") {
                "uuid".into()
            } else if up.contains("DOUB") || up == "REAL" {
                "double".into()
            } else if up.contains("TIME") {
                "timestamp".into()
            } else {
                "text".into()
            }
        }
        DbKind::Sqlite | DbKind::Libsql => {
            if (col.is_primary_key && (col.auto_increment || up.contains("SERIAL")))
                || up.contains("INT")
            {
                "INTEGER".into()
            } else if up.contains("REAL") || up.contains("FLOA") || up.contains("DOUB") {
                "REAL".into()
            } else if up.contains("BLOB") || up.contains("BYTEA") {
                "BLOB".into()
            } else {
                "TEXT".into()
            }
        }
        DbKind::Mongodb | DbKind::Couchdb => match up.as_str() {
            "INT" | "INTEGER" | "BIGINT" | "INT32" | "INT64" => "int".into(),
            "REAL" | "DOUBLE" | "FLOAT" | "DOUBLE PRECISION" => "double".into(),
            "BOOL" | "BOOLEAN" => "bool".into(),
            "OBJECTID" => "objectId".into(),
            _ => "string".into(),
        },
        _ => {
            if up.is_empty() {
                "VARCHAR(255)".into()
            } else {
                up
            }
        }
    }
}

/// Generates clean DDL (`CREATE TABLE` + `CREATE INDEX` or MongoDB validator) for a single table.
pub fn generate_table_ddl(table: &TableSchema, dialect: DbKind, prefer_native: bool) -> String {
    if prefer_native {
        if let Some(ddl) = table.ddl.as_deref().filter(|s| !s.trim().is_empty()) {
            let mut out = ddl.trim().trim_end_matches(';').to_string();
            out.push(';');
            return out;
        }
    }

    if dialect == DbKind::Mongodb {
        let mut props = serde_json::Map::new();
        let mut required = Vec::new();
        for col in &table.columns {
            props.insert(
                col.name.clone(),
                serde_json::json!({ "bsonType": map_column_type(&col.col_type, col, DbKind::Mongodb) }),
            );
            if !col.is_nullable {
                required.push(col.name.clone());
            }
        }
        let spec = serde_json::json!({
            "validator": {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": required,
                    "properties": props
                }
            }
        });
        return format!(
            "db.createCollection(\"{}\", {});",
            table.name,
            serde_json::to_string_pretty(&spec).unwrap_or_default()
        );
    }

    if dialect == DbKind::Redis {
        let sample_keys: Vec<&str> = table.columns.iter().take(10).map(|c| c.name.as_str()).collect();
        return format!(
            "# Redis Keyspace Group: {} ({} keys)\n# Sample keys: {}",
            table.name,
            table.row_count,
            sample_keys.join(", ")
        );
    }

    let qt = quote_dialect_ident(&table.name, dialect);
    let pk_cols: Vec<&ColumnDefinition> = table.columns.iter().filter(|c| c.is_primary_key).collect();
    let single_pk = pk_cols.len() == 1;

    let mut lines: Vec<String> = Vec::new();
    for col in &table.columns {
        let qc = quote_dialect_ident(&col.name, dialect);
        let mapped_type = map_column_type(&col.col_type, col, dialect);
        let mut parts = vec![format!("  {qc} {mapped_type}")];

        if single_pk && col.is_primary_key {
            if dialect == DbKind::Sqlite && col.auto_increment && mapped_type == "INTEGER" {
                parts.push("PRIMARY KEY AUTOINCREMENT".into());
            } else {
                parts.push("PRIMARY KEY".into());
            }
        } else {
            if !col.is_nullable {
                parts.push("NOT NULL".into());
            }
            if col.is_unique {
                parts.push("UNIQUE".into());
            }
        }

        if let Some(def) = col.default_value.as_deref().filter(|d| {
            !d.trim().is_empty() && !d.starts_with("nextval(")
        }) {
            parts.push(format!("DEFAULT {def}"));
        }

        lines.push(parts.join(" "));
    }

    if pk_cols.len() > 1 {
        let pk_list: Vec<String> = pk_cols
            .iter()
            .map(|c| quote_dialect_ident(&c.name, dialect))
            .collect();
        lines.push(format!("  PRIMARY KEY ({})", pk_list.join(", ")));
    }

    for fk in &table.foreign_keys {
        let q_col = quote_dialect_ident(&fk.column, dialect);
        let q_ref_tbl = quote_dialect_ident(&fk.referenced_table, dialect);
        let q_ref_col = quote_dialect_ident(&fk.referenced_column, dialect);
        let mut fk_clause = if let Some(cname) = fk.constraint_name.as_deref().filter(|s| !s.is_empty()) {
            let qc = quote_dialect_ident(cname, dialect);
            format!("  CONSTRAINT {qc} FOREIGN KEY ({q_col}) REFERENCES {q_ref_tbl} ({q_ref_col})")
        } else {
            format!("  FOREIGN KEY ({q_col}) REFERENCES {q_ref_tbl} ({q_ref_col})")
        };
        if let Some(od) = fk.on_delete.as_deref().filter(|s| !s.is_empty() && *s != "NO ACTION") {
            fk_clause.push_str(&format!(" ON DELETE {od}"));
        }
        if let Some(ou) = fk.on_update.as_deref().filter(|s| !s.is_empty() && *s != "NO ACTION") {
            fk_clause.push_str(&format!(" ON UPDATE {ou}"));
        }
        lines.push(fk_clause);
    }

    let mut out = if dialect == DbKind::Clickhouse {
        let order_col = pk_cols
            .first()
            .copied()
            .or_else(|| table.columns.first())
            .map(|c| quote_dialect_ident(&c.name, dialect))
            .unwrap_or_else(|| "tuple()".into());
        format!(
            "CREATE TABLE {qt} (\n{}\n) ENGINE = MergeTree() ORDER BY ({order_col});",
            lines.join(",\n")
        )
    } else {
        format!("CREATE TABLE {qt} (\n{}\n);", lines.join(",\n"))
    };

    if matches!(dialect, DbKind::Clickhouse | DbKind::Cassandra | DbKind::Scylladb | DbKind::Bigquery) {
        return out;
    }

    for idx in &table.indexes {
        if idx.is_primary || idx.columns.is_empty() || idx.name.starts_with("sqlite_autoindex") {
            continue;
        }
        let uniq = if idx.is_unique { "UNIQUE " } else { "" };
        let q_idx = quote_dialect_ident(&idx.name, dialect);
        let q_cols: Vec<String> = idx
            .columns
            .iter()
            .map(|c| quote_dialect_ident(c, dialect))
            .collect();
        out.push_str(&format!(
            "\nCREATE {uniq}INDEX {q_idx} ON {qt} ({});",
            q_cols.join(", ")
        ));
    }

    out
}

/// Generates a full schema DDL script ordered topologically so parent tables precede child FK tables.
pub fn generate_schema_ddl(tables: &[TableSchema], dialect: DbKind) -> String {
    let ordered = topological_sort_tables(tables);
    ordered
        .into_iter()
        .map(|t| generate_table_ddl(t, dialect, dialect == DbKind::Mysql && false))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn topological_sort_tables(tables: &[TableSchema]) -> Vec<&TableSchema> {
    let by_name: HashMap<&str, &TableSchema> =
        tables.iter().map(|t| (t.name.as_str(), t)).collect();
    let mut visited = HashSet::new();
    let mut out = Vec::with_capacity(tables.len());

    fn visit<'a>(
        t: &'a TableSchema,
        by_name: &HashMap<&'a str, &'a TableSchema>,
        visited: &mut HashSet<&'a str>,
        out: &mut Vec<&'a TableSchema>,
    ) {
        if !visited.insert(t.name.as_str()) {
            return;
        }
        for fk in &t.foreign_keys {
            if fk.referenced_table != t.name {
                if let Some(parent) = by_name.get(fk.referenced_table.as_str()) {
                    visit(parent, by_name, visited, out);
                }
            }
        }
        out.push(t);
    }

    for t in tables {
        visit(t, &by_name, &mut visited, &mut out);
    }
    out
}

/// Exports the schema as a Mermaid `erDiagram` markdown block.
pub fn generate_erd_mermaid(tables: &[TableSchema]) -> String {
    let mut out = String::from("erDiagram\n");
    for t in tables {
        let safe_t = sanitize_mermaid_id(&t.name);
        out.push_str(&format!("    {safe_t} {{\n"));
        for c in &t.columns {
            let safe_type = c
                .col_type
                .split_whitespace()
                .next()
                .unwrap_or("TEXT")
                .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_', "_");
            let safe_col = sanitize_mermaid_id(&c.name);
            let is_fk = t.foreign_keys.iter().any(|fk| fk.column == c.name);
            let tag = if c.is_primary_key {
                " PK"
            } else if is_fk {
                " FK"
            } else if c.is_unique {
                " UK"
            } else {
                ""
            };
            out.push_str(&format!("        {safe_type} {safe_col}{tag}\n"));
        }
        out.push_str("    }\n");
    }

    for t in tables {
        let child = sanitize_mermaid_id(&t.name);
        for fk in &t.foreign_keys {
            let parent = sanitize_mermaid_id(&fk.referenced_table);
            out.push_str(&format!(
                "    {parent} ||--o{{ {child} : \"{}\"\n",
                fk.column
            ));
        }
    }
    out
}

fn sanitize_mermaid_id(s: &str) -> String {
    let mapped: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if mapped.is_empty() {
        "entity".into()
    } else {
        mapped
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Exports a standalone dark-mode vector SVG Entity-Relationship Diagram with Bezier FK links.
pub fn generate_erd_svg(tables: &[TableSchema]) -> String {
    if tables.is_empty() {
        return r##"<svg xmlns="http://www.w3.org/2000/svg" width="480" height="120"><rect width="100%" height="100%" fill="#0f172a"/><text x="24" y="64" fill="#94a3b8" font-family="monospace" font-size="14">No tables in schema</text></svg>"##.into();
    }

    let cols_per_row = 3usize;
    let card_w = 240.0_f32;
    let gap_x = 80.0_f32;
    let gap_y = 48.0_f32;
    let pad = 32.0_f32;

    let mut positions: HashMap<&str, (f32, f32, f32)> = HashMap::new();
    let mut row_y = pad;
    let mut row_max_h = 0.0_f32;
    let mut max_w = 600.0_f32;

    for (idx, t) in tables.iter().enumerate() {
        let col_idx = idx % cols_per_row;
        if idx > 0 && col_idx == 0 {
            row_y += row_max_h + gap_y;
            row_max_h = 0.0;
        }
        let x = pad + (col_idx as f32) * (card_w + gap_x);
        let h = 36.0 + (t.columns.len().max(1) as f32) * 22.0 + 10.0;
        row_max_h = row_max_h.max(h);
        max_w = max_w.max(x + card_w + pad);
        positions.insert(t.name.as_str(), (x, row_y, h));
    }
    let total_h = (row_y + row_max_h + pad).max(220.0);

    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{max_w:.0}" height="{total_h:.0}" viewBox="0 0 {max_w:.0} {total_h:.0}">
  <rect width="100%" height="100%" fill="#0b1120"/>"##
    );

    // Draw Bezier FK lines behind cards
    for t in tables {
        let Some(&(cx, cy, _)) = positions.get(t.name.as_str()) else { continue };
        for (fk_i, fk) in t.foreign_keys.iter().enumerate() {
            if let Some(&(px, py, _)) = positions.get(fk.referenced_table.as_str()) {
                let sy = cy + 48.0 + (fk_i as f32 * 18.0);
                let ty = py + 48.0;
                let (sx, tx, c1x, c2x) = if cx >= px {
                    (cx, px + card_w, cx - 40.0, px + card_w + 40.0)
                } else {
                    (cx + card_w, px, cx + card_w + 40.0, px - 40.0)
                };
                svg.push_str(&format!(
                    "\n  <path d=\"M {sx:.1} {sy:.1} C {c1x:.1} {sy:.1}, {c2x:.1} {ty:.1}, {tx:.1} {ty:.1}\" fill=\"none\" stroke=\"#38bdf8\" stroke-width=\"2\" stroke-dasharray=\"5,3\"/>"
                ));
                svg.push_str(&format!(
                    "\n  <circle cx=\"{sx:.1}\" cy=\"{sy:.1}\" r=\"4\" fill=\"#f59e0b\"/><circle cx=\"{tx:.1}\" cy=\"{ty:.1}\" r=\"4\" fill=\"#10b981\"/>"
                ));
            }
        }
    }

    // Draw Table Cards
    for t in tables {
        let &(x, y, h) = &positions[t.name.as_str()];
        svg.push_str(&format!(
            "\n  <g transform=\"translate({x:.1},{y:.1})\">\n    <rect width=\"{card_w:.1}\" height=\"{h:.1}\" rx=\"8\" fill=\"#1e293b\" stroke=\"#334155\" stroke-width=\"1.5\"/>\n    <rect width=\"{card_w:.1}\" height=\"32\" rx=\"8\" fill=\"#1e3a8a\"/>\n    <text x=\"12\" y=\"21\" fill=\"#f8fafc\" font-family=\"monospace\" font-size=\"13\" font-weight=\"bold\">{}</text>",
            xml_escape(&t.name)
        ));
        for (ci, col) in t.columns.iter().enumerate() {
            let cy_row = 52.0 + (ci as f32) * 22.0;
            let is_fk = t.foreign_keys.iter().any(|fk| fk.column == col.name);
            let badge = if col.is_primary_key {
                "PK "
            } else if is_fk {
                "FK "
            } else {
                "   "
            };
            let color = if col.is_primary_key {
                "#fbbf24"
            } else if is_fk {
                "#38bdf8"
            } else {
                "#e2e8f0"
            };
            svg.push_str(&format!(
                "\n    <text x=\"12\" y=\"{cy_row:.1}\" fill=\"{color}\" font-family=\"monospace\" font-size=\"11.5\">{}{}</text>\n    <text x=\"{:.1}\" y=\"{cy_row:.1}\" fill=\"#94a3b8\" font-family=\"monospace\" font-size=\"10.5\" text-anchor=\"end\">{}</text>",
                badge,
                xml_escape(&col.name),
                card_w - 10.0,
                xml_escape(&col.col_type)
            ));
        }
        svg.push_str("\n  </g>");
    }

    svg.push_str("\n</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ForeignKeyDef, IndexDef};

    fn sample_schema() -> Vec<TableSchema> {
        let users = TableSchema {
            name: "users".into(),
            object_type: "TABLE".into(),
            row_count: 10,
            columns: vec![
                ColumnDefinition {
                    name: "id".into(),
                    col_type: "INTEGER".into(),
                    is_primary_key: true,
                    auto_increment: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "email".into(),
                    col_type: "TEXT".into(),
                    is_unique: true,
                    ..Default::default()
                },
            ],
            foreign_keys: vec![],
            indexes: vec![IndexDef {
                name: "idx_users_email".into(),
                columns: vec!["email".into()],
                is_unique: true,
                is_primary: false,
            }],
            ddl: None,
        };
        let orders = TableSchema {
            name: "orders".into(),
            object_type: "TABLE".into(),
            row_count: 25,
            columns: vec![
                ColumnDefinition {
                    name: "id".into(),
                    col_type: "INTEGER".into(),
                    is_primary_key: true,
                    auto_increment: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "user_id".into(),
                    col_type: "INTEGER".into(),
                    ..Default::default()
                },
            ],
            foreign_keys: vec![ForeignKeyDef {
                constraint_name: Some("fk_orders_user".into()),
                column: "user_id".into(),
                referenced_table: "users".into(),
                referenced_column: "id".into(),
                on_delete: Some("CASCADE".into()),
                on_update: None,
            }],
            indexes: vec![],
            ddl: None,
        };
        // Intentionally pass child `orders` first to test topological FK ordering!
        vec![orders, users]
    }

    #[test]
    fn generates_cross_dialect_ddl_mermaid_and_svg_in_topological_order() {
        let schema = sample_schema();
        let pg_ddl = generate_schema_ddl(&schema, DbKind::Postgres);
        let users_pos = pg_ddl.find("CREATE TABLE \"users\"").unwrap();
        let orders_pos = pg_ddl.find("CREATE TABLE \"orders\"").unwrap();
        assert!(users_pos < orders_pos, "parent table must precede FK child");
        assert!(pg_ddl.contains("SERIAL PRIMARY KEY"));
        assert!(pg_ddl.contains("REFERENCES \"users\" (\"id\") ON DELETE CASCADE"));

        let mysql_ddl = generate_schema_ddl(&schema, DbKind::Mysql);
        assert!(mysql_ddl.contains("AUTO_INCREMENT"));
        assert!(mysql_ddl.contains("VARCHAR(255) NOT NULL UNIQUE"));

        let mermaid = generate_erd_mermaid(&schema);
        assert!(mermaid.contains("erDiagram"));
        assert!(mermaid.contains("users ||--o{ orders : \"user_id\""));

        let svg = generate_erd_svg(&schema);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<path d=\"M"));
    }
}
