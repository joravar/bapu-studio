//! DBeaver PRO Parity: Smart Mock Data Generator (`db_mock_gen.rs`).
//! Semantic column-name & SQL-type inference, collision-free `UNIQUE` column suffixes,
//! and Foreign-Key parent pool awareness so generated child rows never violate FK constraints.

use crate::types::{ColumnDefinition, MutationOp, QueryResult, RowMutation, TableSchema};
use rand::Rng;
use std::collections::HashMap;

const FIRST_NAMES: &[&str] = &[
    "Ada", "Grace", "Linus", "Margaret", "Alan", "Katherine", "Donald", "Barbara", "Ken", "Radia",
    "guido", "Bjarne", "Adele", "Leslie", "Shafi", "silvio",
];
const LAST_NAMES: &[&str] = &[
    "Lovelace", "Hopper", "Torvalds", "Hamilton", "Turing", "Johnson", "Knuth", "Liskov",
    "Thompson", "Perlman", "Ritchie", "Goldberg", "Lamport", "Diffie",
];
const CITIES: &[&str] = &[
    "San Francisco", "New York", "London", "Tokyo", "Bengaluru", "Zurich", "Toronto", "Berlin",
    "Seattle", "Austin", "Singapore", "Dublin",
];
const COUNTRIES: &[&str] = &[
    "US", "GB", "JP", "IN", "CH", "CA", "DE", "SG", "IE", "AU",
];
const COMPANIES: &[&str] = &[
    "Bapu Labs", "Acme Cloud", "Hyperion Systems", "Vertex Data", "Nebula Robotics",
    "Quantum Grid", "Apex Telemetry", "Helios AI",
];
const STATUSES: &[&str] = &["active", "paid", "processing", "pending", "completed"];
const PLANS: &[&str] = &["Free", "Starter", "Pro", "Enterprise"];

