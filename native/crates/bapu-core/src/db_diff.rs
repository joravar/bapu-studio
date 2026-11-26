//! DBeaver PRO / Enterprise Parity:
//! 1. **Schema Compare & Forward Migration SQL Generator** (`compare_schemas`)
//! 2. **PK-Keyed Table Data Compare & Sync SQL Generator** (`compare_table_data`)

use crate::db_ddl::{generate_table_ddl, map_column_type, quote_dialect_ident};
use crate::types::{ColumnDefinition, DbKind, QueryResult, TableSchema};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnModification {
    pub column: String,
    pub source_type: String,
    pub target_type: String,
    pub source_nullable: bool,
    pub target_nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchemaDiff {
    pub table_name: String,
    pub added_columns: Vec<ColumnDefinition>,
    pub removed_columns: Vec<ColumnDefinition>,
    pub modified_columns: Vec<ColumnModification>,
    pub added_indexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiffReport {
    pub tables_only_in_source: Vec<String>,
    pub tables_only_in_target: Vec<String>,
    pub modified_tables: Vec<TableSchemaDiff>,
    pub identical_tables: usize,
    pub migration_sql: String,
}

/// Compares `source` schema (desired state, e.g. Dev/Staging) against `target` schema (e.g. Prod)
/// and generates the Forward Migration SQL script to bring `target` in sync with `source`.
pub fn compare_schemas(
    source: &[TableSchema],
    target: &[TableSchema],
    target_dialect: DbKind,
) -> SchemaDiffReport {
    let target_map: HashMap<&str, &TableSchema> =
        target.iter().map(|t| (t.name.as_str(), t)).collect();
    let source_map: HashMap<&str, &TableSchema> =
        source.iter().map(|t| (t.name.as_str(), t)).collect();

    let mut tables_only_in_source = Vec::new();
    let mut tables_only_in_target = Vec::new();
    let mut modified_tables = Vec::new();
    let mut identical_tables = 0usize;
    let mut sql_blocks: Vec<String> = Vec::new();

    for src_tbl in source {
        match target_map.get(src_tbl.name.as_str()) {
            None => {
                tables_only_in_source.push(src_tbl.name.clone());
                sql_blocks.push(format!(
                    "-- New table `{}`\n{}",
                    src_tbl.name,
                    generate_table_ddl(src_tbl, target_dialect, false)
                ));
            }
            Some(tgt_tbl) => {
                let tgt_cols: HashMap<&str, &ColumnDefinition> =
                    tgt_tbl.columns.iter().map(|c| (c.name.as_str(), c)).collect();
                let src_cols: HashMap<&str, &ColumnDefinition> =
                    src_tbl.columns.iter().map(|c| (c.name.as_str(), c)).collect();

                let mut added_columns = Vec::new();
                let mut removed_columns = Vec::new();
                let mut modified_columns = Vec::new();

                let qt = quote_dialect_ident(&src_tbl.name, target_dialect);

                for scol in &src_tbl.columns {
                    match tgt_cols.get(scol.name.as_str()) {
                        None => {
                            added_columns.push(scol.clone());
                            let qc = quote_dialect_ident(&scol.name, target_dialect);
                            let ctype = map_column_type(&scol.col_type, scol, target_dialect);
                            let null_sql = if scol.is_nullable {
                                ""
                            } else if let Some(def) = &scol.default_value {
                                &format!(" NOT NULL DEFAULT {def}")
                            } else {
                                ""
                            };
                            sql_blocks.push(format!(
                                "ALTER TABLE {qt} ADD COLUMN {qc} {ctype}{null_sql};"
                            ));
                        }
                        Some(tcol) => {
                            let s_norm = map_column_type(&scol.col_type, scol, target_dialect);
                            let t_norm = map_column_type(&tcol.col_type, tcol, target_dialect);
                            if s_norm != t_norm || scol.is_nullable != tcol.is_nullable {
                                modified_columns.push(ColumnModification {
                                    column: scol.name.clone(),
                                    source_type: scol.col_type.clone(),
                                    target_type: tcol.col_type.clone(),
                                    source_nullable: scol.is_nullable,
                                    target_nullable: tcol.is_nullable,
                                });
                                let qc = quote_dialect_ident(&scol.name, target_dialect);
                                match target_dialect {
                                    DbKind::Postgres => {
                                        if s_norm != t_norm {
                                            sql_blocks.push(format!(
                                                "ALTER TABLE {qt} ALTER COLUMN {qc} TYPE {s_norm};"
                                            ));
                                        }
                                        if scol.is_nullable != tcol.is_nullable {
                                            let op = if scol.is_nullable {
                                                "DROP NOT NULL"
                                            } else {
                                                "SET NOT NULL"
                                            };
                                            sql_blocks.push(format!(
                                                "ALTER TABLE {qt} ALTER COLUMN {qc} {op};"
                                            ));
                                        }
                                    }
                                    DbKind::Mysql => {
                                        let null_str =
                                            if scol.is_nullable { "NULL" } else { "NOT NULL" };
                                        sql_blocks.push(format!(
                                            "ALTER TABLE {qt} MODIFY COLUMN {qc} {s_norm} {null_str};"
                                        ));
                                    }
                                    _ => {
                                        sql_blocks.push(format!(
                                            "-- SQLite table rebuild required to alter {qt}.{qc} ({t_norm} -> {s_norm})"
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                for tcol in &tgt_tbl.columns {
                    if !src_cols.contains_key(tcol.name.as_str()) {
                        removed_columns.push(tcol.clone());
                        let qc = quote_dialect_ident(&tcol.name, target_dialect);
                        sql_blocks.push(format!(
                            "-- Destructive (review before running): ALTER TABLE {qt} DROP COLUMN {qc};"
                        ));
                    }
                }

                let tgt_idx_names: std::collections::HashSet<&str> =
                    tgt_tbl.indexes.iter().map(|i| i.name.as_str()).collect();
                let mut added_indexes = Vec::new();
                for sidx in &src_tbl.indexes {
                    if !sidx.is_primary
                        && !sidx.columns.is_empty()
                        && !tgt_idx_names.contains(sidx.name.as_str())
                    {
                        added_indexes.push(sidx.name.clone());
                        let uniq = if sidx.is_unique { "UNIQUE " } else { "" };
                        let qi = quote_dialect_ident(&sidx.name, target_dialect);
                        let qcols: Vec<String> = sidx
                            .columns
                            .iter()
                            .map(|c| quote_dialect_ident(c, target_dialect))
                            .collect();
                        sql_blocks.push(format!(
                            "CREATE {uniq}INDEX {qi} ON {qt} ({});",
                            qcols.join(", ")
                        ));
                    }
                }

                if added_columns.is_empty()
                    && removed_columns.is_empty()
                    && modified_columns.is_empty()
                    && added_indexes.is_empty()
                {
                    identical_tables += 1;
                } else {
                    modified_tables.push(TableSchemaDiff {
                        table_name: src_tbl.name.clone(),
                        added_columns,
                        removed_columns,
                        modified_columns,
                        added_indexes,
                    });
                }
            }
        }
    }

    for tgt_tbl in target {
        if !source_map.contains_key(tgt_tbl.name.as_str()) {
            tables_only_in_target.push(tgt_tbl.name.clone());
            let qt = quote_dialect_ident(&tgt_tbl.name, target_dialect);
            sql_blocks.push(format!(
                "-- Extra table in target (uncomment to drop): DROP TABLE {qt};"
            ));
        }
    }

    let migration_sql = if sql_blocks.is_empty() {
        "-- Schemas are 100% identical. No migration statements required.".into()
    } else {
        sql_blocks.join("\n\n")
    };

    SchemaDiffReport {
        tables_only_in_source,
        tables_only_in_target,
        modified_tables,
        identical_tables,
        migration_sql,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowCellDiff {
    pub column: String,
    pub source_val: serde_json::Value,
    pub target_val: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedRowDiff {
    pub pk_key: String,
    pub changes: Vec<RowCellDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDiffReport {
    pub identical_rows: usize,
    pub only_in_source: usize,
    pub only_in_target: usize,
    pub modified_rows: Vec<ModifiedRowDiff>,
    pub sync_sql: String,
}

fn sql_lit(v: &serde_json::Value, dialect: DbKind) -> String {
    match v {
        serde_json::Value::Null => "NULL".into(),
        serde_json::Value::Bool(b) => match dialect {
            DbKind::Sqlite | DbKind::Mysql => if *b { "1".into() } else { "0".into() },
            _ => if *b { "TRUE".into() } else { "FALSE".into() },
        },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn composite_pk_key(row: &HashMap<String, serde_json::Value>, pk_cols: &[String]) -> String {
    pk_cols
        .iter()
        .map(|k| match row.get(k) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => "NULL".into(),
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Compares two `QueryResult` datasets by `pk_cols` and generates `INSERT` / `UPDATE` / `DELETE`
/// SQL statements to synchronize `target` to match `source`.
pub fn compare_table_data(
    source: &QueryResult,
    target: &QueryResult,
    pk_cols: &[String],
    table_name: &str,
    target_dialect: DbKind,
) -> DataDiffReport {
    let effective_pks: Vec<String> = if !pk_cols.is_empty() {
        pk_cols.to_vec()
    } else if let Some(first) = source.columns.first() {
        vec![first.clone()]
    } else {
        vec![]
    };

    let mut src_index: BTreeMap<String, &HashMap<String, serde_json::Value>> = BTreeMap::new();
    for r in &source.rows {
        src_index.insert(composite_pk_key(r, &effective_pks), r);
    }
    let mut tgt_index: BTreeMap<String, &HashMap<String, serde_json::Value>> = BTreeMap::new();
    for r in &target.rows {
        tgt_index.insert(composite_pk_key(r, &effective_pks), r);
    }

    let qt = quote_dialect_ident(table_name, target_dialect);
    let mut identical_rows = 0usize;
    let mut only_in_source = 0usize;
    let mut only_in_target = 0usize;
    let mut modified_rows = Vec::new();
    let mut sync_stmts = Vec::new();

    for (pk, srow) in &src_index {
        match tgt_index.get(pk) {
            None => {
                only_in_source += 1;
                let cols: Vec<String> = source
                    .columns
                    .iter()
                    .map(|c| quote_dialect_ident(c, target_dialect))
                    .collect();
                let vals: Vec<String> = source
                    .columns
                    .iter()
                    .map(|c| sql_lit(srow.get(c).unwrap_or(&serde_json::Value::Null), target_dialect))
                    .collect();
                sync_stmts.push(format!(
                    "INSERT INTO {qt} ({}) VALUES ({});",
                    cols.join(", "),
                    vals.join(", ")
                ));
            }
            Some(trow) => {
                let mut changes = Vec::new();
                for col in &source.columns {
                    let sv = srow.get(col).unwrap_or(&serde_json::Value::Null);
                    let tv = trow.get(col).unwrap_or(&serde_json::Value::Null);
                    if sv != tv {
                        changes.push(RowCellDiff {
                            column: col.clone(),
                            source_val: sv.clone(),
                            target_val: tv.clone(),
                        });
                    }
                }
                if changes.is_empty() {
                    identical_rows += 1;
                } else {
                    let set_sql = changes
                        .iter()
                        .map(|ch| {
                            format!(
                                "{} = {}",
                                quote_dialect_ident(&ch.column, target_dialect),
                                sql_lit(&ch.source_val, target_dialect)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let where_sql = effective_pks
                        .iter()
                        .map(|k| {
                            format!(
                                "{} = {}",
                                quote_dialect_ident(k, target_dialect),
                                sql_lit(srow.get(k).unwrap_or(&serde_json::Value::Null), target_dialect)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    sync_stmts.push(format!("UPDATE {qt} SET {set_sql} WHERE {where_sql};"));
                    modified_rows.push(ModifiedRowDiff {
                        pk_key: pk.clone(),
                        changes,
                    });
                }
            }
        }
    }

    for (pk, trow) in &tgt_index {
        if !src_index.contains_key(pk) {
            only_in_target += 1;
            let where_sql = effective_pks
                .iter()
                .map(|k| {
                    format!(
                        "{} = {}",
                        quote_dialect_ident(k, target_dialect),
                        sql_lit(trow.get(k).unwrap_or(&serde_json::Value::Null), target_dialect)
                    )
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            sync_stmts.push(format!("DELETE FROM {qt} WHERE {where_sql};"));
        }
    }

    let sync_sql = if sync_stmts.is_empty() {
        "-- Table data is 100% identical across source and target.".into()
    } else {
        sync_stmts.join("\n")
    };

    DataDiffReport {
        identical_rows,
        only_in_source,
        only_in_target,
        modified_rows,
        sync_sql,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_compare_detects_added_tables_columns_and_type_changes() {
        let src = vec![
            TableSchema {
                name: "users".into(),
                columns: vec![
                    ColumnDefinition { name: "id".into(), col_type: "INTEGER".into(), is_primary_key: true, ..Default::default() },
                    ColumnDefinition { name: "email".into(), col_type: "VARCHAR(255)".into(), ..Default::default() },
                    ColumnDefinition { name: "tier".into(), col_type: "TEXT".into(), is_nullable: true, ..Default::default() },
                ],
                ..Default::default()
            },
            TableSchema {
                name: "audit_logs".into(),
                columns: vec![ColumnDefinition { name: "id".into(), col_type: "INTEGER".into(), is_primary_key: true, ..Default::default() }],
                ..Default::default()
            },
        ];
        let tgt = vec![TableSchema {
            name: "users".into(),
            columns: vec![
                ColumnDefinition { name: "id".into(), col_type: "INTEGER".into(), is_primary_key: true, ..Default::default() },
                ColumnDefinition { name: "email".into(), col_type: "VARCHAR(100)".into(), ..Default::default() },
            ],
            ..Default::default()
        }];

        let report = compare_schemas(&src, &tgt, DbKind::Postgres);
        assert_eq!(report.tables_only_in_source, vec!["audit_logs"]);
        assert_eq!(report.modified_tables.len(), 1);
        assert_eq!(report.modified_tables[0].added_columns.len(), 1);
        assert_eq!(report.modified_tables[0].modified_columns.len(), 1);
        assert!(report.migration_sql.contains("CREATE TABLE \"audit_logs\""));
        assert!(report.migration_sql.contains("ALTER TABLE \"users\" ADD COLUMN \"tier\""));
    }

    #[test]
    fn data_compare_generates_insert_update_and_delete_sync_sql() {
        let src = QueryResult {
            columns: vec!["id".into(), "name".into()],
            rows: vec![
                HashMap::from([("id".into(), serde_json::json!(1)), ("name".into(), serde_json::json!("Ada Updated"))]),
                HashMap::from([("id".into(), serde_json::json!(3)), ("name".into(), serde_json::json!("Linus"))]),
            ],
            row_count: 2,
            execution_time_ms: 1,
            error: None,
            notices: vec![],
            query_plan: None,
            command: None,
        };
        let tgt = QueryResult {
            columns: vec!["id".into(), "name".into()],
            rows: vec![
                HashMap::from([("id".into(), serde_json::json!(1)), ("name".into(), serde_json::json!("Ada"))]),
                HashMap::from([("id".into(), serde_json::json!(2)), ("name".into(), serde_json::json!("Grace"))]),
            ],
            row_count: 2,
            execution_time_ms: 1,
            error: None,
            notices: vec![],
            query_plan: None,
            command: None,
        };

        let diff = compare_table_data(&src, &tgt, &["id".into()], "users", DbKind::Postgres);
        assert_eq!(diff.only_in_source, 1);
        assert_eq!(diff.only_in_target, 1);
        assert_eq!(diff.modified_rows.len(), 1);
        assert!(diff.sync_sql.contains("UPDATE \"users\" SET \"name\" = 'Ada Updated' WHERE \"id\" = 1;"));
        assert!(diff.sync_sql.contains("INSERT INTO \"users\""));
        assert!(diff.sync_sql.contains("DELETE FROM \"users\" WHERE \"id\" = 2;"));
    }
}
