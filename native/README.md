# Bapu Studio v2.0.0 — Pure-Rust Native Desktop App (`egui` / `eframe`)

Bapu Studio v2.0.0 is a 100% pure-Rust native desktop **API Client (Postman Parity)**, **Universal 33-Engine Database IDE (DBeaver Community + PRO Parity)**, **Real-Time Stream Client**, and **AI Copilot** built with [`egui`](https://github.com/emilk/egui) / [`eframe`](https://github.com/emilk/egui). It eliminates Electron, Chromium, Node.js, Java, and JDBC completely for instant cold starts, minimal RAM footprint, OS-keychain secret protection, and air-gapped script sandboxing.

**License & Clean-Room Guarantee:** Released under `AGPL-3.0-or-later` using permissive (`MIT` / `Apache-2.0`) Rust crates only. Zero proprietary Postman Inc., DBeaver Corp, or Oracle Instant Client C/Java runtimes are bundled.

---

## 33 Supported Database Engines (Zero OCI / JDBC Required)

| Category | Supported Engines | Native Rust Transport / Driver |
| :--- | :--- | :--- |
| **Relational & Distributed SQL (13)** | **PostgreSQL**, **CockroachDB**, **YugabyteDB**, **TimescaleDB** | `sqlx` async PostgreSQL wire protocol |
| | **Microsoft SQL Server (MSSQL)**, **Azure SQL / Synapse** | `tiberius` pure-Rust TDS 7.3 / 8.0 wire protocol |
| | **Oracle Database (19c / 21c / 23ai)** | `oracle-rs` pure-Rust Oracle Net (TNS) thin protocol |
| | **MySQL**, **MariaDB**, **TiDB** | `sqlx` async MySQL/MariaDB wire protocol |
| | **SQLite** | `sqlx` embedded native SQLite3 |
| | **Turso / libSQL** | Native Hrana `/v2/pipeline` HTTP SQL protocol |
| | **Firebird** | `odbc-api` Firebird ODBC driver |
| **Analytical, OLAP & Cloud Warehouses (9)** | **ClickHouse** | Native HTTP SQL (`FORMAT JSON` + `JSONEachRow` + `system.columns`) |
| | **Snowflake** | Native Snowflake SQL REST API v2 (`/api/v2/statements`) |
| | **Google BigQuery** | Native BigQuery Jobs Query REST API (`/bigquery/v2/.../queries`) |
| | **Databricks SQL** | Native Statement Execution API (`/api/2.0/sql/statements`) |
| | **Amazon Redshift** | `sqlx` PostgreSQL wire protocol (`5439`) |
| | **Trino / Presto** | Native `/v1/statement` multi-page async cursor protocol |
| | **QuestDB** | Native `/exec` HTTP SQL API |
| | **DuckDB**, **Teradata Vantage** | `odbc-api` native ODBC bridge |
| **NoSQL, Wide-Column, Graph, Search & Time-Series (8)** | **MongoDB / AWS DocumentDB** | `mongodb` v3 BSON wire protocol (`find`, `aggregate`, `insertMany`, etc.) |
| | **Apache Cassandra**, **ScyllaDB** | `cdrs-tokio` pure-Rust CQL v4/v5 binary protocol + `BATCH` |
| | **Redis / Valkey / Dragonfly** | `redis` async RESP2/RESP3 wire protocol + pipelines |
| | **Elasticsearch / OpenSearch** | Native `/_sql?format=json` + JSON Query DSL `/_search` + `/_mapping` |
| | **Neo4j** | Native Transactional HTTP Cypher API (`/db/{db}/tx/commit`) |
| | **InfluxDB** | Native `/query` InfluxQL time-series API |
| | **Apache CouchDB** | Native Mango `/_find` + `/_all_dbs` REST API |
| **Enterprise RDBMS & Universal ODBC (3)** | **IBM Db2**, **SAP HANA**, **Generic ODBC** | `odbc-api` (`SQLTables`/`SQLColumns` catalog + transactional `TextRowSet` for Sybase, Informix, Vertica, Exasol, Netezza, Access, etc.) |

---

## Feature Highlights

- **API Studio (Postman Parity)**: All 7 HTTP methods, `:path` & query params, Form/Multipart(file)/GraphQL/XML/JSON bodies, OAuth 2.0 live token exchange, AWS SigV4, JWT HS256, RFC-6265 Cookie Jar, multi-scope + 17 dynamic variables (`{{$guid}}`, `{{$randomEmail}}`, etc.), air-gapped `boa_engine` JS assertions + 0ms Auto-Assertion Generator, CSV/JSON Collection Runner, local HTTP Mock Server, 8-language Codegen, and OpenAPI / HAR / Postman / cURL / Markdown Docs / Git-folder sync.
- **Database Studio (DBeaver Community + PRO Parity)**: Categorized + searchable 33-engine connection manager (with mTLS & pure-Rust `russh` SSH bastion tunneling), Interactive Vector ER Diagram (`egui::Painter` + Mermaid & SVG export), Visual Table Designer (Forward Engineering), SQL Formatter, Multi-Statement Script Runner (`Alt+X`), `:param` / `${param}` binder, Inline Data Grid filtering/sorting/pagination, bidirectional Foreign Key hyperlink navigation, transactional staged edits (`Ctrl+S`), 8 export formats (`CSV`, `TSV`, `JSON`, `NDJSON`, `SQL INSERT`, `Markdown`, `HTML`, `XML`), CSV/JSON Import Wizard with column mapping, Schema Compare & Migration DDL, PK Data Compare & Sync SQL, FK-aware Smart Mock Data Generator, and Server Health & Sessions Monitor.

---

## Building & Running Tests

```bash
cd native
cargo test -p bapu-core
cargo run --release -p bapu-app
```
