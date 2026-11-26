//! Comprehensive DBeaver Community + PRO / Enterprise Parity End-to-End Test Suite.
//! 1. Full SQLite relational + view + FK + index + DDL + SQL formatter + `:param` + Mock Gen +
//!    8-format Export + CSV Import + Schema Compare (with live migration!) + Data Compare (with live sync!) + Health
//! 2. Live TCP Server E2E verification against real PostgreSQL, MySQL, MongoDB, and Redis containers
//!    when `BAPU_TEST_PG_URL`, `BAPU_TEST_MYSQL_URL`, `BAPU_TEST_MONGO_URL`, `BAPU_TEST_REDIS_URL` are set.

use bapu_core::db::driver_for;
use bapu_core::db_admin::fetch_server_health;
use bapu_core::db_ddl::{
    generate_erd_mermaid, generate_erd_svg, generate_schema_ddl, generate_table_ddl,
};
use bapu_core::db_diff::{compare_schemas, compare_table_data};
use bapu_core::db_export_import::{
    build_import_mutations, export_query_result, filter_and_sort_row_indices, parse_import_payload,
    ExportFormat, GridFilterOp,
};
use bapu_core::db_mock_gen::generate_mock_mutations;
use bapu_core::db_sql_tools::{
    bind_sql_params, compute_column_stats, execute_sql_script, extract_sql_params, format_sql,
};
use bapu_core::types::{DatabaseConnection, DbKind, MutationOp, RowMutation};
use std::collections::HashMap;

fn make_conn(id: &str, kind: DbKind, url_or_db: &str, db_name: &str) -> DatabaseConnection {
    DatabaseConnection {
        id: id.into(),
        name: id.into(),
        kind,
        connection_string: if kind == DbKind::Sqlite && !url_or_db.starts_with("sqlite:") {
            None
        } else {
            Some(url_or_db.into())
        },
        host: None,
        port: None,
        database: db_name.into(),
        username: None,
        password: None,
        ssl: false,
        ssl_ca_cert: None,
        ssl_client_cert: None,
        ssl_client_key: None,
        ssl_reject_unauthorized: Some(false),
        ssh_enabled: false,
        ssh_host: None,
        ssh_port: None,
        ssh_username: None,
        ssh_password: None,
        ssh_private_key: None,
        ssh_passphrase: None,
        is_connected: false,
        is_demo_db: false,
        tables: vec![],
    }
}

