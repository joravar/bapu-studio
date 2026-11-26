//! Data model for Bapu Studio — backward-compatible with existing JSON files on disk
//! via `#[serde(default)]`, while supporting full API client workflows:
//! all HTTP methods, path variables, form-urlencoded & multipart bodies, OAuth 2.0 / AWS SigV4 /
//! JWT Bearer auth, saved response examples, per-request network settings, collection & global
//! variables, and domain-scoped cookies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub const ALL: [HttpMethod; 7] = [
        HttpMethod::GET,
        HttpMethod::POST,
        HttpMethod::PUT,
        HttpMethod::PATCH,
        HttpMethod::DELETE,
        HttpMethod::HEAD,
        HttpMethod::OPTIONS,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "PATCH" => HttpMethod::PATCH,
            "DELETE" => HttpMethod::DELETE,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            _ => HttpMethod::GET,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default, rename = "isSecret", skip_serializing_if = "Option::is_none")]
    pub is_secret: Option<bool>,
}

/// Multipart form field: either a plain text value or a file upload from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartField {
    pub id: String,
    pub key: String,
    pub value: String,
    #[serde(default, rename = "isFile")]
    pub is_file: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAssertionResult {
    pub name: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyType {
    None,
    Json,
    Form,
    Multipart,
    Raw,
    Xml,
    Graphql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    None,
    Bearer,
    Basic,
    Apikey,
    Oauth2,
    Awssigv4,
    Jwt,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthConfig {
    // Bearer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    // Basic
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    // API Key
    #[serde(default, rename = "apiKeyName", skip_serializing_if = "Option::is_none")]
    pub api_key_name: Option<String>,
    #[serde(default, rename = "apiKeyValue", skip_serializing_if = "Option::is_none")]
    pub api_key_value: Option<String>,
    #[serde(default, rename = "addTo", skip_serializing_if = "Option::is_none")]
    pub add_to: Option<String>, // "header" | "query"

    // OAuth 2.0 (Client Credentials / Password Grant / Access Token)
    #[serde(default, rename = "oauthGrantType", skip_serializing_if = "Option::is_none")]
    pub oauth_grant_type: Option<String>, // "client_credentials" | "password"
    #[serde(default, rename = "oauthTokenUrl", skip_serializing_if = "Option::is_none")]
    pub oauth_token_url: Option<String>,
    #[serde(default, rename = "oauthClientId", skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<String>,
    #[serde(default, rename = "oauthClientSecret", skip_serializing_if = "Option::is_none")]
    pub oauth_client_secret: Option<String>,
    #[serde(default, rename = "oauthScope", skip_serializing_if = "Option::is_none")]
    pub oauth_scope: Option<String>,
    #[serde(default, rename = "oauthHeaderPrefix", skip_serializing_if = "Option::is_none")]
    pub oauth_header_prefix: Option<String>,

    // AWS Signature v4
    #[serde(default, rename = "awsAccessKey", skip_serializing_if = "Option::is_none")]
    pub aws_access_key: Option<String>,
    #[serde(default, rename = "awsSecretKey", skip_serializing_if = "Option::is_none")]
    pub aws_secret_key: Option<String>,
    #[serde(default, rename = "awsSessionToken", skip_serializing_if = "Option::is_none")]
    pub aws_session_token: Option<String>,
    #[serde(default, rename = "awsRegion", skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
    #[serde(default, rename = "awsService", skip_serializing_if = "Option::is_none")]
    pub aws_service: Option<String>,

    // JWT Bearer (HS256 local signing)
    #[serde(default, rename = "jwtSecret", skip_serializing_if = "Option::is_none")]
    pub jwt_secret: Option<String>,
    #[serde(default, rename = "jwtPayload", skip_serializing_if = "Option::is_none")]
    pub jwt_payload: Option<String>,
}

/// Saved example response attached to an `ApiRequest` — backs both offline documentation
/// and the built-in Local Mock Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiExample {
    pub id: String,
    pub name: String,
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub params: Vec<KeyValuePair>,
    #[serde(default, rename = "pathParams", skip_serializing_if = "Vec::is_empty")]
    pub path_params: Vec<KeyValuePair>,
    #[serde(default)]
    pub headers: Vec<KeyValuePair>,
    #[serde(rename = "bodyType")]
    pub body_type: BodyType,
    #[serde(rename = "bodyContent", default)]
    pub body_content: String,
    #[serde(default, rename = "formFields", skip_serializing_if = "Vec::is_empty")]
    pub form_fields: Vec<KeyValuePair>,
    #[serde(default, rename = "multipartFields", skip_serializing_if = "Vec::is_empty")]
    pub multipart_fields: Vec<MultipartField>,
    #[serde(rename = "graphqlQuery", default, skip_serializing_if = "Option::is_none")]
    pub graphql_query: Option<String>,
    #[serde(rename = "graphqlVariables", default, skip_serializing_if = "Option::is_none")]
    pub graphql_variables: Option<String>,
    #[serde(rename = "authType")]
    pub auth_type: AuthType,
    #[serde(rename = "authConfig", default)]
    pub auth_config: AuthConfig,
    #[serde(rename = "preRequestScript", default, skip_serializing_if = "Option::is_none")]
    pub pre_request_script: Option<String>,
    #[serde(rename = "testScript", default, skip_serializing_if = "Option::is_none")]
    pub test_script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<String>,
    #[serde(rename = "collectionId", default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<ApiExample>,
    #[serde(default = "default_true", rename = "followRedirects")]
    pub follow_redirects: bool,
    #[serde(default = "default_true", rename = "verifyTls")]
    pub verify_tls: bool,
    #[serde(default, rename = "timeoutMs", skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl Default for ApiRequest {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Request".into(),
            method: HttpMethod::GET,
            url: String::new(),
            description: None,
            params: vec![],
            path_params: vec![],
            headers: vec![],
            body_type: BodyType::None,
            body_content: String::new(),
            form_fields: vec![],
            multipart_fields: vec![],
            graphql_query: None,
            graphql_variables: None,
            auth_type: AuthType::None,
            auth_config: AuthConfig::default(),
            pre_request_script: None,
            test_script: None,
            tests: None,
            collection_id: None,
            examples: vec![],
            follow_redirects: true,
            verify_tls: true,
            timeout_ms: None,
        }
    }
}

