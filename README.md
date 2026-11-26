# ⚡ Bapu Studio v2.0 — Pure-Rust Native Open-Source Developer Cockpit

<div align="center">

[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Native Rust CI](https://github.com/joravar/bapu-studio/actions/workflows/native-ci.yml/badge.svg)](https://github.com/joravar/bapu-studio/actions/workflows/native-ci.yml)
[![Release: v2.0.0](https://img.shields.io/badge/Release-v2.0.0-emerald.svg)](https://github.com/joravar/bapu-studio/releases/tag/v2.0.0)
[![Platforms](https://img.shields.io/badge/Platforms-Windows%20%7C%20macOS%20%7C%20Linux-informational.svg)](https://github.com/joravar/bapu-studio/releases/tag/v2.0.0)
[![Website](https://img.shields.io/badge/Website-Live-brightgreen.svg)](https://joravar.github.io/bapu-studio/)
[![Sponsor](https://img.shields.io/badge/GitHub-Sponsor-pink.svg)](https://github.com/sponsors/joravar)

**A 100% pure-Rust native desktop cockpit (`egui` / `eframe`) uniting a Postman-class API Studio, a 33-Engine DBeaver-class Database IDE, Real-Time WebSocket/SSE Streams, an OS-Keychain Secrets Vault, and a Local/BYOK AI Copilot — with zero Electron, zero Chromium, zero Node.js, and zero JVM.**

[📦 Downloads](#-downloads) • [✨ What's New in v2.0.0](#-whats-new-in-v200) • [🗄️ 33 Database Engines](#️-33-supported-database-engines) • [📊 Feature Comparison](#-feature-comparison) • [🚀 Build from Source](#-quick-start-build-from-source) • [🔒 Security](#-security--privacy) • [💖 Sponsor](#-support--github-sponsors) • [📄 License](#-license)

</div>

---

## 🌟 Why Bapu Studio v2.0?

Developers shouldn't have to burn **2–4 GB of RAM** running an Electron API client alongside a Java/JVM database IDE just to test an HTTP endpoint, inspect a database schema, and manage `.env` secrets.

**Bapu Studio v2.0** is a ground-up **pure-Rust native desktop rewrite** (~44 MB standalone binary, sub-second cold start, minimal idle RAM) engineered for **simplicity, speed, clean-room openness, and 100% local privacy**.

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                     BAPU STUDIO v2.0 (NATIVE RUST)                                   │
├────────────────────────────────┬──────────────────────────────────┬──────────────────────────────────┤
│         🚀 API STUDIO          │    🗄️ 33-ENGINE DATABASE IDE     │     🔐 SECRETS, STREAMS & AI     │
│ • REST, GraphQL, Multipart, XML│ • Postgres, MSSQL, Oracle, MySQL │ • OS Keychain Vault (DPAPI/Mac)  │
│ • OAuth2, AWS SigV4, JWT HS256 │ • Snowflake, BigQuery, ClickHouse│ • Multi-Scope + Dynamic {{$vars}}│
│ • Air-Gapped Boa JS Assertions │ • Mongo, Redis, Cassandra, Neo4j │ • Live WebSocket & SSE Studio    │
│ • CSV/JSON Runner & Mock Server│ • Interactive ERD & Table Design │ • Local Ollama + OpenAI/Anthropic│
│ • OpenAPI 3, HAR 1.2, Postman  │ • Schema/Data Diff & Mock Seeder │ • 8-Lang Codegen & Git Dir Sync  │
└────────────────────────────────┴──────────────────────────────────┴──────────────────────────────────┘
```

---

## 📦 Downloads

Standalone native binaries are published on [GitHub Releases (`v2.0.0`)](https://github.com/joravar/bapu-studio/releases/tag/v2.0.0) alongside `SHA256SUMS.txt`:

| Operating System | Package | Format | Notes |
| :--- | :--- | :--- | :--- |
| **🪟 Windows (x64)** | [**Bapu-Studio-Portable-x64.exe**](https://github.com/joravar/bapu-studio/releases/download/v2.0.0/Bapu-Studio-Portable-x64.exe) <br> [bapu-studio-windows-x64.zip](https://github.com/joravar/bapu-studio/releases/download/v2.0.0/bapu-studio-windows-x64.zip) | Standalone `.exe` & `.zip` | Zero dependencies (pure-Rust MSSQL TDS & Oracle TNS built in) |
| **🍏 macOS (Apple Silicon & Intel)** | [**bapu-studio-macos-arm64.tar.gz**](https://github.com/joravar/bapu-studio/releases/download/v2.0.0/bapu-studio-macos-arm64.tar.gz) <br> [bapu-studio-macos-x64.tar.gz](https://github.com/joravar/bapu-studio/releases/download/v2.0.0/bapu-studio-macos-x64.tar.gz) | `.tar.gz` | Native Apple Silicon (`M1`–`M4`) & Intel binaries |
| **🐧 Linux (x64)** | [**bapu-studio-linux-x64.tar.gz**](https://github.com/joravar/bapu-studio/releases/download/v2.0.0/bapu-studio-linux-x64.tar.gz) | `.tar.gz` | X11 / Wayland native OpenGL (`glow`) binary |

---

## ✨ What's New in v2.0.0

### 🦀 1. 100% Pure-Rust Native Desktop Architecture (`egui` / `eframe`)
* Replaced Electron, Chromium, Node.js, and bundled JS/WASM drivers with a single native Rust workspace (`native/crates/bapu-core` + `native/crates/bapu-app`).
* Instant cold startup, crisp vector icons & SVG brand logos (`resvg`), responsive 2D-resizable floating studio windows, and persistent Dark/Light HSL themes.

### 🗄️ 2. Universal 33-Engine Database Studio (Zero OCI / Zero JDBC)
* Expanded from 5 engines to **33 relational, analytical/OLAP, cloud warehouse, NoSQL, graph, search, time-series, and enterprise ODBC engines** — including **pure-Rust Microsoft SQL Server / Azure SQL (`tiberius` TDS)** and **pure-Rust Oracle Database 19c/21c/23ai (`oracle-rs` TNS)** with **zero** Oracle Instant Client or Java dependencies.
* **DBeaver Community + PRO Feature Set**:
  * **Interactive Vector ER Diagram (ERD)**: Auto-layout entity cards, live zoom/drag, PK/FK Crow's-foot connectors, plus **Mermaid (`erDiagram`)** and **standalone SVG** export.
  * **Visual Table Designer (Forward Engineering)**: Design columns, types, PKs, defaults, and FK constraints visually with live dialect-accurate `CREATE TABLE` DDL preview and one-click execution.
  * **Power SQL Editor**: Multi-tab persistent SQL scratchpads, dialect **SQL Formatter**, **Multi-Statement Script Runner** (`Alt+X`), interactive `:param` / `${param}` parameter binding bar, dialect `EXPLAIN` plans, and saved SQL snippets.
  * **Transactional Data Grid & FK Navigation**: Inline search, column filter operators, click-header sorting, pagination, **bidirectional Foreign Key hyperlink navigation** (outgoing parent lookup + incoming child references), **Record & Column Stats Inspector**, and transactional staged edits (`BEGIN`/`COMMIT`/`ROLLBACK` via `Ctrl+S`).
  * **8-Format Data Export & CSV/JSON Import Wizard**: Export any result to `CSV`, `TSV`, `JSON`, `NDJSON`, `SQL INSERT`, `Markdown`, `HTML`, or `XML`; import RFC-4180 CSV or JSON arrays with interactive source-to-target column mapping.
  * **Schema Compare, PK Data Compare, FK-Aware Mock Data & Server Health**: Diff schemas across connections to generate `ALTER`/`CREATE` migration DDL, diff table rows by Primary Key to generate `INSERT`/`UPDATE`/`DELETE` sync scripts, seed realistic FK-valid mock rows, and inspect live server sessions/metrics.

### 🚀 3. Complete Postman-Class API Studio
* **Full Protocol & Payload Matrix**: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`, automatic `:param` / `{param}` path variables, query params, JSON, XML, Raw, `x-www-form-urlencoded`, `multipart/form-data` (fields + binary file uploads), and **GraphQL** (query + variables + one-click schema introspection).
* **Enterprise Auth**: Bearer, Basic, API Key (header or query), **OAuth 2.0** (`client_credentials` & `password` grant live token fetcher), **AWS Signature v4** (pure-Rust canonical request & HMAC-SHA256 signing), and **JWT HS256** generator.
* **Scoped Variables, Cookie Jar & Air-Gapped JS Scripting**: `Iteration > Environment > Collection > Global > Dynamic` (`{{$guid}}`, `{{$timestamp}}`, `{{$randomEmail}}`, etc.), RFC-6265 domain/path Cookie Jar, sandboxed `boa_engine` JS scripts (`bapu.*` / `pm.*` + `setNextRequest`), and **0ms Smart Assertion Generator**.
* **Collection Runner, Local Mock Server, 8-Language Codegen & Spec I/O**: Data-driven CSV/JSON Collection Runner with `avg`/`p95` latency metrics, local HTTP Mock Server from saved response examples, 8-language client code generator (`cURL`, `Rust reqwest`, `Python`, `JS fetch`, `Go`, `Java`, `C#`, `PHP`), and lossless **OpenAPI 3.0/3.1**, **HAR 1.2**, **Postman v2.1**, **cURL**, **Markdown API Docs**, and **1-file-per-request Git folder sync**.

---

## 🗄️ 33 Supported Database Engines

| Category | Engines | Native Rust Driver / Protocol |
| :--- | :--- | :--- |
| **Relational & Distributed SQL (13)** | **PostgreSQL**, **CockroachDB**, **YugabyteDB**, **TimescaleDB** | `sqlx` async PostgreSQL wire protocol + TLS + SSH |
| | **Microsoft SQL Server (MSSQL)**, **Azure SQL / Synapse** | `tiberius` pure-Rust TDS 7.3 / 8.0 wire protocol |
| | **Oracle Database (19c / 21c / 23ai)** | `oracle-rs` pure-Rust Oracle Net (TNS) thin protocol |
| | **MySQL**, **MariaDB**, **TiDB** | `sqlx` async MySQL/MariaDB wire protocol + TLS + SSH |
| | **SQLite** | `sqlx` embedded native SQLite3 (`:memory:` or file) |
| | **Turso / libSQL** | Native Hrana `/v2/pipeline` HTTP SQL protocol |
| | **Firebird** | `odbc-api` native ODBC 3.8 driver |
| **Analytical, OLAP & Cloud Warehouses (9)** | **ClickHouse** | Native HTTP SQL (`FORMAT JSON` + `JSONEachRow` + `system.columns`) |
| | **Snowflake** | Native Snowflake SQL REST API v2 (`/api/v2/statements`) |
| | **Google BigQuery** | Native BigQuery Jobs Query REST API |
| | **Databricks SQL** | Native Statement Execution API (`/api/2.0/sql/statements`) |
| | **Amazon Redshift** | `sqlx` PostgreSQL wire protocol (`5439`) |
| | **Trino / Presto** | Native `/v1/statement` multi-page async cursor protocol |
| | **QuestDB** | Native `/exec` HTTP SQL API |
| | **DuckDB**, **Teradata Vantage** | `odbc-api` native ODBC 3.8 bridge |
| **NoSQL, Wide-Column, Graph, Search & Time-Series (8)** | **MongoDB / AWS DocumentDB** | `mongodb` v3 BSON wire protocol + session transactions |
| | **Apache Cassandra**, **ScyllaDB** | `cdrs-tokio` pure-Rust CQL v4/v5 binary protocol + `BATCH` |
| | **Redis / Valkey / Dragonfly** | `redis` async RESP2/RESP3 wire protocol |
| | **Elasticsearch / OpenSearch** | Native `/_sql?format=json` + JSON Query DSL `/_search` |
| | **Neo4j** | Native Transactional HTTP Cypher API (`/db/{db}/tx/commit`) |
| | **InfluxDB** | Native `/query` InfluxQL time-series API |
| | **Apache CouchDB** | Native Mango `/_find` + `/_all_dbs` REST API |
| **Enterprise RDBMS & Universal ODBC (3)** | **IBM Db2**, **SAP HANA**, **Generic ODBC** | `odbc-api` (Sybase ASE, Informix, Vertica, Exasol, Netezza, Access, etc.) |

---

## 📊 Feature Comparison

| Capability | Postman | DBeaver PRO / CE | TablePlus | **Bapu Studio v2.0 (OSS)** |
| :--- | :---: | :---: | :---: | :---: |
| **Runtime Architecture** | Electron + Cloud | Java / Eclipse JVM | Native | **100% Pure Rust (`egui` / `eframe`)** |
| **Binary Footprint** | ~400 MB+ | ~250 MB+ (with JRE) | ~90 MB | **~44 MB Single Native Binary** |
| **100% Offline-First / Zero Telemetry** | ❌ | ✅ (CE) | ✅ | **✅ 100% Local + Zero Telemetry** |
| **Unified API Client + Database IDE** | ❌ (API only) | ❌ (DB only) | ❌ (DB only) | **✅ Unified Cockpit** |
| **Database Engines Supported** | N/A | SQL + NoSQL (PRO) | ~15 | **✅ 33 Engines (SQL, OLAP, NoSQL, ODBC)** |
| **Interactive ERD + Schema/Data Diff** | N/A | ✅ (Diff in PRO) | ❌ | **✅ Built-in Free & Open Source** |
| **Collection Runner + Local Mock Server** | ✅ (Runner gated) | N/A | N/A | **✅ Unlimited Local Runner & Mock** |
| **OS-Keychain Secrets Vault** | ⚠️ Cloud Vault | ⚠️ Master Password | ✅ | **✅ Native OS Keychain (`keyring`)** |
| **WebSocket & SSE Stream Studio** | ⚠️ Separate | ❌ | ❌ | **✅ Dedicated Live Stream Studio** |
| **Local Ollama + BYOK AI Copilot** | ❌ Cloud Postbot | ⚠️ Add-on | ❌ | **✅ Local Ollama + OpenAI/Anthropic** |
| **License** | Proprietary | Apache 2.0 / Commercial | Proprietary | **Open Source (`AGPL-3.0-or-later`)** |

---

## 🚀 Quick Start (Build from Source)

### Prerequisites
* **Rust stable toolchain** (`1.82+`, via [rustup.rs](https://rustup.rs))
* **Linux only**: `sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev libdbus-1-dev unixodbc-dev`

### Build, Test & Run Native Desktop App
```bash
git clone https://github.com/joravar/bapu-studio.git
cd bapu-studio/native

# Run all unit, protocol, Postman parity, DBeaver parity, and UI tests
cargo test --workspace

# Launch Bapu Studio in optimized release mode
cargo run --release -p bapu-app
```

---

## 🔒 Security & Privacy

* **Zero Plaintext Secrets on Disk:** Database passwords, SSH keys/passphrases, secret environment/global variables, and BYOK AI API keys are stored exclusively in your OS credential store (**Windows Credential Manager / DPAPI**, **macOS Keychain**, **Linux Secret Service**) via `keyring`.
* **Air-Gapped Script Sandbox:** Pre-request and test scripts execute inside a pure-Rust `boa_engine` JS context with zero filesystem, network, process, or keychain access and a strict 5-second timeout.
* See [SECURITY.md](SECURITY.md) for full details and vulnerability reporting.

---

## ⚖️ Legal, Trademark & Clean-Room Notice

* **Nominative Fair Use:** Postman®, DBeaver®, TablePlus®, PostgreSQL®, Microsoft SQL Server®, Oracle®, MySQL®, MariaDB®, Snowflake®, Google BigQuery®, Databricks®, ClickHouse®, MongoDB®, Redis®, Cassandra®, Neo4j®, Elasticsearch®, IBM Db2®, SAP HANA®, and other third-party product names or trademarks mentioned are the property of their respective owners and are referenced strictly for descriptive interoperability and comparison. No affiliation or endorsement is implied.
* **100% Clean-Room Independent Implementation:** All source code, protocol adapters, and UI layouts in Bapu Studio are original works authored cleanly from public RFCs, wire-protocol specs, and permissive (`MIT` / `Apache-2.0`) Rust crates, distributed under **GNU AGPLv3 (`AGPL-3.0-or-later`)**.
* **No Warranty ("AS IS"):** Provided without warranty of any kind under Sections 15–16 of the GNU AGPLv3. Always review SQL/DDL/migration statements before executing against production systems.

---

## 💖 Support & GitHub Sponsors

Bapu Studio is independent, community-backed open-source software. Support ongoing development on [GitHub Sponsors](https://github.com/sponsors/joravar).

---

## 📄 License

Licensed under the **GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`)** — see [LICENSE](LICENSE).