#[tokio::test]
async fn e2e_dbeaver_full_sqlite_erd_ddl_diff_import_export_mock_and_admin() {
    let src_file = std::env::temp_dir().join(format!("bapu_dbv_src_{}.sqlite", uuid::Uuid::new_v4()));
    let tgt_file = std::env::temp_dir().join(format!("bapu_dbv_tgt_{}.sqlite", uuid::Uuid::new_v4()));

    let src_cfg = make_conn(
        "sqlite-src",
        DbKind::Sqlite,
        &format!("sqlite://{}?mode=rwc", src_file.to_string_lossy().replace('\\', "/")),
        "main",
    );
    let tgt_cfg = make_conn(
        "sqlite-tgt",
        DbKind::Sqlite,
        &format!("sqlite://{}?mode=rwc", tgt_file.to_string_lossy().replace('\\', "/")),
        "main",
    );

    let driver = driver_for(DbKind::Sqlite);

    // 1. Multi-statement SQL script execution creating tables, FK, unique index, and view
    let setup_script = r#"
        -- Create parent & child tables with FK + Index + View
        CREATE TABLE departments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL
        );
        CREATE TABLE employees (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            dept_id INTEGER NOT NULL,
            email TEXT NOT NULL UNIQUE,
            full_name TEXT NOT NULL,
            salary_cents INTEGER NOT NULL DEFAULT 100000,
            FOREIGN KEY (dept_id) REFERENCES departments(id) ON DELETE CASCADE
        );
        CREATE INDEX idx_emp_dept ON employees(dept_id);
        CREATE VIEW staff_summary AS
            SELECT e.id, e.full_name, d.name AS dept_name, e.salary_cents
            FROM employees e JOIN departments d ON d.id = e.dept_id;
        INSERT INTO departments (code, name) VALUES ('ENG', 'Engineering'), ('RES', 'Research');
    "#;
    let script_rep = execute_sql_script(&*driver, &src_cfg, setup_script, true)
        .await
        .unwrap();
    assert_eq!(script_rep.failed, 0);
    assert_eq!(script_rep.succeeded, 5);

    // 2. Deep Schema Introspection (Tables, Views, PKs, Auto-Inc, Unique, FKs, Indexes, DDL)
    let src_schema = driver.get_schema(&src_cfg).await.unwrap();
    assert_eq!(src_schema.len(), 3); // 2 tables + 1 view
    let emp_tbl = src_schema.iter().find(|t| t.name == "employees").unwrap();
    assert_eq!(emp_tbl.object_type, "TABLE");
    assert_eq!(emp_tbl.foreign_keys.len(), 1);
    assert_eq!(emp_tbl.foreign_keys[0].referenced_table, "departments");
    assert!(emp_tbl.indexes.iter().any(|i| i.name == "idx_emp_dept"));
    assert!(emp_tbl.ddl.as_deref().unwrap_or("").contains("FOREIGN KEY"));
    let view_obj = src_schema.iter().find(|t| t.name == "staff_summary").unwrap();
    assert_eq!(view_obj.object_type, "VIEW");

    // 3. Cross-Dialect DDL + Mermaid ERD + Vector SVG ERD
    for dialect in [DbKind::Postgres, DbKind::Mysql, DbKind::Sqlite, DbKind::Mongodb] {
        let ddl = generate_schema_ddl(&src_schema, dialect);
        assert!(!ddl.is_empty());
        let single_ddl = generate_table_ddl(emp_tbl, dialect, false);
        assert!(!single_ddl.is_empty());
    }
    let mmd = generate_erd_mermaid(&src_schema);
    assert!(mmd.contains("departments ||--o{ employees : \"dept_id\""));
    let svg = generate_erd_svg(&src_schema);
    assert!(svg.contains("<svg") && svg.contains("employees"));

    // 4. FK-Aware Smart Mock Data Generator (insert 20 valid child employees!)
    let mut fk_pools = HashMap::new();
    fk_pools.insert("dept_id".into(), vec![serde_json::json!(1), serde_json::json!(2)]);
    let mock_muts = generate_mock_mutations(emp_tbl, 20, &fk_pools, false);
    assert_eq!(mock_muts.len(), 20);
    let batch_res = driver
        .mutate_batch(&src_cfg, "employees", &mock_muts)
        .await
        .unwrap();
    assert!(batch_res.atomic);
    assert_eq!(batch_res.results.len(), 20);

    // 5. SQL Formatter + `:param` Binding + View Query + Filter/Sort + Column Stats
    let raw_param_sql = "select id, full_name, dept_name, salary_cents from staff_summary where dept_name = :dept order by id asc;";
    let formatted = format_sql(raw_param_sql);
    assert!(formatted.starts_with("SELECT id, full_name"));
    assert_eq!(extract_sql_params(&formatted), vec!["dept".to_string()]);
    let bound_sql = bind_sql_params(
        &formatted,
        &HashMap::from([("dept".into(), "Engineering".into())]),
    );
    let view_qr = driver.query(&src_cfg, &bound_sql).await.unwrap();
    let all_emp_qr = driver
        .query(&src_cfg, "SELECT * FROM employees ORDER BY id")
        .await
        .unwrap();
    assert_eq!(all_emp_qr.row_count, 20);
    let _ = view_qr;

    let stats = compute_column_stats(&all_emp_qr);
    let email_stat = stats.iter().find(|s| s.column == "email").unwrap();
    assert_eq!(email_stat.distinct_count, 20);

    let sorted_desc = filter_and_sort_row_indices(
        &all_emp_qr,
        "",
        None,
        GridFilterOp::Contains,
        "",
        Some("id"),
        false,
    );
    assert_eq!(sorted_desc.first().copied(), Some(19));

    // 6. All 8 Export Formats + CSV Data Import Wizard into Target DB
    for fmt in ExportFormat::ALL {
        let out = export_query_result(&all_emp_qr, "employees", DbKind::Sqlite, fmt);
        assert!(!out.is_empty());
    }

    // Seed target DB with older schema (missing `employees` and missing `bonus` col on `departments`)
    driver
        .query(
            &tgt_cfg,
            "CREATE TABLE departments (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL UNIQUE, name TEXT NOT NULL)",
        )
        .await
        .unwrap();

    // Import departments from CSV into target DB via CSV Import + Column Mapping!
    let dept_qr = driver
        .query(&src_cfg, "SELECT id, code, name FROM departments ORDER BY id")
        .await
        .unwrap();
    let dept_csv = export_query_result(&dept_qr, "departments", DbKind::Sqlite, ExportFormat::Csv);
    let (_, parsed_csv_rows) = parse_import_payload(&dept_csv).unwrap();
    let imp_muts = build_import_mutations(&parsed_csv_rows, &HashMap::new());
    driver
        .mutate_batch(&tgt_cfg, "departments", &imp_muts)
        .await
        .unwrap();

    // 7. Schema Compare & Live Forward Migration Execution!
    let src_tables_only: Vec<_> = src_schema
        .iter()
        .filter(|t| t.object_type == "TABLE")
        .cloned()
        .collect();
    let tgt_schema_before = driver.get_schema(&tgt_cfg).await.unwrap();
    let diff = compare_schemas(&src_tables_only, &tgt_schema_before, DbKind::Sqlite);
    assert_eq!(diff.tables_only_in_source, vec!["employees"]);
    assert!(diff.migration_sql.contains("CREATE TABLE \"employees\""));

    let mig_rep = execute_sql_script(&*driver, &tgt_cfg, &diff.migration_sql, true)
        .await
        .unwrap();
    assert_eq!(mig_rep.failed, 0);

    let tgt_schema_after = driver.get_schema(&tgt_cfg).await.unwrap();
    let diff_after = compare_schemas(&src_tables_only, &tgt_schema_after, DbKind::Sqlite);
    assert!(diff_after.tables_only_in_source.is_empty());

    // 8. PK Table Data Compare & Live Sync SQL Execution!
    // Modify target `departments` so Data Compare has UPDATE + INSERT + DELETE work to do!
    driver
        .query(&tgt_cfg, "UPDATE departments SET name = 'Old Eng' WHERE id = 1")
        .await
        .unwrap();
    driver
        .query(&tgt_cfg, "DELETE FROM departments WHERE id = 2")
        .await
        .unwrap();
    driver
        .query(&tgt_cfg, "INSERT INTO departments (id, code, name) VALUES (99, 'TEMP', 'Temp')")
        .await
        .unwrap();

    let tgt_dept_before = driver
        .query(&tgt_cfg, "SELECT id, code, name FROM departments ORDER BY id")
        .await
        .unwrap();
    let data_diff = compare_table_data(
        &dept_qr,
        &tgt_dept_before,
        &["id".into()],
        "departments",
        DbKind::Sqlite,
    );
    assert_eq!(data_diff.only_in_source, 1); // id=2
    assert_eq!(data_diff.only_in_target, 1); // id=99
    assert_eq!(data_diff.modified_rows.len(), 1); // id=1

    let sync_rep = execute_sql_script(&*driver, &tgt_cfg, &data_diff.sync_sql, true)
        .await
        .unwrap();
    assert_eq!(sync_rep.failed, 0);

    let tgt_dept_after = driver
        .query(&tgt_cfg, "SELECT id, code, name FROM departments ORDER BY id")
        .await
        .unwrap();
    let data_diff_after = compare_table_data(
        &dept_qr,
        &tgt_dept_after,
        &["id".into()],
        "departments",
        DbKind::Sqlite,
    );
    assert_eq!(data_diff_after.only_in_source, 0);
    assert_eq!(data_diff_after.only_in_target, 0);
    assert!(data_diff_after.modified_rows.is_empty());
    assert_eq!(data_diff_after.identical_rows, 2);

    // 9. Server Health Diagnostics
    let health = fetch_server_health(&*driver, &src_cfg).await.unwrap();
    assert!(health.engine_banner.contains("SQLite"));
    assert!(!health.metrics.is_empty());

    drop(driver);
    let _ = std::fs::remove_file(&src_file);
    let _ = std::fs::remove_file(&tgt_file);
}