/// Generates `row_count` realistic `RowMutation::Insert` operations for `table`.
/// `fk_pools` maps `column_name -> Vec<valid_parent_pk_values>` so FK columns always reference real rows!
pub fn generate_mock_mutations(
    table: &TableSchema,
    row_count: usize,
    fk_pools: &HashMap<String, Vec<serde_json::Value>>,
    include_auto_pk: bool,
) -> Vec<RowMutation> {
    let mut rng = rand::thread_rng();
    let batch_seed: u32 = rng.gen_range(1000..9999);
    let mut mutations = Vec::with_capacity(row_count);

    for idx in 0..row_count {
        let seq = (idx + 1) as i64;
        let mut values = HashMap::new();

        for col in &table.columns {
            if col.name == "_id" {
                continue;
            }
            if col.is_primary_key && col.auto_increment && !include_auto_pk {
                continue;
            }

            // 1. Foreign Key pool takes highest priority so relational integrity is guaranteed!
            if let Some(pool) = fk_pools.get(&col.name).filter(|p| !p.is_empty()) {
                let chosen = &pool[rng.gen_range(0..pool.len())];
                values.insert(col.name.clone(), chosen.clone());
                continue;
            }

            let val = generate_column_value(col, seq, batch_seed, &mut rng);
            values.insert(col.name.clone(), val);
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

/// Converts generated mock mutations into a `QueryResult` preview so users can inspect or export SQL INSERTs!
pub fn mutations_to_preview_result(table: &TableSchema, mutations: &[RowMutation]) -> QueryResult {
    let columns: Vec<String> = table
        .columns
        .iter()
        .filter(|c| mutations.first().is_some_and(|m| m.values.contains_key(&c.name)))
        .map(|c| c.name.clone())
        .collect();
    let rows: Vec<HashMap<String, serde_json::Value>> =
        mutations.iter().map(|m| m.values.clone()).collect();
    QueryResult {
        row_count: rows.len() as u64,
        columns,
        rows,
        execution_time_ms: 0,
        error: None,
        notices: vec![],
        query_plan: None,
        command: Some("MOCK_PREVIEW".into()),
    }
}

fn generate_column_value(
    col: &ColumnDefinition,
    seq: i64,
    batch_seed: u32,
    rng: &mut impl Rng,
) -> serde_json::Value {
    let name_lc = col.name.to_ascii_lowercase();
    let type_up = col.col_type.to_ascii_uppercase();

    if col.is_primary_key {
        if type_up.contains("INT") || type_up.contains("SERIAL") {
            return serde_json::json!(seq + (batch_seed as i64) * 10);
        }
        return serde_json::json!(uuid::Uuid::new_v4().to_string());
    }

    let fname = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())];
    let lname = LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())];

    // Numeric SQL types take priority over generic substring heuristics unless boolean
    if type_up == "BOOLEAN" || type_up == "BOOL" || type_up == "TINYINT(1)" || name_lc.starts_with("is_") || name_lc.starts_with("has_") {
        if type_up.contains("INT") && type_up != "TINYINT(1)" {
            return serde_json::json!(if rng.gen_bool(0.75) { 1 } else { 0 });
        }
        return serde_json::json!(rng.gen_bool(0.75));
    }

    if type_up.contains("INT") {
        if name_lc.contains("price") || name_lc.contains("amount") || name_lc.contains("cents") || name_lc.contains("cost") {
            return serde_json::json!(rng.gen_range(500..=50_000));
        }
        if name_lc.contains("age") {
            return serde_json::json!(rng.gen_range(18..=72));
        }
        if name_lc.contains("qty") || name_lc.contains("count") || name_lc.contains("stock") {
            return serde_json::json!(rng.gen_range(1..=250));
        }
        if col.is_unique {
            return serde_json::json!(batch_seed as i64 * 10_000 + seq);
        }
        return serde_json::json!(rng.gen_range(1..=1000));
    }

    if type_up.contains("REAL")
        || type_up.contains("FLOA")
        || type_up.contains("DOUB")
        || type_up.contains("NUMERIC")
        || type_up.contains("DECIMAL")
    {
        let raw: f64 = rng.gen_range(10.0..=999.99);
        let rounded = (raw * 100.0).round() / 100.0;
        return serde_json::json!(rounded);
    }

    if type_up.contains("JSON") {
        return serde_json::json!({ "source": "bapu_mock", "seq": seq });
    }

    if type_up == "DATE" || name_lc.ends_with("_date") || name_lc == "birth_date" {
        let month = rng.gen_range(1..=12);
        let day = rng.gen_range(1..=28);
        return serde_json::json!(format!("2026-{month:02}-{day:02}"));
    }

    if type_up.contains("TIME") || name_lc.ends_with("_at") {
        return serde_json::json!(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
    }

    // String / Text semantic heuristics
    let uniq_suffix = if col.is_unique {
        format!("_{batch_seed}_{seq}")
    } else {
        String::new()
    };

    if name_lc.contains("email") {
        return serde_json::json!(format!(
            "{}.{}{}@bapustudio.dev",
            fname.to_ascii_lowercase(),
            lname.to_ascii_lowercase(),
            if col.is_unique { format!(".{batch_seed}.{seq}") } else { format!("{seq}") }
        ));
    }
    if name_lc == "first_name" || name_lc == "firstname" {
        return serde_json::json!(format!("{fname}{uniq_suffix}"));
    }
    if name_lc == "last_name" || name_lc == "lastname" {
        return serde_json::json!(format!("{lname}{uniq_suffix}"));
    }
    if name_lc.contains("username") || name_lc == "slug" || name_lc == "handle" {
        return serde_json::json!(format!(
            "{}_{}_{}",
            fname.to_ascii_lowercase(),
            batch_seed,
            seq
        ));
    }
    if name_lc.contains("sku") || name_lc.contains("code") {
        return serde_json::json!(format!("SKU-{batch_seed}-{seq:04}"));
    }
    if name_lc.contains("name") || name_lc == "author" {
        return serde_json::json!(format!("{fname} {lname}{uniq_suffix}"));
    }
    if name_lc.contains("phone") || name_lc.contains("mobile") {
        return serde_json::json!(format!("+1-415-555-{:04}", rng.gen_range(1000..=9999)));
    }
    if name_lc.contains("city") {
        return serde_json::json!(CITIES[rng.gen_range(0..CITIES.len())]);
    }
    if name_lc.contains("country") {
        return serde_json::json!(COUNTRIES[rng.gen_range(0..COUNTRIES.len())]);
    }
    if name_lc.contains("company") || name_lc.contains("org") {
        return serde_json::json!(COMPANIES[rng.gen_range(0..COMPANIES.len())]);
    }
    if name_lc.contains("status") || name_lc.contains("state") {
        return serde_json::json!(STATUSES[rng.gen_range(0..STATUSES.len())]);
    }
    if name_lc.contains("plan") || name_lc.contains("tier") {
        return serde_json::json!(PLANS[rng.gen_range(0..PLANS.len())]);
    }
    if name_lc.contains("url") || name_lc.contains("website") {
        return serde_json::json!(format!("https://api.bapustudio.dev/items/{seq}"));
    }
    if name_lc.contains("ip") {
        return serde_json::json!(format!(
            "10.{}.{}.{}",
            rng.gen_range(0..=254),
            rng.gen_range(0..=254),
            rng.gen_range(1..=254)
        ));
    }
    if name_lc.contains("uuid") || name_lc.contains("guid") {
        return serde_json::json!(uuid::Uuid::new_v4().to_string());
    }

    serde_json::json!(format!("sample_{}_{}_{}", col.name, batch_seed, seq))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ForeignKeyDef;
    use std::collections::HashSet;

    #[test]
    fn generates_collision_free_unique_values_and_honors_fk_parent_pools() {
        let tbl = TableSchema {
            name: "orders".into(),
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
                ColumnDefinition {
                    name: "sku".into(),
                    col_type: "TEXT".into(),
                    is_unique: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "customer_email".into(),
                    col_type: "TEXT".into(),
                    is_unique: true,
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "amount_cents".into(),
                    col_type: "INTEGER".into(),
                    ..Default::default()
                },
            ],
            foreign_keys: vec![ForeignKeyDef {
                column: "user_id".into(),
                referenced_table: "users".into(),
                referenced_column: "id".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let mut pools = HashMap::new();
        pools.insert("user_id".into(), vec![serde_json::json!(101), serde_json::json!(202)]);

        let muts = generate_mock_mutations(&tbl, 50, &pools, false);
        assert_eq!(muts.len(), 50);

        let mut skus = HashSet::new();
        let mut emails = HashSet::new();
        for m in &muts {
            assert!(!m.values.contains_key("id"), "auto-increment PK skipped");
            let uid = m.values["user_id"].as_i64().unwrap();
            assert!(uid == 101 || uid == 202, "FK value must come from parent pool");
            assert!(skus.insert(m.values["sku"].as_str().unwrap().to_string()));
            assert!(emails.insert(m.values["customer_email"].as_str().unwrap().to_string()));
        }
    }
}