impl ApiRequest {
    pub fn has_executable_script(&self) -> bool {
        self.pre_request_script.as_deref().is_some_and(|s| !s.trim().is_empty())
            || self.test_script.as_deref().is_some_and(|s| !s.trim().is_empty())
            || self.tests.as_deref().is_some_and(|s| !s.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookieEntry {
    pub domain: String,
    pub path: String,
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default, rename = "httpOnly")]
    pub http_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    #[serde(rename = "statusText")]
    pub status_text: String,
    #[serde(rename = "timeMs")]
    pub time_ms: u64,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub headers: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cookies: Vec<CookieEntry>,
    pub data: serde_json::Value,
    pub timestamp: String,
    #[serde(rename = "httpVersion", default, skip_serializing_if = "Option::is_none")]
    pub http_version: Option<String>,
    #[serde(rename = "remoteAddr", default, skip_serializing_if = "Option::is_none")]
    pub remote_addr: Option<String>,
    #[serde(rename = "testResults", default, skip_serializing_if = "Vec::is_empty")]
    pub test_results: Vec<TestAssertionResult>,
    #[serde(rename = "consoleLogs", default, skip_serializing_if = "Vec::is_empty")]
    pub console_logs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignKeyDef {
    #[serde(rename = "constraintName", default, skip_serializing_if = "Option::is_none")]
    pub constraint_name: Option<String>,
    pub column: String,
    #[serde(rename = "referencedTable")]
    pub referenced_table: String,
    #[serde(rename = "referencedColumn")]
    pub referenced_column: String,
    #[serde(rename = "onDelete", default, skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<String>,
    #[serde(rename = "onUpdate", default, skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
    pub name: String,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(rename = "isUnique", default)]
    pub is_unique: bool,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    #[serde(rename = "isPrimaryKey", default)]
    pub is_primary_key: bool,
    #[serde(rename = "isNullable", default)]
    pub is_nullable: bool,
    #[serde(rename = "defaultValue", default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(rename = "isUnique", default)]
    pub is_unique: bool,
    #[serde(rename = "autoIncrement", default)]
    pub auto_increment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

fn default_object_type() -> String {
    "TABLE".into()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    #[serde(rename = "objectType", default = "default_object_type")]
    pub object_type: String,
    #[serde(rename = "rowCount", default)]
    pub row_count: u64,
    #[serde(default)]
    pub columns: Vec<ColumnDefinition>,
    #[serde(rename = "foreignKeys", default, skip_serializing_if = "Vec::is_empty")]
    pub foreign_keys: Vec<ForeignKeyDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<IndexDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ddl: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SqlSnippet {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialect: Option<DbKind>,
    pub sql: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DbCategory {
    Relational,
    AnalyticalCloud,
    NoSqlGraphSearch,
    EnterpriseOdbc,
}

impl DbCategory {
    pub const ALL: [DbCategory; 4] = [
        DbCategory::Relational,
        DbCategory::AnalyticalCloud,
        DbCategory::NoSqlGraphSearch,
        DbCategory::EnterpriseOdbc,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            DbCategory::Relational => "Relational & Distributed SQL",
            DbCategory::AnalyticalCloud => "Analytical, OLAP & Cloud Warehouses",
            DbCategory::NoSqlGraphSearch => "NoSQL, Graph, Search & Time-Series",
            DbCategory::EnterpriseOdbc => "Enterprise RDBMS & Universal ODBC",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbKind {
    // Relational & Distributed SQL
    #[default]
    Postgres,
    Mysql,
    Mariadb,
    Sqlite,
    Mssql,
    AzureSql,
    Oracle,
    Cockroachdb,
    Yugabytedb,
    Tidb,
    Timescaledb,
    Libsql,
    Firebird,
    // Analytical, OLAP & Cloud Warehouses
    Clickhouse,
    Duckdb,
    Snowflake,
    Bigquery,
    Databricks,
    Redshift,
    Trino,
    Questdb,
    Teradata,
    // NoSQL, Wide-Column, Graph, Search & Time-Series
    Mongodb,
    Redis,
    Cassandra,
    Scylladb,
    Elasticsearch,
    Neo4j,
    Influxdb,
    Couchdb,
    // Enterprise & Universal ODBC
    Db2,
    SapHana,
    Odbc,
}

impl DbKind {
    pub const ALL: [DbKind; 33] = [
        DbKind::Postgres,
        DbKind::Mysql,
        DbKind::Mariadb,
        DbKind::Sqlite,
        DbKind::Mssql,
        DbKind::AzureSql,
        DbKind::Oracle,
        DbKind::Cockroachdb,
        DbKind::Yugabytedb,
        DbKind::Tidb,
        DbKind::Timescaledb,
        DbKind::Libsql,
        DbKind::Firebird,
        DbKind::Clickhouse,
        DbKind::Duckdb,
        DbKind::Snowflake,
        DbKind::Bigquery,
        DbKind::Databricks,
        DbKind::Redshift,
        DbKind::Trino,
        DbKind::Questdb,
        DbKind::Teradata,
        DbKind::Mongodb,
        DbKind::Redis,
        DbKind::Cassandra,
        DbKind::Scylladb,
        DbKind::Elasticsearch,
        DbKind::Neo4j,
        DbKind::Influxdb,
        DbKind::Couchdb,
        DbKind::Db2,
        DbKind::SapHana,
        DbKind::Odbc,
    ];

    pub const FEATURED: [DbKind; 10] = [
        DbKind::Postgres,
        DbKind::Mssql,
        DbKind::Oracle,
        DbKind::Mysql,
        DbKind::Sqlite,
        DbKind::Clickhouse,
        DbKind::Snowflake,
        DbKind::Mongodb,
        DbKind::Cassandra,
        DbKind::Redis,
    ];

    pub fn category(&self) -> DbCategory {
        match self {
            DbKind::Postgres
            | DbKind::Mysql
            | DbKind::Mariadb
            | DbKind::Sqlite
            | DbKind::Mssql
            | DbKind::AzureSql
            | DbKind::Oracle
            | DbKind::Cockroachdb
            | DbKind::Yugabytedb
            | DbKind::Tidb
            | DbKind::Timescaledb
            | DbKind::Libsql
            | DbKind::Firebird => DbCategory::Relational,

            DbKind::Clickhouse
            | DbKind::Duckdb
            | DbKind::Snowflake
            | DbKind::Bigquery
            | DbKind::Databricks
            | DbKind::Redshift
            | DbKind::Trino
            | DbKind::Questdb
            | DbKind::Teradata => DbCategory::AnalyticalCloud,

            DbKind::Mongodb
            | DbKind::Redis
            | DbKind::Cassandra
            | DbKind::Scylladb
            | DbKind::Elasticsearch
            | DbKind::Neo4j
            | DbKind::Influxdb
            | DbKind::Couchdb => DbCategory::NoSqlGraphSearch,

            DbKind::Db2 | DbKind::SapHana | DbKind::Odbc => DbCategory::EnterpriseOdbc,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DbKind::Postgres => "PostgreSQL",
            DbKind::Mysql => "MySQL",
            DbKind::Mariadb => "MariaDB",
            DbKind::Sqlite => "SQLite",
            DbKind::Mssql => "SQL Server (MSSQL)",
            DbKind::AzureSql => "Azure SQL",
            DbKind::Oracle => "Oracle Database",
            DbKind::Cockroachdb => "CockroachDB",
            DbKind::Yugabytedb => "YugabyteDB",
            DbKind::Tidb => "TiDB",
            DbKind::Timescaledb => "TimescaleDB",
            DbKind::Libsql => "Turso / libSQL",
            DbKind::Firebird => "Firebird",
            DbKind::Clickhouse => "ClickHouse",
            DbKind::Duckdb => "DuckDB",
            DbKind::Snowflake => "Snowflake",
            DbKind::Bigquery => "Google BigQuery",
            DbKind::Databricks => "Databricks SQL",
            DbKind::Redshift => "Amazon Redshift",
            DbKind::Trino => "Trino / Presto",
            DbKind::Questdb => "QuestDB",
            DbKind::Teradata => "Teradata",
            DbKind::Mongodb => "MongoDB",
            DbKind::Redis => "Redis / Valkey",
            DbKind::Cassandra => "Apache Cassandra",
            DbKind::Scylladb => "ScyllaDB",
            DbKind::Elasticsearch => "Elasticsearch",
            DbKind::Neo4j => "Neo4j (Cypher)",
            DbKind::Influxdb => "InfluxDB",
            DbKind::Couchdb => "Apache CouchDB",
            DbKind::Db2 => "IBM Db2",
            DbKind::SapHana => "SAP HANA",
            DbKind::Odbc => "Generic ODBC",
        }
    }

    pub fn short_badge(&self) -> &'static str {
        match self {
            DbKind::Postgres => "PG",
            DbKind::Mysql => "MY",
            DbKind::Mariadb => "MDB",
            DbKind::Sqlite => "SQL",
            DbKind::Mssql => "MS",
            DbKind::AzureSql => "AZ",
            DbKind::Oracle => "ORA",
            DbKind::Cockroachdb => "CRDB",
            DbKind::Yugabytedb => "YB",
            DbKind::Tidb => "TiDB",
            DbKind::Timescaledb => "TSDB",
            DbKind::Libsql => "LSQL",
            DbKind::Firebird => "FB",
            DbKind::Clickhouse => "CH",
            DbKind::Duckdb => "DUCK",
            DbKind::Snowflake => "SNOW",
            DbKind::Bigquery => "BQ",
            DbKind::Databricks => "DBX",
            DbKind::Redshift => "RS",
            DbKind::Trino => "TRN",
            DbKind::Questdb => "QDB",
            DbKind::Teradata => "TD",
            DbKind::Mongodb => "MGO",
            DbKind::Redis => "RDS",
            DbKind::Cassandra => "CQL",
            DbKind::Scylladb => "SCY",
            DbKind::Elasticsearch => "ES",
            DbKind::Neo4j => "NEO",
            DbKind::Influxdb => "IFX",
            DbKind::Couchdb => "CCH",
            DbKind::Db2 => "DB2",
            DbKind::SapHana => "HANA",
            DbKind::Odbc => "ODBC",
        }
    }

    pub fn default_port(&self) -> &'static str {
        match self {
            DbKind::Postgres | DbKind::Timescaledb => "5432",
            DbKind::Mysql | DbKind::Mariadb => "3306",
            DbKind::Sqlite | DbKind::Duckdb | DbKind::Odbc => "",
            DbKind::Mssql | DbKind::AzureSql => "1433",
            DbKind::Oracle => "1521",
            DbKind::Cockroachdb => "26257",
            DbKind::Yugabytedb => "5433",
            DbKind::Tidb => "4000",
            DbKind::Libsql => "8080",
            DbKind::Firebird => "3050",
            DbKind::Clickhouse => "8123",
            DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks => "443",
            DbKind::Redshift => "5439",
            DbKind::Trino => "8080",
            DbKind::Questdb => "9000",
            DbKind::Teradata => "1025",
            DbKind::Mongodb => "27017",
            DbKind::Redis => "6379",
            DbKind::Cassandra | DbKind::Scylladb => "9042",
            DbKind::Elasticsearch => "9200",
            DbKind::Neo4j => "7474",
            DbKind::Influxdb => "8086",
            DbKind::Couchdb => "5984",
            DbKind::Db2 => "50000",
            DbKind::SapHana => "30015",
        }
    }

    pub fn default_database(&self) -> &'static str {
        match self {
            DbKind::Postgres | DbKind::Timescaledb | DbKind::Yugabytedb => "postgres",
            DbKind::Cockroachdb => "defaultdb",
            DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => "test",
            DbKind::Sqlite => ":memory:",
            DbKind::Duckdb => "memory",
            DbKind::Mssql | DbKind::AzureSql => "master",
            DbKind::Oracle => "FREEPDB1",
            DbKind::Libsql => "main",
            DbKind::Firebird => "employee.fdb",
            DbKind::Clickhouse => "default",
            DbKind::Snowflake => "SNOWFLAKE_SAMPLE_DATA.TPCH_SF1",
            DbKind::Bigquery => "my-gcp-project.my_dataset",
            DbKind::Databricks => "main.default",
            DbKind::Redshift => "dev",
            DbKind::Trino => "memory.default",
            DbKind::Questdb => "qdb",
            DbKind::Teradata => "DBC",
            DbKind::Mongodb => "test",
            DbKind::Redis => "0",
            DbKind::Cassandra | DbKind::Scylladb => "system_schema",
            DbKind::Elasticsearch => "_all",
            DbKind::Neo4j => "neo4j",
            DbKind::Influxdb => "telegraf",
            DbKind::Couchdb => "mydb",
            DbKind::Db2 => "SAMPLE",
            DbKind::SapHana => "SYSTEMDB",
            DbKind::Odbc => "DSN=MyDataSource",
        }
    }

    pub fn default_user(&self) -> &'static str {
        match self {
            DbKind::Postgres | DbKind::Timescaledb | DbKind::Yugabytedb => "postgres",
            DbKind::Cockroachdb | DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => "root",
            DbKind::Mssql | DbKind::AzureSql => "sa",
            DbKind::Oracle | DbKind::SapHana => "SYSTEM",
            DbKind::Clickhouse => "default",
            DbKind::Redshift => "awsuser",
            DbKind::Trino => "bapu",
            DbKind::Questdb | DbKind::Couchdb => "admin",
            DbKind::Cassandra | DbKind::Scylladb => "cassandra",
            DbKind::Elasticsearch => "elastic",
            DbKind::Neo4j => "neo4j",
            DbKind::Db2 => "db2inst1",
            DbKind::Teradata => "dbc",
            DbKind::Firebird => "SYSDBA",
            _ => "",
        }
    }

    pub fn uri_hint(&self) -> &'static str {
        match self {
            DbKind::Mssql | DbKind::AzureSql => {
                "Server=tcp:localhost,1433;Database=master;User Id=sa;Password=...;TrustServerCertificate=true"
            }
            DbKind::Oracle => "localhost:1521/FREEPDB1",
            DbKind::Clickhouse => "http://localhost:8123",
            DbKind::Libsql => "https://your-db-org.turso.io  (Password = Auth Token)",
            DbKind::Trino => "http://localhost:8080",
            DbKind::Snowflake => "https://<account>.snowflakecomputing.com  (Password = OAuth/PAT Token)",
            DbKind::Bigquery => "https://bigquery.googleapis.com  (Database = project.dataset, Password = OAuth2 Token)",
            DbKind::Databricks => "https://<workspace>.cloud.databricks.com  (User = Warehouse ID, Password = PAT)",
            DbKind::Cassandra | DbKind::Scylladb => "127.0.0.1:9042",
            DbKind::Elasticsearch => "http://localhost:9200",
            DbKind::Neo4j => "http://localhost:7474",
            DbKind::Influxdb => "http://localhost:8086",
            DbKind::Couchdb => "http://localhost:5984",
            DbKind::Db2 => "Driver={IBM DB2 ODBC DRIVER};Hostname=localhost;Port=50000;Database=SAMPLE;Uid=db2inst1;Pwd=...;",
            DbKind::SapHana => "Driver={HDBODBC};ServerNode=localhost:30015;UID=SYSTEM;PWD=...;",
            DbKind::Teradata => "Driver={Teradata Database ODBC Driver};DBCName=localhost;UID=dbc;PWD=...;",
            DbKind::Duckdb => "Driver={DuckDB Driver};Database=:memory:;",
            DbKind::Firebird => "Driver={Firebird/InterBase(r) driver};Dbname=localhost:employee.fdb;UID=SYSDBA;PWD=masterkey;",
            DbKind::Odbc => "DSN=MyEnterpriseDB;UID=user;PWD=secret;  (or Driver={...};Server=...;)",
            _ => "postgres://user:pass@host:5432/dbname",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnection {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: DbKind,
    #[serde(rename = "connectionString", default, skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    pub database: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub ssl: bool,
    #[serde(rename = "sslCaCert", default, skip_serializing_if = "Option::is_none")]
    pub ssl_ca_cert: Option<String>,
    #[serde(rename = "sslClientCert", default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_cert: Option<String>,
    #[serde(rename = "sslClientKey", default, skip_serializing_if = "Option::is_none")]
    pub ssl_client_key: Option<String>,
    #[serde(rename = "sslRejectUnauthorized", default, skip_serializing_if = "Option::is_none")]
    pub ssl_reject_unauthorized: Option<bool>,
    #[serde(rename = "sshEnabled", default)]
    pub ssh_enabled: bool,
    #[serde(rename = "sshHost", default, skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    #[serde(rename = "sshPort", default, skip_serializing_if = "Option::is_none")]
    pub ssh_port: Option<String>,
    #[serde(rename = "sshUsername", default, skip_serializing_if = "Option::is_none")]
    pub ssh_username: Option<String>,
    #[serde(rename = "sshPassword", default, skip_serializing_if = "Option::is_none")]
    pub ssh_password: Option<String>,
    #[serde(rename = "sshPrivateKey", default, skip_serializing_if = "Option::is_none")]
    pub ssh_private_key: Option<String>,
    #[serde(rename = "sshPassphrase", default, skip_serializing_if = "Option::is_none")]
    pub ssh_passphrase: Option<String>,
    #[serde(rename = "isConnected", default)]
    pub is_connected: bool,
    #[serde(rename = "isDemoDb", default)]
    pub is_demo_db: bool,
    #[serde(default)]
    pub tables: Vec<TableSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlScriptTab {
    pub id: String,
    #[serde(rename = "databaseId")]
    pub database_id: String,
    pub name: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    #[serde(rename = "rowCount")]
    pub row_count: u64,
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<String>,
    #[serde(rename = "queryPlan", default, skip_serializing_if = "Option::is_none")]
    pub query_plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<KeyValuePair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub requests: Vec<ApiRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<KeyValuePair>,
    #[serde(rename = "folderPath", default, skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MutationOp {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowMutation {
    pub op: MutationOp,
    #[serde(default)]
    pub values: HashMap<String, serde_json::Value>,
    #[serde(default, rename = "where")]
    pub where_clause: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationItemResult {
    pub op: MutationOp,
    #[serde(rename = "rowsAffected")]
    pub rows_affected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMutationResult {
    pub atomic: bool,
    pub results: Vec<MutationItemResult>,
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