#[tokio::test]
async fn e2e_live_postgres_mysql_mongodb_and_redis_when_available() {
    // 1. Live PostgreSQL E2E
    if let Ok(pg_url) = std::env::var("BAPU_TEST_PG_URL") {
        let cfg = make_conn("pg-live", DbKind::Postgres, &pg_url, "postgres");
        let drv = driver_for(DbKind::Postgres);
        let banner = drv.test_connection(&cfg).await.expect("Postgres test_connection");
        assert!(banner.contains("PostgreSQL"));

        let script = r#"
            DROP VIEW IF EXISTS v_active_projects;
            DROP TABLE IF EXISTS tasks;
            DROP TABLE IF EXISTS projects;
            CREATE TABLE projects (
                id SERIAL PRIMARY KEY,
                slug TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL
            );
            CREATE TABLE tasks (
                id SERIAL PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                estimate_pts INTEGER NOT NULL DEFAULT 3
            );
            CREATE INDEX idx_tasks_proj ON tasks(project_id);
            CREATE VIEW v_active_projects AS SELECT id, slug, title FROM projects;
            INSERT INTO projects (slug, title) VALUES ('apollo', 'Apollo Guidance'), ('bapu', 'Bapu Native');
        "#;
        let rep = execute_sql_script(&*drv, &cfg, script, true).await.unwrap();
        assert_eq!(rep.failed, 0, "Postgres script items: {:?}", rep.items);

        let schema = drv.get_schema(&cfg).await.unwrap();
        let proj = schema.iter().find(|t| t.name == "projects").expect("projects table");
        assert!(proj.columns.iter().any(|c| c.name == "id" && c.is_primary_key && c.auto_increment));
        assert_eq!(proj.row_count, 2);
        let tasks = schema.iter().find(|t| t.name == "tasks").expect("tasks table");
        assert_eq!(tasks.foreign_keys.len(), 1);
        assert_eq!(tasks.foreign_keys[0].referenced_table, "projects");
        assert!(tasks.indexes.iter().any(|i| i.name == "idx_tasks_proj"));
        assert!(schema.iter().any(|t| t.name == "v_active_projects" && t.object_type == "VIEW"));

        // FK-aware mock data generation + atomic commit on Postgres
        let mut pools = HashMap::new();
        pools.insert("project_id".into(), vec![serde_json::json!(1), serde_json::json!(2)]);
        let mock_tasks = generate_mock_mutations(tasks, 10, &pools, false);
        let mb = drv.mutate_batch(&cfg, "tasks", &mock_tasks).await.unwrap();
        assert!(mb.atomic && mb.results.len() == 10);

        // Atomic rollback check on Postgres (duplicate UNIQUE slug)
        let bad = drv
            .mutate_batch(
                &cfg,
                "projects",
                &[
                    RowMutation {
                        op: MutationOp::Insert,
                        values: HashMap::from([
                            ("slug".into(), serde_json::json!("new_valid_slug")),
                            ("title".into(), serde_json::json!("Should Rollback")),
                        ]),
                        where_clause: HashMap::new(),
                    },
                    RowMutation {
                        op: MutationOp::Insert,
                        values: HashMap::from([
                            ("slug".into(), serde_json::json!("apollo")), // duplicate!
                            ("title".into(), serde_json::json!("Dup")),
                        ]),
                        where_clause: HashMap::new(),
                    },
                ],
            )
            .await;
        assert!(bad.is_err());
        let cnt = drv.query(&cfg, "SELECT COUNT(*)::bigint AS n FROM projects").await.unwrap();
        assert_eq!(cnt.rows[0]["n"], serde_json::json!(2));

        let plan = drv
            .query(&cfg, "EXPLAIN (FORMAT JSON) SELECT * FROM tasks WHERE project_id = 1")
            .await
            .unwrap();
        assert!(plan.query_plan.is_some());

        let health = fetch_server_health(&*drv, &cfg).await.unwrap();
        assert!(health.engine_banner.contains("PostgreSQL"));
    }

    // 2. Live MySQL E2E
    if let Ok(mysql_url) = std::env::var("BAPU_TEST_MYSQL_URL") {
        let cfg = make_conn("mysql-live", DbKind::Mysql, &mysql_url, "testdb");
        let drv = driver_for(DbKind::Mysql);
        let banner = drv.test_connection(&cfg).await.expect("MySQL test_connection");
        assert!(banner.contains("MySQL"));

        let script = r#"
            DROP VIEW IF EXISTS v_customers;
            DROP TABLE IF EXISTS invoices;
            DROP TABLE IF EXISTS customers;
            CREATE TABLE customers (
                id INT AUTO_INCREMENT PRIMARY KEY,
                email VARCHAR(190) NOT NULL UNIQUE,
                company VARCHAR(190) NOT NULL
            );
            CREATE TABLE invoices (
                id INT AUTO_INCREMENT PRIMARY KEY,
                customer_id INT NOT NULL,
                total_cents INT NOT NULL,
                INDEX idx_inv_cust (customer_id),
                CONSTRAINT fk_inv_cust FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE CASCADE
            );
            CREATE VIEW v_customers AS SELECT id, email, company FROM customers;
            INSERT INTO customers (email, company) VALUES ('ops@acme.io', 'Acme'), ('eng@bapu.dev', 'Bapu');
        "#;
        let rep = execute_sql_script(&*drv, &cfg, script, true).await.unwrap();
        assert_eq!(rep.failed, 0, "MySQL script items: {:?}", rep.items);

        let schema = drv.get_schema(&cfg).await.unwrap();
        let cust = schema.iter().find(|t| t.name == "customers").expect("customers");
        assert_eq!(cust.row_count, 2);
        assert!(cust.ddl.as_deref().unwrap_or("").contains("CREATE TABLE"));
        let inv = schema.iter().find(|t| t.name == "invoices").expect("invoices");
        assert_eq!(inv.foreign_keys.len(), 1);
        assert_eq!(inv.foreign_keys[0].referenced_table, "customers");
        assert!(schema.iter().any(|t| t.name == "v_customers" && t.object_type == "VIEW"));

        let mut pools = HashMap::new();
        pools.insert("customer_id".into(), vec![serde_json::json!(1), serde_json::json!(2)]);
        let mock_invs = generate_mock_mutations(inv, 8, &pools, false);
        let mb = drv.mutate_batch(&cfg, "invoices", &mock_invs).await.unwrap();
        assert!(mb.atomic && mb.results.len() == 8);

        let health = fetch_server_health(&*drv, &cfg).await.unwrap();
        assert!(health.engine_banner.contains("MySQL"));
    }

    // 3. Live MongoDB E2E
    if let Ok(mongo_url) = std::env::var("BAPU_TEST_MONGO_URL") {
        let cfg = make_conn("mongo-live", DbKind::Mongodb, &mongo_url, "bapu_e2e");
        let drv = driver_for(DbKind::Mongodb);
        let banner = drv.test_connection(&cfg).await.expect("MongoDB test_connection");
        assert!(banner.contains("MongoDB"));

        let _ = drv.query(&cfg, "events.deleteMany({})").await.unwrap();
        drv.query(
            &cfg,
            r#"events.insertMany([{"service":"auth","ms":12},{"service":"auth","ms":28},{"service":"billing","ms":45}])"#,
        )
        .await
        .unwrap();

        let cnt = drv.query(&cfg, r#"events.countDocuments({"service":"auth"})"#).await.unwrap();
        assert_eq!(cnt.rows[0]["count"], serde_json::json!(2));

        let agg = drv
            .query(
                &cfg,
                r#"events.aggregate([{"$group":{"_id":"$service","avg_ms":{"$avg":"$ms"}}}])"#,
            )
            .await
            .unwrap();
        assert_eq!(agg.row_count, 2);

        let schema = drv.get_schema(&cfg).await.unwrap();
        let ev_col = schema.iter().find(|t| t.name == "events").expect("events collection");
        assert_eq!(ev_col.object_type, "COLLECTION");
        assert!(!ev_col.indexes.is_empty());

        let health = fetch_server_health(&*drv, &cfg).await.unwrap();
        assert!(health.engine_banner.contains("MongoDB"));
    }

    // 4. Live Redis E2E
    if let Ok(redis_url) = std::env::var("BAPU_TEST_REDIS_URL") {
        let cfg = make_conn("redis-live", DbKind::Redis, &redis_url, "0");
        let drv = driver_for(DbKind::Redis);
        let banner = drv.test_connection(&cfg).await.expect("Redis test_connection");
        assert!(banner.contains("Redis"));

        let pipe_res = drv
            .query(
                &cfg,
                "FLUSHDB\nSET app:version \"2.0.0\"\nHSET user:42 name \"Ada\" role \"Principal\"\nLPUSH jobs \"job1\" \"job2\"\nGET app:version",
            )
            .await
            .unwrap();
        assert_eq!(pipe_res.command.as_deref(), Some("PIPELINE"));
        assert_eq!(pipe_res.rows.last().unwrap()["result"], serde_json::json!("2.0.0"));

        let schema = drv.get_schema(&cfg).await.unwrap();
        assert!(schema.iter().any(|t| t.name == "string"));
        assert!(schema.iter().any(|t| t.name == "hash"));
        assert!(schema.iter().any(|t| t.name == "list"));

        let health = fetch_server_health(&*drv, &cfg).await.unwrap();
        assert!(health.engine_banner.contains("Redis"));
    }
}
